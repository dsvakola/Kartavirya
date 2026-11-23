// Month calendar: what was done each day, notes saved that day, and the daily note.
import { escapeHtml, dayKey, formatShort, categoryById, categoryChip } from "./common.js";
import { icon, hydrateIcons } from "./icons.js";
import { askConfirm, showToast, run } from "./ui.js";

const calTitle = document.getElementById("calTitle");
const calWeekdays = document.getElementById("calWeekdays");
const calGrid = document.getElementById("calGrid");
const calDay = document.getElementById("calDay");

let getData = () => ({ reminders: [], notes: {}, settings: { categories: [] } });
let month = new Date(new Date().getFullYear(), new Date().getMonth(), 1);
let selected = dayKey(new Date());
let draft = null; // { day, text }: unsaved text of the daily note

export function initCalendar(dataGetter) {
  getData = dataGetter;
  const names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
  calWeekdays.innerHTML = names.map((n) => `<span>${n}</span>`).join("");
  document.getElementById("calPrev").addEventListener("click", () => moveMonth(-1));
  document.getElementById("calNext").addEventListener("click", () => moveMonth(1));
  document.getElementById("calToday").addEventListener("click", () => goToDay(dayKey(new Date())));
  calGrid.addEventListener("keydown", onGridKey);
}

function parseKey(key) {
  const [y, m, d] = key.split("-").map(Number);
  return new Date(y, m - 1, d);
}

function indexData() {
  const { reminders } = getData();
  const done = {};
  const noteItems = {};
  for (const r of reminders) {
    if (r.deleted_at) continue;
    if (r.kind === "note") {
      const k = dayKey(r.created_at);
      (noteItems[k] ||= []).push(r);
    } else if (r.is_completed && r.completed_at) {
      const k = dayKey(r.completed_at);
      (done[k] ||= []).push(r);
    }
  }
  return { done, noteItems };
}

export async function flushNote() {
  const area = document.getElementById("dailyNoteText");
  if (!area || !draft) return true;
  const { notes } = getData();
  const saved = notes[draft.day] || "";
  const text = area.value;
  if (text === saved) {
    draft = null;
    return true;
  }
  if (!text.trim() && saved.trim()) {
    const ok = await askConfirm(
      "Remove daily note",
      "The note for this day is empty now. Remove the saved note?\n(A backup copy of the previous notes file is kept.)",
      "Remove note"
    );
    if (!ok) {
      area.value = saved;
      draft = null;
      return false;
    }
  }
  try {
    await run("set_daily_note", { date: draft.day, text });
    showToast(text.trim() ? "Daily note saved." : "Daily note removed.");
    draft = null;
    return true;
  } catch {
    return false;
  }
}

async function moveMonth(delta) {
  await flushNote();
  month = new Date(month.getFullYear(), month.getMonth() + delta, 1);
  renderCalendar();
}

export async function goToDay(key, focusNote = false) {
  await flushNote();
  selected = key;
  const d = parseKey(key);
  month = new Date(d.getFullYear(), d.getMonth(), 1);
  renderCalendar();
  if (focusNote) {
    const area = document.getElementById("dailyNoteText");
    if (area) {
      area.focus();
      area.setSelectionRange(area.value.length, area.value.length);
    }
  } else {
    const cell = calGrid.querySelector(`[data-day="${key}"]`);
    if (cell) cell.focus();
  }
}

function onGridKey(e) {
  const step = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -7, ArrowDown: 7 }[e.key];
  if (step) {
    e.preventDefault();
    const d = parseKey(selected);
    d.setDate(d.getDate() + step);
    goToDay(dayKey(d));
  } else if (e.key === "PageUp" || e.key === "PageDown") {
    e.preventDefault();
    const d = parseKey(selected);
    d.setMonth(d.getMonth() + (e.key === "PageUp" ? -1 : 1));
    goToDay(dayKey(d));
  }
}

