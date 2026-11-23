mod schedule;
mod storage;

use crate::schedule::Reminder;
use chrono::Local;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration as StdDuration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;
use uuid::Uuid;

const PRIVACY_URL: &str = "https://github.com/dsvakola/Kartavirya/blob/main/privacy-policy.md";
const WEBSITE_URL: &str = "https://vsa.edu.in/";
const EMAIL_URL: &str = "mailto:softinquiry@vsa.edu.in";

pub struct Shared {
    reminders: Mutex<Vec<Reminder>>,
    storage_path: PathBuf,
    minimize_to_tray: Mutex<bool>,
    /// Ids of reminders waiting in the floating alert window, oldest first.
    alert_queue: Mutex<Vec<String>>,
    load_warning: Mutex<Option<String>>,
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
        if let Err(e) = storage::save(&shared.storage_path, &list) {
            // Keep memory and disk in step: undo the change if it could not be saved.
            *list = before;
            return Err(e);
        }
        result
    };
    let _ = app.emit("reminder-updated", ());
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
    beep();
    if let Some(win) = app.get_webview_window("alert") {
        let _ = win.center();
        let _ = win.show();
        let _ = win.set_always_on_top(true);
        let _ = win.set_focus();
        let _ = win.request_user_attention(Some(tauri::UserAttentionType::Critical));
    }
    let _ = app.emit_to("alert", "alerts-changed", ());
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
    lock(&state.load_warning).take()
}

#[tauri::command]
fn get_reminders(state: AppState) -> Vec<Reminder> {
    lock(&state.reminders).clone()
}

#[tauri::command]
fn add_reminder(
    app: AppHandle,
    state: AppState,
    text: String,
    remind_type: String,
    remind_at: String,
) -> Result<Reminder, String> {
    schedule::validate(&text, &remind_type, Some(&remind_at))?;
    let reminder = Reminder {
        id: Uuid::new_v4().to_string(),
        text: text.trim().to_string(),
        created_at: Local::now().to_rfc3339(),
        remind_type,
        remind_at,
        is_completed: false,
        snooze_count: 0,
        last_alerted: None,
        base_at: None,
        deleted_at: None,
    };
    let copy = reminder.clone();
    mutate(&app, &state, move |list| {
        list.insert(0, copy);
        Ok(())
    })?;
    Ok(reminder)
}

#[tauri::command]
fn update_reminder(
    app: AppHandle,
    state: AppState,
    id: String,
    text: String,
    remind_type: String,
    remind_at: Option<String>,
) -> Result<(), String> {
    schedule::validate(&text, &remind_type, remind_at.as_deref())?;
    mutate(&app, &state, |list| {
        let r = find(list, &id)?;
        schedule::apply_edit(r, text.trim().to_string(), remind_type, remind_at);
        Ok(())
    })
}

#[tauri::command]
fn toggle_reminder(app: AppHandle, state: AppState, id: String) -> Result<(), String> {
    let completed = mutate(&app, &state, |list| {
        let r = find(list, &id)?;
        r.is_completed = !r.is_completed;
        Ok(r.is_completed)
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
        schedule::snooze(find(list, &id)?, minutes, Local::now());
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

/// Reminders currently shown in the floating alert window.
#[tauri::command]
fn get_alert_queue(state: AppState) -> Vec<Reminder> {
    let ids = lock(&state.alert_queue).clone();
    let list = lock(&state.reminders);
    ids.iter()
        .filter_map(|id| list.iter().find(|r| &r.id == id).cloned())
        .collect()
}

/// Handles a button in the floating alert window: "dismiss", "snooze" or "complete".
#[tauri::command]
fn alert_action(app: AppHandle, state: AppState, id: String, action: String) -> Result<(), String> {
    match action.as_str() {
        "dismiss" => {}
        "snooze" => {
            mutate(&app, &state, |list| {
                schedule::snooze(find(list, &id)?, 10, Local::now());
                Ok(())
            })?;
        }
        "complete" => {
            mutate(&app, &state, |list| {
                find(list, &id)?.is_completed = true;
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

#[tauri::command]
fn hide_main_window(app: AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}

#[tauri::command]
fn get_minimize_to_tray(state: AppState) -> bool {
    *lock(&state.minimize_to_tray)
}

#[tauri::command]
fn set_minimize_to_tray(state: AppState, enabled: bool) {
    *lock(&state.minimize_to_tray) = enabled;
}

// ------------------------------------------------------- background timer

fn start_scheduler(app: AppHandle, shared: Arc<Shared>) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(StdDuration::from_secs(5));

            let fired = {
                let mut list = lock(&shared.reminders);
                let fired = schedule::process_due(&mut list, Local::now());
                if !fired.is_empty()
                    && let Err(e) = storage::save(&shared.storage_path, &list)
                {
                    eprintln!("Kartavirya: {e}");
                }
                fired
            };

            if fired.is_empty() {
                continue;
            }

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
            raise_alerts(&app, &shared, &ids);
        }
    });
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
        .setup(|app| {
            let handle = app.handle().clone();

            // %APPDATA%\in.edu.vsa.kartavirya\reminders.json
            let app_data = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| PathBuf::from("."));
            let storage_path = app_data.join("reminders.json");
            let loaded = storage::load(&storage_path);

            let shared = Arc::new(Shared {
                reminders: Mutex::new(loaded.list),
                storage_path,
                minimize_to_tray: Mutex::new(false),
                alert_queue: Mutex::new(Vec::new()),
                load_warning: Mutex::new(loaded.warning),
            });
            app.manage(Arc::clone(&shared));

            start_scheduler(handle.clone(), Arc::clone(&shared));

            for key in ["F12", "Ctrl+F12", "Alt+F12", "Alt+K"] {
                let _ = app
                    .global_shortcut()
                    .on_shortcut(key, move |app, _shortcut, event| {
                        if event.state() == ShortcutState::Pressed {
                            show_popup_window(app);
                        }
                    });
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
            let new_i = MenuItem::with_id(app, "new", "New Reminder (F12)", true, None::<&str>)?;
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
                            && *lock(&shared_events.minimize_to_tray) =>
                    {
                        let _ = win.hide();
                    }
                    _ => {}
                });
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
            get_alert_queue,
            alert_action,
            show_popup,
            hide_popup,
            hide_main_window,
            get_minimize_to_tray,
            set_minimize_to_tray
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| eprintln!("Kartavirya could not start: {e}"));
}
