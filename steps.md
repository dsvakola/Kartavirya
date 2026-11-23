# Development Steps Log – Kartavirya

## Project Details
- **Software Name:** Kartavirya – Reminder Companion
- **Authors:** Prof. Dattaraj S. Vidyasagar & Prof. Yash D. Vidyasagar (Vidyasagar Academy)
- **Repository:** https://github.com/dsvakola/Kartavirya
- **License:** GNU General Public License v3.0 (GPL-3.0)

---

## Version 1.0.0 (Ready for Build)
- **Step 1:** Initialized Git repository on `main` branch with remote origin pointing to `https://github.com/dsvakola/Kartavirya.git`.
- **Step 2:** Formulated and created `plan.md` outlining the architecture, F12 popup flow, Esc/Enter keyboard interactions, light blue aesthetics, alert mechanism, and Vidyasagar Academy standards.
- **Step 3:** Received user approval with requirement refinement: relaxed Rule 12 (no menu bar) for a simplified, lightweight, keyboard-first design with two dedicated buttons for About and Help.
- **Step 4:** Bootstrapped Tauri v2 project using `pnpm` and Vanilla JS/HTML/CSS for instant startup and low memory footprint.
- **Step 5:** Generated candidate app icons. User reviewed and approved the **Calendar Alert** icon design.
- **Step 6:** Generated multi-resolution Windows icons (`icon.ico`, `icon.png`, and Store logos) using Tauri icon pipeline.
- **Step 7:** Implemented global shortcut (`F12`) using `tauri-plugin-global-shortcut`.
- **Step 8:** Implemented Quick Capture popup (`popup.html`, `popup.js`, `popup.css`):
  - Auto-focused input field on summon.
  - `Esc` key cancels and dismisses without saving.
  - `Enter` key automatically captures current date & timestamp and selected reminder frequency (Hourly, Today Evening, Daily, Weekly, Custom), saves to local JSON storage, and dismisses popup.
  - `Alt + 1` to `Alt + 5` for rapid keyboard frequency selection.
- **Step 9:** Implemented Main Dashboard (`index.html`, `app.js`, `styles.css`):
  - Light blue palette (`#f0f7ff`) with high-contrast typography.
  - Only two header buttons for **About** and **Help** (no menu bar).
  - Search bar and status filters (All, Active, Completed).
  - In-app animated alert modal with catchy audio chime (synthesized Web Audio API) and Windows notification.
  - Status bar with live clock and background listener status.
- **Step 10:** Implemented Vidyasagar Academy modal dialogs:
  - **About Window:** Displays logo (`assets/logo.png`), author attribution (`Prof. Dattaraj S. Vidyasagar and Prof. Yash D. Vidyasagar, for Vidyasagar Academy`), clickable Privacy Policy link, email (`softinquiry@vsa.edu.in`), and website (`https://vsa.edu.in/`).
  - **Help Window:** Search bar at the top, left-pane categorized topic index, right-pane detailed instructions, and Version History.
- **Step 11:** Configured Git author identity (`Prof. Dattaraj S. Vidyasagar` / `softinquiry@vsa.edu.in`).
- **Step 12:** Created automated `build.bat` script producing only two files in `Release/`: `setup.exe` (NSIS installer with desktop shortcut) and `portable.exe` (standalone executable).
- **Step 13:** User tested v1.0.0 and provided feedback:
  1. Add reminder editing capability to update tasks and notes.
  2. Enhance F12 / global shortcut support.
  3. Allow `Esc` to hide/minimize the main window to the system tray.
- **Step 14:** Implemented requested improvements:
  - Added `update_reminder` Rust command and interactive Edit modal (with double-click and ✎ Edit button).
  - Enhanced F12 handling with in-window listener and multiple global shortcut fallback triggers (`F12`, `Ctrl+F12`, `Alt+F12`, `Alt+K`).
  - Added `hide_main_window` Rust command; `Esc` on main window now minimizes/hides the app cleanly to the system tray.
- **Step 15:** UI Synchronicity Refinement:
  - Synchronized schedule selection buttons in the Edit Reminder dialog to exactly match the Quick Capture popup (styled pill buttons with `[ 1 ]`, `[ 2 ]`, `[ 3 ]`, `[ 4 ]`, `[ 5 ]` badge numbers, active blue states, identical typography, padding, and focus states).
  - Harmonized all input boxes and dialog buttons (`btn-primary`, `btn-secondary`) across all modals.
  - Added `Alt + 1-5` frequency shortcuts inside the Edit dialog for keyboard synchronicity.
- **Step 16:** Investigated reminder notification issue reported by user:
  - Root Cause:
    1. Tasks with "Hourly" schedule are scheduled for `+1 hour` into the future (e.g. Added at 8:19 PM -> Due at 9:19 PM; at 8:45 PM only 26 minutes had elapsed).
    2. For earlier due tasks (e.g., Task 3 added at 7:26 PM and due at 8:26 PM), the background scheduler triggered at 8:26 PM, but because the app was hidden/minimized to the tray, Chromium suspended `AudioContext` and the window remained hidden, silently auto-rescheduling to the next day (`21 Nov 2026, 08:26 PM`).
  - Solutions Implemented:
    1. Foreground Pop-up: When any reminder triggers, the Rust backend automatically unminimizes, shows, and brings the main window right in front of the user with `request_user_attention(Critical)` (flashing taskbar).
    2. Native Windows Audio: Added direct Win32 `MessageBeep(0x40)` so a loud, clear, pleasant system notification chime sounds directly through Windows even if all windows are minimized or WebView audio is sleeping.
    3. Added `🔔 Test` button on every reminder card to instantly test the audio chime and animated alert modal on demand.
    4. Added `[ 2 ] In 5 Mins` quick reminder frequency option in both Quick Capture and Edit dialogs for easy short-term reminders and fast testing.

