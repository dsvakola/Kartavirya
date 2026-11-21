// Dedicated Top-Level Alert Controller for Kartavirya
const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : async () => {};
const listen = window.__TAURI__ ? window.__TAURI__.event.listen : () => {};

const alertTaskText = document.getElementById("alertTaskText");
const alertScheduleType = document.getElementById("alertScheduleType");
const alertAddedDate = document.getElementById("alertAddedDate");
const alertTimestamp = document.getElementById("alertTimestamp");

const btnDismiss = document.getElementById("btnDismiss");
const btnSnooze10 = document.getElementById("btnSnooze10");
const btnComplete = document.getElementById("btnComplete");

let currentReminderId = null;

function playAlarmTone() {
  try {
    const AudioCtx = window.AudioContext || window.webkitAudioContext;
    if (!AudioCtx) return;
    const ctx = new AudioCtx();
    if (ctx.state === "suspended") {
      ctx.resume();
    }
    const now = ctx.currentTime;
    const notes = [587.33, 739.99, 880.00]; // Vibrant D5, F#5, A5 chime
    notes.forEach((freq, idx) => {
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = "sine";
      osc.frequency.setValueAtTime(freq, now + idx * 0.14);
      gain.gain.setValueAtTime(0.35, now + idx * 0.14);
      gain.gain.exponentialRampToValueAtTime(0.001, now + idx * 0.14 + 0.55);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start(now + idx * 0.14);
      osc.stop(now + idx * 0.14 + 0.6);
    });
  } catch (err) {
    console.warn("Audio tone error:", err);
  }
}

function formatTime(isoStr) {
  if (!isoStr) return "";
  try {
    const d = new Date(isoStr);
    return d.toLocaleString("en-IN", {
      day: "2-digit",
      month: "short",
      hour: "2-digit",
      minute: "2-digit",
      hour12: true
    });
  } catch {
    return isoStr;
  }
}

function displayAlarm(reminder) {
  currentReminderId = reminder.id;
  alertTaskText.textContent = reminder.text;
  
  const freqLabels = {
    "5min": "In 5 Mins",
    hourly: "Hourly",
    today_evening: "Today Evening",
    daily: "Daily",
    weekly: "Weekly",
    custom: "Custom Date/Time"
  };
  alertScheduleType.textContent = "🔄 " + (freqLabels[reminder.remind_type] || reminder.remind_type);
  alertAddedDate.textContent = "Added: " + formatTime(reminder.created_at);
  alertTimestamp.textContent = "Due: " + formatTime(reminder.remind_at);

  playAlarmTone();
}

async function completeAndClose() {
  if (currentReminderId) {
    try {
      await invoke("toggle_reminder", { id: currentReminderId });
    } catch (err) {
      console.error(err);
    }
  }
  await dismissAndClose();
}

async function snoozeAndClose() {
  if (currentReminderId) {
    try {
      await invoke("snooze_reminder", { id: currentReminderId, minutes: 10 });
    } catch (err) {
      console.error(err);
    }
  }
  await dismissAndClose();
}

async function dismissAndClose() {
  try {
    await invoke("hide_alert_window");
  } catch (err) {
    console.error(err);
  }
}

btnComplete.addEventListener("click", completeAndClose);
btnSnooze10.addEventListener("click", snoozeAndClose);
btnDismiss.addEventListener("click", dismissAndClose);

window.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    e.preventDefault();
    dismissAndClose();
  } else if (e.key === "Enter") {
    e.preventDefault();
    completeAndClose();
  }
});

if (window.__TAURI__) {
  listen("show-alarm-modal", (event) => {
    if (event.payload) {
      displayAlarm(event.payload);
    }
  });
}
