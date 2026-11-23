// Quick Capture window.
import { invoke, listen, tauri, bootSettings, renderCategoryPicker, formatShort, toLocalInput } from "./common.js";
import { hydrateIcons } from "./icons.js";

const $ = (id) => document.getElementById(id);
const taskInput = $("taskInput");
const liveTimestamp = $("liveTimestamp");
const freqButtons = document.querySelectorAll("#freqButtons .freq-btn");
const customTimePicker = $("customTimePicker");
const customDatetime = $("customDatetime");
const scheduleSection = $("scheduleSection");
const kindButtons = document.querySelectorAll("#kindSwitch button");
const categoryPick = $("categoryPick");
const categoryRow = $("categoryRow");
const detectedHint = $("detectedHint");
const detectedText = $("detectedText");
const popupError = $("popupError");
const btnSave = $("btnSave");
const btnCancel = $("btnCancel");

let settings = null;
let kind = "reminder";
let selectedFreq = "hourly";
let manualFreq = false; // the user picked a schedule by hand
let detected = null; // { remindType, remindAt } found in the text
let ignoreDetected = false;
let category = null;
let parseTimer = null;
let lastFit = 0;

hydrateIcons();

function updateTime() {
  liveTimestamp.textContent = new Date().toLocaleDateString("en-IN", {
    day: "2-digit", month: "short", year: "numeric", hour: "2-digit", minute: "2-digit", hour12: true
  });
}
setInterval(updateTime, 1000);
updateTime();

// ---- Window size: always as tall as the content needs, so no button is cut off ----
function fit() {
  requestAnimationFrame(() => {
    const need =
      $("popupHeader").offsetHeight + $("popupContent").scrollHeight + $("popupFooter").offsetHeight + 24 + 16 + 4;
    if (Math.abs(need - lastFit) < 2) return;
    lastFit = need;
    invoke("fit_popup", { height: need }).catch(() => {});
  });
}

function showError(msg) {
  popupError.textContent = msg || "";
  popupError.classList.toggle("hidden", !msg);
  fit();
}

function setDefaultCustomDate() {
  const nextHour = new Date(Date.now() + 60 * 60 * 1000);
  nextHour.setMinutes(Math.ceil(nextHour.getMinutes() / 5) * 5, 0, 0);
  customDatetime.value = toLocalInput(nextHour.toISOString());
}

function setFrequency(freq) {
  selectedFreq = freq;
  freqButtons.forEach((btn) => btn.classList.toggle("active", btn.dataset.freq === freq));
  customTimePicker.classList.toggle("hidden", freq !== "custom" || kind === "note");
  fit();
}

function userPickedFrequency(freq) {
  manualFreq = true;
  detected = null;
  detectedHint.classList.add("hidden");
  setFrequency(freq);
}

freqButtons.forEach((btn) => btn.addEventListener("click", () => userPickedFrequency(btn.dataset.freq)));
customDatetime.addEventListener("input", () => {
  manualFreq = true;
  detected = null;
  detectedHint.classList.add("hidden");
});

function setKind(next) {
  kind = next;
  kindButtons.forEach((b) => b.classList.toggle("active", b.dataset.kind === next));
  const note = next === "note";
  scheduleSection.classList.toggle("hidden", note);
  customTimePicker.classList.toggle("hidden", note || selectedFreq !== "custom");
  detectedHint.classList.toggle("hidden", note || !detected);
  btnSave.textContent = note ? "Save Note" : "Save Reminder";
  fit();
}
kindButtons.forEach((b) => b.addEventListener("click", () => setKind(b.dataset.kind)));

// ---- Natural-language detection ("Call client at 4 PM", "Pay fees tomorrow") ----
const REPEAT_TEXT = { hourly: "repeats every hour", daily: "repeats daily", weekly: "repeats weekly" };

