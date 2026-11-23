# Kartavirya – Reminder Companion

A small Windows tray app for quick reminders. Press **F12** anywhere, type a note (a task, or where you kept your keys), pick a schedule and press **Enter**. Kartavirya alerts you on time with a floating alert window, a chime and a Windows notification.

## Features
- Global quick capture: F12 (also Ctrl+F12, Alt+F12, Alt+K).
- Schedules: Every Hour, In 5 Mins, Today Evening, Daily, Weekly, Custom date/time (Alt+1 to Alt+6).
- Floating alert window with Mark Done, Snooze 10m and Dismiss.
- Search, filters, edit, snooze, and Recently Deleted with restore.
- Safe storage: atomic saves, automatic backup, damaged files are kept aside.
- Runs from the system tray; only one copy runs at a time.

## Data
Reminders are stored only on your PC in `%APPDATA%\in.edu.vsa.kartavirya\reminders.json` (backup: `reminders.bak.json`).

## Build
Requirements: Rust (stable, MSVC), Node.js and pnpm.

Double-click `build.bat`. It runs `pnpm install` and `pnpm tauri build`, and puts `setup.exe` and `portable.exe` in the `Release` folder.

## About
Developed by: Prof. Dattaraj S. Vidyasagar & Prof. Yash D. Vidyasagar, for Vidyasagar Academy.
Website: https://vsa.edu.in/software/ · Email: softinquiry@vsa.edu.in
Licence: GNU General Public License v3.0 (see `LICENSE`). Privacy policy: `privacy-policy.md`.
