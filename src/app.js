// Main Controller for Kartavirya – Reminder Companion
const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : async (cmd, args) => { console.log(cmd, args); return []; };
const listen = window.__TAURI__ ? window.__TAURI__.event.listen : () => {};

// State
let reminders = [];
let currentFilter = "all";
let searchQuery = "";
let activeAlertReminder = null;

// DOM Elements
const remindersList = document.getElementById("remindersList");
const emptyState = document.getElementById("emptyState");
const searchInput = document.getElementById("searchInput");
const btnClearSearch = document.getElementById("btnClearSearch");
const filterButtons = document.querySelectorAll(".filter-btn");
const countAll = document.getElementById("countAll");
const countActive = document.getElementById("countActive");
const countCompleted = document.getElementById("countCompleted");
const statusClock = document.getElementById("statusClock");

// Buttons
const btnNewReminder = document.getElementById("btnNewReminder");
const btnEmptyAdd = document.getElementById("btnEmptyAdd");
const btnHelp = document.getElementById("btnHelp");
const btnAbout = document.getElementById("btnAbout");
const btnMinimizeTray = document.getElementById("btnMinimizeTray");
const chkMinimizeToTray = document.getElementById("chkMinimizeToTray");

// Modals
const aboutOverlay = document.getElementById("aboutOverlay");
const btnCloseAbout = document.getElementById("btnCloseAbout");
const btnOkAbout = document.getElementById("btnOkAbout");

const helpOverlay = document.getElementById("helpOverlay");
const btnCloseHelp = document.getElementById("btnCloseHelp");
const btnOkHelp = document.getElementById("btnOkHelp");
const helpSearchInput = document.getElementById("helpSearchInput");
const helpNav = document.getElementById("helpNav");
const helpContentPane = document.getElementById("helpContentPane");

// Edit Modal
const editOverlay = document.getElementById("editOverlay");
const btnCloseEdit = document.getElementById("btnCloseEdit");
const btnCancelEdit = document.getElementById("btnCancelEdit");
const btnSaveEdit = document.getElementById("btnSaveEdit");
const editText = document.getElementById("editText");
const editFreqButtons = document.querySelectorAll("#editFreqButtons .freq-btn");
const editCustomPicker = document.getElementById("editCustomPicker");
const editCustomDatetime = document.getElementById("editCustomDatetime");
let editingReminderId = null;
let editSelectedFreq = "hourly";

// Alert Modal
const alertOverlay = document.getElementById("alertOverlay");
const alertTaskText = document.getElementById("alertTaskText");
const alertCreatedInfo = document.getElementById("alertCreatedInfo");
const alertTime = document.getElementById("alertTime");
const btnAlertSnooze = document.getElementById("btnAlertSnooze");
const btnAlertDismiss = document.getElementById("btnAlertDismiss");
const btnAlertComplete = document.getElementById("btnAlertComplete");

// Sound Chime using Web Audio API
function playCatchyChime() {
  try {
    const AudioCtx = window.AudioContext || window.webkitAudioContext;
    if (!AudioCtx) return;
    const ctx = new AudioCtx();
    if (ctx.state === "suspended") {
      ctx.resume();
    }
    const now = ctx.currentTime;

    // 3-note pleasant melodic alert chime (C5 -> E5 -> G5)
    const notes = [523.25, 659.25, 783.99];
    notes.forEach((freq, idx) => {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();

      osc.type = "sine";
      osc.frequency.setValueAtTime(freq, now + idx * 0.12);

      gain.gain.setValueAtTime(0.2, now + idx * 0.12);
      gain.gain.exponentialRampToValueAtTime(0.001, now + idx * 0.12 + 0.45);

      osc.connect(gain);
      gain.connect(ctx.destination);

      osc.start(now + idx * 0.12);
      osc.stop(now + idx * 0.12 + 0.5);
    });
  } catch (err) {
    console.warn("Audio chime note error:", err);
  }
}

// Clock updater
function updateClock() {
  const now = new Date();
  statusClock.textContent = now.toLocaleTimeString("en-IN", { hour: "2-digit", minute: "2-digit", second: "2-digit" });
}
setInterval(updateClock, 1000);
updateClock();

