// Floating alert window for Kartavirya.
// Rust keeps the queue of due reminders; this window shows the first one.
import { invoke, listen, tauri, bootSettings, categoryById, categoryChip, formatShort, FREQ_LABELS } from "./common.js";
import { hydrateIcons, icon } from "./icons.js";
import { playSound } from "./sound.js";

const $ = (id) => document.getElementById(id);
const alertTaskText = $("alertTaskText");
const alertScheduleType = $("alertScheduleType");
const alertCategory = $("alertCategory");
const alertAddedDate = $("alertAddedDate");
const alertTimestamp = $("alertTimestamp");
const alertQueueInfo = $("alertQueueInfo");
const snoozeLabel = $("snoozeLabel");
const snoozeMore = $("snoozeMore");

let settings = null;
let currentReminder = null;
let busy = false;

hydrateIcons();

function minutesText(m) {
  if (m % 1440 === 0) return `${m / 1440}d`;
  if (m % 60 === 0) return `${m / 60}h`;
  return `${m}m`;
}

function drawSnoozeLabel() {
  if (settings) snoozeLabel.textContent = `Snooze ${minutesText(settings.snoozeMinutes)}`;
}

async function refreshQueue() {
  let queue = [];
  try {
    queue = await invoke("get_alert_queue");
  } catch (err) {
    console.error(err);
  }
  if (!Array.isArray(queue) || queue.length === 0) {
    currentReminder = null;
    return;
  }
  const reminder = queue[0];
  const isNew = !currentReminder || reminder.id !== currentReminder.id;
  currentReminder = reminder;

  alertTaskText.textContent = reminder.text;
  alertScheduleType.innerHTML = `${icon("repeat")}${FREQ_LABELS[reminder.remind_type] || reminder.remind_type}`;
  alertCategory.innerHTML = categoryChip(categoryById(settings, reminder.category));
  alertAddedDate.textContent = "Added: " + formatShort(reminder.created_at);
  alertTimestamp.textContent = "Due Now";
  alertQueueInfo.textContent = queue.length > 1 ? `1 of ${queue.length} reminders` : "";
  snoozeMore.value = "";

  if (isNew && settings) playSound(settings.sound, settings.volume);
}

async function act(action, minutes) {
  if (!currentReminder || busy) return;
  busy = true;
  try {
    await invoke("alert_action", { id: currentReminder.id, action, minutes: minutes ?? null });
  } catch (err) {
    console.error(err);
  }
  currentReminder = null;
  busy = false;
  await refreshQueue();
}

$("btnComplete").addEventListener("click", () => act("complete"));
$("btnSnooze").addEventListener("click", () => act("snooze"));
$("btnDismiss").addEventListener("click", () => act("dismiss"));
snoozeMore.addEventListener("change", () => {
  const v = snoozeMore.value;
  if (!v) return;
  if (v === "tomorrow") act("snooze_tomorrow");
  else act("snooze", Number(v));
});

window.addEventListener("keydown", (e) => {
  if (e.target === snoozeMore) return;
  if (e.key === "Escape") {
    e.preventDefault();
    act("dismiss");
  } else if (e.key === "Enter") {
    e.preventDefault();
    act("complete");
  }
});

async function start() {
  settings = await bootSettings((s) => {
    settings = s;
    drawSnoozeLabel();
  });
  drawSnoozeLabel();
  if (tauri) listen("alerts-changed", () => refreshQueue());
  await refreshQueue();
}
start();
