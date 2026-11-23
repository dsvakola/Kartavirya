mod backup;
mod nlp;
mod schedule;
mod settings;
mod storage;

use crate::backup::{ExportFile, ImportPlan, ImportPreview};
use crate::schedule::{KIND_NOTE, KIND_REMINDER, Reminder};
use crate::settings::Settings;
use chrono::Local;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration as StdDuration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, LogicalSize, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
use uuid::Uuid;

const PRIVACY_URL: &str = "https://github.com/dsvakola/Kartavirya/blob/main/privacy-policy.md";
const WEBSITE_URL: &str = "https://vsa.edu.in/";
const EMAIL_URL: &str = "mailto:softinquiry@vsa.edu.in";
const AUTOSTART_NAME: &str = "Kartavirya";
const POPUP_WIDTH: f64 = 560.0;

pub struct Shared {
    reminders: Mutex<Vec<Reminder>>,
    notes: Mutex<BTreeMap<String, String>>,
    settings: Mutex<Settings>,
    reminders_path: PathBuf,
    notes_path: PathBuf,
    settings_path: PathBuf,
    /// Ids of reminders waiting in the floating alert window, oldest first.
    alert_queue: Mutex<Vec<String>>,
    warnings: Mutex<Vec<String>>,
    /// Reminders that came due while Kartavirya was closed (shown once).
    missed: Mutex<Vec<Reminder>>,
    /// An import that was previewed and waits for the user's confirmation.
    pending_import: Mutex<Option<ImportPlan>>,
}

type AppState<'a> = State<'a, Arc<Shared>>;

/// Locks a mutex, recovering the data even if another thread panicked.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Runs `f` on the reminder list, saves it, and tells all windows.
fn mutate<R>(
    app: &AppHandle,
    shared: &Shared,
    f: impl FnOnce(&mut Vec<Reminder>) -> Result<R, String>,
) -> Result<R, String> {
    let result = {
        let mut list = lock(&shared.reminders);
        let before = list.clone();
        let result = f(&mut list)?;
        if let Err(e) = storage::save(&shared.reminders_path, &*list) {
            // Keep memory and disk in step: undo the change if it could not be saved.
            *list = before;
            return Err(e);
        }
        result
    };
    let _ = app.emit("reminder-updated", ());
    Ok(result)
}

/// Same as `mutate`, for the daily notes.
fn mutate_notes<R>(
    app: &AppHandle,
    shared: &Shared,
    f: impl FnOnce(&mut BTreeMap<String, String>) -> Result<R, String>,
) -> Result<R, String> {
    let result = {
        let mut notes = lock(&shared.notes);
        let before = notes.clone();
        let result = f(&mut notes)?;
        if let Err(e) = storage::save(&shared.notes_path, &*notes) {
            *notes = before;
            return Err(e);
        }
        result
    };
    let _ = app.emit("notes-updated", ());
    Ok(result)
}

fn find<'a>(list: &'a mut [Reminder], id: &str) -> Result<&'a mut Reminder, String> {
    list.iter_mut()
        .find(|r| r.id == id)
        .ok_or_else(|| "This reminder no longer exists.".to_string())
}

fn remove_from_alerts(app: &AppHandle, shared: &Shared, id: &str) {
    let empty = {
        let mut q = lock(&shared.alert_queue);
        q.retain(|x| x != id);
        q.is_empty()
    };
    let _ = app.emit_to("alert", "alerts-changed", ());
    if empty && let Some(win) = app.get_webview_window("alert") {
        let _ = win.hide();
    }
}

fn show_window(app: &AppHandle, label: &str) {
    if let Some(win) = app.get_webview_window(label) {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
    }
}

