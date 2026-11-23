// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Pako's terms of use: TERMS.md at the project's root, word for word the page at
//! sookera.com/terms, and TERMS.fr.md, word for word sookera.com/fr/terms (both copied from the
//! website by scripts/copy-terms.mjs). Replacing those files is all a new version takes: the
//! first-start step and Help › About Pako show the one in Pako's language, build.rs turns
//! TERMS.md into plain text for the installer, and the date both carry ("Last updated" /
//! « Dernière mise à jour ») is the version people accept: one version for both languages.
//!
//! build.rs uses this file too (`#[path]`), so it stays plain Rust with no crate imports.
// (The plain-text conversion is used by build.rs and the tests, not by Pako itself.)
#![cfg_attr(not(test), allow(dead_code))]

/// TERMS.md as it is.
pub const TEXT: &str = include_str!("../../TERMS.md");

/// TERMS.fr.md as it is (the window shows it in French; Rust only checks it).
pub const TEXT_FR: &str = include_str!("../../TERMS.fr.md");

/// The version of the terms this Pako carries ("2026-11-23"), saved in the settings when they're
/// accepted. `None` only if TERMS.md has no "Last updated <day> <month> <year>" line (build.rs
/// refuses to build then).
pub fn version() -> Option<String> {
    version_of(TEXT)
}

/// The date of a "Last updated 18 November 2026" line, or of the French « Dernière mise à jour :
/// 18 novembre 2026 » (a colon after either is fine), as "2026-11-18".
pub fn version_of(md: &str) -> Option<String> {
    const MONTHS: [&str; 12] =
        ["january", "february", "march", "april", "may", "june", "july", "august", "september", "october", "november", "december"];
    const MONTHS_FR: [&str; 12] =
        ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"];
    md.lines().find_map(|line| {
        let rest = line.trim().trim_matches(|c| c == '*' || c == '_').trim();
        let rest = rest.strip_prefix("Last updated").or_else(|| rest.strip_prefix("Dernière mise à jour"))?;
        // (trim also takes the French no-break spaces before the colon.)
        let rest = rest.trim_start().trim_start_matches(':').trim();
        let mut words = rest.split_whitespace();
        let day = words.next()?;
        let day: u32 = day.strip_suffix("er").unwrap_or(day).parse().ok()?;
        let name = words.next()?.to_lowercase();
        let month = MONTHS.iter().chain(MONTHS_FR.iter()).position(|m| *m == name)? % 12 + 1;
        let year: u32 = words.next()?.trim_end_matches('.').parse().ok()?;
        ((1..=31).contains(&day) && year >= 2000).then(|| format!("{year:04}-{month:02}-{day:02}"))
    })
}

/// The terms as plain text for Pako Setup's "Terms of use" page (NSIS shows .txt or .rtf only).
/// The words stay the same; only the Markdown marks go: headings lose their #, **bold** and
/// `code` become plain, a link becomes its text and its address in brackets (just the text when
/// the text already is the address), and an HTML comment is left out. Paragraphs are kept, one
/// line each (the installer wraps them), list items keep their "- ". Windows line ends.
pub fn plain_text(md: &str) -> String {
    let mut blocks: Vec<String> = Vec::new();
    let mut para: Vec<String> = Vec::new();
    let mut list: Vec<String> = Vec::new();
    let flush = |para: &mut Vec<String>, list: &mut Vec<String>, blocks: &mut Vec<String>| {
        if !para.is_empty() {
            blocks.push(para.join(" "));
            para.clear();
        }
        if !list.is_empty() {
            blocks.push(list.join("\r\n"));
            list.clear();
        }
    };
    let mut in_comment = false;
    for raw in md.lines() {
        let line = raw.trim_end().trim_start_matches('\u{feff}');
        let t = line.trim_start();
        if in_comment {
            in_comment = !t.contains("-->");
            continue;
        }
        if t.starts_with("<!--") {
            in_comment = !t.contains("-->");
            continue;
        }
        if t.is_empty() {
            flush(&mut para, &mut list, &mut blocks);
            continue;
        }
        let hashes = t.chars().take_while(|&c| c == '#').count();
        if (1..=6).contains(&hashes) && t[hashes..].starts_with(' ') {
            flush(&mut para, &mut list, &mut blocks);
            blocks.push(inline(t[hashes..].trim()));
            continue;
        }
        if let Some(item) = list_item(t) {
            if !para.is_empty() {
                flush(&mut para, &mut list, &mut blocks);
            }
            list.push(item);
            continue;
        }
        // An indented line right under a list item goes on with that item.
        if line.starts_with([' ', '\t']) && para.is_empty() {
            if let Some(last) = list.last_mut() {
                last.push(' ');
                last.push_str(&inline(t));
                continue;
            }
        }
        if !list.is_empty() {
            flush(&mut para, &mut list, &mut blocks);
        }
        para.push(inline(t));
    }
    flush(&mut para, &mut list, &mut blocks);
    let mut out = blocks.join("\r\n\r\n");
    out.push_str("\r\n");
    out
}

