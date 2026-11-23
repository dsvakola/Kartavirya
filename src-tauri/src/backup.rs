//! Export and import of the user's data.
//!
//! Import never overwrites anything: reminders whose id already exists are
//! skipped, and a day that already has a daily note keeps its own note.
//! The caller shows the preview first and the user confirms.

use crate::schedule::{KIND_NOTE, KIND_REMINDER, REMIND_TYPES, Reminder, parse_time};
use crate::settings::{Category, ICONS};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

pub const APP_NAME: &str = "Kartavirya";
pub const FORMAT: u32 = 1;
pub const MAX_FILE_BYTES: u64 = 20 * 1024 * 1024;
pub const MAX_NOTE_CHARS: usize = 20_000;
pub const MAX_TEXT_CHARS: usize = 5_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFile {
    pub app: String,
    pub format: u32,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub exported_at: String,
    #[serde(default)]
    pub reminders: Vec<Reminder>,
    #[serde(default)]
    pub daily_notes: BTreeMap<String, String>,
    #[serde(default)]
    pub categories: Vec<Category>,
}

#[derive(Debug, Default)]
pub struct ImportPlan {
    pub add: Vec<Reminder>,
    pub existing: usize,
    pub invalid: usize,
    pub add_notes: BTreeMap<String, String>,
    pub kept_notes: usize,
    pub add_categories: Vec<Category>,
    pub file_reminders: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub file_reminders: usize,
    pub new_reminders: usize,
    pub existing_reminders: usize,
    pub invalid_reminders: usize,
    pub new_notes: usize,
    pub kept_notes: usize,
    pub new_categories: usize,
}

impl ImportPlan {
    pub fn preview(&self) -> ImportPreview {
        ImportPreview {
            file_reminders: self.file_reminders,
            new_reminders: self.add.len(),
            existing_reminders: self.existing,
            invalid_reminders: self.invalid,
            new_notes: self.add_notes.len(),
            kept_notes: self.kept_notes,
            new_categories: self.add_categories.len(),
        }
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.add.is_empty() && self.add_notes.is_empty() && self.add_categories.is_empty()
    }
}

pub fn parse_export(data: &str) -> Result<ExportFile, String> {
    let file: ExportFile = serde_json::from_str(data)
        .map_err(|_| "This is not a Kartavirya backup file.".to_string())?;
    if file.app != APP_NAME {
        return Err("This is not a Kartavirya backup file.".into());
    }
    if file.format > FORMAT {
        return Err("This backup was made by a newer version of Kartavirya.".into());
    }
    Ok(file)
}

fn valid_reminder(r: &Reminder) -> bool {
    !r.id.trim().is_empty()
        && r.id.len() <= 64
        && !r.text.trim().is_empty()
        && r.text.chars().count() <= MAX_TEXT_CHARS
        && REMIND_TYPES.contains(&r.remind_type.as_str())
        && (r.kind == KIND_REMINDER || r.kind == KIND_NOTE)
        && parse_time(&r.created_at).is_some()
        && parse_time(&r.remind_at).is_some()
        && r.category.as_ref().is_none_or(|c| c.len() <= 64)
        && r.completed_at
            .as_deref()
            .is_none_or(|t| parse_time(t).is_some())
        && r.deleted_at
            .as_deref()
            .is_none_or(|t| parse_time(t).is_some())
}

fn valid_category(c: &Category) -> bool {
    let hex = c.color.len() == 7
        && c.color.starts_with('#')
        && c.color[1..].chars().all(|ch| ch.is_ascii_hexdigit());
    !c.id.trim().is_empty()
        && c.id.len() <= 64
        && !c.name.trim().is_empty()
        && c.name.chars().count() <= 30
        && hex
        && ICONS.contains(&c.icon.as_str())
}

pub fn valid_day(key: &str) -> bool {
    key.len() == 10 && NaiveDate::parse_from_str(key, "%Y-%m-%d").is_ok()
}

