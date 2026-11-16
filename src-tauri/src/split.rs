// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Split archives: a big archive cut into parts (name.part1.rar, name.7z.001, name.zip +
//! name.z01, name.rar + name.r00). Pako keeps the parts of one set together in a folder
//! named after the archive, says when a part is missing, and can extract the set once
//! every part is downloaded (with 7-Zip).

use std::collections::BTreeMap;

/// One part of a set, read from its file name.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    /// The archive's name without the part numbering ("Project.part3.rar" -> "Project").
    pub base: String,
    /// Position in the set (see `Family`); used to spot missing parts.
    pub index: u32,
    pub family: Family,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Family {
    /// name.part1.rar, name.part2.rar ...
    PartRar,
    /// name.7z.001, name.zip.001, name.001 ...
    Numbered,
    /// name.z01, name.z02 ... and name.zip (the last part)
    Zip,
    /// name.rar (first) and name.r00, name.r01 ...
    OldRar,
}

/// Index given to "name.zip" in a .z01 set: it is the last part.
const ZIP_LAST: u32 = u32::MAX;

fn digits(s: &str) -> Option<u32> {
    (!s.is_empty() && s.len() <= 5 && s.chars().all(|c| c.is_ascii_digit())).then(|| s.parse().ok()).flatten()
}

/// Reads a file name as a possible part of a split archive.
pub fn part(name: &str) -> Option<Part> {
    // to_ascii_lowercase keeps byte positions, so slices of `lower` fit `name` too.
    let lower = name.to_ascii_lowercase();
    let dot = lower.rfind('.')?;
    let ext = &lower[dot + 1..];
    let stem = &lower[..dot];
    if ext == "rar" {
        if let Some(p) = stem.rfind(".part") {
            if let Some(n) = digits(&stem[p + 5..]) {
                return Some(Part { base: name[..p].to_string(), index: n, family: Family::PartRar });
            }
        }
        return Some(Part { base: name[..dot].to_string(), index: 0, family: Family::OldRar });
    }
    if ext == "zip" {
        return Some(Part { base: name[..dot].to_string(), index: ZIP_LAST, family: Family::Zip });
    }
    if ext.len() >= 3 {
        if let Some(n) = digits(ext) {
            let mut base = &name[..dot];
            for kind in [".7z", ".zip", ".rar"] {
                if stem.ends_with(kind) {
                    base = &name[..dot - kind.len()];
                }
            }
            return Some(Part { base: base.to_string(), index: n, family: Family::Numbered });
        }
        // strip_prefix, not split_at(1): the extension may start with a non-ASCII letter
        // ("J.R.R.トールキン", "Dr.Éric"), and cutting inside it would crash Pako.
        if let Some(n) = ext.strip_prefix('z').and_then(digits) {
            return Some(Part { base: name[..dot].to_string(), index: n, family: Family::Zip });
        }
        if let Some(n) = ext.strip_prefix('r').and_then(digits) {
            return Some(Part { base: name[..dot].to_string(), index: n + 1, family: Family::OldRar });
        }
    }
    None
}

/// Which set a file name belongs to (same archive name and same naming style), if any.
pub fn set_key(name: &str) -> Option<String> {
    part(name).map(|p| format!("{:?}|{}", p.family, p.base.to_lowercase()))
}

/// A set as the window gets it: which items are its parts, and what's missing.
#[derive(serde::Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SetView {
    pub key: String,
    pub base: String,
    pub ids: Vec<String>,
    pub missing: Vec<String>,
}

/// A set found among some file names.
#[derive(Debug, Clone)]
pub struct SplitSet {
    /// Same as `set_key` of its parts.
    pub key: String,
    pub base: String,
    /// Positions (in the list given) of the files of this set.
    pub members: Vec<usize>,
    /// The file 7-Zip opens to extract the whole set.
    pub first: usize,
    /// Plain words about missing parts ("part 3 missing"); empty when complete as far as we can tell.
    pub missing: Vec<String>,
}

