// Help window: topics, live search, version history.
import { openOverlay, closeOverlay } from "./ui.js";

export const helpTopics = {
  intro: {
    title: "1. Introduction & The Legend of Kartavirya",
    content: `
      <p><strong>Kartavirya – Reminder Companion</strong> is a fast desktop assistant for tasks, misplaced items and notes.</p>
      <p><strong>The Legend:</strong> King Kartavirya Arjuna was a renowned monarch of the ancient Haihayas kingdom with capital at Mahishmati. Celebrated in ancient lore as having a thousand hands (<em>Sahasrabahu</em>), tradition holds that when devotees chant his name upon misplacing car keys, spectacles, documents, or valuables, his thousand hands swiftly help recover what was lost.</p>
      <p>This software works in the same spirit: whenever you leave an item somewhere or have a task to remember, press your Quick Capture shortcut (<strong>F12</strong> by default) and write it down. Kartavirya takes care of the reminders.</p>
    `
  },
  quick_capture: {
    title: "2. Quick Capture",
    content: `
      <p>Press the Quick Capture shortcut from anywhere on your PC (default <kbd>F12</kbd>; change it in <em>Settings</em>).</p>
      <ul>
        <li><strong>Type naturally.</strong> Kartavirya finds the time in your words and fills the schedule for you. Examples: <em>Call client at 4 PM</em>, <em>Pay fees tomorrow</em>, <em>Check oven in 20 min</em>, <em>Meeting on Friday 10:30</em>, <em>Medicine every day at 8 pm</em>, <em>Birthday on 25 Dec</em>. A blue line shows what was detected; click the × to ignore it, or pick a schedule yourself.</li>
        <li><strong>Reminder or Note only.</strong> A <em>Reminder</em> alerts you. A <em>Note only</em> is kept and searchable but never alerts (<kbd>Alt+N</kbd>).</li>
        <li><strong>Category.</strong> Choose a coloured category, or press <kbd>Alt+C</kbd> to cycle through them.</li>
        <li><kbd>Enter</kbd> saves, <kbd>Esc</kbd> discards. The window grows to fit its content, so the buttons are always visible.</li>
      </ul>
    `
  },
  keyboard: {
    title: "3. Keyboard Shortcuts",
    content: `
      <p>Kartavirya can be used without the mouse.</p>
      <ul>
        <li><kbd>F12</kbd> (or your own shortcut): open Quick Capture from anywhere.</li>
        <li><kbd>F1</kbd> Help · <kbd>Ctrl+,</kbd> Settings · <kbd>Ctrl+N</kbd> New · <kbd>Ctrl+F</kbd> Search.</li>
        <li><kbd>Ctrl+1</kbd> List · <kbd>Ctrl+2</kbd> Calendar · <kbd>Ctrl+3</kbd> Today's Note.</li>
        <li>In the list: <kbd>↑</kbd> <kbd>↓</kbd> move between items, <kbd>Home</kbd> / <kbd>End</kbd> jump, <kbd>Space</kbd> marks done, <kbd>Enter</kbd> edits, <kbd>Delete</kbd> moves to Recently Deleted.</li>
        <li>In the calendar: arrow keys move between days, <kbd>PgUp</kbd> / <kbd>PgDn</kbd> change month, <kbd>Ctrl+Enter</kbd> saves the daily note.</li>
        <li>Schedule shortcuts in Quick Capture and Edit: <kbd>Alt+1</kbd> Every Hour, <kbd>Alt+2</kbd> In 5 Mins, <kbd>Alt+3</kbd> Today Evening, <kbd>Alt+4</kbd> Daily, <kbd>Alt+5</kbd> Weekly, <kbd>Alt+6</kbd> Custom.</li>
        <li>Alert window: <kbd>Enter</kbd> marks done, <kbd>Esc</kbd> dismisses.</li>
        <li><kbd>Esc</kbd> closes the open dialog; with no dialog open it sends the window to the tray.</li>
      </ul>
    `
  },
  misplaced: {
    title: "4. Tracking Misplaced Items",
    content: `
      <ul>
        <li>Whenever you put something in an unusual place, press <kbd>F12</kbd> and note it: <em>"Car spare keys in master bedroom top drawer"</em>. Choose <em>Note only</em> if you do not need an alert.</li>
        <li>Use the search bar to find it later: type <em>"keys"</em> or <em>"drawer"</em>.</li>
        <li>Mark it done once you have found it.</li>
      </ul>
    `
  },
  schedules: {
    title: "5. Schedules, Snooze & Alerts",
    content: `
      <ul>
        <li><strong>Every Hour</strong> repeats each hour. <strong>Daily</strong> and <strong>Weekly</strong> repeat at the time set in Settings (default 9:00 AM).</li>
        <li><strong>In 5 Mins</strong>, <strong>Today Evening</strong> (default 8:00 PM) and <strong>Custom</strong> alert once. Afterwards they show <em>Overdue</em> until you mark them done or snooze them.</li>
        <li>Repeating reminders keep their schedule. If the PC was off, you get one alert when it starts, and the next one is at the normal time again.</li>
        <li><strong>Alert window:</strong> appears on top with a sound and a Windows notification. Choose <em>Mark Done</em>, <em>Snooze</em> (your default length, or 5, 15, 30 minutes, 1 or 2 hours, or tomorrow morning) or <em>Dismiss</em>.</li>
        <li><strong>Missed reminders:</strong> if two or more reminders came due while Kartavirya was closed, you see one summary instead of many alert windows.</li>
        <li><strong>Editing</strong> only the text keeps the due time. The time changes only when you pick a different schedule.</li>
      </ul>
    `
  },
  lists: {
    title: "6. Groups, Categories & Notes",
    content: `
      <ul>
        <li>The list is grouped into <strong>Overdue</strong>, <strong>Today</strong>, <strong>Upcoming</strong>, <strong>Notes</strong> and <strong>Done</strong>.</li>
        <li><strong>Categories</strong> have a colour and an icon. Add, rename or remove them in Settings. Removing a category never removes your reminders. Use the category menu next to the filters to show one category.</li>
        <li><strong>Notes only</strong> are items without alerts, for things you only want to keep and search.</li>
        <li><strong>Recently Deleted</strong> keeps deleted items until you remove them permanently. Everything can be restored from there.</li>
      </ul>
    `
  },
  calendar: {
    title: "7. Calendar & Daily Notes",
    content: `
      <ul>
        <li>The <strong>Calendar</strong> shows each month. A green tick with a number shows how many items you finished that day, with small coloured dots for their categories.</li>
        <li>Click a day to see what was done, the notes you saved that day, and to write that day's <strong>daily note</strong>. Press <em>Today's Note</em> (<kbd>Ctrl+3</kbd>) to jump to today.</li>
        <li>A daily note is saved when you press <em>Save</em>, <kbd>Ctrl+Enter</kbd>, or move away. Emptying a note asks first.</li>
        <li>Items finished before version 1.2.0 have no finish date, so they do not appear on past days.</li>
      </ul>
    `
  },
  settings: {
    title: "8. Settings & Backup",
    content: `
      <ul>
        <li><strong>Quick Capture shortcut:</strong> click the box and press the new keys, for example <kbd>Ctrl+Shift+K</kbd>. If another program already uses it, Kartavirya tells you and keeps the old one.</li>
        <li><strong>Times:</strong> evening time, daily time and snooze length.</li>
        <li><strong>Start with Windows:</strong> starts Kartavirya quietly in the system tray when you sign in.</li>
        <li><strong>Appearance:</strong> follow Windows light/dark, or choose light or dark.</li>
        <li><strong>Alert sound:</strong> choose a sound and volume, and press <em>Test</em>.</li>
        <li><strong>Export</strong> saves all reminders, notes, daily notes and categories to one file. <strong>Import</strong> reads such a file, shows a preview, and adds only what is missing. Nothing you already have is changed or replaced.</li>
        <li>Your data is in <code>%APPDATA%\\in.edu.vsa.kartavirya</code>. Each file keeps a <code>.bak.json</code> copy of its previous version.</li>
      </ul>
    `
  },
  tray: {
    title: "9. System Tray & Minimizing",
    content: `
      <ul>
        <li><strong>Minimize to Tray</strong> hides the window into the system tray; reminders and the shortcut keep working.</li>
        <li><kbd>Esc</kbd> on the dashboard and the window close button (×) also hide to the tray.</li>
        <li>The normal minimize button (_) also sends Kartavirya to the tray. You can turn this off in <em>Settings</em>.</li>
        <li>Left-click the tray icon to show or hide the window; right-click for <em>Open Dashboard</em>, <em>Minimize to System Tray</em>, <em>New Reminder</em> and <em>Quit Kartavirya</em>.</li>
      </ul>
    `
  },
  version_history: {
    title: "10. Version History",
    content: `
      <p><strong>Version 1.2.1 (November 2026):</strong></p>
      <ul>
        <li>No more startup message about F12 already being registered; shortcut errors now appear only when you change the shortcut in Settings.</li>
        <li>Release files now carry the version number in their names.</li>
      </ul>
      <p><strong>Version 1.2.0 (November 2026):</strong></p>
      <ul>
        <li>The minimize button now goes to the system tray by default; the status bar shows the developer and version.</li>
        <li>Fixed: the Quick Capture window cut off its bottom buttons. It now fits its content.</li>
        <li><strong>Settings window:</strong> custom shortcut, times, snooze length, start with Windows, appearance, sound, categories, backup.</li>
        <li><strong>Natural-language capture</strong> ("call client at 4 pm", "pay fees tomorrow").</li>
        <li><strong>Categories</strong> with colours and icons; <strong>notes-only</strong> items; <strong>daily notes</strong>; <strong>month calendar</strong> of what was done.</li>
        <li>List grouped into Overdue, Today, Upcoming, Notes and Done; keyboard navigation.</li>
        <li>Snooze choices in the alert window; alert sound choice, volume and test; <strong>missed reminders</strong> summary.</li>
        <li><strong>Dark mode</strong> that follows Windows; crisp vector icons everywhere.</li>
        <li><strong>Export / Import</strong> with preview, never overwriting.</li>
      </ul>
      <p><strong>Version 1.1.1 (Reliability Fixes - November 2026):</strong></p>
      <ul>
        <li>Safe storage with automatic backup, safe delete with Undo and Recently Deleted.</li>
        <li>Floating alert window with Mark Done, Snooze and Dismiss.</li>
        <li>One-time reminders alert once; daily and weekly reminders no longer drift.</li>
        <li>Only one copy of Kartavirya can run at a time.</li>
      </ul>
      <p><strong>Version 1.1.0 (November 2026):</strong></p>
      <ul>
        <li>Minimize to System Tray options, installer icon, alert window and sound.</li>
      </ul>
      <p><strong>Version 1.0.0 (Initial Release - November 2026):</strong></p>
      <ul>
        <li>Global Quick Capture shortcut, background scheduler, search and filters, About and Help.</li>
      </ul>
    `
  }
};

