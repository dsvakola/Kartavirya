// Settings dialog: shortcut, times, appearance, sound, categories, backup.
import { escapeHtml, applyTheme } from "./common.js";
import { icon, hydrateIcons, CATEGORY_ICONS } from "./icons.js";
import { SOUND_CHOICES, playSound } from "./sound.js";
import { openOverlay, closeOverlay, askConfirm, showToast, run } from "./ui.js";

const $ = (id) => document.getElementById(id);
const overlay = $("settingsOverlay");
const setHotkey = $("setHotkey");
const hotkeyHint = $("hotkeyHint");
const setEvening = $("setEvening");
const setDaily = $("setDaily");
const setSnooze = $("setSnooze");
const setStartup = $("setStartup");
const setTray = $("setTray");
const setTheme = $("setTheme");
const setSound = $("setSound");
const setVolume = $("setVolume");
const volumeValue = $("volumeValue");
const catList = $("catList");

const COLORS = ["#2563eb", "#d97706", "#16a34a", "#dc2626", "#7c3aed", "#0891b2", "#db2777", "#65a30d"];
const HOTKEY_HELP = "Click the box and press the keys you want, for example Ctrl+Shift+K.";

let getSettings = () => ({});
let hotkeyValue = "F12";
let cats = [];
let openPickerId = null;

setSound.innerHTML = SOUND_CHOICES.map(([v, l]) => `<option value="${v}">${l}</option>`).join("");

export function initSettings(settingsGetter) {
  getSettings = settingsGetter;
  $("btnCloseSettings").addEventListener("click", closeSettings);
  $("btnCancelSettings").addEventListener("click", closeSettings);
  $("btnSaveSettings").addEventListener("click", save);
  $("btnHotkeyReset").addEventListener("click", () => setHotkeyValue("F12"));
  $("btnTestSound").addEventListener("click", () => playSound(setSound.value, Number(setVolume.value)));
  setVolume.addEventListener("input", () => (volumeValue.textContent = `${setVolume.value}%`));
  setTheme.addEventListener("change", () => applyTheme(setTheme.value));
  $("btnAddCategory").addEventListener("click", addCategory);
  $("btnExport").addEventListener("click", doExport);
  $("btnImport").addEventListener("click", doImport);
  setHotkey.addEventListener("focus", () => {
    setHotkey.classList.add("recording");
    hotkeyHint.textContent = "Press the new shortcut now… (Esc to cancel)";
    hotkeyHint.classList.remove("error");
  });
  setHotkey.addEventListener("blur", () => {
    setHotkey.classList.remove("recording");
    if (!hotkeyHint.classList.contains("error")) hotkeyHint.textContent = HOTKEY_HELP;
  });
  setHotkey.addEventListener("keydown", recordHotkey);
}

export function openSettings() {
  const s = getSettings();
  setHotkeyValue(s.hotkey);
  setEvening.value = s.eveningTime;
  setDaily.value = s.dailyTime;
  setSnooze.value = s.snoozeMinutes;
  setStartup.checked = Boolean(s.startWithWindows);
  setTray.checked = Boolean(s.minimizeToTray);
  setTheme.value = s.theme;
  setSound.value = s.sound;
  setVolume.value = s.volume;
  volumeValue.textContent = `${s.volume}%`;
  cats = (s.categories || []).map((c) => ({ ...c }));
  openPickerId = null;
  hotkeyHint.textContent = HOTKEY_HELP;
  hotkeyHint.classList.remove("error");
  drawCategories();
  openOverlay(overlay, closeSettings, $("btnSaveSettings"));
}

export function closeSettings() {
  // Undo the theme preview if the dialog is cancelled.
  applyTheme(getSettings().theme);
  closeOverlay(overlay);
}