// Fetch reminders from backend
async function loadReminders() {
  try {
    const list = await invoke("get_reminders");
    reminders = Array.isArray(list) ? list : [];
    renderReminders();
  } catch (err) {
    console.error("Failed to load reminders:", err);
  }
}

// Format friendly date
function formatTimestamp(isoStr) {
  if (!isoStr) return "";
  try {
    const d = new Date(isoStr);
    return d.toLocaleString("en-IN", {
      day: "2-digit",
      month: "short",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      hour12: true
    });
  } catch {
    return isoStr;
  }
}

// Render cards
function renderReminders() {
  const query = searchQuery.toLowerCase().trim();

  const filtered = reminders.filter((item) => {
    // Filter status
    if (currentFilter === "active" && item.is_completed) return false;
    if (currentFilter === "completed" && !item.is_completed) return false;

    // Search query
    if (query && !item.text.toLowerCase().includes(query)) return false;

    return true;
  });

  // Update counts
  const total = reminders.length;
  const activeCount = reminders.filter((r) => !r.is_completed).length;
  const completedCount = total - activeCount;

  countAll.textContent = total;
  countActive.textContent = activeCount;
  countCompleted.textContent = completedCount;

  if (filtered.length === 0) {
    remindersList.innerHTML = "";
    emptyState.classList.remove("hidden");
    return;
  }

  emptyState.classList.add("hidden");

  remindersList.innerHTML = filtered.map((item) => {
    const freqLabels = {
      "5min": "In 5 Mins",
      hourly: "Hourly",
      today_evening: "Today Evening",
      daily: "Daily",
      weekly: "Weekly",
      custom: "Custom Date/Time"
    };
    const freqText = freqLabels[item.remind_type] || item.remind_type;
    const dueTime = formatTimestamp(item.remind_at);
    const createdTime = formatTimestamp(item.created_at);

    return `
      <div class="reminder-card ${item.is_completed ? "completed" : ""}" data-id="${item.id}">
        <div class="card-left">
          <input type="checkbox" class="card-checkbox" ${item.is_completed ? "checked" : ""} title="Toggle Completion" />
          <div class="card-content">
            <div class="card-text" title="Double click to edit">${escapeHtml(item.text)}</div>
            <div class="card-meta">
              <span class="freq-tag">🔄 ${freqText}</span>
              <span class="time-tag">⏰ Due: ${dueTime}</span>
              <span class="created-tag">📅 Added: ${createdTime}</span>
            </div>
          </div>
        </div>
        <div class="card-right">
          <button class="btn-card-action btn-test" title="Test Alarm Alert Now">🔔 Test</button>
          <button class="btn-card-action btn-edit" title="Edit Note or Schedule">✎ Edit</button>
          ${!item.is_completed ? `<button class="btn-card-action btn-snooze" title="Snooze 1 Hour">+1h</button>` : ""}
          <button class="btn-card-action btn-delete" title="Delete Reminder">✕</button>
        </div>
      </div>
    `;
  }).join("");

  // Attach card event listeners
  remindersList.querySelectorAll(".reminder-card").forEach((card) => {
    const id = card.dataset.id;
    const item = reminders.find((r) => r.id === id);
    const chk = card.querySelector(".card-checkbox");
    const btnDel = card.querySelector(".btn-delete");
    const btnSnooze = card.querySelector(".btn-snooze");
    const btnEdit = card.querySelector(".btn-edit");
    const btnTest = card.querySelector(".btn-test");
    const cardTextEl = card.querySelector(".card-text");

    if (chk) {
      chk.addEventListener("change", async () => {
        await invoke("toggle_reminder", { id });
        await loadReminders();
      });
    }

    if (btnTest) {
      btnTest.addEventListener("click", async () => {
        await invoke("test_reminder_alert", { id });
      });
    }

    if (btnEdit && item) {
      btnEdit.addEventListener("click", () => openEditModal(item));
    }

    if (cardTextEl && item) {
      cardTextEl.addEventListener("dblclick", () => openEditModal(item));
    }

    if (btnDel) {
      btnDel.addEventListener("click", async () => {
        await invoke("delete_reminder", { id });
        await loadReminders();
      });
    }

    if (btnSnooze) {
      btnSnooze.addEventListener("click", async () => {
        await invoke("snooze_reminder", { id, minutes: 60 });
        await loadReminders();
      });
    }
  });
}

