// Popup controller for Kartavirya Quick Capture
const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : async (cmd, args) => { console.log(cmd, args); };

const taskInput = document.getElementById("taskInput");
const liveTimestamp = document.getElementById("liveTimestamp");
const freqButtons = document.querySelectorAll(".freq-btn");
const customTimePicker = document.getElementById("customTimePicker");
const customDatetime = document.getElementById("customDatetime");
const btnSave = document.getElementById("btnSave");
const btnCancel = document.getElementById("btnCancel");

let selectedFreq = "hourly";

// Update live time display
function updateTime() {
  const now = new Date();
  const options = { day: "2-digit", month: "short", year: "numeric", hour: "2-digit", minute: "2-digit", hour12: true };
  liveTimestamp.textContent = now.toLocaleDateString("en-IN", options);
}
setInterval(updateTime, 1000);
updateTime();

// Initialize custom date picker with default (+1 hour)
function setDefaultCustomDate() {
  const nextHour = new Date(Date.now() + 60 * 60 * 1000);
  nextHour.setMinutes(Math.ceil(nextHour.getMinutes() / 5) * 5);
  const localIso = new Date(nextHour.getTime() - nextHour.getTimezoneOffset() * 60000).toISOString().slice(0, 16);
  customDatetime.value = localIso;
}
setDefaultCustomDate();

// Frequency button handlers
function setFrequency(freq) {
  selectedFreq = freq;
  freqButtons.forEach((btn) => {
    if (btn.dataset.freq === freq) {
      btn.classList.add("active");
    } else {
      btn.classList.remove("active");
    }
  });

  if (freq === "custom") {
    customTimePicker.classList.remove("hidden");
  } else {
    customTimePicker.classList.add("hidden");
  }
}

freqButtons.forEach((btn) => {
  btn.addEventListener("click", () => setFrequency(btn.dataset.freq));
});

// Calculate scheduled reminder target time
function calculateRemindAt(freq) {
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
      if (customDatetime.value) {
        target = new Date(customDatetime.value);
      } else {
        target = new Date(now.getTime() + 60 * 60 * 1000);
      }
      break;
    default:
      target = new Date(now.getTime() + 60 * 60 * 1000);
  }

  return target.toISOString();
}

// Save action
async function saveAndClose() {
  const text = taskInput.value.trim();
  if (!text) {
    // Empty text, just close without saving
    await cancelAndClose();
    return;
  }

  const remindAt = calculateRemindAt(selectedFreq);

  try {
    await invoke("add_reminder", {
      text: text,
      remindType: selectedFreq,
      remindAt: remindAt
    });
  } catch (err) {
    console.error("Failed to add reminder:", err);
  }

  taskInput.value = "";
  setFrequency("hourly");
  await cancelAndClose();
}

// Cancel action (dismiss without saving)
async function cancelAndClose() {
  taskInput.value = "";
  try {
    await invoke("hide_popup");
  } catch (err) {
    console.error("Failed to hide popup:", err);
  }
}

btnSave.addEventListener("click", saveAndClose);
btnCancel.addEventListener("click", cancelAndClose);

// Keyboard controls
window.addEventListener("keydown", (e) => {
  // Esc: Discard & Close
  if (e.key === "Escape") {
    e.preventDefault();
    cancelAndClose();
    return;
  }

  // Enter (without Shift): Save & Close
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    saveAndClose();
    return;
  }

  // Quick number keys (1-6) with Alt key
  if (e.altKey) {
    if (e.key === "1") { e.preventDefault(); setFrequency("hourly"); }
    if (e.key === "2") { e.preventDefault(); setFrequency("5min"); }
    if (e.key === "3") { e.preventDefault(); setFrequency("today_evening"); }
    if (e.key === "4") { e.preventDefault(); setFrequency("daily"); }
    if (e.key === "5") { e.preventDefault(); setFrequency("weekly"); }
    if (e.key === "6") { e.preventDefault(); setFrequency("custom"); }
  }
});

// Auto focus on input when window gains focus
window.addEventListener("focus", () => {
  taskInput.focus();
  taskInput.select();
});

if (window.__TAURI__) {
  window.__TAURI__.event.listen("tauri://focus", () => {
    taskInput.focus();
    taskInput.select();
  });
}

document.addEventListener("DOMContentLoaded", () => {
  taskInput.focus();
});