// ---- Shortcut recorder ----
function keyName(e) {
  const c = e.code;
  if (/^Key[A-Z]$/.test(c)) return c.slice(3);
  if (/^Digit[0-9]$/.test(c)) return c.slice(5);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(c)) return c;
  const map = { Space: "Space", Enter: "Enter", Tab: "Tab", Backspace: "Backspace", Insert: "Insert", Delete: "Delete", Home: "Home", End: "End", PageUp: "PageUp", PageDown: "PageDown", ArrowUp: "Up", ArrowDown: "Down", ArrowLeft: "Left", ArrowRight: "Right" };
  return map[c] || null;
}

function recordHotkey(e) {
  e.preventDefault();
  e.stopPropagation();
  if (e.key === "Escape") {
    setHotkey.blur();
    return;
  }
  if (["Control", "Alt", "Shift", "Meta"].includes(e.key)) return;
  const key = keyName(e);
  const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean);
  if (!key) {
    return hotkeyError("That key cannot be used. Try a letter, a number or an F key.");
  }
  const isF = /^F\d+$/.test(key);
  if (!isF && mods.length === 0) {
    return hotkeyError("Please include Ctrl, Alt or Shift, so normal typing keeps working.");
  }
  setHotkeyValue([...mods, key].join("+"));
  setHotkey.blur();
}

function hotkeyError(msg) {
  hotkeyHint.textContent = msg;
  hotkeyHint.classList.add("error");
}

function setHotkeyValue(v) {
  hotkeyValue = v;
  setHotkey.value = v;
  hotkeyHint.classList.remove("error");
}

// ---- Categories ----
function newId() {
  return `cat-${Date.now().toString(36)}${Math.floor(Math.random() * 1296).toString(36)}`;
}

function addCategory() {
  if (cats.length >= 30) {
    showToast("At most 30 categories are allowed.");
    return;
  }
  const used = new Set(cats.map((c) => c.name.toLowerCase()));
  let n = cats.length + 1;
  while (used.has(`category ${n}`)) n++;
  const color = COLORS[cats.length % COLORS.length];
  cats.push({ id: newId(), name: `Category ${n}`, color, icon: "tag" });
  drawCategories();
  const inputs = catList.querySelectorAll('input[type="text"]');
  const last = inputs[inputs.length - 1];
  if (last) {
    last.focus();
    last.select();
  }
}

function drawCategories() {
  catList.innerHTML = cats
    .map(
      (c) => `
    <div class="cat-edit" data-id="${escapeHtml(c.id)}">
      <div class="cat-edit-row">
        <button type="button" class="cat-icon-btn" style="--c:${escapeHtml(c.color)}" data-act="icon" aria-label="Change icon for ${escapeHtml(c.name)}" title="Change icon">${icon(c.icon)}</button>
        <input type="color" value="${escapeHtml(c.color)}" data-act="color" aria-label="Colour of ${escapeHtml(c.name)}" title="Colour" />
        <input type="text" value="${escapeHtml(c.name)}" maxlength="30" data-act="name" aria-label="Category name" />
        <button type="button" class="btn-card-action btn-delete" data-act="remove" aria-label="Remove ${escapeHtml(c.name)}" title="Remove category">${icon("trash")}</button>
      </div>
      ${
        openPickerId === c.id
          ? `<div class="cat-icon-grid" style="--c:${escapeHtml(c.color)}">${CATEGORY_ICONS.map(
              (n) => `<button type="button" data-act="pick" data-icon-name="${n}" class="${n === c.icon ? "active" : ""}" aria-label="${n}" title="${n}">${icon(n)}</button>`
            ).join("")}</div>`
          : ""
      }
    </div>`
    )
    .join("");
  catList.querySelectorAll(".cat-edit").forEach((row) => {
    const c = cats.find((x) => x.id === row.dataset.id);
    row.querySelector('[data-act="name"]').addEventListener("input", (e) => (c.name = e.target.value));
    row.querySelector('[data-act="color"]').addEventListener("input", (e) => {
      c.color = e.target.value;
      row.querySelector(".cat-icon-btn").style.setProperty("--c", c.color);
      const grid = row.querySelector(".cat-icon-grid");
      if (grid) grid.style.setProperty("--c", c.color);
    });
    row.querySelector('[data-act="icon"]').addEventListener("click", () => {
      openPickerId = openPickerId === c.id ? null : c.id;
      drawCategories();
    });
    row.querySelector('[data-act="remove"]').addEventListener("click", async () => {
      const ok = await askConfirm(
        "Remove category",
        `Remove the category "${c.name}"?\nYour reminders are not removed; they will simply show no category.`,
        "Remove category"
      );
      if (!ok) return;
      cats = cats.filter((x) => x.id !== c.id);
      drawCategories();
    });
    row.querySelectorAll('[data-act="pick"]').forEach((b) =>
      b.addEventListener("click", () => {
        c.icon = b.dataset.iconName;
        openPickerId = null;
        drawCategories();
      })
    );
  });
  hydrateIcons(catList);
}