function escapeHtml(str) {
  if (!str) return "";
  return str.replace(/[&<>'"]/g, 
    tag => ({
      '&': '&amp;',
      '<': '&lt;',
      '>': '&gt;',
      "'": '&#39;',
      '"': '&quot;'
    }[tag] || tag)
  );
}

// Search handling
searchInput.addEventListener("input", (e) => {
  searchQuery = e.target.value;
  if (searchQuery) {
    btnClearSearch.classList.remove("hidden");
  } else {
    btnClearSearch.classList.add("hidden");
  }
  renderReminders();
});

btnClearSearch.addEventListener("click", () => {
  searchInput.value = "";
  searchQuery = "";
  btnClearSearch.classList.add("hidden");
  renderReminders();
  searchInput.focus();
});

// Filters
filterButtons.forEach((btn) => {
  btn.addEventListener("click", () => {
    filterButtons.forEach((b) => b.classList.remove("active"));
    btn.classList.add("active");
    currentFilter = btn.dataset.filter;
    renderReminders();
  });
});

// New reminder button triggers popup window
async function triggerQuickEntry() {
  try {
    await invoke("show_popup");
  } catch (err) {
    console.error("Failed to show popup:", err);
  }
}

btnNewReminder.addEventListener("click", triggerQuickEntry);
btnEmptyAdd.addEventListener("click", triggerQuickEntry);

if (btnMinimizeTray) {
  btnMinimizeTray.addEventListener("click", async () => {
    try {
      await invoke("hide_main_window");
    } catch (err) {
      console.error("Failed to hide to tray:", err);
    }
  });
}

if (chkMinimizeToTray) {
  const savedMinToTray = localStorage.getItem("kartavirya_min_to_tray") === "true";
  chkMinimizeToTray.checked = savedMinToTray;
  if (savedMinToTray && window.__TAURI__) {
    invoke("set_minimize_to_tray", { enabled: true }).catch(() => {});
  }

  chkMinimizeToTray.addEventListener("change", async () => {
    const isChecked = chkMinimizeToTray.checked;
    localStorage.setItem("kartavirya_min_to_tray", isChecked ? "true" : "false");
    try {
      await invoke("set_minimize_to_tray", { enabled: isChecked });
    } catch (err) {
      console.error("Failed to update minimize_to_tray:", err);
    }
  });
}

// Alert Notification Modal
function showAlert(reminder) {
  activeAlertReminder = reminder;
  alertTaskText.textContent = reminder.text;
  alertCreatedInfo.textContent = `Logged: ${formatTimestamp(reminder.created_at)}`;
  alertTime.textContent = formatTimestamp(reminder.remind_at);
  alertOverlay.classList.remove("hidden");
  playCatchyChime();
}

btnAlertDismiss.addEventListener("click", () => {
  alertOverlay.classList.add("hidden");
  activeAlertReminder = null;
});

btnAlertComplete.addEventListener("click", async () => {
  if (activeAlertReminder) {
    await invoke("toggle_reminder", { id: activeAlertReminder.id });
    await loadReminders();
  }
  alertOverlay.classList.add("hidden");
  activeAlertReminder = null;
});

btnAlertSnooze.addEventListener("click", async () => {
  if (activeAlertReminder) {
    await invoke("snooze_reminder", { id: activeAlertReminder.id, minutes: 10 });
    await loadReminders();
  }
  alertOverlay.classList.add("hidden");
  activeAlertReminder = null;
});

// About Modal
function openAbout() {
  aboutOverlay.classList.remove("hidden");
}
function closeAbout() {
  aboutOverlay.classList.add("hidden");
}
btnAbout.addEventListener("click", openAbout);
btnCloseAbout.addEventListener("click", closeAbout);
btnOkAbout.addEventListener("click", closeAbout);

// Help Modal & Documentation Topics
const helpTopics = {
  intro: {
    title: "1. Introduction & The Legend of Kartavirya",
    content: `
      <p><strong>Kartavirya – Reminder Companion</strong> is an ultra-fast desktop assistant engineered for peace of mind.</p>
      <p><strong>The Legend:</strong> King Kartavirya Arjuna was a renowned monarch of the ancient Haihayas kingdom with capital at Mahishmati. Celebrated in ancient lore as having a thousand hands (<em>Sahasrabahu</em>), tradition holds that when devotees chant his name upon misplacing car keys, spectacles, documents, or valuables, his thousand hands swiftly help recover what was lost.</p>
      <p>This software operates in the same protective spirit: whenever you leave an item somewhere or have a task to remember, simply press <strong>F12</strong> to record it. Kartavirya takes care of timely reminders so you never lose track again.</p>
    `
  },
  quick_capture: {
    title: "2. Quick Capture (F12 Hotkey)",
    content: `
      <p>Kartavirya is designed to stay out of your way until the moment you need it:</p>
      <ul>
        <li><strong>Global Hotkey (F12):</strong> Press <strong>F12</strong> from anywhere on your PC—while browsing the web, working on documents, or coding.</li>
        <li><strong>Instant Focus:</strong> The compact Quick Capture popup immediately appears with the text cursor ready.</li>
        <li><strong>Escape Key (Esc):</strong> Press <kbd>Esc</kbd> to discard the entry immediately without saving anything.</li>
        <li><strong>Enter Key (Enter):</strong> Press <kbd>Enter</kbd> to save the note with automatic date & timestamp and selected schedule, then close the box.</li>
      </ul>
    `
  },
  keyboard: {
    title: "3. Keyboard Navigation Guide",
    content: `
      <p>Kartavirya is built for maximum keyboard efficiency:</p>
      <ul>
        <li><kbd>F12</kbd>: Summon Quick Capture popup globally.</li>
        <li><kbd>Enter</kbd>: Save reminder in popup.</li>
        <li><kbd>Esc</kbd>: Cancel/close popup or any open dialog (About, Help, Alert).</li>
        <li><kbd>Alt + 1</kbd> to <kbd>Alt + 5</kbd>: Switch reminder frequency in popup (Hourly, Today Evening, Daily, Weekly, Custom).</li>
        <li><kbd>Tab</kbd> / <kbd>Shift+Tab</kbd>: Navigate controls cleanly without touching the mouse.</li>
      </ul>
    `
  },
  misplaced: {
    title: "4. Tracking Misplaced Items",
    content: `
      <p>How to use Kartavirya for locating frequently misplaced belongings:</p>
      <ul>
        <li>Whenever you place an important item in an unusual place, press <kbd>F12</kbd> and note it: <em>"Car spare keys in master bedroom top drawer"</em>.</li>
        <li>Use the search bar on the main dashboard to search anytime: type <em>"keys"</em> or <em>"drawer"</em> to immediately recall the location.</li>
        <li>Mark completed once retrieved.</li>
      </ul>
    `
  },
  schedules: {
    title: "5. Frequencies & Alert Engine",
    content: `
      <p>Kartavirya features a built-in background scheduler:</p>
      <ul>
        <li><strong>Hourly:</strong> Re-alerts every hour until completed.</li>
        <li><strong>Today Evening:</strong> Prompts at 8:00 PM today.</li>
        <li><strong>Daily:</strong> Alerts every morning at 9:00 AM.</li>
        <li><strong>Weekly:</strong> Reminds you on the same day every week.</li>
        <li><strong>Custom:</strong> Choose an exact calendar date and time.</li>
        <li><strong>Catchy Alerts:</strong> When a task is due, a high-visibility animated card appears with an audio chime and Windows notification.</li>
      </ul>
    `
  },
  tray: {
    title: "6. System Tray & Minimizing Options",
    content: `
      <p>Kartavirya provides multiple flexible ways to minimize and keep running in the background:</p>
      <ul>
        <li><strong>Minimize to Tray Button:</strong> Click the <strong>📥 Minimize to Tray</strong> button in the top-right header to instantly vanish the app from the Windows taskbar into the system tray.</li>
        <li><strong>Escape Key (<kbd>Esc</kbd>):</strong> Pressing <kbd>Esc</kbd> while viewing the dashboard cleanly minimizes the app to the tray.</li>
        <li><strong>Close Button (<kbd>×</kbd>):</strong> Clicking the standard window close button hides the window to the tray, keeping your reminder timers and global <kbd>F12</kbd> shortcut active.</li>
        <li><strong>Minimize Button Sends to Tray (Option):</strong> Check the <em>"Minimize button sends to Tray"</em> checkbox in the bottom status bar if you want clicking the standard Windows minimize button (<kbd>_</kbd>) to also vanish the app to the system tray instead of staying on the taskbar.</li>
        <li><strong>System Tray Icon:</strong> Left-clicking the Kartavirya icon in the Windows notification area toggles the window between open and hidden. Right-clicking brings up quick options: <em>Open Dashboard</em>, <em>Minimize to System Tray</em>, <em>New Reminder (F12)</em>, and <em>Quit Kartavirya</em>.</li>
      </ul>
    `
  },
  version_history: {
    title: "7. Version History",
    content: `
      <p><strong>Version 1.1.0 (Active Milestone - November 2026):</strong></p>
      <ul>
        <li><strong>Minimize to System Tray Options:</strong> Dedicated header button, system tray menu item, and status-bar toggle allowing the app button to vanish from the taskbar into the notification tray.</li>
        <li><strong>Installer Icon:</strong> Native app icon cleanly integrated into NSIS <code>setup.exe</code> installer for Windows Explorer.</li>
        <li><strong>Dual-Channel Audio & Alert Window:</strong> Fail-safe Windows <code>MessageBeep</code> alert and top-level floating reminder window.</li>
        <li><strong>UI Synchronicity:</strong> Unified badge shortcuts (<kbd>1</kbd> to <kbd>6</kbd>) and synchronized modal layouts across Quick Capture and Edit dialogs.</li>
      </ul>
      <p><strong>Version 1.0.0 (Initial Release - November 2026):</strong></p>
      <ul>
        <li>Lightweight, distraction-free light blue UI.</li>
        <li>Global system-wide hotkey (<kbd>F12</kbd>) for instant task capture.</li>
        <li>Keyboard-first workflow (<kbd>Enter</kbd> to save, <kbd>Esc</kbd> to discard).</li>
        <li>Background scheduler engine with periodic reminder verification.</li>
        <li>Catchy audio chimes and in-app animated alert modal.</li>
        <li>Search bar and filtering (All, Active, Completed).</li>
        <li>Modal About & Help dialogs meeting Vidyasagar Academy standards.</li>
      </ul>
    `
  }
};

function selectHelpTopic(topicKey) {
  const topic = helpTopics[topicKey] || helpTopics.intro;
  helpContentPane.innerHTML = `
    <h3>${topic.title}</h3>
    ${topic.content}
    <hr style="border: 0; border-top: 1px solid #e2e8f0; margin: 20px 0;" />
    <div style="font-size: 11px; color: #64748b; line-height: 1.4;">
      <strong>Version History:</strong> v1.1.0 – Tray Minimization & Alert Enhancements | v1.0.0 – Initial Release
    </div>
  `;

  helpNav.querySelectorAll(".help-nav-item").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.topic === topicKey);
  });
}

