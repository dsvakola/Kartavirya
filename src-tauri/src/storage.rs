//! Safe storage for reminders.json
//!
//! - Saves are atomic: write a temporary file, flush it to disk, then rename.
//! - The previous good file is kept as reminders.bak.json.
//! - A damaged file is never overwritten: it is moved aside to
//!   reminders.corrupt-<date-time>.json and the backup is loaded instead.

use crate::schedule::Reminder;
use chrono::Local;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct LoadOutcome {
    pub list: Vec<Reminder>,
    /// A message for the user when something had to be recovered.
    pub warning: Option<String>,
}

fn sibling(path: &Path, name: &str) -> PathBuf {
    path.with_file_name(name)
}

pub fn backup_path(path: &Path) -> PathBuf {
    sibling(path, "reminders.bak.json")
}

fn temp_path(path: &Path) -> PathBuf {
    sibling(path, "reminders.json.tmp")
}

fn read_list(path: &Path) -> Result<Vec<Reminder>, String> {
    let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
    if data.trim().is_empty() {
        return Err("the file is empty".into());
    }
    serde_json::from_str::<Vec<Reminder>>(&data).map_err(|e| e.to_string())
}

/// Moves a damaged file aside so it is never overwritten. Returns the new path.
fn quarantine(path: &Path) -> Option<PathBuf> {
    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    let target = sibling(path, &format!("reminders.corrupt-{stamp}.json"));
    fs::rename(path, &target).ok().map(|_| target)
}

pub fn load(path: &Path) -> LoadOutcome {
    let bak = backup_path(path);

    if path.exists() {
        match read_list(path) {
            Ok(list) => {
                return LoadOutcome {
                    list,
                    warning: None,
                };
            }
            Err(err) => {
                let moved = quarantine(path);
                let kept = match &moved {
                    Some(p) => format!("It was kept safely as {}.", p.display()),
                    None => format!(
                        "It could not be moved, so it was left untouched at {}.",
                        path.display()
                    ),
                };
                if let Ok(list) = read_list(&bak) {
                    return LoadOutcome {
                        list,
                        warning: Some(format!(
                            "The reminders file was damaged ({err}). Your reminders were restored from the last backup. {kept}"
                        )),
                    };
                }
                return LoadOutcome {
                    list: Vec::new(),
                    warning: Some(format!(
                        "The reminders file was damaged ({err}) and no backup could be read. {kept}"
                    )),
                };
            }
        }
    }

    // No main file: recover from the backup if one exists (for example after
    // a crash between the two renames of a save).
    if bak.exists()
        && let Ok(list) = read_list(&bak)
    {
        return LoadOutcome {
            list,
            warning: Some("Your reminders were restored from the last backup.".into()),
        };
    }
    LoadOutcome {
        list: Vec::new(),
        warning: None,
    }
}

pub fn save(path: &Path, list: &[Reminder]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Could not create data folder: {e}"))?;
    }
    let json =
        serde_json::to_string_pretty(list).map_err(|e| format!("Could not save reminders: {e}"))?;

    let tmp = temp_path(path);
    {
        let mut f = fs::File::create(&tmp).map_err(|e| format!("Could not save reminders: {e}"))?;
        f.write_all(json.as_bytes())
            .and_then(|_| f.sync_all())
            .map_err(|e| format!("Could not save reminders: {e}"))?;
    }

    // Keep the previous version as the backup, then move the new file in place.
    if path.exists() {
        let _ = fs::rename(path, backup_path(path));
    }
    fs::rename(&tmp, path).map_err(|e| format!("Could not save reminders: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!("kartavirya-test-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }
        fn file(&self) -> PathBuf {
            self.0.join("reminders.json")
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn sample(text: &str) -> Reminder {
        Reminder {
            id: uuid::Uuid::new_v4().to_string(),
            text: text.into(),
            created_at: "2026-11-23T10:00:00+05:30".into(),
            remind_type: "custom".into(),
            remind_at: "2026-11-23T11:00:00+05:30".into(),
            is_completed: false,
            snooze_count: 0,
            last_alerted: None,
            base_at: None,
            deleted_at: None,
        }
    }

    #[test]
    fn save_then_load_round_trip() {
        let dir = TempDir::new();
        save(&dir.file(), &[sample("a"), sample("b")]).unwrap();
        let out = load(&dir.file());
        assert_eq!(out.list.len(), 2);
        assert!(out.warning.is_none());
        assert!(!temp_path(&dir.file()).exists());
    }

    #[test]
    fn second_save_keeps_previous_as_backup() {
        let dir = TempDir::new();
        save(&dir.file(), &[sample("first")]).unwrap();
        save(&dir.file(), &[sample("first"), sample("second")]).unwrap();
        let bak = read_list(&backup_path(&dir.file())).unwrap();
        assert_eq!(bak.len(), 1);
        assert_eq!(load(&dir.file()).list.len(), 2);
    }

    #[test]
    fn corrupt_file_is_kept_and_backup_restored() {
        let dir = TempDir::new();
        save(&dir.file(), &[sample("good")]).unwrap();
        save(&dir.file(), &[sample("good"), sample("newer")]).unwrap();
        fs::write(dir.file(), "{ this is not json").unwrap();

        let out = load(&dir.file());
        assert_eq!(out.list.len(), 1);
        assert_eq!(out.list[0].text, "good");
        assert!(out.warning.is_some());

        let corrupt: Vec<_> = fs::read_dir(&dir.0)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("reminders.corrupt-")
            })
            .collect();
        assert_eq!(corrupt.len(), 1);
        assert_eq!(
            fs::read_to_string(corrupt[0].path()).unwrap(),
            "{ this is not json"
        );

        // Saving now must not destroy the good backup with the damaged data.
        save(&dir.file(), &out.list).unwrap();
        assert_eq!(read_list(&backup_path(&dir.file())).unwrap().len(), 1);
    }

    #[test]
    fn corrupt_without_backup_starts_empty_but_keeps_file() {
        let dir = TempDir::new();
        fs::write(dir.file(), "garbage").unwrap();
        let out = load(&dir.file());
        assert!(out.list.is_empty());
        assert!(out.warning.is_some());
        assert!(!dir.file().exists());
        let count = fs::read_dir(&dir.0).unwrap().count();
        assert_eq!(count, 1);
    }

    #[test]
    fn missing_main_uses_backup() {
        let dir = TempDir::new();
        save(&dir.file(), &[sample("x")]).unwrap();
        save(&dir.file(), &[sample("x"), sample("y")]).unwrap();
        fs::remove_file(dir.file()).unwrap();
        assert_eq!(load(&dir.file()).list.len(), 1);
    }

    #[test]
    fn nothing_on_disk_is_empty_without_warning() {
        let dir = TempDir::new();
        let out = load(&dir.file());
        assert!(out.list.is_empty());
        assert!(out.warning.is_none());
    }
}
