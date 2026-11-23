//! Core reminder logic: data model and scheduling rules.
//! Kept free of Tauri types so it can be tested with `cargo test`.

use chrono::{DateTime, Duration, FixedOffset, Local, LocalResult, NaiveDateTime, TimeZone};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Reminder {
    pub id: String,
    pub text: String,
    pub created_at: String,
    /// "5min", "hourly", "today_evening", "daily", "weekly", "custom"
    pub remind_type: String,
    pub remind_at: String,
    pub is_completed: bool,
    pub snooze_count: u32,
    /// When the alert last fired. A one-time reminder whose `last_alerted`
    /// is at or after `remind_at` has already fired and will not fire again
    /// until it is snoozed or rescheduled.
    #[serde(default)]
    pub last_alerted: Option<String>,
    /// For recurring reminders that were snoozed: the original scheduled
    /// slot, so the next occurrence follows the schedule, not the snooze.
    #[serde(default)]
    pub base_at: Option<String>,
    /// Set when the reminder is moved to "Recently Deleted".
    #[serde(default)]
    pub deleted_at: Option<String>,
    /// "reminder" (alerts) or "note" (kept and searchable, never alerts).
    #[serde(default = "default_kind")]
    pub kind: String,
    /// Id of a category from the settings, if any.
    #[serde(default)]
    pub category: Option<String>,
    /// When it was marked done, for the month calendar.
    #[serde(default)]
    pub completed_at: Option<String>,
}

fn default_kind() -> String {
    KIND_REMINDER.to_string()
}

pub const KIND_REMINDER: &str = "reminder";
pub const KIND_NOTE: &str = "note";

/// Times of day (hour, minute) used by "Today Evening" and "Daily"/"Weekly".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DayTimes {
    pub evening: (u32, u32),
    pub daily: (u32, u32),
}

impl Default for DayTimes {
    fn default() -> Self {
        DayTimes {
            evening: (20, 0),
            daily: (9, 0),
        }
    }
}

/// Parses "HH:MM" (24-hour).
pub fn parse_hhmm(s: &str) -> Option<(u32, u32)> {
    let (h, m) = s.trim().split_once(':')?;
    let h: u32 = h.parse().ok()?;
    let m: u32 = m.parse().ok()?;
    (h < 24 && m < 60).then_some((h, m))
}

/// The local date-time on the same day as `day` at `hm`.
pub fn at_time(day: DateTime<Local>, hm: (u32, u32)) -> DateTime<Local> {
    let naive = day
        .date_naive()
        .and_hms_opt(hm.0, hm.1, 0)
        .unwrap_or_else(|| day.naive_local());
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(v) => v,
        LocalResult::Ambiguous(a, _) => a,
        LocalResult::None => day,
    }
}

/// First due time for a new or rescheduled reminder.
pub fn compute_due(
    remind_type: &str,
    now: DateTime<Local>,
    times: DayTimes,
    custom: Option<&str>,
) -> Result<DateTime<Local>, String> {
    Ok(match remind_type {
        "5min" => now + Duration::minutes(5),
        "hourly" => now + Duration::hours(1),
        "today_evening" => {
            let t = at_time(now, times.evening);
            if t > now {
                t
            } else {
                at_time(add_local_days(now, 1), times.daily)
            }
        }
        "daily" => {
            let t = at_time(now, times.daily);
            if t > now {
                t
            } else {
                at_time(add_local_days(now, 1), times.daily)
            }
        }
        "weekly" => at_time(add_local_days(now, 7), times.daily),
        "custom" => custom
            .and_then(parse_time)
            .map(|t| t.with_timezone(&Local))
            .ok_or_else(|| "Please choose the alert date and time.".to_string())?,
        "none" => now,
        other => return Err(format!("Unknown reminder schedule: {other}")),
    })
}

pub const REMIND_TYPES: [&str; 7] = [
    "5min",
    "hourly",
    "today_evening",
    "daily",
    "weekly",
    "custom",
    "none",
];

pub fn is_recurring(remind_type: &str) -> bool {
    matches!(remind_type, "hourly" | "daily" | "weekly")
}

pub fn parse_time(s: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(s).ok()
}

/// Adds `days` calendar days to `t`, keeping the same local wall-clock time.
fn add_local_days(t: DateTime<Local>, days: i64) -> DateTime<Local> {
    let naive: NaiveDateTime = t.naive_local() + Duration::days(days);
    match Local.from_local_datetime(&naive) {
        LocalResult::Single(v) => v,
        LocalResult::Ambiguous(a, _) => a,
        LocalResult::None => t + Duration::days(days),
    }
}

