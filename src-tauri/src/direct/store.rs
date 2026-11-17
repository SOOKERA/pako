// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Sookera. Pako by Sookera; see NOTICE.md for the additional terms.
//! Direct's own files, kept only on this PC: the contacts and the history of transfers.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Most history entries kept (oldest dropped first).
const MAX_HISTORY: usize = 2000;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    /// Their Pako ID (public key).
    pub id: String,
    /// The name you gave them.
    pub name: String,
    /// The first handshake went through (both sides said yes).
    #[serde(default)]
    pub trusted: bool,
    /// "Can send to me without asking".
    #[serde(default)]
    pub auto_accept: bool,
    /// Where their files go (None = Downloads\From <name>).
    #[serde(default)]
    pub save_folder: Option<String>,
    /// Last time their Pako was reachable (ms).
    #[serde(default)]
    pub last_seen: Option<u64>,
    #[serde(default)]
    pub added: u64,
    /// Your tags for them ("Friends", "Work"…). Only on this PC, never sent to them.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Shown under Favourites.
    #[serde(default)]
    pub favourite: bool,
    /// The name they chose for themselves (sent when you connect). Your own name for them
    /// (`name`) is never changed by it.
    #[serde(default)]
    pub their_name: Option<String>,
    /// They declined connecting (or removed you): Pako stops trying by itself, and only asks
    /// again when you press Connect.
    #[serde(default)]
    pub declined: bool,
    /// You said "this is my device" (with the safety code): shown in My devices, not in Direct's
    /// people.
    #[serde(default)]
    pub device: bool,
    /// Their Pako says you're one of its devices too (what it said last).
    #[serde(default)]
    pub their_device: bool,
    /// "This device can control my Pako": it may see your downloads and give orders. Only counts
    /// for a device paired both ways.
    #[serde(default)]
    pub control: bool,
    /// They let you control their Pako (what their Pako said last).
    #[serde(default)]
    pub their_control: bool,
    /// A device's name for its PC ("Desk PC"), as it said last. Your own name for it (`name`)
    /// follows it until you rename the device here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub their_pc_name: Option<String>,
}

/// Someone you declined or removed: until `until` (ms), they're turned away without asking you.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Blocked {
    pub id: String,
    pub until: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    /// The transfer's id.
    pub id: String,
    /// The contact's Pako ID.
    pub contact: String,
    /// You sent it (else you received it).
    pub outgoing: bool,
    pub name: String,
    pub files: u32,
    pub size: u64,
    /// When it ended (ms).
    pub date: u64,
    /// "local" (the home network), "direct6", "direct" or "relay" (how it travelled), if known.
    #[serde(default)]
    pub path: Option<String>,
    /// "done", "declined", "cancelled" or "failed".
    pub result: String,
    #[serde(default)]
    pub message: Option<String>,
    /// Where received files were saved (a file or a folder).
    #[serde(default)]
    pub saved_to: Option<String>,
    /// How long the bytes were moving (ms; 0 = never started).
    #[serde(default)]
    pub duration_ms: u64,
    /// Average speed while moving (bytes per second).
    #[serde(default)]
    pub avg_speed: u64,
    /// Best speed over about one second (bytes per second).
    #[serde(default)]
    pub peak_speed: u64,
}

/// Most device history lines kept (oldest dropped first).
const MAX_DEVICE_LOG: usize = 1000;

/// One line of a device's history: something one of your devices did on another.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceLog {
    /// The other device's Pako ID.
    pub device: String,
    /// ms
    pub date: u64,
    /// You did it from this PC on that device (else that device did it here).
    pub by_me: bool,
    /// What happened, in plain words ("Added 2 links", "Paused Movie.mkv").
    pub text: String,
    pub ok: bool,
}

/// Sends kept per device in My devices › Links (oldest dropped first).
pub const MAX_SENT: usize = 200;

/// One link of a send to one of your devices, and the downloads it became there.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SentLink {
    pub link: String,
    /// Its name as known here (a playlist's video, a file), if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    /// The downloads it became on that PC (its list's ids). Empty: an older Pako there (no
    /// live status), or it went to Links there.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ids: Vec<String>,
    /// That PC didn't take it (the row stayed here).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub failed: bool,
}

/// One send of links to one of your devices (a card in My devices › Links).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SentCard {
    pub id: String,
    pub device: String,
    /// ms
    pub date: u64,
    pub links: Vec<SentLink>,
    /// What that PC said ("Added 3 links to the downloads").
    pub text: String,
}

/// Keeps the newest `MAX_SENT` sends of each device.
pub fn trim_sent(list: &mut Vec<SentCard>) {
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut keep = vec![false; list.len()];
    for (i, c) in list.iter().enumerate().rev() {
        let n = seen.entry(c.device.clone()).or_default();
        *n += 1;
        keep[i] = *n <= MAX_SENT;
    }
    let mut i = 0;
    list.retain(|_| {
        i += 1;
        keep[i - 1]
    });
}

pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    fn contacts_path(&self) -> PathBuf {
        self.dir.join("contacts.json")
    }

    fn history_path(&self) -> PathBuf {
        self.dir.join("direct_history.json")
    }

    fn tags_path(&self) -> PathBuf {
        self.dir.join("direct_tags.json")
    }

    fn blocked_path(&self) -> PathBuf {
        self.dir.join("direct_blocked.json")
    }

    fn device_log_path(&self) -> PathBuf {
        self.dir.join("device_log.json")
    }

    /// What your devices did with each other (orders given and taken, permission changes).
    pub fn load_device_log(&self) -> Vec<DeviceLog> {
        read(&self.device_log_path())
    }

    pub fn save_device_log(&self, list: &mut Vec<DeviceLog>) {
        if list.len() > MAX_DEVICE_LOG {
            let extra = list.len() - MAX_DEVICE_LOG;
            list.drain(..extra);
        }
        write(&self.device_log_path(), list);
    }

    fn sent_path(&self) -> PathBuf {
        self.dir.join("device_sent.json")
    }

    /// Links sent to your devices (My devices › Links), oldest first.
    pub fn load_sent(&self) -> Vec<SentCard> {
        read(&self.sent_path())
    }

    pub fn save_sent(&self, list: &mut Vec<SentCard>) {
        trim_sent(list);
        write(&self.sent_path(), list);
    }

    fn parts_path(&self) -> PathBuf {
        self.dir.join("direct_parts.json")
    }

    /// People turned away without asking, still in effect at `now` (ms).
    pub fn load_blocked(&self, now: u64) -> Vec<Blocked> {
        let mut list: Vec<Blocked> = read(&self.blocked_path());
        list.retain(|b| b.until > now);
        list
    }

    pub fn save_blocked(&self, list: &[Blocked]) {
        write(&self.blocked_path(), list);
    }

    /// Unfinished ".pakorecv" files (".pakopart" up to Pako 0.1.2) of transfers that were
    /// running (to clean up after a crash).
    pub fn load_parts(&self) -> Vec<String> {
        read(&self.parts_path())
    }

    pub fn save_parts(&self, list: &[String]) {
        write(&self.parts_path(), list);
    }

    /// Your tags, in your order; tags found on contacts but missing from the file are added.
    pub fn load_tags(&self, contacts: &[Contact]) -> Vec<String> {
        let mut tags: Vec<String> = read(&self.tags_path());
        for c in contacts {
            for t in &c.tags {
                if find_tag(&tags, t).is_none() {
                    tags.push(t.clone());
                }
            }
        }
        tags
    }

    pub fn save_tags(&self, tags: &[String]) {
        write(&self.tags_path(), tags);
    }

    pub fn load_contacts(&self) -> Vec<Contact> {
        read(&self.contacts_path())
    }

    pub fn save_contacts(&self, list: &[Contact]) {
        write(&self.contacts_path(), list);
    }

    pub fn load_history(&self) -> Vec<HistoryItem> {
        read(&self.history_path())
    }

    pub fn save_history(&self, list: &mut Vec<HistoryItem>) {
        if list.len() > MAX_HISTORY {
            let extra = list.len() - MAX_HISTORY;
            list.drain(..extra);
        }
        write(&self.history_path(), list);
    }
}

// ---------- tags (pure, so they can be tested) ----------

/// Longest tag name kept.
const MAX_TAG: usize = 30;
/// Most tags kept.
const MAX_TAGS: usize = 100;

/// A tag name as typed: trimmed, inner spaces collapsed, at most 30 characters. None = empty.
pub fn clean_tag(name: &str) -> Option<String> {
    let s: String = name.split_whitespace().collect::<Vec<_>>().join(" ").chars().filter(|c| !c.is_control()).take(MAX_TAG).collect();
    (!s.is_empty()).then_some(s)
}

/// The existing tag with this name, ignoring case.
pub fn find_tag(tags: &[String], name: &str) -> Option<usize> {
    tags.iter().position(|t| t.eq_ignore_ascii_case(name) || t.to_lowercase() == name.to_lowercase())
}

/// Adds a tag (or returns the existing one with the same name).
pub fn add_tag(tags: &mut Vec<String>, name: &str) -> Result<String, String> {
    let name = clean_tag(name).ok_or("Give the tag a name.")?;
    if let Some(i) = find_tag(tags, &name) {
        return Ok(tags[i].clone());
    }
    if tags.len() >= MAX_TAGS {
        return Err(format!("You can have up to {MAX_TAGS} tags."));
    }
    tags.push(name.clone());
    Ok(name)
}