fn show_popup_window(app: &AppHandle) {
    if let Some(popup) = app.get_webview_window("popup") {
        let _ = popup.center();
        let _ = popup.show();
        let _ = popup.set_focus();
        let _ = app.emit_to("popup", "popup-shown", ());
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn MessageBeep(u_type: u32) -> i32;
}

fn beep() {
    #[cfg(target_os = "windows")]
    // SAFETY: MessageBeep takes a plain integer and has no memory effects.
    unsafe {
        MessageBeep(0x0000_0040); // MB_ICONASTERISK
    }
}

/// Puts reminders in the floating alert window and brings it to the front.
fn raise_alerts(app: &AppHandle, shared: &Shared, ids: &[String]) {
    {
        let mut q = lock(&shared.alert_queue);
        for id in ids {
            if !q.contains(id) {
                q.push(id.clone());
            }
        }
    }
    if lock(&shared.settings).sound == "windows" {
        beep();
    }
    if let Some(win) = app.get_webview_window("alert") {
        let _ = win.center();
        let _ = win.show();
        let _ = win.set_always_on_top(true);
        let _ = win.set_focus();
        let _ = win.request_user_attention(Some(tauri::UserAttentionType::Critical));
    }
    let _ = app.emit_to("alert", "alerts-changed", ());
}

fn day_times(shared: &Shared) -> schedule::DayTimes {
    lock(&shared.settings).day_times()
}

// ------------------------------------------------------------- start with Windows

#[cfg(target_os = "windows")]
const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

#[cfg(target_os = "windows")]
fn autostart_enabled() -> bool {
    windows_registry::CURRENT_USER
        .open(RUN_KEY)
        .and_then(|k| k.get_string(AUTOSTART_NAME))
        .is_ok()
}

#[cfg(target_os = "windows")]
fn set_autostart(enabled: bool) -> Result<(), String> {
    let key = windows_registry::CURRENT_USER
        .create(RUN_KEY)
        .map_err(|e| format!("Could not change the Windows start-up setting: {e}"))?;
    if enabled {
        let exe =
            std::env::current_exe().map_err(|e| format!("Could not find the program file: {e}"))?;
        key.set_string(AUTOSTART_NAME, format!("\"{}\" --minimized", exe.display()))
            .map_err(|e| format!("Could not change the Windows start-up setting: {e}"))
    } else if key.get_string(AUTOSTART_NAME).is_ok() {
        key.remove_value(AUTOSTART_NAME)
            .map_err(|e| format!("Could not change the Windows start-up setting: {e}"))
    } else {
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
fn autostart_enabled() -> bool {
    false
}

#[cfg(not(target_os = "windows"))]
fn set_autostart(_enabled: bool) -> Result<(), String> {
    Err("Start with Windows is only available on Windows.".into())
}

// ---------------------------------------------------------------- hotkeys

fn on_hotkey(
    app: &AppHandle,
    _shortcut: &Shortcut,
    event: tauri_plugin_global_shortcut::ShortcutEvent,
) {
    if event.state() == ShortcutState::Pressed {
        show_popup_window(app);
    }
}

/// Registers the Quick Capture shortcut. The default F12 also keeps its
/// three extra shortcuts. All shortcuts are checked before any change.
fn register_hotkeys(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    let mut keys = vec![hotkey.trim().to_string()];
    if hotkey.trim().eq_ignore_ascii_case("F12") {
        keys.extend(["Ctrl+F12", "Alt+F12", "Alt+K"].map(String::from));
    }
    let mut parsed = Vec::new();
    for k in &keys {
        let sc: Shortcut = k
            .parse()
            .map_err(|_| format!("\"{k}\" is not a valid shortcut."))?;
        parsed.push(sc);
    }
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    for (i, sc) in parsed.into_iter().enumerate() {
        if let Err(e) = gs.on_shortcut(sc, on_hotkey)
            && i == 0
        {
            return Err(format!(
                "The shortcut \"{hotkey}\" could not be used. Another program may be using it. ({e})"
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- commands

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    name: String,
    version: String,
}

#[tauri::command]
fn get_app_info(app: AppHandle) -> AppInfo {
    let info = app.package_info();
    AppInfo {
        name: info.name.clone(),
        version: info.version.to_string(),
    }
}

#[tauri::command]
fn open_link(app: AppHandle, target: String) -> Result<(), String> {
    let url = match target.as_str() {
        "privacy" => PRIVACY_URL,
        "website" => WEBSITE_URL,
        "email" => EMAIL_URL,
        _ => return Err("Unknown link.".into()),
    };
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| format!("Could not open the link: {e}"))
}

#[tauri::command]
fn get_load_warning(state: AppState) -> Option<String> {
    let mut w = lock(&state.warnings);
    if w.is_empty() {
        None
    } else {
        Some(std::mem::take(&mut *w).join("\n"))
    }
}

#[tauri::command]
fn get_settings(state: AppState) -> Settings {
    let mut s = lock(&state.settings).clone();
    s.start_with_windows = autostart_enabled();
    s
}

#[tauri::command]
fn save_settings(app: AppHandle, state: AppState, settings: Settings) -> Result<Settings, String> {
    settings::validate(&settings)?;
    let mut new = settings;
    new.hotkey = new.hotkey.trim().to_string();
    let old = lock(&state.settings).clone();

    if new.hotkey != old.hotkey
        && let Err(e) = register_hotkeys(&app, &new.hotkey)
    {
        let _ = register_hotkeys(&app, &old.hotkey);
        return Err(e);
    }
    if new.start_with_windows != autostart_enabled() {
        set_autostart(new.start_with_windows)?;
    }
    if let Err(e) = storage::save(&state.settings_path, &new) {
        if new.hotkey != old.hotkey {
            let _ = register_hotkeys(&app, &old.hotkey);
        }
        return Err(e);
    }
    *lock(&state.settings) = new.clone();
    let _ = app.emit("settings-changed", new.clone());
    Ok(new)
}

#[tauri::command]
fn get_reminders(state: AppState) -> Vec<Reminder> {
    lock(&state.reminders).clone()
}

fn check_category(shared: &Shared, category: &Option<String>) -> Result<(), String> {
    if let Some(id) = category
        && !lock(&shared.settings)
            .categories
            .iter()
            .any(|c| &c.id == id)
    {
        return Err("This category no longer exists.".into());
    }
    Ok(())
}

fn normalize_kind(kind: Option<String>) -> Result<String, String> {
    match kind.as_deref() {
        None | Some("") | Some(KIND_REMINDER) => Ok(KIND_REMINDER.into()),
        Some(KIND_NOTE) => Ok(KIND_NOTE.into()),
        Some(_) => Err("Unknown item type.".into()),
    }
}

#[tauri::command]
fn add_reminder(
    app: AppHandle,
    state: AppState,
    text: String,
    kind: Option<String>,
    remind_type: String,
    remind_at: Option<String>,
    category: Option<String>,
) -> Result<Reminder, String> {
    let kind = normalize_kind(kind)?;
    let remind_type = if kind == KIND_NOTE {
        "none".to_string()
    } else {
        remind_type
    };
    schedule::validate(&text, &remind_type, remind_at.as_deref())?;
    if kind == KIND_REMINDER && remind_type == "none" {
        return Err("Please choose when to remind you.".into());
    }
    check_category(&state, &category)?;
    let now = Local::now();
    let due = match remind_at {
        Some(at) => at,
        None => schedule::compute_due(&remind_type, now, day_times(&state), None)?.to_rfc3339(),
    };
    let reminder = Reminder {
        id: Uuid::new_v4().to_string(),
        text: text.trim().to_string(),
        created_at: now.to_rfc3339(),
        remind_type,
        remind_at: due,
        is_completed: false,
        snooze_count: 0,
        last_alerted: None,
        base_at: None,
        deleted_at: None,
        kind,
        category,
        completed_at: None,
    };
    let copy = reminder.clone();
    mutate(&app, &state, move |list| {
        list.insert(0, copy);
        Ok(())
    })?;
    Ok(reminder)
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
fn update_reminder(
    app: AppHandle,
    state: AppState,
    id: String,
    text: String,
    kind: Option<String>,
    remind_type: String,
    remind_at: Option<String>,
    reschedule: bool,
    category: Option<String>,
) -> Result<(), String> {
    let kind = normalize_kind(kind)?;
    let remind_type = if kind == KIND_NOTE {
        "none".to_string()
    } else {
        remind_type
    };
    schedule::validate(&text, &remind_type, remind_at.as_deref())?;
    if kind == KIND_REMINDER && remind_type == "none" {
        return Err("Please choose when to remind you.".into());
    }
    check_category(&state, &category)?;
    let times = day_times(&state);
    mutate(&app, &state, |list| {
        let r = find(list, &id)?;
        let was_note = r.kind == KIND_NOTE;
        let new_due = if kind == KIND_REMINDER && (reschedule || was_note) {
            Some(match remind_at {
                Some(at) => at,
                None => {
                    schedule::compute_due(&remind_type, Local::now(), times, None)?.to_rfc3339()
                }
            })
        } else {
            None
        };
        schedule::apply_edit(r, text.trim().to_string(), remind_type, new_due);
        r.kind = kind;
        r.category = category;
        Ok(())
    })
}

#[tauri::command]
fn toggle_reminder(app: AppHandle, state: AppState, id: String) -> Result<(), String> {
    let completed = mutate(&app, &state, |list| {
        let r = find(list, &id)?;
        let done = !r.is_completed;
        schedule::set_completed(r, done, Local::now());
        Ok(done)
    })?;
    if completed {
        remove_from_alerts(&app, &state, &id);
    }
    Ok(())
}

/// Moves a reminder to "Recently Deleted". It can be restored from there.
#[tauri::command]
fn delete_reminder(app: AppHandle, state: AppState, id: String) -> Result<(), String> {
    mutate(&app, &state, |list| {
        let r = find(list, &id)?;
        r.deleted_at = Some(Local::now().to_rfc3339());
        Ok(())
    })?;
    remove_from_alerts(&app, &state, &id);
    Ok(())
}

#[tauri::command]
fn restore_reminder(app: AppHandle, state: AppState, id: String) -> Result<(), String> {
    mutate(&app, &state, |list| {
        find(list, &id)?.deleted_at = None;
        Ok(())
    })
}

/// Permanently removes one reminder that is already in "Recently Deleted".
/// The previous file is still kept as reminders.bak.json.
#[tauri::command]
fn delete_forever(app: AppHandle, state: AppState, id: String) -> Result<(), String> {
    mutate(&app, &state, |list| {
        let before = list.len();
        list.retain(|r| !(r.id == id && r.deleted_at.is_some()));
        if list.len() == before {
            return Err("Only reminders in Recently Deleted can be removed permanently.".into());
        }
        Ok(())
    })
}

#[tauri::command]
fn empty_trash(app: AppHandle, state: AppState) -> Result<usize, String> {
    mutate(&app, &state, |list| {
        let before = list.len();
        list.retain(|r| r.deleted_at.is_none());
        Ok(before - list.len())
    })
}

#[tauri::command]
fn snooze_reminder(
    app: AppHandle,
    state: AppState,
    id: String,
    minutes: u32,
) -> Result<(), String> {
    if minutes == 0 || minutes > 7 * 24 * 60 {
        return Err("Snooze time is not valid.".into());
    }
    mutate(&app, &state, |list| {
        let r = find(list, &id)?;
        if r.kind == KIND_NOTE {
            return Err("Notes have no alert to snooze.".into());
        }
        schedule::snooze(r, minutes, Local::now());
        Ok(())
    })?;
    remove_from_alerts(&app, &state, &id);
    Ok(())
}

#[tauri::command]
fn test_reminder_alert(app: AppHandle, state: AppState, id: String) -> Result<(), String> {
    let exists = lock(&state.reminders).iter().any(|r| r.id == id);
    if !exists {
        return Err("This reminder no longer exists.".into());
    }
    raise_alerts(&app, &state, &[id]);
    Ok(())
}

/// Plays the Windows system sound (the "windows" alert sound choice).
#[tauri::command]
fn play_system_sound() {
    beep();
}

/// Reminders currently shown in the floating alert window.
#[tauri::command]
fn get_alert_queue(state: AppState) -> Vec<Reminder> {
    let ids = lock(&state.alert_queue).clone();
    let list = lock(&state.reminders);
    ids.iter()
        .filter_map(|id| list.iter().find(|r| &r.id == id).cloned())
        .collect()
}

/// Handles a button in the floating alert window:
/// "dismiss", "snooze" (with `minutes`, or the default length), "snooze_tomorrow" or "complete".
#[tauri::command]
fn alert_action(
    app: AppHandle,
    state: AppState,
    id: String,
    action: String,
    minutes: Option<u32>,
) -> Result<(), String> {
    let now = Local::now();
    match action.as_str() {
        "dismiss" => {}
        "snooze" => {
            let mins = minutes.unwrap_or_else(|| lock(&state.settings).snooze_minutes);
            if mins == 0 || mins > 7 * 24 * 60 {
                return Err("Snooze time is not valid.".into());
            }
            mutate(&app, &state, |list| {
                schedule::snooze(find(list, &id)?, mins, now);
                Ok(())
            })?;
        }
        "snooze_tomorrow" => {
            let until = schedule::tomorrow_morning(now, day_times(&state));
            mutate(&app, &state, |list| {
                schedule::snooze_until(find(list, &id)?, until);
                Ok(())
            })?;
        }
        "complete" => {
            mutate(&app, &state, |list| {
                schedule::set_completed(find(list, &id)?, true, now);
                Ok(())
            })?;
        }
        _ => return Err("Unknown action.".into()),
    }
    remove_from_alerts(&app, &state, &id);
    Ok(())
}

#[tauri::command]
fn show_popup(app: AppHandle) {
    show_popup_window(&app);
}

#[tauri::command]
fn hide_popup(app: AppHandle) {
    if let Some(popup) = app.get_webview_window("popup") {
        let _ = popup.hide();
    }
}

/// Resizes the Quick Capture window to fit its content, so that no button is cut off.
#[tauri::command]
fn fit_popup(app: AppHandle, height: f64) {
    if let Some(popup) = app.get_webview_window("popup") {
        let h = height.clamp(240.0, 640.0);
        let _ = popup.set_size(LogicalSize::new(POPUP_WIDTH, h));
        let _ = popup.center();
    }
}

#[tauri::command]
fn hide_main_window(app: AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}

#[tauri::command]
fn get_daily_notes(state: AppState) -> BTreeMap<String, String> {
    lock(&state.notes).clone()
}

/// Saves the note of one day (`date` is "YYYY-MM-DD"). An empty text removes the note.
#[tauri::command]
fn set_daily_note(
    app: AppHandle,
    state: AppState,
    date: String,
    text: String,
) -> Result<(), String> {
    if !backup::valid_day(&date) {
        return Err("The date is not valid.".into());
    }
    if text.chars().count() > backup::MAX_NOTE_CHARS {
        return Err("The note is too long.".into());
    }
    mutate_notes(&app, &state, |notes| {
        if text.trim().is_empty() {
            notes.remove(&date);
        } else {
            notes.insert(date, text);
        }
        Ok(())
    })
}

#[tauri::command]
fn parse_quick(state: AppState, text: String) -> Option<nlp::Parsed> {
    nlp::parse_quick(&text, Local::now(), day_times(&state))
}

/// Reminders that came due while Kartavirya was closed. Returned once.
#[tauri::command]
fn take_missed(state: AppState) -> Vec<Reminder> {
    std::mem::take(&mut *lock(&state.missed))
}

// ------------------------------------------------------------ export / import

fn file_stamp() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

/// Asks where to save, then writes all reminders, daily notes and categories.
/// Returns the saved path, or `None` when the user cancelled.
#[tauri::command]
async fn export_data(app: AppHandle, state: AppState<'_>) -> Result<Option<String>, String> {
    let picked = app
        .dialog()
        .file()
        .set_title("Save Kartavirya backup")
        .set_file_name(format!("kartavirya-backup-{}.json", file_stamp()))
        .add_filter("Kartavirya backup", &["json"])
        .blocking_save_file();
    let Some(picked) = picked else {
        return Ok(None);
    };
    let path = picked
        .into_path()
        .map_err(|e| format!("Could not use this location: {e}"))?;
    let file = ExportFile {
        app: backup::APP_NAME.into(),
        format: backup::FORMAT,
        version: app.package_info().version.to_string(),
        exported_at: Local::now().to_rfc3339(),
        reminders: lock(&state.reminders).clone(),
        daily_notes: lock(&state.notes).clone(),
        categories: lock(&state.settings).categories.clone(),
    };
    let json = serde_json::to_string_pretty(&file)
        .map_err(|e| format!("Could not create the backup: {e}"))?;
    // Never replace an existing file's only copy: keep it as a .bak when overwriting.
    storage::write_atomic(&path, &json, true)?;
    Ok(Some(path.display().to_string()))
}

/// Asks for a backup file and prepares an import. Nothing is changed yet:
/// the preview is shown, and `import_apply` runs only after the user confirms.
#[tauri::command]
async fn import_pick(app: AppHandle, state: AppState<'_>) -> Result<Option<ImportPreview>, String> {
    let picked = app
        .dialog()
        .file()
        .set_title("Choose a Kartavirya backup")
        .add_filter("Kartavirya backup", &["json"])
        .blocking_pick_file();
    let Some(picked) = picked else {
        return Ok(None);
    };
    let path = picked
        .into_path()
        .map_err(|e| format!("Could not use this file: {e}"))?;
    let size = std::fs::metadata(&path)
        .map_err(|e| format!("Could not read the file: {e}"))?
        .len();
    if size > backup::MAX_FILE_BYTES {
        return Err("This file is too large to be a Kartavirya backup.".into());
    }
    let data =
        std::fs::read_to_string(&path).map_err(|e| format!("Could not read the file: {e}"))?;
    let file = backup::parse_export(&data)?;
    let plan = backup::plan_import(
        &lock(&state.reminders),
        &lock(&state.notes),
        &lock(&state.settings).categories,
        file,
    );
    let preview = plan.preview();
    *lock(&state.pending_import) = Some(plan);
    Ok(Some(preview))
}

/// Adds what the preview promised. Existing data is not changed or replaced.
#[tauri::command]
fn import_apply(app: AppHandle, state: AppState) -> Result<ImportPreview, String> {
    let plan = lock(&state.pending_import)
        .take()
        .ok_or_else(|| "There is no import waiting. Please choose the file again.".to_string())?;
    let preview = plan.preview();

    if !plan.add_categories.is_empty() {
        let mut s = lock(&state.settings).clone();
        s.categories.extend(plan.add_categories.clone());
        settings::validate(&s)?;
        storage::save(&state.settings_path, &s)?;
        *lock(&state.settings) = s.clone();
        let _ = app.emit("settings-changed", s);
    }
    if !plan.add_notes.is_empty() {
        let add = plan.add_notes.clone();
        mutate_notes(&app, &state, move |notes| {
            for (day, text) in add {
                notes.entry(day).or_insert(text);
            }
            Ok(())
        })?;
    }
    if !plan.add.is_empty() {
        let add = plan.add;
        mutate(&app, &state, move |list| {
            list.extend(add);
            Ok(())
        })?;
    }
    Ok(preview)
}

#[tauri::command]
fn cancel_import(state: AppState) {
    *lock(&state.pending_import) = None;
}

// ------------------------------------------------------- background timer

/// Tells the user about reminders that fired: notification and alert window.
fn announce(app: &AppHandle, shared: &Shared, fired: Vec<Reminder>) {
    let _ = app.emit("reminder-updated", ());
    for item in &fired {
        let _ = app
            .notification()
            .builder()
            .title("Kartavirya – Reminder Due!")
            .body(&item.text)
            .show();
    }
    let ids: Vec<String> = fired.into_iter().map(|r| r.id).collect();
    raise_alerts(app, shared, &ids);
}

fn start_scheduler(app: AppHandle, shared: Arc<Shared>) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(StdDuration::from_secs(5));

            let fired = {
                let mut list = lock(&shared.reminders);
                let fired = schedule::process_due(&mut list, Local::now());
                if !fired.is_empty()
                    && let Err(e) = storage::save(&shared.reminders_path, &*list)
                {
                    eprintln!("Kartavirya: {e}");
                }
                fired
            };

            if !fired.is_empty() {
                announce(&app, &shared, fired);
            }
        }
    });
}

/// Runs once at start-up. One missed reminder alerts as usual; several are
/// collected into a single summary instead of a stream of alert windows.
fn handle_startup_due(app: &AppHandle, shared: &Shared) {
    let fired = {
        let mut list = lock(&shared.reminders);
        let fired = schedule::process_due(&mut list, Local::now());
        if !fired.is_empty()
            && let Err(e) = storage::save(&shared.reminders_path, &*list)
        {
            eprintln!("Kartavirya: {e}");
        }
        fired
    };
    match fired.len() {
        0 => {}
        1 => announce(app, shared, fired),
        n => {
            let _ = app
                .notification()
                .builder()
                .title("Kartavirya – Missed reminders")
                .body(format!(
                    "{n} reminders came due while Kartavirya was closed."
                ))
                .show();
            *lock(&shared.missed) = fired;
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be first: a second launch just brings the running app forward.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_window(app, "main");
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let handle = app.handle().clone();

            // %APPDATA%\in.edu.vsa.kartavirya\
            let app_data = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            let reminders_path = app_data.join("reminders.json");
            let notes_path = app_data.join("daily-notes.json");
            let settings_path = app_data.join("settings.json");

            let mut warnings = Vec::new();
            let loaded = storage::load::<Vec<Reminder>>(&reminders_path, "reminders");
            warnings.extend(loaded.warning);
            let notes = storage::load::<BTreeMap<String, String>>(&notes_path, "daily notes");
            warnings.extend(notes.warning);
            let loaded_settings = storage::load::<Settings>(&settings_path, "settings");
            warnings.extend(loaded_settings.warning);

            let shared = Arc::new(Shared {
                reminders: Mutex::new(loaded.value),
                notes: Mutex::new(notes.value),
                settings: Mutex::new(loaded_settings.value.clone()),
                reminders_path,
                notes_path,
                settings_path,
                alert_queue: Mutex::new(Vec::new()),
                warnings: Mutex::new(warnings),
                missed: Mutex::new(Vec::new()),
                pending_import: Mutex::new(None),
            });
            app.manage(Arc::clone(&shared));

            handle_startup_due(&handle, &shared);
            start_scheduler(handle.clone(), Arc::clone(&shared));

            // No startup banner if the shortcut is busy (for example when a
            // second copy starts). Errors are shown only when the user
            // changes the shortcut in Settings.
            if register_hotkeys(&handle, &loaded_settings.value.hotkey).is_err() {
                let _ = register_hotkeys(&handle, "F12");
            }

            // System tray
            let show_i = MenuItem::with_id(app, "show", "Open Dashboard", true, None::<&str>)?;
            let min_i = MenuItem::with_id(
                app,
                "min_tray",
                "Minimize to System Tray",
                true,
                None::<&str>,
            )?;
            let new_i = MenuItem::with_id(app, "new", "New Reminder", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit Kartavirya", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_i, &min_i, &new_i, &quit_i])?;

            if let Some(icon) = app.default_window_icon() {
                TrayIconBuilder::new()
                    .icon(icon.clone())
                    .tooltip("Kartavirya – Reminder Companion")
                    .menu(&tray_menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id().as_ref() {
                        "quit" => app.exit(0),
                        "show" => show_window(app, "main"),
                        "min_tray" => {
                            if let Some(win) = app.get_webview_window("main") {
                                let _ = win.hide();
                            }
                        }
                        "new" => show_popup_window(app),
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            let app = tray.app_handle();
                            if let Some(win) = app.get_webview_window("main") {
                                if win.is_visible().unwrap_or(false)
                                    && !win.is_minimized().unwrap_or(false)
                                {
                                    let _ = win.hide();
                                } else {
                                    show_window(app, "main");
                                }
                            }
                        }
                    })
                    .build(app)?;
            }

            // Main window: close hides to tray; minimize optionally hides to tray.
            if let Some(main_win) = app.get_webview_window("main") {
                let win = main_win.clone();
                let shared_events = Arc::clone(&shared);
                main_win.on_window_event(move |event| match event {
                    tauri::WindowEvent::CloseRequested { api, .. } => {
                        api.prevent_close();
                        let _ = win.hide();
                    }
                    tauri::WindowEvent::Resized(size)
                        if size.width == 0
                            && size.height == 0
                            && lock(&shared_events.settings).minimize_to_tray =>
                    {
                        let _ = win.hide();
                    }
                    _ => {}
                });
                // Started by Windows at log-in: stay in the tray.
                let hidden_start = std::env::args().any(|a| a == "--minimized");
                if !hidden_start {
                    let _ = main_win.show();
                }
            }

            // Popup and alert windows are reused, so closing them (Alt+F4) only hides them.
            if let Some(popup) = app.get_webview_window("popup") {
                let win = popup.clone();
                popup.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win.hide();
                    }
                });
            }
            if let Some(alert) = app.get_webview_window("alert") {
                let win = alert.clone();
                let shared_alert = Arc::clone(&shared);
                alert.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        lock(&shared_alert.alert_queue).clear();
                        let _ = win.hide();
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            open_link,
            get_load_warning,
            get_settings,
            save_settings,
            get_reminders,
            add_reminder,
            update_reminder,
            toggle_reminder,
            delete_reminder,
            restore_reminder,
            delete_forever,
            empty_trash,
            snooze_reminder,
            test_reminder_alert,
            play_system_sound,
            get_alert_queue,
            alert_action,
            show_popup,
            hide_popup,
            fit_popup,
            hide_main_window,
            get_daily_notes,
            set_daily_note,
            parse_quick,
            take_missed,
            export_data,
            import_pick,
            import_apply,
            cancel_import
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| eprintln!("Kartavirya could not start: {e}"));
}
