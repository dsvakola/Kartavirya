//! Safe storage for the app's JSON files (reminders, daily notes, settings).
//!
//! - Saves are atomic: write a temporary file, flush it to disk, then rename.
//! - The previous good file is kept as `<name>.bak.json`.
//! - A damaged file is never overwritten: it is moved aside to
//!   `<name>.corrupt-<date-time>.json` and the backup is loaded instead.

use chrono::Local;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct LoadOutcome<T> {
    pub value: T,
    /// A message for the user when something had to be recovered.
    pub warning: Option<String>,
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "data".into())
}

pub fn backup_path(path: &Path) -> PathBuf {
    path.with_file_name(format!("{}.bak.json", stem(path)))
}

fn temp_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "data.json".into());
    path.with_file_name(format!("{name}.tmp"))
}

fn read_value<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
    if data.trim().is_empty() {
        return Err("the file is empty".into());
    }
    serde_json::from_str::<T>(&data).map_err(|e| e.to_string())
}

/// Moves a damaged file aside so it is never overwritten. Returns the new path.
fn quarantine(path: &Path) -> Option<PathBuf> {
    let stamp = Local::now().format("%Y%m%d-%H%M%S");
    let target = path.with_file_name(format!("{}.corrupt-{stamp}.json", stem(path)));
    fs::rename(path, &target).ok().map(|_| target)
}

/// Loads a file. `what` names the data in warnings, e.g. "reminders".
pub fn load<T: DeserializeOwned + Default>(path: &Path, what: &str) -> LoadOutcome<T> {
    let bak = backup_path(path);

    if path.exists() {
        match read_value::<T>(path) {
            Ok(value) => {
                return LoadOutcome {
                    value,
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
                if let Ok(value) = read_value::<T>(&bak) {
                    return LoadOutcome {
                        value,
                        warning: Some(format!(
                            "The {what} file was damaged ({err}). Your {what} were restored from the last backup. {kept}"
                        )),
                    };
                }
                return LoadOutcome {
                    value: T::default(),
                    warning: Some(format!(
                        "The {what} file was damaged ({err}) and no backup could be read. {kept}"
                    )),
                };
            }
        }
    }

    // No main file: recover from the backup if one exists (for example after
    // a crash between the two renames of a save).
    if bak.exists()
        && let Ok(value) = read_value::<T>(&bak)
    {
        return LoadOutcome {
            value,
            warning: Some(format!("Your {what} were restored from the last backup.")),
        };
    }
    LoadOutcome {
        value: T::default(),
        warning: None,
    }
}

/// Writes `json` to `path` atomically (temporary file, flush, rename).
pub fn write_atomic(path: &Path, json: &str, keep_backup: bool) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Could not create data folder: {e}"))?;
    }
    let tmp = temp_path(path);
    {
        let mut f = fs::File::create(&tmp).map_err(|e| format!("Could not save: {e}"))?;
        f.write_all(json.as_bytes())
            .and_then(|_| f.sync_all())
            .map_err(|e| format!("Could not save: {e}"))?;
    }
    // Keep the previous version as the backup, then move the new file in place.
    if keep_backup && path.exists() {
        let _ = fs::rename(path, backup_path(path));
    }
    fs::rename(&tmp, path).map_err(|e| format!("Could not save: {e}"))?;
    Ok(())
}

pub fn save<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<(), String> {
    let json =
        serde_json::to_string_pretty(value).map_err(|e| format!("Could not save data: {e}"))?;
    write_atomic(path, &json, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule::Reminder;
    use std::collections::BTreeMap;

    pub struct TempDir(pub PathBuf);
    impl TempDir {
        pub fn new() -> Self {
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
        Reminder::new_for_test(text)
    }

    fn load_list(p: &Path) -> LoadOutcome<Vec<Reminder>> {
        load(p, "reminders")
    }

    #[test]
    fn save_then_load_round_trip() {
        let dir = TempDir::new();
        save(&dir.file(), &[sample("a"), sample("b")]).unwrap();
        let out = load_list(&dir.file());
        assert_eq!(out.value.len(), 2);
        assert!(out.warning.is_none());
        assert!(!temp_path(&dir.file()).exists());
    }

    #[test]
    fn backup_name_is_unchanged_for_reminders() {
        let p = PathBuf::from("x").join("reminders.json");
        assert!(backup_path(&p).ends_with("reminders.bak.json"));
        assert!(temp_path(&p).ends_with("reminders.json.tmp"));
    }

    #[test]
    fn second_save_keeps_previous_as_backup() {
        let dir = TempDir::new();
        save(&dir.file(), &[sample("first")]).unwrap();
        save(&dir.file(), &[sample("first"), sample("second")]).unwrap();
        let bak: Vec<Reminder> = read_value(&backup_path(&dir.file())).unwrap();
        assert_eq!(bak.len(), 1);
        assert_eq!(load_list(&dir.file()).value.len(), 2);
    }

    #[test]
    fn corrupt_file_is_kept_and_backup_restored() {
        let dir = TempDir::new();
        save(&dir.file(), &[sample("good")]).unwrap();
        save(&dir.file(), &[sample("good"), sample("newer")]).unwrap();
        fs::write(dir.file(), "{ this is not json").unwrap();

        let out = load_list(&dir.file());
        assert_eq!(out.value.len(), 1);
        assert_eq!(out.value[0].text, "good");
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
        save(&dir.file(), &out.value).unwrap();
        let bak: Vec<Reminder> = read_value(&backup_path(&dir.file())).unwrap();
        assert_eq!(bak.len(), 1);
    }

    #[test]
    fn corrupt_without_backup_starts_empty_but_keeps_file() {
        let dir = TempDir::new();
        fs::write(dir.file(), "garbage").unwrap();
        let out = load_list(&dir.file());
        assert!(out.value.is_empty());
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
        assert_eq!(load_list(&dir.file()).value.len(), 1);
    }

    #[test]
    fn nothing_on_disk_is_empty_without_warning() {
        let dir = TempDir::new();
        let out = load_list(&dir.file());
        assert!(out.value.is_empty());
        assert!(out.warning.is_none());
    }

    #[test]
    fn daily_notes_map_round_trip() {
        let dir = TempDir::new();
        let p = dir.0.join("daily-notes.json");
        let mut m = BTreeMap::new();
        m.insert("2026-11-23".to_string(), "Met the principal".to_string());
        save(&p, &m).unwrap();
        let out: LoadOutcome<BTreeMap<String, String>> = load(&p, "daily notes");
        assert_eq!(out.value.get("2026-11-23").unwrap(), "Met the principal");
        assert_eq!(backup_path(&p), dir.0.join("daily-notes.bak.json"));
    }
}
