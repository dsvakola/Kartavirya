// Shared helpers for all Kartavirya windows.
import { icon } from "./icons.js";

export const tauri = window.__TAURI__ || null;
// The window can open a moment before the backend has finished starting; in that case
// the backend says its state is not ready yet, so we wait briefly and try again.
const rawInvoke = tauri ? tauri.core.invoke : null;
export const invoke = rawInvoke
  ? async (cmd, args) => {
      for (let attempt = 0; ; attempt++) {
        try {
          return await rawInvoke(cmd, args);
        } catch (err) {
          if (attempt < 40 && String(err).includes("not managed")) {
            await new Promise((r) => setTimeout(r, 100));
            continue;
          }
          throw err;
        }
      }
    }
  : async (cmd, args) => {
      console.log("invoke (not in Tauri):", cmd, args);
      return null;
    };
export const listen = tauri ? tauri.event.listen : async () => () => {};

export const DEFAULT_SETTINGS = {
  hotkey: "F12",
  eveningTime: "20:00",
  dailyTime: "09:00",
  snoozeMinutes: 10,
  startWithWindows: false,
  minimizeToTray: false,
  theme: "system",
  sound: "chime",
  volume: 70,
  categories: []
};

export const FREQ_LABELS = {
  "5min": "In 5 Mins",
  hourly: "Hourly",
  today_evening: "Today Evening",
  daily: "Daily",
  weekly: "Weekly",
  custom: "Custom Date/Time",
  none: "No alert"
};

export function escapeHtml(str) {
  if (str === null || str === undefined) return "";
  return String(str).replace(/[&<>'"]/g, (tag) => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    "'": "&#39;",
    '"': "&quot;"
  }[tag] || tag));
}

export async function fetchSettings() {
  try {
    const s = await invoke("get_settings");
    return s ? { ...DEFAULT_SETTINGS, ...s } : { ...DEFAULT_SETTINGS };
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
}

const darkQuery = window.matchMedia ? window.matchMedia("(prefers-color-scheme: dark)") : null;
let currentTheme = "system";

/** Applies "system", "light" or "dark" to this window. */
export function applyTheme(theme) {
  currentTheme = theme || "system";
  const root = document.documentElement;
  if (currentTheme === "system") {
    root.removeAttribute("data-theme");
  } else {
    root.setAttribute("data-theme", currentTheme);
  }
}
if (darkQuery) {
  // "system" is handled in CSS; nothing else to do when Windows switches theme.
  darkQuery.addEventListener?.("change", () => applyTheme(currentTheme));
}

export function categoryById(settings, id) {
  if (!id) return null;
  return (settings.categories || []).find((c) => c.id === id) || null;
}

/** HTML for a coloured category chip. */
export function categoryChip(cat) {
  if (!cat) return "";
  return `<span class="cat-chip" style="--c:${escapeHtml(cat.color)}">${icon(cat.icon)}${escapeHtml(cat.name)}</span>`;
}

/** Category picker buttons. `selectedId` may be null ("none"). */
export function renderCategoryPicker(container, settings, selectedId, onPick) {
  const none = `<button type="button" data-cat="" class="${selectedId ? "" : "active"}">No category</button>`;
  const rest = (settings.categories || [])
    .map(
      (c) =>
        `<button type="button" data-cat="${escapeHtml(c.id)}" style="--c:${escapeHtml(c.color)}" class="${c.id === selectedId ? "active" : ""}" title="${escapeHtml(c.name)}">${icon(c.icon)}${escapeHtml(c.name)}</button>`
    )
    .join("");
  container.innerHTML = none + rest;
  container.querySelectorAll("button").forEach((btn) => {
    btn.addEventListener("click", () => {
      container.querySelectorAll("button").forEach((b) => b.classList.toggle("active", b === btn));
      onPick(btn.dataset.cat || null);
    });
  });
}

export function formatDateTime(iso) {
  if (!iso) return "";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return String(iso);
  return d.toLocaleString("en-IN", {
    day: "2-digit",
    month: "short",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    hour12: true
  });
}

export function formatShort(iso) {
  if (!iso) return "";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return String(iso);
  return d.toLocaleString("en-IN", {
    day: "2-digit",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
    hour12: true
  });
}

/** "YYYY-MM-DD" in local time. */
export function dayKey(date) {
  const d = date instanceof Date ? date : new Date(date);
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${d.getFullYear()}-${m}-${day}`;
}

/** Value for `<input type="datetime-local">` from an ISO string. */
export function toLocalInput(iso) {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "";
  return new Date(d.getTime() - d.getTimezoneOffset() * 60000).toISOString().slice(0, 16);
}

/** Starts a window: loads settings, applies the theme and keeps it in step. */
export async function bootSettings(onChange) {
  let settings = await fetchSettings();
  applyTheme(settings.theme);
  await listen("settings-changed", (event) => {
    settings = { ...DEFAULT_SETTINGS, ...event.payload };
    applyTheme(settings.theme);
    if (onChange) onChange(settings);
  });
  return settings;
}