---

## Version 1.1.0 (Active Milestone - Branch `v1.1.0`)
- **Step 1:** User initiated modification request per Rule 8. Created and switched to dedicated Git branch `v1.1.0`.
- **Step 2:** Bumped version to `1.1.0` in `package.json`, `Cargo.toml`, and `tauri.conf.json`.
- **Step 3:** Overcame Windows environmental notification barriers:
  - Created dedicated top-level floating Alarm Window (`alert.html`, `alert.css`, `alert.js`) configured with `alwaysOnTop: true`.
  - Added OS-level sleep resistance and foreground lock bypass (`SetWindowPos`, `SetForegroundWindow`, `AttachThreadInput`).
  - Implemented dual-channel audio: direct Win32 hardware `MessageBeep` / kernel tones + Web Audio API synthesizer.
- **Step 4:** Added Minimize to System Tray capability:
  - Added header action button `[ 📥 To Tray ]` and `Esc` shortcut to vanish the app from the taskbar into the system tray.
  - Intercepted window `CloseRequested` event so clicking `[X]` hides to tray instead of killing background timers.
- **Step 5:** Fixed NSIS installer icon: explicitly bound `installerIcon: "icons/icon.ico"` in `tauri.conf.json` so `setup.exe` displays the Calendar Alert icon in Windows Explorer.
- **Step 6:** Enhanced `build.bat` with automated process termination to prevent "Access is denied" file-lock errors.
- **Step 7:** System Tray & Taskbar Minimization Options:
  - Added prominent header button `[ 📥 Minimize to Tray ]` to instantly vanish the window and taskbar button into the system tray while remaining active.
  - Added status bar setting toggle `☑ Minimize button sends to Tray` allowing users to choose whether the standard Windows minimize button (`_`) keeps the app in the taskbar or sends it directly into the system tray.
  - Added "Minimize to System Tray" option in system tray context menu, with left-click toggle (show / hide / restore).
  - Wired `get_minimize_to_tray` and `set_minimize_to_tray` commands in Rust backend with state synchronization and `localStorage` persistence.
  - Added Topic 6 in Help window explaining all system tray options and updated Version History at the bottom of the Help page.
- **Step 8:** NSIS Setup Installer Icon Resolution:
  - Explicitly bound `installerIcon: "icons/icon.ico"` and `uninstallerIcon: "icons/icon.ico"` in `tauri.conf.json`.
  - Updated `build.bat` with reverse-date sorting (`dir /o-d`) and pre-build cleanup of `bundle\nsis\*.exe` to prevent stale version installer collisions.
  - Verified extracted icon MD5 checksum between `setup.exe` and `portable.exe` (`1BC2722F5DB2814C5038C9A225356666`) confirming 100% icon fidelity.
  - Freshly built `Release/` folder with exactly two deliverables: `setup.exe` and `portable.exe`.

---

## Version 1.1.1 (Reliability Fixes)
- Safe storage: atomic save (temp file + rename), previous file kept as `reminders.bak.json`, a damaged file is moved to `reminders.corrupt-<date-time>.json` and the backup is loaded; a warning banner tells the user.
- Safe delete: confirmation dialog, Undo toast, and a "Recently Deleted" filter with Restore, Delete Forever and Empty (each confirmed).
- Floating alert window wired in Rust (alert queue, `get_alert_queue`, `alert_action`): Mark Done, Snooze 10m, Dismiss; Esc/Enter keys.
- One-time reminders (In 5 Mins, Today Evening, Custom) alert once, then show "Overdue".
- Hourly/daily/weekly next time counted from the scheduled slot; snooze keeps the schedule.
- Editing only the text keeps the due time.
- About reads the version from tauri.conf.json via Rust; links open through Rust (fixed privacy, website, e-mail only).
- Single-instance plugin; popup/alert windows hide instead of closing on Alt+F4.
- Correct Alt+1..6 shortcut labels; F1 opens Help.
- Strict CSP, least-privilege capability (core only), no `unwrap()` in command code, template leftovers removed, README rewritten, `pnpm install` added to build.bat.
- Core logic split into `schedule.rs` and `storage.rs` with 20 `cargo test` tests.

---

## Version 1.2.0 (New Additions & Improvements)
- Fixed the Quick Capture box: the window had a fixed height, so wrapped content hid the Discard and Save buttons. Layout is now header / scrolling middle / footer, and the window resizes to fit (`fit_popup`). Alert window got the same layout.
- Settings window (settings.json): custom hotkey, evening/daily times, snooze length, start with Windows, theme, alert sound and volume, categories.
- Natural-language quick capture (`nlp.rs`), notes-only items, categories (colour + SVG icon), daily notes (`daily-notes.json`), month calendar, missed-reminders summary, grouped list, keyboard navigation, snooze choices, export/import with preview.
- Dark mode following Windows; all emoji replaced by inline SVG icons.
- Minimize goes to tray by default; status bar shows "Developed by" and the version.
- Rust: `settings.rs`, `nlp.rs`, `backup.rs` added; `storage.rs` made generic. 46 `cargo test` tests; fmt and clippy clean.

---

## Version 1.2.1 (Small Improvements)
- Removed the startup banner about F12 already being registered (e.g. when a second copy starts). If the saved shortcut is busy at startup, F12 is tried quietly. Errors show only when the shortcut is changed in Settings.
- `build.bat` names the Release files with the version from `tauri.conf.json`: `Kartavirya-<version>-setup.exe` and `Kartavirya-<version>-portable.exe`.