pub fn plan_import(
    existing: &[Reminder],
    notes: &BTreeMap<String, String>,
    categories: &[Category],
    file: ExportFile,
) -> ImportPlan {
    let mut plan = ImportPlan {
        file_reminders: file.reminders.len(),
        ..ImportPlan::default()
    };
    let mut seen: HashSet<&str> = existing.iter().map(|r| r.id.as_str()).collect();
    let mut accepted: Vec<Reminder> = Vec::new();
    let mut ids_in_file: HashSet<String> = HashSet::new();
    for mut r in file.reminders {
        if !valid_reminder(&r) {
            plan.invalid += 1;
            continue;
        }
        if seen.contains(r.id.as_str()) || !ids_in_file.insert(r.id.clone()) {
            plan.existing += 1;
            continue;
        }
        r.text = r.text.trim().to_string();
        accepted.push(r);
    }
    seen.clear();
    plan.add = accepted;

    for (day, text) in file.daily_notes {
        if !valid_day(&day) || text.trim().is_empty() || text.chars().count() > MAX_NOTE_CHARS {
            continue;
        }
        if notes.contains_key(&day) {
            if notes.get(&day).map(String::as_str) != Some(text.as_str()) {
                plan.kept_notes += 1;
            }
        } else {
            plan.add_notes.insert(day, text);
        }
    }

    let have: HashSet<&str> = categories.iter().map(|c| c.id.as_str()).collect();
    let names: HashSet<String> = categories.iter().map(|c| c.name.to_lowercase()).collect();
    let mut new_names = HashSet::new();
    for c in file.categories {
        if !valid_category(&c)
            || have.contains(c.id.as_str())
            || names.contains(&c.name.to_lowercase())
            || !new_names.insert(c.name.to_lowercase())
        {
            continue;
        }
        if categories.len() + plan.add_categories.len() >= 30 {
            break;
        }
        plan.add_categories.push(c);
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::default_categories;

    fn file(reminders: Vec<Reminder>) -> ExportFile {
        ExportFile {
            app: APP_NAME.into(),
            format: FORMAT,
            version: "1.2.0".into(),
            exported_at: "2026-11-23T10:00:00+05:30".into(),
            reminders,
            daily_notes: BTreeMap::new(),
            categories: Vec::new(),
        }
    }

    #[test]
    fn export_round_trips_through_json() {
        let mut f = file(vec![Reminder::new_for_test("keys")]);
        f.daily_notes.insert("2026-11-23".into(), "note".into());
        let json = serde_json::to_string(&f).unwrap();
        let back = parse_export(&json).unwrap();
        assert_eq!(back.reminders.len(), 1);
        assert_eq!(back.daily_notes.len(), 1);
    }

    #[test]
    fn rejects_other_files() {
        assert!(parse_export("[]").is_err());
        assert!(parse_export(r#"{"app":"Other","format":1}"#).is_err());
        assert!(parse_export(r#"{"app":"Kartavirya","format":99}"#).is_err());
        assert!(parse_export("garbage").is_err());
    }

    #[test]
    fn import_adds_only_new_items() {
        let a = Reminder::new_for_test("a");
        let b = Reminder::new_for_test("b");
        let existing = vec![a.clone()];
        let plan = plan_import(
            &existing,
            &BTreeMap::new(),
            &default_categories(),
            file(vec![a, b.clone(), b]),
        );
        assert_eq!(plan.add.len(), 1);
        assert_eq!(plan.existing, 2); // a already there, second b duplicated in the file
        assert_eq!(plan.invalid, 0);
    }

    #[test]
    fn import_ignores_invalid_items() {
        let mut bad_type = Reminder::new_for_test("x");
        bad_type.remind_type = "yearly".into();
        let mut empty = Reminder::new_for_test("  ");
        empty.id = "e".into();
        let mut bad_time = Reminder::new_for_test("t");
        bad_time.remind_at = "tomorrow".into();
        let plan = plan_import(
            &[],
            &BTreeMap::new(),
            &[],
            file(vec![bad_type, empty, bad_time]),
        );
        assert_eq!(plan.invalid, 3);
        assert!(plan.is_empty());
    }

    #[test]
    fn existing_daily_note_is_never_overwritten() {
        let mut have = BTreeMap::new();
        have.insert("2026-11-23".to_string(), "mine".to_string());
        let mut f = file(vec![]);
        f.daily_notes.insert("2026-11-23".into(), "theirs".into());
        f.daily_notes.insert("2026-11-24".into(), "new".into());
        f.daily_notes.insert("not-a-date".into(), "x".into());
        let plan = plan_import(&[], &have, &[], f);
        assert_eq!(plan.add_notes.len(), 1);
        assert!(plan.add_notes.contains_key("2026-11-24"));
        assert_eq!(plan.kept_notes, 1);
    }

    #[test]
    fn categories_are_added_when_new_and_valid() {
        let mut f = file(vec![]);
        f.categories = vec![
            Category {
                id: "gym".into(),
                name: "Gym".into(),
                color: "#112233".into(),
                icon: "heart".into(),
            },
            Category {
                id: "work".into(),
                name: "Work again".into(),
                color: "#112233".into(),
                icon: "heart".into(),
            },
            Category {
                id: "bad".into(),
                name: "Bad".into(),
                color: "red".into(),
                icon: "heart".into(),
            },
        ];
        let plan = plan_import(&[], &BTreeMap::new(), &default_categories(), f);
        assert_eq!(plan.add_categories.len(), 1);
        assert_eq!(plan.add_categories[0].id, "gym");
    }
}