export function renderCalendar() {
  const { notes, settings } = getData();
  const { done, noteItems } = indexData();
  const todayKey = dayKey(new Date());

  calTitle.textContent = month.toLocaleDateString("en-IN", { month: "long", year: "numeric" });

  const first = new Date(month.getFullYear(), month.getMonth(), 1);
  const start = new Date(first);
  start.setDate(1 - first.getDay());

  let html = "";
  for (let i = 0; i < 42; i++) {
    const d = new Date(start);
    d.setDate(start.getDate() + i);
    const key = dayKey(d);
    const doneList = done[key] || [];
    const hasNote = Boolean(notes[key]) || (noteItems[key] || []).length > 0;
    const classes = [
      "cal-cell",
      d.getMonth() !== month.getMonth() ? "other-month" : "",
      key === todayKey ? "today" : "",
      key === selected ? "selected" : ""
    ].join(" ");
    const dots = [...new Set(doneList.map((r) => categoryById(settings, r.category)?.color || "#64748b"))]
      .slice(0, 4)
      .map((c) => `<i style="--dot:${escapeHtml(c)}"></i>`)
      .join("");
    const label =
      d.toLocaleDateString("en-IN", { weekday: "long", day: "numeric", month: "long", year: "numeric" }) +
      (doneList.length ? `, ${doneList.length} done` : "") +
      (hasNote ? ", has notes" : "");
    html += `<button type="button" class="${classes}" data-day="${key}" role="gridcell" aria-label="${escapeHtml(label)}" aria-selected="${key === selected}" tabindex="${key === selected ? 0 : -1}">
      <span class="cal-num">${d.getDate()}</span>
      <span class="cal-marks">
        ${doneList.length ? `<span class="cal-done">${icon("check")}${doneList.length}</span>` : ""}
        ${hasNote ? `<span class="cal-notemark">${icon("note")}</span>` : ""}
        ${dots ? `<span class="cal-dots">${dots}</span>` : ""}
      </span>
    </button>`;
  }
  calGrid.innerHTML = html;
  calGrid.querySelectorAll(".cal-cell").forEach((cell) => {
    cell.addEventListener("click", () => goToDay(cell.dataset.day));
  });
  renderDayPanel(done, noteItems);
}

function renderDayPanel(done, noteItems) {
  const { notes, settings } = getData();
  const d = parseKey(selected);
  const title = d.toLocaleDateString("en-IN", { weekday: "long", day: "numeric", month: "long", year: "numeric" });
  const doneList = [...(done[selected] || [])].sort((a, b) => a.completed_at.localeCompare(b.completed_at));
  const noteList = noteItems[selected] || [];

  const item = (r, when) => {
    const cat = categoryById(settings, r.category);
    return `<li class="day-item" style="--card-c:${escapeHtml(cat ? cat.color : "var(--border-input)")}">
      <span class="day-text">${escapeHtml(r.text)}</span>
      <span class="day-meta">${categoryChip(cat)}<span>${escapeHtml(when)}</span></span>
    </li>`;
  };

  const text = draft && draft.day === selected ? draft.text : notes[selected] || "";
  calDay.innerHTML = `
    <h3 class="day-title">${escapeHtml(title)}</h3>
    <section class="day-section">
      <h4>Done (${doneList.length})</h4>
      ${
        doneList.length
          ? `<ul class="day-list">${doneList
              .map((r) => item(r, "Done " + new Date(r.completed_at).toLocaleTimeString("en-IN", { hour: "2-digit", minute: "2-digit", hour12: true })))
              .join("")}</ul>`
          : `<p class="day-empty">Nothing was marked done on this day.</p>`
      }
    </section>
    ${
      noteList.length
        ? `<section class="day-section"><h4>Notes saved (${noteList.length})</h4><ul class="day-list">${noteList
            .map((r) => item(r, formatShort(r.created_at)))
            .join("")}</ul></section>`
        : ""
    }
    <section class="day-section">
      <h4>Daily note</h4>
      <textarea id="dailyNoteText" maxlength="20000" placeholder="Write something about this day… (Ctrl+Enter saves)" aria-label="Daily note for ${escapeHtml(title)}"></textarea>
      <div class="note-actions">
        <span id="noteStatus">${notes[selected] ? "Saved" : "No note yet"}</span>
        <button type="button" id="btnSaveNote" class="btn-primary">Save note</button>
      </div>
    </section>
  `;
  const area = document.getElementById("dailyNoteText");
  area.value = text;
  area.addEventListener("input", () => {
    draft = { day: selected, text: area.value };
    document.getElementById("noteStatus").textContent = "Not saved yet";
  });
  area.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && e.ctrlKey) {
      e.preventDefault();
      flushNote();
    }
  });
  area.addEventListener("blur", () => {
    // Save when the user clicks away to something other than the Save button.
    setTimeout(() => {
      if (draft && document.activeElement !== document.getElementById("btnSaveNote")) flushNote();
    }, 120);
  });
  document.getElementById("btnSaveNote").addEventListener("click", flushNote);
  hydrateIcons(calDay);
}