/// Sets a contact's tags (any new name is added to the list, in the list's spelling).
pub fn set_contact_tags(tags: &mut Vec<String>, contact: &mut Contact, wanted: &[String]) -> Result<(), String> {
    let mut out: Vec<String> = Vec::new();
    for w in wanted {
        let t = add_tag(tags, w)?;
        if find_tag(&out, &t).is_none() {
            out.push(t);
        }
    }
    contact.tags = out;
    Ok(())
}

/// Renames a tag everywhere; renaming onto an existing tag merges the two.
pub fn rename_tag(tags: &mut Vec<String>, contacts: &mut [Contact], old: &str, new: &str) -> Result<(), String> {
    let i = find_tag(tags, old).ok_or("That tag doesn't exist any more.")?;
    let new = clean_tag(new).ok_or("Give the tag a name.")?;
    let from = tags[i].clone();
    let into = match find_tag(tags, &new) {
        Some(j) if j != i => {
            tags.remove(i);
            tags[if j > i { j - 1 } else { j }].clone()
        }
        _ => {
            tags[i] = new.clone();
            new
        }
    };
    for c in contacts.iter_mut() {
        let had = c.tags.iter().any(|t| t.eq_ignore_ascii_case(&from));
        c.tags.retain(|t| !t.eq_ignore_ascii_case(&from));
        if had && find_tag(&c.tags, &into).is_none() {
            c.tags.push(into.clone());
        }
    }
    Ok(())
}

/// Deletes a tag from the list and from every contact (the contacts stay).
pub fn delete_tag(tags: &mut Vec<String>, contacts: &mut [Contact], name: &str) {
    if let Some(i) = find_tag(tags, name) {
        let gone = tags.remove(i);
        for c in contacts.iter_mut() {
            c.tags.retain(|t| !t.eq_ignore_ascii_case(&gone));
        }
    }
}

