use chrono::{DateTime, Duration, Local};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration as StdDuration;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tauri_plugin_notification::NotificationExt;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reminder {
    pub id: String,
    pub text: String,
    pub created_at: String,
    pub remind_type: String, // "hourly", "today_evening", "daily", "weekly", "custom"
    pub remind_at: String,
    pub is_completed: bool,
    pub snooze_count: u32,
    #[serde(default)]
    pub last_alerted: Option<String>,
}

pub struct AppState {
    pub reminders: Arc<Mutex<Vec<Reminder>>>,
    pub storage_path: PathBuf,
}

fn load_storage(path: &PathBuf) -> Vec<Reminder> {
    if path.exists() {
        if let Ok(data) = fs::read_to_string(path) {
            if let Ok(list) = serde_json::from_str::<Vec<Reminder>>(&data) {
                return list;
            }
        }
    }
    Vec::new()
}

fn save_storage(path: &PathBuf, list: &[Reminder]) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(list) {
        let _ = fs::write(path, json);
    }
}

// Commands
#[tauri::command]
fn get_reminders(state: State<AppState>) -> Vec<Reminder> {
    let lock = state.reminders.lock().unwrap();
    lock.clone()
}

#[tauri::command]
fn add_reminder(
    app: AppHandle,
    state: State<AppState>,
    text: String,
    remind_type: String,
    remind_at: String,
) -> Result<Reminder, String> {
    let now = Local::now().to_rfc3339();
    let reminder = Reminder {
        id: Uuid::new_v4().to_string(),
        text,
        created_at: now,
        remind_type,
        remind_at,
        is_completed: false,
        snooze_count: 0,
        last_alerted: None,
    };

    {
        let mut lock = state.reminders.lock().unwrap();
        lock.insert(0, reminder.clone());
        save_storage(&state.storage_path, &lock);
    }

    let _ = app.emit("reminder-updated", ());
    Ok(reminder)
}

#[tauri::command]
fn update_reminder(
    app: AppHandle,
    state: State<AppState>,
    id: String,
    text: String,
    remind_type: String,
    remind_at: String,
) -> Result<(), String> {
    {
        let mut lock = state.reminders.lock().unwrap();
        if let Some(item) = lock.iter_mut().find(|r| r.id == id) {
            item.text = text;
            item.remind_type = remind_type;
            item.remind_at = remind_at;
        }
        save_storage(&state.storage_path, &lock);
    }

    let _ = app.emit("reminder-updated", ());
    Ok(())
}

#[tauri::command]
fn toggle_reminder(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    {
        let mut lock = state.reminders.lock().unwrap();
        if let Some(item) = lock.iter_mut().find(|r| r.id == id) {
            item.is_completed = !item.is_completed;
        }
        save_storage(&state.storage_path, &lock);
    }

    let _ = app.emit("reminder-updated", ());
    Ok(())
}

#[tauri::command]
fn delete_reminder(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    {
        let mut lock = state.reminders.lock().unwrap();
        lock.retain(|r| r.id != id);
        save_storage(&state.storage_path, &lock);
    }

    let _ = app.emit("reminder-updated", ());
    Ok(())
}

#[tauri::command]
fn snooze_reminder(
    app: AppHandle,
    state: State<AppState>,
    id: String,
    minutes: u32,
) -> Result<(), String> {
    {
        let mut lock = state.reminders.lock().unwrap();
        if let Some(item) = lock.iter_mut().find(|r| r.id == id) {
            let next_time = Local::now() + Duration::minutes(minutes as i64);
            item.remind_at = next_time.to_rfc3339();
            item.snooze_count += 1;
        }
        save_storage(&state.storage_path, &lock);
    }

    let _ = app.emit("reminder-updated", ());
    Ok(())
}

#[tauri::command]
fn show_popup(app: AppHandle) -> Result<(), String> {
    if let Some(popup) = app.get_webview_window("popup") {
        let _ = popup.center();
        let _ = popup.show();
        let _ = popup.set_focus();
    }
    Ok(())
}

#[tauri::command]
fn hide_popup(app: AppHandle) -> Result<(), String> {
    if let Some(popup) = app.get_webview_window("popup") {
        let _ = popup.hide();
    }
    Ok(())
}

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn MessageBeep(u_type: u32) -> i32;
}

#[tauri::command]
fn test_reminder_alert(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    let reminder_opt = {
        let lock = state.reminders.lock().unwrap();
        lock.iter().find(|r| r.id == id).cloned()
    };

    if let Some(item) = reminder_opt {
        #[cfg(target_os = "windows")]
        unsafe {
            MessageBeep(0x00000040);
        }

        if let Some(win) = app.get_webview_window("main") {
            let _ = win.unminimize();
            let _ = win.show();
            let _ = win.set_always_on_top(true);
            let _ = win.set_focus();
            let _ = win.request_user_attention(Some(tauri::UserAttentionType::Critical));
            let _ = win.set_always_on_top(false);
        }

        let _ = app.emit("reminder-alert", &item);
    }
    Ok(())
}