/// "- item", "* item" or "1. item", without its Markdown marks ("- " is kept, a number too).
fn list_item(t: &str) -> Option<String> {
    if let Some(rest) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")).or_else(|| t.strip_prefix("+ ")) {
        return Some(format!("- {}", inline(rest.trim_start())));
    }
    let digits = t.chars().take_while(char::is_ascii_digit).count();
    let rest = t[digits..].strip_prefix(". ").filter(|_| digits > 0)?;
    Some(format!("{}. {}", &t[..digits], inline(rest.trim_start())))
}

/// One line without its Markdown marks: **bold**, __bold__, `code`, \escapes and [links](…).
fn inline(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(c) = rest.chars().next() {
        if c == '\\' {
            if let Some(next) = rest[1..].chars().next() {
                out.push(next);
                rest = &rest[1 + next.len_utf8()..];
                continue;
            }
        }
        if rest.starts_with("**") || rest.starts_with("__") {
            rest = &rest[2..];
            continue;
        }
        if c == '`' {
            rest = &rest[1..];
            continue;
        }
        if c == '[' {
            if let Some((text, url, after)) = link(rest) {
                let text = inline(text);
                let bare = url.trim_start_matches("https://").trim_start_matches("http://").trim_start_matches("mailto:");
                out.push_str(&text);
                if bare.trim_end_matches('/') != text.trim_end_matches('/') {
                    out.push_str(&format!(" ({url})"));
                }
                rest = after;
                continue;
            }
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

/// "[text](url)…" → (text, url, what follows).
fn link(s: &str) -> Option<(&str, &str, &str)> {
    let close = s.find("](")?;
    let text = &s[1..close];
    if text.contains('[') {
        return None;
    }
    let after = &s[close + 2..];
    let end = after.find(')')?;
    let url = after[..end].trim();
    (!url.is_empty() && !url.contains(char::is_whitespace)).then_some((text, url, &after[end + 1..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TERMS.md comes with Pako (and goes into Pako Setup): it must be there, with words and a date.
    #[test]
    fn terms_file_is_there() {
        assert!(TEXT.trim().len() > 200, "TERMS.md is missing or empty");
        assert!(TEXT.contains("Terms of use"));
        assert!(version().is_some(), "TERMS.md has no 'Last updated <day> <month> <year>' line");
        assert!(!plain_text(TEXT).trim().is_empty());
    }

    /// TERMS.fr.md comes with Pako too, word for word sookera.com/fr/terms: the same version
    /// (date) as TERMS.md, the same sections, and nothing left of the website's own marks.
    #[test]
    fn french_terms_are_in_step() {
        assert!(TEXT_FR.trim().len() > 200, "TERMS.fr.md is missing or empty");
        assert!(TEXT_FR.contains("Conditions d’utilisation"));
        assert_eq!(version_of(TEXT_FR), version(), "TERMS.fr.md must carry the same date as TERMS.md");
        for (name, text) in [("TERMS.md", TEXT), ("TERMS.fr.md", TEXT_FR)] {
            assert!(
                !text.contains("{#") && !text.contains("<!--"),
                "{name} still has the website's marks: copy it with scripts/copy-terms.mjs"
            );
        }
        let heads = |s: &str| s.lines().filter(|l| l.starts_with('#')).count();
        assert_eq!(heads(TEXT), heads(TEXT_FR), "the same sections in both");
    }

    #[test]
    fn version_is_the_date() {
        assert_eq!(version_of("# Terms\n\nLast updated 18 November 2026\n").as_deref(), Some("2026-11-18"));
        assert_eq!(version_of("Last updated: 3 may 2027.").as_deref(), Some("2027-05-03"));
        assert_eq!(version_of("*Last updated 1 January 2030*").as_deref(), Some("2030-01-01"));
        assert_eq!(version_of("Dernière mise à jour : 23 novembre 2026").as_deref(), Some("2026-11-23"));
        assert_eq!(version_of("Dernière mise à jour\u{a0}: 1er août 2027").as_deref(), Some("2027-08-01"));
        assert_eq!(version_of("*Dernière mise à jour : 3 Décembre 2030*").as_deref(), Some("2030-12-03"));
        assert_eq!(version_of("Last updated soon"), None);
        assert_eq!(version_of("No date here"), None);
    }

    #[test]
    fn markdown_becomes_plain_text() {
        let md = "<!-- PLACEHOLDER: replace\n  with the final text -->\n# Terms of use\n\nLast updated 18 November 2026\n\nThese terms cover\nSookera's apps.\n\n## The short version\n\n- Our apps are **free software**.\n- Use `them` well,\n  every day.\n\n1. One\n2. Two\n\nWrite to [support@sookera.com](mailto:support@sookera.com) or see [sookera.com/terms](https://sookera.com/terms) and [the licence](https://www.gnu.org/licenses/gpl-3.0.html). A \\*star\\*.\n";
        let want = "Terms of use\r\n\r\nLast updated 18 November 2026\r\n\r\nThese terms cover Sookera's apps.\r\n\r\nThe short version\r\n\r\n- Our apps are free software.\r\n- Use them well, every day.\r\n\r\n1. One\r\n2. Two\r\n\r\nWrite to support@sookera.com or see sookera.com/terms and the licence (https://www.gnu.org/licenses/gpl-3.0.html). A *star*.\r\n";
        assert_eq!(plain_text(md), want);
        // The same from a file checked out with Windows line ends.
        assert_eq!(plain_text(&md.replace('\n', "\r\n")), want);
    }

    /// Only the marks go: every word of TERMS.md is in the installer's text, in the same order.
    #[test]
    fn no_words_change() {
        let words = |s: &str| -> Vec<String> {
            s.split(|c: char| !c.is_alphanumeric() && c != '\'')
                .filter(|w| !w.is_empty())
                .map(str::to_string)
                .collect()
        };
        // The Markdown source without its link addresses (the plain text shows an address only
        // when it differs from the link's words).
        let mut source = String::new();
        let mut rest = TEXT;
        while let Some(at) = rest.find("](") {
            source.push_str(&rest[..at]);
            rest = &rest[at..];
            rest = &rest[rest.find(')').map_or(rest.len(), |e| e + 1)..];
        }
        source.push_str(rest);
        let plain = plain_text(TEXT);
        let mut from = words(&plain).into_iter();
        for w in words(&source) {
            assert!(from.any(|p| p == w), "'{w}' from TERMS.md isn't in the installer's text");
        }
        assert!(!plain.contains("**") && !plain.contains("](") && !plain.contains("\n#"));
        assert!(plain.contains("\r\n") && !plain.replace("\r\n", "").contains('\n'));
    }
}