function openHelp() {
  helpOverlay.classList.remove("hidden");
  selectHelpTopic("intro");
}
function closeHelp() {
  helpOverlay.classList.add("hidden");
}
btnHelp.addEventListener("click", openHelp);
btnCloseHelp.addEventListener("click", closeHelp);
btnOkHelp.addEventListener("click", closeHelp);

helpNav.querySelectorAll(".help-nav-item").forEach((btn) => {
  btn.addEventListener("click", () => selectHelpTopic(btn.dataset.topic));
});

// Help live search
helpSearchInput.addEventListener("input", (e) => {
  const q = e.target.value.toLowerCase().trim();
  helpNav.querySelectorAll(".help-nav-item").forEach((btn) => {
    const key = btn.dataset.topic;
    const topic = helpTopics[key];
    const text = (topic.title + " " + topic.content).toLowerCase();
    if (!q || text.includes(q)) {
      btn.style.display = "block";
    } else {
      btn.style.display = "none";
    }
  });
});

// Edit Modal Logic
function openEditModal(reminder) {
  editingReminderId = reminder.id;
  editText.value = reminder.text;
  setEditFrequency(reminder.remind_type || "hourly");

  if (reminder.remind_at) {
    try {
      const d = new Date(reminder.remind_at);
      const localIso = new Date(d.getTime() - d.getTimezoneOffset() * 60000).toISOString().slice(0, 16);
      editCustomDatetime.value = localIso;
    } catch {
      // fallback
    }
  }

  editOverlay.classList.remove("hidden");
  setTimeout(() => editText.focus(), 50);
}