#[tauri::command]
fn hide_main_window(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
    Ok(())
}

// Background scheduler
fn start_scheduler(app: AppHandle, state_reminders: Arc<Mutex<Vec<Reminder>>>, storage_path: PathBuf) {
    std::thread::spawn(move || loop {
        std::thread::sleep(StdDuration::from_secs(5));

        let now = Local::now();
        let mut updated = false;
        let mut triggered_alerts = Vec::new();

        {
            let mut lock = state_reminders.lock().unwrap();
            for reminder in lock.iter_mut() {
                if reminder.is_completed {
                    continue;
                }

                if let Ok(due_time) = DateTime::parse_from_rfc3339(&reminder.remind_at) {
                    if due_time <= now {
                        // Check if alerted within the last 60 seconds to avoid spam
                        let should_alert = match &reminder.last_alerted {
                            Some(last_str) => {
                                if let Ok(last_dt) = DateTime::parse_from_rfc3339(last_str) {
                                    now.signed_duration_since(last_dt) > Duration::seconds(60)
                                } else {
                                    true
                                }
                            }
                            None => true,
                        };

                        if should_alert {
                            reminder.last_alerted = Some(now.to_rfc3339());
                            triggered_alerts.push(reminder.clone());
                            updated = true;

                            // Calculate next recurring time if applicable
                            match reminder.remind_type.as_str() {
                                "5min" => {
                                    let next = now + Duration::minutes(5);
                                    reminder.remind_at = next.to_rfc3339();
                                }
                                "hourly" => {
                                    let next = now + Duration::hours(1);
                                    reminder.remind_at = next.to_rfc3339();
                                }
                                "daily" => {
                                    let next = now + Duration::days(1);
                                    reminder.remind_at = next.to_rfc3339();
                                }
                                "weekly" => {
                                    let next = now + Duration::weeks(1);
                                    reminder.remind_at = next.to_rfc3339();
                                }
                                _ => {
                                    // One-time / custom / today_evening: keep reminder active until user marks done or snoozes
                                }
                            }
                        }
                    }
                }
            }

            if updated {
                save_storage(&storage_path, &lock);
            }
        }

        if !triggered_alerts.is_empty() {
            let _ = app.emit("reminder-updated", ());

            #[cfg(target_os = "windows")]
            unsafe {
                MessageBeep(0x00000040); // MB_ICONASTERISK (Windows system chime)
            }

            if let Some(win) = app.get_webview_window("main") {
                let _ = win.unminimize();
                let _ = win.show();
                let _ = win.set_always_on_top(true);
                let _ = win.set_focus();
                let _ = win.request_user_attention(Some(tauri::UserAttentionType::Critical));
                let _ = win.set_always_on_top(false);
            }

            for item in triggered_alerts {
                // Windows toast notification
                let _ = app
                    .notification()
                    .builder()
                    .title("Kartavirya – Reminder Due!")
                    .body(&item.text)
                    .show();

                // In-app alert event
                let _ = app.emit("reminder-alert", &item);
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Setup storage path: %APPDATA%\Kartavirya\reminders.json
            let app_data = app.path().app_data_dir().unwrap_or_else(|_| PathBuf::from("."));
            let storage_path = app_data.join("reminders.json");
            let initial_reminders = load_storage(&storage_path);
            let reminders_arc = Arc::new(Mutex::new(initial_reminders));

            // State management
            app.manage(AppState {
                reminders: Arc::clone(&reminders_arc),
                storage_path: storage_path.clone(),
            });

            // Start background reminder checker
            start_scheduler(app_handle.clone(), Arc::clone(&reminders_arc), storage_path);

            // Register global shortcuts: F12, Ctrl+F12, Alt+F12, and Alt+K
            for shortcut_key in ["F12", "Ctrl+F12", "Alt+F12", "Alt+K"] {
                let _ = app.global_shortcut().on_shortcut(shortcut_key, move |app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        if let Some(popup) = app.get_webview_window("popup") {
                            let _ = popup.center();
                            let _ = popup.show();
                            let _ = popup.set_focus();
                        }
                    }
                });
            }

            // Setup System Tray
            let show_i = MenuItem::with_id(app, "show", "Open Dashboard", true, None::<&str>)?;
            let new_i = MenuItem::with_id(app, "new", "New Reminder (F12)", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit Kartavirya", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&show_i, &new_i, &quit_i])?;

            if let Some(icon) = app.default_window_icon() {
                let _tray = TrayIconBuilder::new()
                    .icon(icon.clone())
                    .menu(&tray_menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id().as_ref() {
                        "quit" => {
                            app.exit(0);
                        }
                        "show" => {
                            if let Some(win) = app.get_webview_window("main") {
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                        "new" => {
                            if let Some(win) = app.get_webview_window("popup") {
                                let _ = win.center();
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
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
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                    })
                    .build(app)?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_reminders,
            add_reminder,
            update_reminder,
            toggle_reminder,
            delete_reminder,
            snooze_reminder,
            test_reminder_alert,
            show_popup,
            hide_popup,
            hide_main_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