/// Next occurrence of a recurring reminder strictly after `now`, counted
/// from its scheduled slot `base` (not from when the alert fired).
pub fn next_occurrence(
    remind_type: &str,
    base: DateTime<Local>,
    now: DateTime<Local>,
) -> Option<DateTime<Local>> {
    match remind_type {
        "hourly" => {
            let step = Duration::hours(1);
            if base > now {
                return Some(base);
            }
            let elapsed = (now - base).num_seconds();
            let steps = elapsed / step.num_seconds() + 1;
            Some(base + Duration::seconds(steps * step.num_seconds()))
        }
        "daily" | "weekly" => {
            let step_days = if remind_type == "daily" { 1 } else { 7 };
            if base > now {
                return Some(base);
            }
            let elapsed_days = (now.date_naive() - base.date_naive()).num_days();
            let mut k = (elapsed_days / step_days).max(0);
            let mut next = add_local_days(base, k * step_days);
            while next <= now {
                k += 1;
                next = add_local_days(base, k * step_days);
            }
            Some(next)
        }
        _ => None,
    }
}

/// Returns true when the reminder should alert now.
pub fn is_due(r: &Reminder, now: DateTime<Local>) -> bool {
    if r.is_completed || r.deleted_at.is_some() || r.kind == KIND_NOTE {
        return false;
    }
    let Some(due) = parse_time(&r.remind_at) else {
        return false;
    };
    if due > now {
        return false;
    }
    match r.last_alerted.as_deref().and_then(parse_time) {
        Some(last) => last < due,
        None => true,
    }
}

/// Runs one scheduler pass. Returns the reminders that fired (as they were
/// when they fired). Recurring reminders are moved to their next slot.
pub fn process_due(list: &mut [Reminder], now: DateTime<Local>) -> Vec<Reminder> {
    let mut fired = Vec::new();
    for r in list.iter_mut() {
        if !is_due(r, now) {
            continue;
        }
        fired.push(r.clone());
        r.last_alerted = Some(now.to_rfc3339());
        if is_recurring(&r.remind_type) {
            let base_str = r.base_at.take().unwrap_or_else(|| r.remind_at.clone());
            let base = parse_time(&base_str)
                .map(|t| t.with_timezone(&Local))
                .unwrap_or(now);
            if let Some(next) = next_occurrence(&r.remind_type, base, now) {
                r.remind_at = next.to_rfc3339();
            }
        }
    }
    fired
}

/// Snoozes a reminder by `minutes` from `now`.
pub fn snooze(r: &mut Reminder, minutes: u32, now: DateTime<Local>) {
    snooze_until(r, now + Duration::minutes(i64::from(minutes)));
}

/// Snoozes a reminder until `until`.
pub fn snooze_until(r: &mut Reminder, until: DateTime<Local>) {
    if is_recurring(&r.remind_type) && r.base_at.is_none() {
        // Remember the scheduled slot, so the next occurrence keeps the schedule.
        r.base_at = Some(r.remind_at.clone());
    }
    r.remind_at = until.to_rfc3339();
    r.snooze_count += 1;
}

/// "Tomorrow morning": the daily time on the next day.
pub fn tomorrow_morning(now: DateTime<Local>, times: DayTimes) -> DateTime<Local> {
    at_time(add_local_days(now, 1), times.daily)
}

/// Marks done or not done, keeping the date it was done for the calendar.
pub fn set_completed(r: &mut Reminder, done: bool, now: DateTime<Local>) {
    r.is_completed = done;
    r.completed_at = done.then(|| now.to_rfc3339());
}

/// Applies an edit. When `remind_at` is `None`, the due time is kept.
pub fn apply_edit(r: &mut Reminder, text: String, remind_type: String, remind_at: Option<String>) {
    r.text = text;
    let type_changed = r.remind_type != remind_type;
    r.remind_type = remind_type;
    if let Some(at) = remind_at {
        if at != r.remind_at {
            r.remind_at = at;
            r.last_alerted = None;
            r.base_at = None;
        }
    } else if type_changed {
        r.base_at = None;
    }
}

/// Validates input for add/edit. Returns a user-readable message on error.
pub fn validate(text: &str, remind_type: &str, remind_at: Option<&str>) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("The reminder text is empty.".into());
    }
    if !REMIND_TYPES.contains(&remind_type) {
        return Err(format!("Unknown reminder schedule: {remind_type}"));
    }
    if text.chars().count() > 5000 {
        return Err("The text is too long (5000 characters at most).".into());
    }
    if let Some(at) = remind_at
        && parse_time(at).is_none()
    {
        return Err("The reminder time is not valid.".into());
    }
    Ok(())
}

