//! App settings and categories, saved as settings.json in the app data folder.

use crate::schedule::{DayTimes, parse_hhmm};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    pub id: String,
    pub name: String,
    /// "#rrggbb"
    pub color: String,
    /// One of `ICONS`.
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// Global shortcut that opens Quick Capture, e.g. "F12" or "Ctrl+Shift+K".
    pub hotkey: String,
    /// "HH:MM" for "Today Evening".
    pub evening_time: String,
    /// "HH:MM" for "Daily", "Weekly" and "Tomorrow morning".
    pub daily_time: String,
    /// Default snooze length in the alert window.
    pub snooze_minutes: u32,
    pub start_with_windows: bool,
    pub minimize_to_tray: bool,
    /// "system", "light" or "dark".
    pub theme: String,
    /// One of `SOUNDS`.
    pub sound: String,
    /// 0 to 100.
    pub volume: u32,
    pub categories: Vec<Category>,
}

pub const THEMES: [&str; 3] = ["system", "light", "dark"];
pub const SOUNDS: [&str; 6] = ["chime", "bell", "digital", "soft", "windows", "none"];
pub const ICONS: [&str; 12] = [
    "user",
    "briefcase",
    "home",
    "heart",
    "key",
    "book",
    "cart",
    "phone",
    "star",
    "car",
    "pill",
    "tag",
];

fn cat(id: &str, name: &str, color: &str, icon: &str) -> Category {
    Category {
        id: id.into(),
        name: name.into(),
        color: color.into(),
        icon: icon.into(),
    }
}

pub fn default_categories() -> Vec<Category> {
    vec![
        cat("personal", "Personal", "#2563eb", "user"),
        cat("work", "Work", "#d97706", "briefcase"),
        cat("home", "Home", "#16a34a", "home"),
        cat("health", "Health", "#dc2626", "heart"),
        cat("items", "Misplaced Items", "#7c3aed", "key"),
    ]
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            hotkey: "F12".into(),
            evening_time: "20:00".into(),
            daily_time: "09:00".into(),
            snooze_minutes: 10,
            start_with_windows: false,
            minimize_to_tray: true,
            theme: "system".into(),
            sound: "chime".into(),
            volume: 70,
            categories: default_categories(),
        }
    }
}

impl Settings {
    pub fn day_times(&self) -> DayTimes {
        let d = DayTimes::default();
        DayTimes {
            evening: parse_hhmm(&self.evening_time).unwrap_or(d.evening),
            daily: parse_hhmm(&self.daily_time).unwrap_or(d.daily),
        }
    }
}

fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

/// Checks settings from the UI. Returns a user-readable message on error.
/// The hotkey is checked separately, by registering it.
pub fn validate(s: &Settings) -> Result<(), String> {
    if parse_hhmm(&s.evening_time).is_none() {
        return Err("The evening time is not valid.".into());
    }
    if parse_hhmm(&s.daily_time).is_none() {
        return Err("The daily time is not valid.".into());
    }
    if !(1..=720).contains(&s.snooze_minutes) {
        return Err("Snooze length must be between 1 and 720 minutes.".into());
    }
    if !THEMES.contains(&s.theme.as_str()) {
        return Err("Unknown theme.".into());
    }
    if !SOUNDS.contains(&s.sound.as_str()) {
        return Err("Unknown alert sound.".into());
    }
    if s.volume > 100 {
        return Err("Volume must be between 0 and 100.".into());
    }
    if s.hotkey.trim().is_empty() {
        return Err("Please choose a Quick Capture shortcut.".into());
    }
    if s.categories.len() > 30 {
        return Err("At most 30 categories are allowed.".into());
    }
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    for c in &s.categories {
        let name = c.name.trim();
        if name.is_empty() || name.chars().count() > 30 {
            return Err("Each category needs a name of 1 to 30 characters.".into());
        }
        if c.id.trim().is_empty() || !ids.insert(c.id.as_str()) {
            return Err("Category ids must be unique.".into());
        }
        if !names.insert(name.to_lowercase()) {
            return Err(format!("There are two categories named \"{name}\"."));
        }
        if !is_hex_color(&c.color) {
            return Err(format!("The colour of \"{name}\" is not valid."));
        }
        if !ICONS.contains(&c.icon.as_str()) {
            return Err(format!("The icon of \"{name}\" is not valid."));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid() {
        assert!(validate(&Settings::default()).is_ok());
        assert_eq!(Settings::default().day_times(), DayTimes::default());
    }

    #[test]
    fn missing_fields_take_defaults() {
        let s: Settings = serde_json::from_str(r#"{"hotkey":"Ctrl+Alt+R"}"#).unwrap();
        assert_eq!(s.hotkey, "Ctrl+Alt+R");
        assert_eq!(s.snooze_minutes, 10);
        assert_eq!(s.categories.len(), 5);
    }

    #[test]
    fn rejects_bad_values() {
        let mut s = Settings {
            evening_time: "25:00".into(),
            ..Settings::default()
        };
        assert!(validate(&s).is_err());
        s = Settings {
            snooze_minutes: 0,
            ..Settings::default()
        };
        assert!(validate(&s).is_err());
        s = Settings::default();
        s.categories[1].name = "personal".into();
        assert!(validate(&s).is_err());
        s = Settings::default();
        s.categories[0].color = "blue".into();
        assert!(validate(&s).is_err());
        s = Settings::default();
        s.categories[0].icon = "<script>".into();
        assert!(validate(&s).is_err());
    }

    #[test]
    fn custom_times_feed_the_scheduler() {
        let s = Settings {
            evening_time: "18:45".into(),
            daily_time: "06:30".into(),
            ..Settings::default()
        };
        let t = s.day_times();
        assert_eq!(t.evening, (18, 45));
        assert_eq!(t.daily, (6, 30));
    }
}