const helpOverlay = document.getElementById("helpOverlay");
const helpSearchInput = document.getElementById("helpSearchInput");
const helpNav = document.getElementById("helpNav");
const helpContentPane = document.getElementById("helpContentPane");

function buildNav() {
  helpNav.innerHTML = Object.entries(helpTopics)
    .map(([key, t], i) => `<button class="help-nav-item${i === 0 ? " active" : ""}" data-topic="${key}">${t.title}</button>`)
    .join("");
  helpNav.querySelectorAll(".help-nav-item").forEach((btn) => {
    btn.addEventListener("click", () => selectHelpTopic(btn.dataset.topic));
  });
}

export function selectHelpTopic(key) {
  const topic = helpTopics[key] || helpTopics.intro;
  helpContentPane.innerHTML = `
    <h3>${topic.title}</h3>
    ${topic.content}
    <hr />
    <div class="help-history"><strong>Version History:</strong> v1.2.1 – Quieter Startup &amp; Versioned Files | v1.2.0 – Settings, Categories, Calendar &amp; Notes | v1.1.1 – Reliability Fixes | v1.1.0 – Tray &amp; Alert Enhancements | v1.0.0 – Initial Release</div>
  `;
  helpContentPane.scrollTop = 0;
  helpNav.querySelectorAll(".help-nav-item").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.topic === key);
  });
}

export function openHelp() {
  helpSearchInput.value = "";
  filterNav("");
  openOverlay(helpOverlay, closeHelp, helpSearchInput);
  selectHelpTopic("intro");
}

export function closeHelp() {
  closeOverlay(helpOverlay);
}

function filterNav(q) {
  helpNav.querySelectorAll(".help-nav-item").forEach((btn) => {
    const topic = helpTopics[btn.dataset.topic];
    const text = (topic.title + " " + topic.content.replace(/<[^>]+>/g, " ")).toLowerCase();
    btn.classList.toggle("hidden-by-search", Boolean(q) && !text.includes(q));
  });
}

buildNav();
helpSearchInput.addEventListener("input", (e) => filterNav(e.target.value.toLowerCase().trim()));
document.getElementById("btnCloseHelp").addEventListener("click", closeHelp);
document.getElementById("btnOkHelp").addEventListener("click", closeHelp);