function closeEditModal() {
  editOverlay.classList.add("hidden");
  editingReminderId = null;
}

function setEditFrequency(freq) {
  editSelectedFreq = freq;
  editFreqButtons.forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.freq === freq);
  });
  if (freq === "custom") {
    editCustomPicker.classList.remove("hidden");
  } else {
    editCustomPicker.classList.add("hidden");
  }
}

editFreqButtons.forEach((btn) => {
  btn.addEventListener("click", () => setEditFrequency(btn.dataset.freq));
});

function calculateEditRemindAt(freq) {
  const now = new Date();
  let target = new Date();

  switch (freq) {
    case "hourly":
      target = new Date(now.getTime() + 60 * 60 * 1000);
      break;
    case "5min":
      target = new Date(now.getTime() + 5 * 60 * 1000);
      break;
    case "today_evening":
      target.setHours(20, 0, 0, 0);
      if (target <= now) {
        target.setDate(target.getDate() + 1);
        target.setHours(9, 0, 0, 0);
      }
      break;
    case "daily":
      target.setDate(target.getDate() + 1);
      target.setHours(9, 0, 0, 0);
      break;
    case "weekly":
      target.setDate(target.getDate() + 7);
      target.setHours(9, 0, 0, 0);
      break;
    case "custom":
      if (editCustomDatetime.value) {
        target = new Date(editCustomDatetime.value);
      } else {
        target = new Date(now.getTime() + 60 * 60 * 1000);
      }
      break;
    default:
      target = new Date(now.getTime() + 60 * 60 * 1000);
  }

  return target.toISOString();
}

