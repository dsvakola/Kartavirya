# Kartavirya – Reminder Companion

A small Windows tray app for quick reminders. Press **F12** anywhere, type a note (a task, or where you kept your keys), pick a schedule and press **Enter**. Kartavirya alerts you on time with a floating alert window, a chime and a Windows notification.

## Features
- Global quick capture (F12 by default, changeable in Settings). Type naturally: "call client at 4 pm", "pay fees tomorrow", "check oven in 20 min". The Quick Capture window fits its content, so its buttons are always visible.
- Reminders and **notes-only** items; coloured **categories** with vector icons.
- List grouped into Overdue, Today, Upcoming, Notes and Done; full keyboard navigation.
- **Month calendar** of what was done each day, with **daily notes**.
- Floating alert window with Mark Done, Dismiss and snooze choices (default length, 5/15/30 min, 1/2 h, tomorrow morning); alert sound choice, volume and test.
- Summary of reminders missed while the app was closed.
- Settings window: shortcut, times, snooze length, start with Windows, light/dark/follow Windows, sound, categories.
- Export and Import (with preview; import never overwrites).
- Safe storage: atomic saves, automatic backups, Recently Deleted with restore.
- Minimize goes to the system tray by default; only one copy runs at a time.

## Data
Stored only on your PC in `%APPDATA%\in.edu.vsa.kartavirya\`: `reminders.json`, `daily-notes.json` and `settings.json`, each with a `.bak.json` copy of its previous version.

## Build
Requirements: Rust (stable, MSVC), Node.js and pnpm.

Double-click `build.bat`. It runs `pnpm install` and `pnpm tauri build`, and puts `setup.exe` and `portable.exe` in the `Release` folder.

## About
Developed by: Prof. Dattaraj S. Vidyasagar & Prof. Yash D. Vidyasagar, for Vidyasagar Academy.
Website: https://vsa.edu.in/software/ · Email: softinquiry@vsa.edu.in
Licence: GNU General Public License v3.0 (see `LICENSE`). Privacy policy: `privacy-policy.md`.
Icons: Lucide (ISC licence).