async function detect() {
  if (kind === "note" || manualFreq) return;
  const text = taskInput.value.trim();
  let found = null;
  if (text) {
    try {
      found = await invoke("parse_quick", { text });
    } catch {
      found = null;
    }
  }
  if (kind === "note" || manualFreq || taskInput.value.trim() !== text) return;
  if (found && !ignoreDetected) {
    detected = found;
    setFrequency(found.remindType);
    if (found.remindType === "custom") customDatetime.value = toLocalInput(found.remindAt);
    const extra = REPEAT_TEXT[found.remindType] ? ` · ${REPEAT_TEXT[found.remindType]}` : "";
    detectedText.textContent = `Detected: ${formatShort(found.remindAt)}${extra}`;
    detectedHint.classList.remove("hidden");
  } else {
    if (detected) setFrequency("hourly");
    detected = null;
    detectedHint.classList.add("hidden");
  }
  fit();
}

taskInput.addEventListener("input", () => {
  showError("");
  clearTimeout(parseTimer);
  parseTimer = setTimeout(detect, 180);
});

$("btnIgnoreDetected").addEventListener("click", () => {
  ignoreDetected = true;
  detected = null;
  detectedHint.classList.add("hidden");
  setFrequency("hourly");
  taskInput.focus();
});

// ---- Categories ----
function drawCategories() {
  const has = settings && settings.categories && settings.categories.length > 0;
  categoryRow.classList.toggle("hidden", !has);
  if (has) renderCategoryPicker(categoryPick, settings, category, (id) => (category = id));
  fit();
}

function cycleCategory() {
  const list = [null, ...((settings && settings.categories) || []).map((c) => c.id)];
  category = list[(list.indexOf(category) + 1) % list.length];
  drawCategories();
}

// ---- Save / discard ----
function buildRemindAt() {
  if (kind === "note") return { remindType: "none", remindAt: null };
  if (selectedFreq === "custom") {
    if (!customDatetime.value) throw new Error("Please choose the alert date and time.");
    return { remindType: "custom", remindAt: new Date(customDatetime.value).toISOString() };
  }
  if (detected && detected.remindType === selectedFreq && !manualFreq) {
    return { remindType: selectedFreq, remindAt: detected.remindAt };
  }
  return { remindType: selectedFreq, remindAt: null };
}

async function hide() {
  try {
    await invoke("hide_popup");
  } catch (err) {
    console.error("Failed to hide popup:", err);
  }
}

async function saveAndClose() {
  const text = taskInput.value.trim();
  if (!text) {
    await cancelAndClose();
    return;
  }
  try {
    const { remindType, remindAt } = buildRemindAt();
    await invoke("add_reminder", { text, kind, remindType, remindAt, category });
  } catch (err) {
    // Keep the window open and the text safe, and tell the user why.
    showError(String(err.message || err));
    return;
  }
  resetForm();
  await hide();
}

async function cancelAndClose() {
  resetForm();
  await hide();
}

function resetForm() {
  taskInput.value = "";
  kind = "reminder";
  manualFreq = false;
  detected = null;
  ignoreDetected = false;
  category = null;
  detectedHint.classList.add("hidden");
  showError("");
  setDefaultCustomDate();
  setFrequency("hourly");
  setKind("reminder");
  drawCategories();
}

btnSave.addEventListener("click", saveAndClose);
btnCancel.addEventListener("click", cancelAndClose);

window.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    e.preventDefault();
    cancelAndClose();
    return;
  }
  if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
    e.preventDefault();
    saveAndClose();
    return;
  }
  if (e.altKey) {
    const map = { 1: "hourly", 2: "5min", 3: "today_evening", 4: "daily", 5: "weekly", 6: "custom" };
    if (map[e.key] && kind === "reminder") {
      e.preventDefault();
      userPickedFrequency(map[e.key]);
    } else if (e.key.toLowerCase() === "n") {
      e.preventDefault();
      setKind(kind === "note" ? "reminder" : "note");
    } else if (e.key.toLowerCase() === "c") {
      e.preventDefault();
      cycleCategory();
    }
  }
});

function focusInput() {
  taskInput.focus();
  taskInput.select();
}

async function start() {
  settings = await bootSettings((s) => {
    settings = s;
    drawCategories();
  });
  resetForm();
  focusInput();
  fit();
}
start();

if (tauri) {
  listen("popup-shown", () => {
    resetForm();
    focusInput();
    fit();
  });
  listen("tauri://focus", focusInput);
}
window.addEventListener("focus", focusInput);