async function saveEditModal() {
  if (!editingReminderId) return;
  const newText = editText.value.trim();
  if (!newText) return;

  const newRemindAt = calculateEditRemindAt(editSelectedFreq);

  try {
    await invoke("update_reminder", {
      id: editingReminderId,
      text: newText,
      remindType: editSelectedFreq,
      remindAt: newRemindAt
    });
    closeEditModal();
    await loadReminders();
  } catch (err) {
    console.error("Failed to update reminder:", err);
  }
}

btnSaveEdit.addEventListener("click", saveEditModal);
btnCloseEdit.addEventListener("click", closeEditModal);
btnCancelEdit.addEventListener("click", closeEditModal);

editText.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    saveEditModal();
  }
});

// Global Keyboard Handler (F12 and Esc)
window.addEventListener("keydown", async (e) => {
  // Inside Edit modal, Alt + 1-6 switches frequency
  if (!editOverlay.classList.contains("hidden") && e.altKey) {
    if (e.key === "1") { e.preventDefault(); setEditFrequency("hourly"); return; }
    if (e.key === "2") { e.preventDefault(); setEditFrequency("5min"); return; }
    if (e.key === "3") { e.preventDefault(); setEditFrequency("today_evening"); return; }
    if (e.key === "4") { e.preventDefault(); setEditFrequency("daily"); return; }
    if (e.key === "5") { e.preventDefault(); setEditFrequency("weekly"); return; }
    if (e.key === "6") { e.preventDefault(); setEditFrequency("custom"); return; }
  }

  // F12: Open Quick Capture popup from inside the main window
  if (e.key === "F12") {
    e.preventDefault();
    triggerQuickEntry();
    return;
  }

  // Escape Key: Close modals first, or hide main window to system tray
  if (e.key === "Escape") {
    if (!editOverlay.classList.contains("hidden")) {
      closeEditModal();
    } else if (!alertOverlay.classList.contains("hidden")) {
      alertOverlay.classList.add("hidden");
    } else if (!aboutOverlay.classList.contains("hidden")) {
      closeAbout();
    } else if (!helpOverlay.classList.contains("hidden")) {
      closeHelp();
    } else {
      // No dialog is open: hide the main window to the system tray!
      try {
        await invoke("hide_main_window");
      } catch (err) {
        console.error("Failed to hide main window:", err);
      }
    }
  }
});

// Listen to backend events
if (window.__TAURI__) {
  // Listen for newly added reminders from popup
  listen("reminder-updated", () => {
    loadReminders();
  });

  // Listen for scheduled reminder alarms
  listen("reminder-alert", (event) => {
    if (event.payload) {
      showAlert(event.payload);
    }
  });
}

// Initial load
document.addEventListener("DOMContentLoaded", () => {
  loadReminders();
});
