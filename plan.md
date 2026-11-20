# Kartavirya – Reminder Companion: Development Plan

**Developed by:** Prof. Dattaraj S. Vidyasagar and Prof. Yash D. Vidyasagar, for Vidyasagar Academy  
**Repository:** https://github.com/dsvakola/Kartavirya  
**License:** GNU General Public License v3.0 (GPL-3.0)

---

## 1. Project Overview & Core Concept
**Kartavirya – Reminder Companion** is a fast, lightweight, and intuitive desktop companion designed to help users track misplaced items, daily to-dos, and timely reminders. Inspired by King Kartavirya Arjuna, who swiftly locates lost possessions with his thousand hands, the application ensures that no task or misplaced item is forgotten.

---

## 2. Technical Stack
- **Framework:** Tauri v2 (Rust backend + Web Frontend)
- **Package Manager:** `pnpm`
- **Frontend:** HTML5, CSS3 (Light Blue aesthetic `#eaf2fb`, `#dbeafe`), Vanilla JavaScript (ultra-lightweight, zero overhead, instant response)
- **Data Persistence:** Local JSON storage managed via Rust
- **Global Hotkey:** `tauri-plugin-global-shortcut` (F12 shortcut)
- **System Tray:** Tauri tray icon with quick actions (Show, New Reminder, Quit)
- **Alerts:** Catchy audio notification chime & custom visual toast alerts

---

## 3. Key Features & User Interaction Flow

### A. Quick Capture Popup (`F12`)
- **Global Hotkey (`F12`):** Instantly displays a compact, focused popup box in the center of the screen.
- **Auto Focus:** The input box is instantly focused on appearance.
- **Escape Key (`Esc`):** Immediately hides/closes the popup without saving anything.
- **Enter Key (`Enter`):** Automatically timestamps the current date & time, stores the task/reminder note along with the selected reminder schedule, and hides the popup.
- **Reminder Frequency Selection:**
  - One-time (at specific time or within X hours)
  - Every hour
  - Daily
  - Weekly
  - Custom

### B. Aesthetic & Visual Design
- **Theme:** Clean, modern light blue background (`#eaf2fb`, `#dbeafe`, `#1d4ed8` accents).
- **Catchy Alerts:** Distinctive, pleasing visual notification card with sound chime.
- **Maximum Keyboard Orientation:** Rapid task entry with `Enter` and quick dismiss with `Esc`.

### C. Main Dashboard & Window Architecture
- **Simplified UI (Menu Bar Omitted):** As requested by the user, the traditional top menu bar is omitted to keep the app ultra-clean and lightweight.
- **Top Actions:** Two compact buttons in the top right for **About** and **Help**.
- **Reminder List:** Displays all captured reminders with search/filter, status (active/completed), and quick actions (mark done, delete, edit).
- **Modal Dialogs:** About and Help open as modal overlays/dialogs so user context is never lost.

### D. Vidyasagar Academy Standards
- **About Box:**
  - Vidyasagar Academy logo (`E:\Radha\logo.png`).
  - Attribution: `Developed by: Prof. Dattaraj S. Vidyasagar and Prof. Yash D. Vidyasagar, for Vidyasagar Academy`.
  - Clickable link to GitHub repository Privacy Policy page.
  - Clickable email: `softinquiry@vsa.edu.in` (`mailto:softinquiry@vsa.edu.in`).
  - Clickable website: `https://vsa.edu.in/`.
- **Help Box:**
  - Search box at the top.
  - Left pane: categorized topic index (Quick Start, Keyboard Shortcuts, F12 Popup, Managing Reminders, Troubleshooting).
  - Right pane: detailed topic instructions and Version History at the bottom.
- **App Icons:** 2 distinct candidates presented for user approval.

---

## 4. Build, Packaging & Distribution
1. **Windows Deliverables (`Release/` folder):**
   - Automated `build.bat` generating:
     1. `setup.exe` (NSIS installer with desktop shortcut)
     2. `portable.exe` (standalone executable)
2. **Microsoft Store:** MSIX package ready for Partner Center submission.
3. **Linux Deliverable:** SnapCraft snap package.
