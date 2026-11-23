# Kartavirya – Reminder Companion

**Version 1.2.1** · Windows desktop app · Developed for Vidyasagar Academy

Kartavirya is a small Windows tray app for quick reminders. Press **F12** anywhere, type a note (a task, or where you kept your keys), pick a schedule and press **Enter**. Kartavirya alerts you on time with a floating alert window, a chime and a Windows notification.

## The legend
King Kartavirya Arjuna (*Sahasrabahu*, "the one with a thousand hands") ruled the Haihaya kingdom from Mahishmati. Tradition says that when devotees chant his name after misplacing keys, spectacles, documents or valuables, his thousand hands help recover what was lost. The app works in the same spirit: note down what you left where, or what you must do, and let Kartavirya remember it for you.

## Features

### Quick Capture
- Global shortcut, **F12** by default (changeable in Settings). F12 also keeps the extra shortcuts Ctrl+F12, Alt+F12 and Alt+K.
- **Natural-language entry**: "call client at 4 pm", "pay fees tomorrow", "check oven in 20 min", "meeting on Friday 10:30", "medicine every day at 8 pm", "birthday on 25 Dec". The detected schedule is shown and can be ignored or changed.
- **Reminder** (alerts) or **Note only** (kept and searchable, never alerts).
- Coloured **category** per item. The window fits its content, so the Save and Discard buttons are always visible.
- `Enter` saves, `Esc` discards.

### Schedules
- Every Hour, In 5 Mins, Today Evening, Daily, Weekly, Custom date and time.
- Repeating reminders keep their schedule. One-time reminders alert once, then show *Overdue* until done or snoozed.
- If the PC was off, repeating reminders alert once when it starts, then continue at their normal time.
- Editing only the text keeps the due time.

### Alerts
- Floating alert window on top of other windows, with a sound and a Windows notification.
- **Mark Done**, **Dismiss**, or **Snooze** (your default length, 5/15/30 min, 1/2 h, tomorrow morning).
- Choice of alert sound, volume control and a Test button.
- **Missed reminders**: if two or more came due while the app was closed, one summary is shown instead of many windows.

### Organising
- List grouped into **Overdue, Today, Upcoming, Notes and Done**, with search and filters.
- **Categories** with colours and vector icons (add, rename, remove; removing never deletes reminders).
- **Recently Deleted** with Restore, Delete Forever and Empty (each confirmed), plus an Undo toast.
- **Month calendar** showing what you finished each day, with **daily notes**.
- Full keyboard navigation (see below).

### Settings
Quick Capture shortcut, evening time, daily time, snooze length, start with Windows (quietly in the tray), light / dark / follow Windows, alert sound and volume, categories, minimize-to-tray behaviour.

### Backup and safety
- **Export / Import** of reminders, notes, daily notes and categories. Import shows a preview and only adds what is missing; it never overwrites.
- Atomic saves (temporary file, then rename). Each data file keeps a `.bak.json` copy of its previous version. A damaged file is moved aside as `*.corrupt-<date-time>.json`, the backup is loaded and a banner tells you.
- Only one copy of the app runs at a time.
- If the chosen shortcut is already used by another program, Settings tells you and keeps the old one. At startup a busy shortcut falls back to F12 quietly.

### System tray
Minimize (and `Esc`, and the close button) send the app to the tray; reminders and the shortcut keep working. Left-click the tray icon to show or hide; right-click for Open Dashboard, Minimize to System Tray, New Reminder and Quit.

## Keyboard shortcuts
| Keys | Action |
|---|---|
| `F12` (or your own) | Quick Capture from anywhere |
| `F1` | Help |
| `Ctrl+,` | Settings |
| `Ctrl+N` / `Ctrl+F` | New reminder / Search |
| `Ctrl+1` / `Ctrl+2` / `Ctrl+3` | List / Calendar / Today's note |
| `↑` `↓` `Home` `End` | Move in the list |
| `Space` / `Enter` / `Delete` | Mark done / Edit / Move to Recently Deleted |
| `PgUp` / `PgDn`, arrows | Calendar month / day |
| `Ctrl+Enter` | Save daily note |
| `Alt+1` … `Alt+6` | Every Hour, In 5 Mins, Today Evening, Daily, Weekly, Custom |
| `Alt+N` / `Alt+C` | Note only / Cycle category (Quick Capture) |
| `Enter` / `Esc` in alert | Mark done / Dismiss |

## Data
Stored only on your PC in `%APPDATA%\in.edu.vsa.kartavirya\`: `reminders.json`, `daily-notes.json` and `settings.json`, each with a `.bak.json` copy. Nothing is sent over the network. See `privacy-policy.md`.

## Technology
Tauri v2 with a Rust backend (scheduler, storage, natural-language parser, settings, backup), a plain HTML/CSS/JavaScript frontend, Windows WebView2 (no bundled browser), strict CSP and least-privilege capabilities. 46 unit tests (`cargo test`); `cargo fmt` and `cargo clippy -D warnings` clean. Package manager: **pnpm** only.

## Build
Requirements: Rust (stable, MSVC target), Node.js (build tools only) and pnpm.

Double-click `build.bat`. It reads the version from `src-tauri\tauri.conf.json`, runs `pnpm install` and `pnpm tauri build`, and puts these files in the `Release` folder:
- `Kartavirya-<version>-setup.exe` – installer with desktop shortcut
- `Kartavirya-<version>-portable.exe` – standalone portable version

For development: `pnpm install` then `pnpm tauri dev`.

## Version history
- **1.2.1** – No startup banner when F12 is busy; Release files carry the version number in their names.
- **1.2.0** – Settings window, natural-language capture, categories, notes-only items, daily notes, month calendar, snooze choices, alert sounds, missed-reminders summary, dark mode, export/import with preview, tray minimize by default.
- **1.1.1** – Reliability fixes: safe storage and backups, Undo and Recently Deleted, floating alert window, no schedule drift, single instance.
- **1.1.0** – Minimize-to-tray options, installer icon, alert window and sound.
- **1.0.0** – Initial release.

## Branches
- `main` – project overview (this README) and licence.
- `v1.2.1` – latest version with full source code.
- `v1.1.1` – previous version: reliability fixes.
- `v1.1.0` – earlier version: tray options and alert window.

## About
Developed by: Prof. Dattaraj S. Vidyasagar & Prof. Yash D. Vidyasagar, for Vidyasagar Academy.
Website: https://vsa.edu.in/software/ · Email: softinquiry@vsa.edu.in
Licence: GNU General Public License v3.0 (see `LICENSE`). Privacy policy: `privacy-policy.md`.
Icons: Lucide (ISC licence).
