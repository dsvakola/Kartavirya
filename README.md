# Kartavirya – Reminder Companion

**Version 1.1.1 (Reliability Fixes)** · Windows desktop app · Tauri v2 + Rust

Press **F12** anywhere, type a task or where you kept something, pick a schedule and press **Enter**. Kartavirya reminds you with a floating alert window, a chime and a Windows notification. The latest version is on branch `v1.2.1`.

## Features
- Global Quick Capture (F12 and Ctrl+F12, Alt+F12, Alt+K) with schedules: Every Hour, In 5 Mins, Today Evening, Daily, Weekly, Custom.
- Floating alert window: Mark Done, Snooze 10 min, Dismiss; `Esc` / `Enter` keys.
- One-time reminders alert once, then show *Overdue*. Hourly, daily and weekly reminders are counted from the scheduled slot, so they do not drift; snooze keeps the schedule.
- Search and filters (All, Active, Completed); editing only the text keeps the due time.
- **Safe storage:** atomic save, previous file kept as `reminders.bak.json`, a damaged file is moved to `reminders.corrupt-<date-time>.json` and the backup is loaded, with a warning banner.
- **Safe delete:** confirmation, Undo toast, and a *Recently Deleted* filter with Restore, Delete Forever and Empty.
- Minimize to system tray; only one copy runs at a time.
- About and Help windows (searchable help with index); F1 opens Help.
- Strict CSP and least-privilege capabilities; 20 `cargo test` tests.

## Data
Stored only on your PC in `%APPDATA%\in.edu.vsa.kartavirya\` (`reminders.json` with `.bak.json` copy). Nothing is sent over the network. See `privacy-policy.md`.

## Build
Requirements: Rust (stable, MSVC), Node.js and pnpm. Double-click `build.bat`; it runs `pnpm install` and `pnpm tauri build` and puts `setup.exe` and `portable.exe` in the `Release` folder.

## Version history
- **1.1.1** – Reliability fixes (this branch).
- **1.1.0** – Minimize-to-tray options, installer icon, alert window and sound.
- **1.0.0** – Initial release.

## Branches
`main` overview · `v1.2.1` latest · `v1.1.1` (this branch) · `v1.1.0` earlier.

## About
Developed by: Prof. Dattaraj S. Vidyasagar & Prof. Yash D. Vidyasagar, for Vidyasagar Academy.
Website: https://vsa.edu.in/software/ · Email: softinquiry@vsa.edu.in
Licence: GNU General Public License v3.0 (see `LICENSE`). Privacy policy: `privacy-policy.md`.
Icons: Lucide (ISC licence).