/// A list file; an unreadable one is set aside (never overwritten) and the list starts empty.
fn read<T: for<'de> Deserialize<'de>>(path: &PathBuf) -> Vec<T> {
    // Briefly busy (a backup or a virus scan has it open): tried again a moment later, so the
    // list isn't taken as empty and then saved over.
    let mut tries = 0;
    let bytes = loop {
        match std::fs::read(path) {
            Ok(b) => break b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
            Err(_) if tries < 4 => {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
            Err(_) => {
                // Still unreadable: kept aside (never overwritten) when that works.
                let mut aside = path.clone();
                aside.set_extension(format!("unreadable-{}.json", crate::model::now_secs()));
                let _ = std::fs::rename(path, aside);
                return Vec::new();
            }
        }
    };
    match serde_json::from_slice::<Vec<T>>(&bytes) {
        Ok(v) => v,
        Err(_) => {
            let mut aside = path.clone();
            aside.set_extension(format!("unreadable-{}.json", crate::model::now_secs()));
            let _ = std::fs::rename(path, aside);
            Vec::new()
        }
    }
}

fn write<T: Serialize>(path: &PathBuf, list: &[T]) {
    if let Ok(bytes) = serde_json::to_vec_pretty(list) {
        let _ = crate::paths::write_atomic(path, &bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person(name: &str, tags: &[&str]) -> Contact {
        Contact {
            id: name.to_lowercase(),
            name: name.into(),
            trusted: true,
            auto_accept: false,
            save_folder: None,
            last_seen: None,
            added: 0,
            tags: tags.iter().map(|t| t.to_string()).collect(),
            favourite: false,
            their_name: None,
            declined: false,
            ..Default::default()
        }
    }

    #[test]
    fn tags_are_cleaned_and_not_doubled() {
        let mut tags = vec!["Friends".to_string()];
        assert_eq!(add_tag(&mut tags, "  friends ").unwrap(), "Friends");
        assert_eq!(add_tag(&mut tags, " Work   team ").unwrap(), "Work team");
        assert!(add_tag(&mut tags, "   ").is_err());
        assert_eq!(clean_tag(&"x".repeat(80)).unwrap().len(), MAX_TAG);
        assert_eq!(tags, vec!["Friends", "Work team"]);
        let mut c = person("Rado", &[]);
        set_contact_tags(&mut tags, &mut c, &["work TEAM".into(), "Family".into(), "family".into()]).unwrap();
        assert_eq!(c.tags, vec!["Work team", "Family"]);
        assert_eq!(tags, vec!["Friends", "Work team", "Family"]);
    }

    #[test]
    fn renaming_and_deleting_tags_reach_every_contact() {
        let mut tags = vec!["Friends".to_string(), "Work".to_string(), "Clients".to_string()];
        let mut people = vec![person("Rado", &["Friends", "Work"]), person("Mia", &["Clients"])];
        rename_tag(&mut tags, &mut people, "work", "Job").unwrap();
        assert_eq!(tags, vec!["Friends", "Job", "Clients"]);
        assert_eq!(people[0].tags, vec!["Friends", "Job"]);
        // Renaming onto an existing tag merges them.
        rename_tag(&mut tags, &mut people, "Clients", "job").unwrap();
        assert_eq!(tags, vec!["Friends", "Job"]);
        assert_eq!(people[1].tags, vec!["Job"]);
        delete_tag(&mut tags, &mut people, "JOB");
        assert_eq!(tags, vec!["Friends"]);
        assert_eq!(people[0].tags, vec!["Friends"]);
        assert!(people[1].tags.is_empty());
    }

    /// The sends to each device are kept, the newest 200 per device, and read back as saved.
    #[test]
    fn sent_links_are_kept_per_device_and_bounded() {
        let dir = std::env::temp_dir().join(format!("pako-sent-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::new(dir.clone());
        assert!(store.load_sent().is_empty(), "nothing yet");
        let card = |device: &str, n: u64| SentCard {
            id: format!("{device}-{n}"),
            device: device.into(),
            date: n,
            links: vec![SentLink { link: "https://a.com/x.zip".into(), ids: vec![format!("d{n}")], ..Default::default() }],
            text: "Added 1 link to the downloads".into(),
        };
        let mut list: Vec<SentCard> = (0..(MAX_SENT as u64 + 10)).map(|n| card("home", n)).collect();
        list.push(card("laptop", 1));
        store.save_sent(&mut list);
        let back = store.load_sent();
        assert_eq!(back.iter().filter(|c| c.device == "home").count(), MAX_SENT);
        assert_eq!(back.iter().find(|c| c.device == "home").unwrap().date, 10, "the oldest went first");
        assert_eq!(back.last().unwrap(), &card("laptop", 1));
        // Written compactly: what isn't said is left out, and reads back the same.
        let json = serde_json::to_string(&back[0].links[0]).unwrap();
        assert_eq!(json, r#"{"link":"https://a.com/x.zip","ids":["d10"]}"#);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_older_history_file_has_no_speeds() {
        let old: Vec<HistoryItem> = serde_json::from_str(
            r#"[{"id":"t1","contact":"a","outgoing":true,"name":"x","files":1,"size":10,"date":5,"result":"done"}]"#,
        )
        .unwrap();
        assert_eq!((old[0].duration_ms, old[0].avg_speed, old[0].peak_speed), (0, 0, 0));
        assert!(old[0].path.is_none());
    }

    #[test]
    fn an_older_contacts_file_has_no_tags_and_no_favourite() {
        let old: Vec<Contact> = serde_json::from_str(r#"[{"id":"a","name":"Rado","trusted":true}]"#).unwrap();
        assert!(old[0].tags.is_empty() && !old[0].favourite && old[0].their_name.is_none());
    }

    #[test]
    fn contacts_and_history_round_trip() {
        let dir = std::env::temp_dir().join(format!("pako-direct-store-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = Store::new(dir.clone());
        assert!(store.load_contacts().is_empty());
        let c = Contact {
            id: "abc".into(),
            name: "Rado".into(),
            trusted: true,
            auto_accept: false,
            save_folder: None,
            last_seen: Some(5),
            added: 1,
            tags: vec!["Friends".into()],
            favourite: true,
            their_name: Some("Rado R.".into()),
            declined: true,
            device: true,
            their_device: true,
            control: true,
            their_control: false,
            their_pc_name: Some("Desk PC".into()),
        };
        store.save_contacts(&[c.clone()]);
        assert_eq!(store.load_contacts(), vec![c]);
        // An older file without the newer fields still reads.
        std::fs::write(dir.join("contacts.json"), r#"[{"id":"x","name":"Old"}]"#).unwrap();
        let old = store.load_contacts();
        assert_eq!(old[0].name, "Old");
        assert!(!old[0].trusted && !old[0].auto_accept);
        // A damaged file is set aside, not lost.
        std::fs::write(dir.join("contacts.json"), b"{not json").unwrap();
        assert!(store.load_contacts().is_empty());
        assert!(std::fs::read_dir(&dir).unwrap().any(|e| e.unwrap().file_name().to_string_lossy().contains("unreadable")));
        let mut h: Vec<HistoryItem> = (0..(MAX_HISTORY + 5))
            .map(|i| HistoryItem {
                id: i.to_string(),
                contact: "abc".into(),
                outgoing: i % 2 == 0,
                name: "x".into(),
                files: 1,
                size: 1,
                date: i as u64,
                path: None,
                result: "done".into(),
                message: None,
                saved_to: None,
                duration_ms: 0,
                avg_speed: 0,
                peak_speed: 0,
            })
            .collect();
        store.save_history(&mut h);
        let back = store.load_history();
        assert_eq!(back.len(), MAX_HISTORY);
        assert_eq!(back[0].id, "5", "the oldest go first");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
