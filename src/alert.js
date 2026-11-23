// Floating alert window for Kartavirya.
// Rust keeps the queue of due reminders; this window shows the first one.
const invoke = window.__TAURI__ ? window.__TAURI__.core.invoke : async () => [];
const listen = window.__TAURI__ ? window.__TAURI__.event.listen : () => {};

const alertTaskText = document.getElementById("alertTaskText");
const alertScheduleType = document.getElementById("alertScheduleType");
const alertAddedDate = document.getElementById("alertAddedDate");
const alertTimestamp = document.getElementById("alertTimestamp");
const alertQueueInfo = document.getElementById("alertQueueInfo");

const btnDismiss = document.getElementById("btnDismiss");
const btnSnooze10 = document.getElementById("btnSnooze10");
const btnComplete = document.getElementById("btnComplete");

let currentReminderId = null;
let busy = false;

const freqLabels = {
  "5min": "In 5 Mins",
  hourly: "Hourly",
  today_evening: "Today Evening",
  daily: "Daily",
  weekly: "Weekly",
  custom: "Custom Date/Time"
};

function playAlarmTone() {
  try {
    const AudioCtx = window.AudioContext || window.webkitAudioContext;
    if (!AudioCtx) return;
    const ctx = new AudioCtx();
    if (ctx.state === "suspended") ctx.resume();
    const now = ctx.currentTime;
    const notes = [587.33, 739.99, 880.0];
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
  const d = new Date(isoStr);
  if (isNaN(d.getTime())) return isoStr;
  return d.toLocaleString("en-IN", { day: "2-digit", month: "short", hour: "2-digit", minute: "2-digit", hour12: true });
}

async function refreshQueue() {
  let queue = [];
  try {
    queue = await invoke("get_alert_queue");
  } catch (err) {
    console.error(err);
  }
  if (!Array.isArray(queue) || queue.length === 0) {
    currentReminderId = null;
    return;
  }
  const reminder = queue[0];
  const isNew = reminder.id !== currentReminderId;
  currentReminderId = reminder.id;

  alertTaskText.textContent = reminder.text;
  alertScheduleType.textContent = "🔄 " + (freqLabels[reminder.remind_type] || reminder.remind_type);
  alertAddedDate.textContent = "Added: " + formatTime(reminder.created_at);
  alertTimestamp.textContent = "Due Now";
  alertQueueInfo.textContent = queue.length > 1 ? `1 of ${queue.length} reminders` : "";

  if (isNew) playAlarmTone();
}

async function act(action) {
  if (!currentReminderId || busy) return;
  busy = true;
  try {
    await invoke("alert_action", { id: currentReminderId, action });
  } catch (err) {
    console.error(err);
  }
  currentReminderId = null;
  busy = false;
  await refreshQueue();
}

btnComplete.addEventListener("click", () => act("complete"));
btnSnooze10.addEventListener("click", () => act("snooze"));
btnDismiss.addEventListener("click", () => act("dismiss"));

window.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    e.preventDefault();
    act("dismiss");
  } else if (e.key === "Enter") {
    e.preventDefault();
    act("complete");
  }
});

if (window.__TAURI__) {
  listen("alerts-changed", () => refreshQueue());
}
refreshQueue();