#[cfg(test)]
impl Reminder {
    pub fn new_for_test(text: &str) -> Self {
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
            kind: KIND_REMINDER.into(),
            category: None,
            completed_at: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(s: &str) -> DateTime<Local> {
        let naive = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap();
        Local.from_local_datetime(&naive).single().unwrap()
    }

    fn reminder(kind: &str, due: DateTime<Local>) -> Reminder {
        Reminder {
            id: "1".into(),
            text: "Car keys".into(),
            created_at: due.to_rfc3339(),
            remind_type: kind.into(),
            remind_at: due.to_rfc3339(),
            is_completed: false,
            snooze_count: 0,
            last_alerted: None,
            base_at: None,
            deleted_at: None,
            kind: KIND_REMINDER.into(),
            category: None,
            completed_at: None,
        }
    }

    fn due_of(r: &Reminder) -> DateTime<Local> {
        parse_time(&r.remind_at).unwrap().with_timezone(&Local)
    }

    #[test]
    fn one_time_fires_once_only() {
        let mut list = vec![reminder("custom", at("2026-11-23 10:00"))];
        assert_eq!(process_due(&mut list, at("2026-11-23 10:00")).len(), 1);
        // Later passes, minutes and hours later, must not fire again.
        assert!(process_due(&mut list, at("2026-11-23 10:02")).is_empty());
        assert!(process_due(&mut list, at("2026-11-23 18:00")).is_empty());
        assert!(!list[0].is_completed);
    }

    #[test]
    fn today_evening_and_5min_are_one_time() {
        for kind in ["today_evening", "5min"] {
            let mut list = vec![reminder(kind, at("2026-11-23 20:00"))];
            assert_eq!(process_due(&mut list, at("2026-11-23 20:00")).len(), 1);
            assert!(process_due(&mut list, at("2026-11-23 21:00")).is_empty());
        }
    }

    #[test]
    fn snoozed_one_time_fires_again() {
        let mut list = vec![reminder("custom", at("2026-11-23 10:00"))];
        process_due(&mut list, at("2026-11-23 10:00"));
        snooze(&mut list[0], 10, at("2026-11-23 10:01"));
        assert!(process_due(&mut list, at("2026-11-23 10:05")).is_empty());
        assert_eq!(process_due(&mut list, at("2026-11-23 10:11")).len(), 1);
        assert_eq!(list[0].snooze_count, 1);
    }

    #[test]
    fn daily_keeps_scheduled_time_after_pc_was_off() {
        let mut list = vec![reminder("daily", at("2026-11-20 09:00"))];
        // PC was off for three days; it starts at 14:37.
        let fired = process_due(&mut list, at("2026-11-23 14:37"));
        assert_eq!(fired.len(), 1);
        assert_eq!(due_of(&list[0]), at("2026-11-24 09:00"));
    }

    #[test]
    fn daily_fired_late_does_not_drift() {
        let mut list = vec![reminder("daily", at("2026-11-23 09:00"))];
        process_due(&mut list, at("2026-11-23 09:00"));
        assert_eq!(due_of(&list[0]), at("2026-11-24 09:00"));
        assert!(process_due(&mut list, at("2026-11-23 09:01")).is_empty());
    }

    #[test]
    fn weekly_keeps_weekday_and_time() {
        let mut list = vec![reminder("weekly", at("2026-11-02 09:00"))];
        process_due(&mut list, at("2026-11-23 11:00"));
        assert_eq!(due_of(&list[0]), at("2026-11-30 09:00"));
    }

    #[test]
    fn hourly_stays_on_the_hour_slot() {
        let mut list = vec![reminder("hourly", at("2026-11-23 10:15"))];
        process_due(&mut list, at("2026-11-23 13:40"));
        assert_eq!(due_of(&list[0]), at("2026-11-23 14:15"));
    }

    #[test]
    fn snoozed_daily_returns_to_schedule() {
        let mut list = vec![reminder("daily", at("2026-11-23 09:00"))];
        // Snoozed before it fires: 09:00 -> 09:10.
        snooze(&mut list[0], 10, at("2026-11-23 09:00"));
        let fired = process_due(&mut list, at("2026-11-23 09:10"));
        assert_eq!(fired.len(), 1);
        assert_eq!(due_of(&list[0]), at("2026-11-24 09:00"));
        assert!(list[0].base_at.is_none());
    }

    #[test]
    fn snooze_after_daily_alert_keeps_tomorrow_slot() {
        let mut list = vec![reminder("daily", at("2026-11-23 09:00"))];
        process_due(&mut list, at("2026-11-23 09:00"));
        snooze(&mut list[0], 10, at("2026-11-23 09:01"));
        assert_eq!(process_due(&mut list, at("2026-11-23 09:11")).len(), 1);
        assert_eq!(due_of(&list[0]), at("2026-11-24 09:00"));
    }

    #[test]
    fn completed_and_deleted_never_fire() {
        let mut a = reminder("custom", at("2026-11-23 10:00"));
        a.is_completed = true;
        let mut b = reminder("custom", at("2026-11-23 10:00"));
        b.deleted_at = Some(at("2026-11-23 09:00").to_rfc3339());
        let mut list = vec![a, b];
        assert!(process_due(&mut list, at("2026-11-23 11:00")).is_empty());
    }

    #[test]
    fn edit_text_only_keeps_due_time() {
        let mut r = reminder("daily", at("2026-11-24 09:00"));
        let before = r.remind_at.clone();
        apply_edit(&mut r, "New text".into(), "daily".into(), None);
        assert_eq!(r.text, "New text");
        assert_eq!(r.remind_at, before);
    }

    #[test]
    fn edit_new_time_rearms_alert() {
        let mut list = vec![reminder("custom", at("2026-11-23 10:00"))];
        process_due(&mut list, at("2026-11-23 10:00"));
        let new_at = at("2026-11-23 12:00").to_rfc3339();
        apply_edit(&mut list[0], "x".into(), "custom".into(), Some(new_at));
        assert!(list[0].last_alerted.is_none());
        assert_eq!(process_due(&mut list, at("2026-11-23 12:00")).len(), 1);
    }

    #[test]
    fn old_files_without_new_fields_still_load() {
        let json = r#"[{"id":"a","text":"t","created_at":"2026-11-23T10:00:00+05:30",
            "remind_type":"hourly","remind_at":"2026-11-23T11:00:00+05:30",
            "is_completed":false,"snooze_count":0}]"#;
        let list: Vec<Reminder> = serde_json::from_str(json).unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].deleted_at.is_none());
    }

    #[test]
    fn validate_rejects_bad_input() {
        assert!(validate("  ", "hourly", None).is_err());
        assert!(validate("x", "yearly", None).is_err());
        assert!(validate("x", "custom", Some("not a date")).is_err());
        assert!(validate("x", "custom", Some("2026-11-23T10:00:00Z")).is_ok());
    }

    #[test]
    fn notes_never_fire() {
        let mut n = reminder("none", at("2026-11-23 10:00"));
        n.kind = KIND_NOTE.into();
        let mut list = vec![n];
        assert!(process_due(&mut list, at("2026-11-24 10:00")).is_empty());
    }

    #[test]
    fn old_files_default_to_reminder_kind() {
        let json = r#"[{"id":"a","text":"t","created_at":"2026-11-23T10:00:00+05:30",
            "remind_type":"custom","remind_at":"2026-11-23T11:00:00+05:30",
            "is_completed":true,"snooze_count":0}]"#;
        let list: Vec<Reminder> = serde_json::from_str(json).unwrap();
        assert_eq!(list[0].kind, KIND_REMINDER);
        assert!(list[0].category.is_none());
        assert!(list[0].completed_at.is_none());
    }

    #[test]
    fn compute_due_uses_settings_times() {
        let times = DayTimes {
            evening: (19, 30),
            daily: (7, 15),
        };
        let now = at("2026-11-23 10:00");
        assert_eq!(
            compute_due("today_evening", now, times, None).unwrap(),
            at("2026-11-23 19:30")
        );
        assert_eq!(
            compute_due("daily", now, times, None).unwrap(),
            at("2026-11-24 07:15")
        );
        assert_eq!(
            compute_due("daily", at("2026-11-23 06:00"), times, None).unwrap(),
            at("2026-11-23 07:15")
        );
        assert_eq!(
            compute_due("weekly", now, times, None).unwrap(),
            at("2026-11-30 07:15")
        );
        assert_eq!(
            compute_due("5min", now, times, None).unwrap(),
            at("2026-11-23 10:05")
        );
    }

    #[test]
    fn evening_after_evening_time_moves_to_tomorrow_morning() {
        let due = compute_due(
            "today_evening",
            at("2026-11-23 21:00"),
            DayTimes::default(),
            None,
        )
        .unwrap();
        assert_eq!(due, at("2026-11-24 09:00"));
    }

    #[test]
    fn custom_needs_a_time() {
        assert!(compute_due("custom", at("2026-11-23 10:00"), DayTimes::default(), None).is_err());
    }

    #[test]
    fn parse_hhmm_checks_range() {
        assert_eq!(parse_hhmm("07:05"), Some((7, 5)));
        assert_eq!(parse_hhmm("24:00"), None);
        assert_eq!(parse_hhmm("x"), None);
    }

    #[test]
    fn snooze_tomorrow_and_completion_date() {
        let now = at("2026-11-23 22:00");
        let mut r = reminder("custom", at("2026-11-23 21:00"));
        snooze_until(&mut r, tomorrow_morning(now, DayTimes::default()));
        assert_eq!(due_of(&r), at("2026-11-24 09:00"));
        set_completed(&mut r, true, now);
        assert!(r.completed_at.is_some());
        set_completed(&mut r, false, now);
        assert!(r.completed_at.is_none());
    }
}