/// Groups file names into split sets. A lone "name.rar" or "name.zip" is not a set;
/// a set needs at least two parts.
pub fn find_sets(names: &[&str]) -> Vec<SplitSet> {
    let mut groups: BTreeMap<(Family, String), Vec<(usize, u32)>> = BTreeMap::new();
    for (i, n) in names.iter().enumerate() {
        if let Some(p) = part(n) {
            groups.entry((p.family, p.base.to_lowercase())).or_default().push((i, p.index));
        }
    }
    let mut out = Vec::new();
    for ((family, base_lower), mut list) in groups {
        list.sort_by_key(|&(_, idx)| idx);
        // The plain .rar/.zip only counts when numbered parts sit next to it.
        let numbered = match family {
            Family::OldRar => list.iter().filter(|(_, x)| *x > 0).count(),
            Family::Zip => list.iter().filter(|(_, x)| *x != ZIP_LAST).count(),
            _ => list.len(),
        };
        if list.len() < 2 || numbered == 0 {
            continue;
        }
        let base = part(names[list[0].0]).map(|p| p.base).unwrap_or_default();
        let mut missing = Vec::new();
        let have: Vec<u32> = list.iter().map(|&(_, x)| x).filter(|&x| x != ZIP_LAST).collect();
        let (start, label): (u32, fn(u32) -> String) = match family {
            Family::PartRar => (1, |n| format!("part {n} missing")),
            Family::Numbered => (1, |n| format!("part {n:03} missing")),
            Family::Zip => (1, |n| format!(".z{n:02} missing")),
            Family::OldRar => (0, |n| if n == 0 { ".rar missing".into() } else { format!(".r{:02} missing", n - 1) }),
        };
        if let Some(&max) = have.iter().max() {
            for n in start..=max {
                if !have.contains(&n) {
                    missing.push(label(n));
                }
            }
        }
        if family == Family::Zip && !list.iter().any(|&(_, x)| x == ZIP_LAST) {
            missing.push("the .zip part (the last one) missing".into());
        }
        let first = match family {
            Family::Zip => list.iter().find(|&&(_, x)| x == ZIP_LAST).map(|&(i, _)| i),
            _ => list.first().map(|&(i, _)| i),
        }
        .unwrap_or(list[0].0);
        let key = format!("{family:?}|{base_lower}");
        out.push(SplitSet { key, base, members: list.iter().map(|&(i, _)| i).collect(), first, missing });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_part_names() {
        assert_eq!(part("Project.part03.rar").unwrap().index, 3);
        assert_eq!(part("Project.part03.rar").unwrap().base, "Project");
        assert_eq!(part("Film.7z.002").unwrap().base, "Film");
        assert_eq!(part("Film.7z.002").unwrap().family, Family::Numbered);
        assert_eq!(part("Backup.z01").unwrap().family, Family::Zip);
        assert_eq!(part("Old.r00").unwrap().index, 1);
        assert!(part("photo.jpg").is_none());
    }

    #[test]
    fn non_ascii_names_dont_crash() {
        for name in ["J.R.R.トールキン", "Dr.Éric", "Dr.Éric.mkv", "a.ééé", "a.zé1", "a.ré01", "a.€", "a.é", "", ".", "a.", "トールキン.part1.rar"] {
            let _ = part(name);
            let _ = set_key(name);
        }
        assert!(part("J.R.R.トールキン").is_none());
        assert!(part("Dr.Éric").is_none());
        assert_eq!(part("トールキン.part1.rar").unwrap().base, "トールキン");
        assert_eq!(part("Éric.z02").unwrap().family, Family::Zip);
        let names = ["J.R.R.トールキン", "Dr.Éric", "X.part1.rar", "X.part2.rar"];
        assert_eq!(find_sets(&names).len(), 1);
    }

    #[test]
    fn groups_sets_and_spots_missing_parts() {
        let names = ["A.part1.rar", "A.part2.rar", "A.part4.rar", "B.7z.001", "B.7z.002", "notes.txt", "C.zip", "D.zip", "D.z01"];
        let sets = find_sets(&names);
        let a = sets.iter().find(|s| s.base == "A").unwrap();
        assert_eq!(a.members.len(), 3);
        assert_eq!(a.missing, vec!["part 3 missing"]);
        assert_eq!(names[a.first], "A.part1.rar");
        let b = sets.iter().find(|s| s.base == "B").unwrap();
        assert!(b.missing.is_empty());
        let d = sets.iter().find(|s| s.base == "D").unwrap();
        assert_eq!(names[d.first], "D.zip");
        // A lone .zip is just a zip.
        assert!(sets.iter().all(|s| s.base != "C"));
    }

    #[test]
    fn a_later_part_finds_its_set() {
        let sets = find_sets(&["X.part1.rar", "X.part2.rar"]);
        assert_eq!(set_key("x.PART3.rar").as_deref(), Some(sets[0].key.as_str()));
        assert_ne!(set_key("Y.part3.rar"), Some(sets[0].key.clone()));
    }
}