// ---- Save ----
async function save() {
  const settings = {
    ...getSettings(),
    hotkey: hotkeyValue,
    eveningTime: setEvening.value,
    dailyTime: setDaily.value,
    snoozeMinutes: Number(setSnooze.value),
    startWithWindows: setStartup.checked,
    minimizeToTray: setTray.checked,
    theme: setTheme.value,
    sound: setSound.value,
    volume: Number(setVolume.value),
    categories: cats.map((c) => ({ ...c, name: c.name.trim() }))
  };
  if (!settings.eveningTime || !settings.dailyTime) {
    showToast("Please set both times.");
    return;
  }
  if (!Number.isInteger(settings.snoozeMinutes) || settings.snoozeMinutes < 1 || settings.snoozeMinutes > 720) {
    showToast("Snooze length must be a whole number from 1 to 720 minutes.");
    return;
  }
  try {
    await run("save_settings", { settings });
  } catch {
    return; // the reason is shown; the dialog stays open
  }
  closeOverlay(overlay);
  showToast("Settings saved.");
}

// ---- Backup ----
async function doExport() {
  try {
    const path = await run("export_data");
    if (path) showToast(`Backup saved to ${path}`);
  } catch {
    /* shown */
  }
}

async function doImport() {
  let preview;
  try {
    preview = await run("import_pick");
  } catch {
    return;
  }
  if (!preview) return;
  const nothing = preview.newReminders === 0 && preview.newNotes === 0 && preview.newCategories === 0;
  const lines = [
    `The file has ${preview.fileReminders} item${preview.fileReminders === 1 ? "" : "s"}.`,
    `• ${preview.newReminders} new item${preview.newReminders === 1 ? "" : "s"} will be added`,
    `• ${preview.existingReminders} already here (skipped)`
  ];
  if (preview.invalidReminders) lines.push(`• ${preview.invalidReminders} damaged (ignored)`);
  lines.push(`• ${preview.newNotes} new daily note${preview.newNotes === 1 ? "" : "s"} will be added`);
  if (preview.keptNotes) lines.push(`• ${preview.keptNotes} day${preview.keptNotes === 1 ? "" : "s"} already have a note (your note is kept)`);
  if (preview.newCategories) lines.push(`• ${preview.newCategories} new categor${preview.newCategories === 1 ? "y" : "ies"} will be added`);
  lines.push("", "Nothing you already have will be changed or replaced.");
  if (nothing) {
    await run("cancel_import").catch(() => {});
    await askConfirm("Import preview", lines.join("\n") + "\n\nThere is nothing new to add.", "OK");
    return;
  }
  const ok = await askConfirm("Import preview", lines.join("\n"), "Import");
  if (!ok) {
    await run("cancel_import").catch(() => {});
    return;
  }
  try {
    const done = await run("import_apply");
    showToast(`Imported ${done.newReminders} item${done.newReminders === 1 ? "" : "s"} and ${done.newNotes} daily note${done.newNotes === 1 ? "" : "s"}.`);
  } catch {
    /* shown */
  }
}
