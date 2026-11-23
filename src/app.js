// Main window controller for Kartavirya – Reminder Companion
import {
  invoke, listen, tauri, bootSettings, escapeHtml, categoryById, categoryChip,
  renderCategoryPicker, formatDateTime, formatShort, toLocalInput, dayKey, FREQ_LABELS
} from "./common.js";
import { icon, hydrateIcons } from "./icons.js";
import {
  openOverlay, closeOverlay, anyOverlayOpen, escapeTopOverlay, askConfirm, showToast, run
} from "./ui.js";
import { openHelp } from "./help.js";
import { initSettings, openSettings } from "./settings-ui.js";
import { initCalendar, renderCalendar, goToDay, flushNote } from "./calendar.js";

const $ = (id) => document.getElementById(id);

// ---- State ----
let settings = null;
let reminders = [];
let dailyNotes = {};
let currentFilter = "all";
let categoryFilter = "";
let searchQuery = "";
let view = "list";
let selectedId = null;

// ---- Elements ----
const remindersList = $("remindersList");
const emptyState = $("emptyState");
const searchInput = $("searchInput");
const btnClearSearch = $("btnClearSearch");
const filterButtons = document.querySelectorAll(".filter-btn");
const categoryFilterEl = $("categoryFilter");
const trashBar = $("trashBar");
const loadWarning = $("loadWarning");
const loadWarningText = $("loadWarningText");
const listView = $("listView");
const calendarView = $("calendarView");
const filterSection = $("filterSection");

hydrateIcons();

// ---- Data ----
async function loadReminders() {
  try {
    const list = await invoke("get_reminders");
    reminders = Array.isArray(list) ? list : [];
    renderAll();
  } catch (err) {
    console.error("Failed to load reminders:", err);
  }
}

async function loadNotes() {
  try {
    const n = await invoke("get_daily_notes");
    dailyNotes = n && typeof n === "object" ? n : {};
    if (view === "calendar") renderCalendar();
  } catch (err) {
    console.error("Failed to load daily notes:", err);
  }
}

function renderAll() {
  renderReminders();
  if (view === "calendar") renderCalendar();
}

// ---- Rendering the list ----
const GROUPS = [
  ["overdue", "Overdue"],
  ["today", "Today"],
  ["upcoming", "Upcoming"],
  ["notes", "Notes"],
  ["done", "Done"]
];

function groupOf(item, nowMs, tomorrowMs) {
  if (item.kind === "note") return "notes";
  if (item.is_completed) return "done";
  const due = new Date(item.remind_at).getTime();
  if (due <= nowMs) return "overdue";
  return due < tomorrowMs ? "today" : "upcoming";
}

function matchesCategory(item) {
  if (!categoryFilter) return true;
  const known = categoryById(settings, item.category);
  if (categoryFilter === "__none__") return !known;
  return known && known.id === categoryFilter;
}

function cardHtml(item, nowMs) {
  const isNote = item.kind === "note";
  const isDeleted = Boolean(item.deleted_at);
  const cat = categoryById(settings, item.category);
  const isOverdue = !isNote && !item.is_completed && !isDeleted && new Date(item.remind_at).getTime() <= nowMs;
  const dueTime = formatDateTime(item.remind_at);

  const actions = isDeleted
    ? `<button class="btn-card-action text-btn btn-restore" data-act="restore" title="Restore this item">${icon("restore")} Restore</button>
       <button class="btn-card-action text-btn btn-delete-forever" data-act="forever" title="Remove permanently">Delete Forever</button>`
    : `${isNote ? "" : `<button class="btn-card-action" data-act="test" title="Test the alert now" aria-label="Test the alert now">${icon("bell")}</button>`}
       <button class="btn-card-action" data-act="edit" title="Edit (Enter)" aria-label="Edit">${icon("edit")}</button>
       ${!isNote && !item.is_completed ? `<button class="btn-card-action text-btn" data-act="snooze" title="Snooze 1 hour" aria-label="Snooze one hour">${icon("moon")} +1h</button>` : ""}
       <button class="btn-card-action btn-delete" data-act="delete" title="Move to Recently Deleted (Delete)" aria-label="Delete">${icon("trash")}</button>`;

  const lead = isNote
    ? `<span class="card-note-icon" title="Note only">${icon("note")}</span>`
    : `<input type="checkbox" class="card-checkbox" ${item.is_completed ? "checked" : ""} ${isDeleted ? "disabled" : ""} aria-label="Mark done" title="Toggle done (Space)" />`;

  const meta = isNote
    ? `${categoryChip(cat)}<span class="freq-tag">${icon("note")} Note</span>`
    : `${categoryChip(cat)}
       <span class="freq-tag">${icon("repeat")} ${escapeHtml(FREQ_LABELS[item.remind_type] || item.remind_type)}</span>
       <span class="meta-item">${icon("clock")} Due: ${escapeHtml(dueTime)}</span>
       ${isOverdue ? `<span class="overdue-tag">${icon("alert")} Overdue</span>` : ""}`;

  return `
    <div class="reminder-card ${item.is_completed ? "completed" : ""} ${isOverdue ? "overdue" : ""} ${item.id === selectedId ? "selected" : ""}"
         data-id="${escapeHtml(item.id)}" tabindex="${item.id === selectedId ? 0 : -1}" role="listitem"
         style="--card-c:${escapeHtml(cat ? cat.color : "var(--border-card)")}">
      <div class="card-left">
        ${lead}
        <div class="card-content">
          <div class="card-text" title="Double click to edit">${escapeHtml(item.text)}</div>
          <div class="card-meta">
            ${meta}
            ${isDeleted ? `<span class="meta-item">${icon("trash")} Deleted: ${escapeHtml(formatDateTime(item.deleted_at))}</span>` : ""}
            ${item.is_completed && item.completed_at ? `<span class="meta-item">${icon("check")} Done: ${escapeHtml(formatShort(item.completed_at))}</span>` : ""}
            <span class="meta-item">${icon("calendar")} Added: ${escapeHtml(formatDateTime(item.created_at))}</span>
          </div>
        </div>
      </div>
      <div class="card-right">${actions}</div>
    </div>`;
}

function renderReminders() {
  if (!settings) return;
  const query = searchQuery.toLowerCase().trim();
  const now = new Date();
  const nowMs = now.getTime();
  const tomorrowMs = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1).getTime();

  const live = reminders.filter((r) => !r.deleted_at);
  const counts = {
    all: live.length,
    active: live.filter((r) => r.kind !== "note" && !r.is_completed).length,
    completed: live.filter((r) => r.kind !== "note" && r.is_completed).length,
    notes: live.filter((r) => r.kind === "note").length,
    deleted: reminders.length - live.length
  };
  $("countAll").textContent = counts.all;
  $("countActive").textContent = counts.active;
  $("countCompleted").textContent = counts.completed;
  $("countNotes").textContent = counts.notes;
  $("countDeleted").textContent = counts.deleted;
  trashBar.classList.toggle("hidden", currentFilter !== "deleted" || counts.deleted === 0);

  const filtered = reminders.filter((item) => {
    if (currentFilter === "deleted") {
      if (!item.deleted_at) return false;
    } else if (item.deleted_at) {
      return false;
    }
    if (currentFilter === "active" && (item.kind === "note" || item.is_completed)) return false;
    if (currentFilter === "completed" && (item.kind === "note" || !item.is_completed)) return false;
    if (currentFilter === "notes" && item.kind !== "note") return false;
    if (!matchesCategory(item)) return false;
    if (query) {
      const cat = categoryById(settings, item.category);
      const hay = (item.text + " " + (cat ? cat.name : "")).toLowerCase();
      if (!hay.includes(query)) return false;
    }
    return true;
  });

  const hadFocus = remindersList.contains(document.activeElement);

  if (filtered.length === 0) {
    remindersList.innerHTML = "";
    emptyState.classList.remove("hidden");
    const searching = query || categoryFilter;
    $("emptyTitle").textContent = searching ? "Nothing matches" : currentFilter === "deleted" ? "Recently Deleted is empty" : "No Reminders Found";
    $("emptyText").innerHTML = searching
      ? "Try a different search or category."
      : currentFilter === "deleted"
        ? "Deleted items appear here and can be restored."
        : `Press <kbd class="hotkey-label">${escapeHtml(settings.hotkey)}</kbd> anywhere on your computer to instantly log an item or task.`;
    $("btnEmptyAdd").classList.toggle("hidden", Boolean(searching) || currentFilter === "deleted");
    return;
  }
  emptyState.classList.add("hidden");

  // Keep the selected card valid
  if (!filtered.some((r) => r.id === selectedId)) selectedId = filtered[0].id;

  let html = "";
  if (currentFilter === "deleted") {
    html = filtered.map((r) => cardHtml(r, nowMs)).join("");
  } else if (currentFilter === "completed" || currentFilter === "notes") {
    const sorted = [...filtered].sort((a, b) =>
      currentFilter === "completed"
        ? (b.completed_at || "").localeCompare(a.completed_at || "")
        : b.created_at.localeCompare(a.created_at)
    );
    html = sorted.map((r) => cardHtml(r, nowMs)).join("");
  } else {
    const buckets = { overdue: [], today: [], upcoming: [], notes: [], done: [] };
    filtered.forEach((r) => buckets[groupOf(r, nowMs, tomorrowMs)].push(r));
    buckets.overdue.sort((a, b) => a.remind_at.localeCompare(b.remind_at));
    buckets.today.sort((a, b) => a.remind_at.localeCompare(b.remind_at));
    buckets.upcoming.sort((a, b) => a.remind_at.localeCompare(b.remind_at));
    buckets.notes.sort((a, b) => b.created_at.localeCompare(a.created_at));
    buckets.done.sort((a, b) => (b.completed_at || "").localeCompare(a.completed_at || ""));
    for (const [key, label] of GROUPS) {
      const list = buckets[key];
      if (!list.length) continue;
      html += `<div class="group-title ${key}" role="presentation">${label}<span class="count">${list.length}</span></div>`;
      html += list.map((r) => cardHtml(r, nowMs)).join("");
    }
  }
  remindersList.innerHTML = html;
  if (hadFocus) {
    const card = remindersList.querySelector(`.reminder-card[data-id="${CSS.escape(selectedId)}"]`);
    if (card) card.focus();
  }
}

// ---- List events (one set of listeners for all cards) ----
function cardOf(el) {
  const card = el.closest(".reminder-card");
  return card ? { card, id: card.dataset.id, item: reminders.find((r) => r.id === card.dataset.id) } : null;
}

function selectCard(id, focus = true) {
  selectedId = id;
  remindersList.querySelectorAll(".reminder-card").forEach((c) => {
    const on = c.dataset.id === id;
    c.classList.toggle("selected", on);
    c.tabIndex = on ? 0 : -1;
    if (on && focus) c.focus();
  });
}

async function deleteItem(item) {
  const ok = await askConfirm(
    "Delete",
    `Move "${item.text.length > 80 ? item.text.slice(0, 80) + "…" : item.text}" to Recently Deleted? You can restore it later.`,
    "Delete"
  );
  if (!ok) return;
  try {
    await run("delete_reminder", { id: item.id });
    showToast("Moved to Recently Deleted.", "Undo", async () => {
      await run("restore_reminder", { id: item.id }).catch(() => {});
      await loadReminders();
    });
  } catch {
    /* shown */
  }
  await loadReminders();
}

remindersList.addEventListener("click", async (e) => {
  const found = cardOf(e.target);
  if (!found) return;
  const { id, item } = found;
  selectCard(id, false);
  const btn = e.target.closest("[data-act]");
  if (!btn || !item) return;
  switch (btn.dataset.act) {
    case "test":
      await run("test_reminder_alert", { id }).catch(() => {});
      break;
    case "edit":
      openEditModal(item);
      break;
    case "snooze":
      await run("snooze_reminder", { id, minutes: 60 }).catch(() => {});
      await loadReminders();
      break;
    case "delete":
      await deleteItem(item);
      break;
    case "restore":
      await run("restore_reminder", { id }).catch(() => {});
      await loadReminders();
      break;
    case "forever": {
      const ok = await askConfirm("Delete Forever", `Permanently remove "${item.text}"? This cannot be undone.`, "Delete Forever");
      if (!ok) return;
      await run("delete_forever", { id }).catch(() => {});
      await loadReminders();
      break;
    }
  }
});

remindersList.addEventListener("change", async (e) => {
  if (!e.target.classList.contains("card-checkbox")) return;
  const found = cardOf(e.target);
  if (!found) return;
  selectedId = found.id;
  await run("toggle_reminder", { id: found.id }).catch(() => {});
  await loadReminders();
});

remindersList.addEventListener("dblclick", (e) => {
  if (!e.target.classList.contains("card-text")) return;
  const found = cardOf(e.target);
  if (found && found.item && !found.item.deleted_at) openEditModal(found.item);
});

remindersList.addEventListener("keydown", async (e) => {
  if (!e.target.classList.contains("reminder-card")) return;
  const cards = [...remindersList.querySelectorAll(".reminder-card")];
  const i = cards.indexOf(e.target);
  const move = (n) => {
    e.preventDefault();
    const c = cards[Math.max(0, Math.min(cards.length - 1, n))];
    if (c) {
      selectCard(c.dataset.id);
      c.scrollIntoView({ block: "nearest" });
    }
  };
  const item = reminders.find((r) => r.id === e.target.dataset.id);
  switch (e.key) {
    case "ArrowDown": move(i + 1); break;
    case "ArrowUp": move(i - 1); break;
    case "Home": move(0); break;
    case "End": move(cards.length - 1); break;
    case " ":
      if (item && !item.deleted_at && item.kind !== "note") {
        e.preventDefault();
        selectedId = item.id;
        await run("toggle_reminder", { id: item.id }).catch(() => {});
        await loadReminders();
      }
      break;
    case "Enter":
      if (item && !item.deleted_at) {
        e.preventDefault();
        openEditModal(item);
      }
      break;
    case "Delete":
      if (item && !item.deleted_at) {
        e.preventDefault();
        const next = cards[i + 1] || cards[i - 1];
        if (next) selectedId = next.dataset.id;
        await deleteItem(item);
      }
      break;
  }
});

$("btnEmptyTrash").addEventListener("click", async () => {
  const count = reminders.filter((r) => r.deleted_at).length;
  if (count === 0) return;
  const ok = await askConfirm(
    "Empty Recently Deleted",
    `Permanently remove ${count} deleted item${count === 1 ? "" : "s"}? This cannot be undone.`,
    "Delete Forever"
  );
  if (!ok) return;
  await run("empty_trash").catch(() => {});
  await loadReminders();
});

$("btnCloseWarning").addEventListener("click", () => loadWarning.classList.add("hidden"));

async function checkLoadWarning() {
  try {
    const warning = await invoke("get_load_warning");
    if (warning) {
      loadWarningText.textContent = warning;
      loadWarning.classList.remove("hidden");
    }
  } catch {
    /* not running inside Tauri */
  }
}

// ---- Search, filters, view ----
searchInput.addEventListener("input", (e) => {
  searchQuery = e.target.value;
  btnClearSearch.classList.toggle("hidden", !searchQuery);
  if (view !== "list") setView("list");
  renderReminders();
});

btnClearSearch.addEventListener("click", () => {
  searchInput.value = "";
  searchQuery = "";
  btnClearSearch.classList.add("hidden");
  renderReminders();
  searchInput.focus();
});

filterButtons.forEach((btn) => {
  btn.addEventListener("click", () => {
    filterButtons.forEach((b) => b.classList.remove("active"));
    btn.classList.add("active");
    currentFilter = btn.dataset.filter;
    renderReminders();
  });
});

categoryFilterEl.addEventListener("change", () => {
  categoryFilter = categoryFilterEl.value;
  renderReminders();
});

function drawCategoryFilter() {
  const options = [`<option value="">All categories</option>`]
    .concat((settings.categories || []).map((c) => `<option value="${escapeHtml(c.id)}">${escapeHtml(c.name)}</option>`))
    .concat([`<option value="__none__">No category</option>`]);
  categoryFilterEl.innerHTML = options.join("");
  if (![...categoryFilterEl.options].some((o) => o.value === categoryFilter)) categoryFilter = "";
  categoryFilterEl.value = categoryFilter;
}

async function setView(next) {
  if (view === "calendar" && next !== "calendar") await flushNote();
  view = next;
  document.querySelectorAll("#viewSwitch button").forEach((b) => b.classList.toggle("active", b.dataset.view === next));
  const cal = next === "calendar";
  listView.classList.toggle("hidden", cal);
  filterSection.classList.toggle("hidden", cal);
  calendarView.classList.toggle("hidden", !cal);
  if (cal) renderCalendar();
  else renderReminders();
}

document.querySelectorAll("#viewSwitch button").forEach((b) => b.addEventListener("click", () => setView(b.dataset.view)));

async function openTodayNote() {
  await setView("calendar");
  await goToDay(dayKey(new Date()), true);
}
$("btnTodayNote").addEventListener("click", openTodayNote);

// ---- New reminder ----
async function triggerQuickEntry() {
  try {
    await invoke("show_popup");
  } catch (err) {
    console.error("Failed to show popup:", err);
  }
}
$("btnNewReminder").addEventListener("click", triggerQuickEntry);
$("btnEmptyAdd").addEventListener("click", triggerQuickEntry);

$("btnMinimizeTray").addEventListener("click", async () => {
  try {
    await invoke("hide_main_window");
  } catch (err) {
    console.error("Failed to hide to tray:", err);
  }
});

// ---- About ----
const aboutOverlay = $("aboutOverlay");
async function loadAppInfo() {
  try {
    const info = await invoke("get_app_info");
    if (info && info.version) {
      $("aboutVersion").textContent = info.version;
      $("statusVersion").textContent = info.version;
    }
  } catch {
    /* not running inside Tauri */
  }
}
[["linkPrivacy", "privacy"], ["linkWebsite", "website"], ["linkEmail", "email"]].forEach(([elId, target]) => {
  $(elId).addEventListener("click", async (e) => {
    e.preventDefault();
    await run("open_link", { target }).catch(() => {});
  });
});
const closeAbout = () => closeOverlay(aboutOverlay);
$("btnAbout").addEventListener("click", () => openOverlay(aboutOverlay, closeAbout, $("btnOkAbout")));
$("btnCloseAbout").addEventListener("click", closeAbout);
$("btnOkAbout").addEventListener("click", closeAbout);
$("btnHelp").addEventListener("click", openHelp);
$("btnSettings").addEventListener("click", openSettings);

// ---- Edit dialog ----
const editOverlay = $("editOverlay");
const editText = $("editText");
const editFreqButtons = document.querySelectorAll("#editFreqButtons .freq-btn");
const editCustomPicker = $("editCustomPicker");
const editCustomDatetime = $("editCustomDatetime");
const editKindButtons = document.querySelectorAll("#editKindSwitch button");
let editingId = null;
let editKind = "reminder";
let editFreq = "hourly";
let editOriginal = { kind: "reminder", freq: "hourly", custom: "" };
let editCategory = null;

function setEditKind(kind) {
  editKind = kind;
  editKindButtons.forEach((b) => b.classList.toggle("active", b.dataset.kind === kind));
  $("editScheduleSection").classList.toggle("hidden", kind === "note");
  editCustomPicker.classList.toggle("hidden", kind === "note" || editFreq !== "custom");
}

function setEditFrequency(freq) {
  editFreq = freq;
  editFreqButtons.forEach((btn) => btn.classList.toggle("active", btn.dataset.freq === freq));
  editCustomPicker.classList.toggle("hidden", freq !== "custom" || editKind === "note");
}

function openEditModal(item) {
  editingId = item.id;
  editText.value = item.text;
  $("editTitle").textContent = item.kind === "note" ? "Edit Note" : "Edit Reminder";
  const freq = item.remind_type && item.remind_type !== "none" ? item.remind_type : "hourly";
  editCustomDatetime.value = item.remind_at ? toLocalInput(item.remind_at) : "";
  editOriginal = { kind: item.kind || "reminder", freq, custom: editCustomDatetime.value };
  editCategory = categoryById(settings, item.category) ? item.category : null;
  setEditFrequency(freq);
  setEditKind(editOriginal.kind);
  renderCategoryPicker($("editCategoryPick"), settings, editCategory, (id) => (editCategory = id));
  $("editCategoryRow").classList.toggle("hidden", !(settings.categories || []).length);
  openOverlay(editOverlay, closeEdit, editText);
}

function closeEdit() {
  closeOverlay(editOverlay);
  editingId = null;
}

async function saveEdit() {
  if (!editingId) return;
  const text = editText.value.trim();
  if (!text) {
    showToast("The text is empty.");
    return;
  }
  const isNote = editKind === "note";
  const scheduleChanged =
    !isNote &&
    (editOriginal.kind === "note" ||
      editFreq !== editOriginal.freq ||
      (editFreq === "custom" && editCustomDatetime.value !== editOriginal.custom));
  let remindAt = null;
  if (scheduleChanged && editFreq === "custom") {
    if (!editCustomDatetime.value) {
      showToast("Please choose the alert date and time.");
      return;
    }
    remindAt = new Date(editCustomDatetime.value).toISOString();
  }
  try {
    await run("update_reminder", {
      id: editingId,
      text,
      kind: editKind,
      remindType: isNote ? "none" : editFreq,
      remindAt,
      reschedule: scheduleChanged,
      category: editCategory
    });
    closeEdit();
    await loadReminders();
  } catch {
    /* shown */
  }
}

editKindButtons.forEach((b) => b.addEventListener("click", () => setEditKind(b.dataset.kind)));
editFreqButtons.forEach((btn) => btn.addEventListener("click", () => setEditFrequency(btn.dataset.freq)));
$("btnSaveEdit").addEventListener("click", saveEdit);
$("btnCloseEdit").addEventListener("click", closeEdit);
$("btnCancelEdit").addEventListener("click", closeEdit);
editText.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && !e.shiftKey && !e.isComposing) {
    e.preventDefault();
    saveEdit();
  }
});

// ---- Missed reminders summary ----
const missedOverlay = $("missedOverlay");
const closeMissed = () => closeOverlay(missedOverlay);
$("btnCloseMissed").addEventListener("click", closeMissed);
$("btnOkMissed").addEventListener("click", closeMissed);

async function checkMissed() {
  if (!tauri || anyOverlayOpen()) return;
  let missed = [];
  try {
    missed = await invoke("take_missed");
  } catch {
    return;
  }
  if (!Array.isArray(missed) || missed.length === 0) return;
  $("missedIntro").textContent =
    `${missed.length} reminders came due while Kartavirya was closed. They stay in your list as Overdue until you mark them done or snooze them.`;
  $("missedList").innerHTML = missed
    .map((r) => {
      const cat = categoryById(settings, r.category);
      return `<div class="missed-item" data-id="${escapeHtml(r.id)}" style="--card-c:${escapeHtml(cat ? cat.color : "var(--border-input)")}">
        <div>
          <div class="missed-text">${escapeHtml(r.text)}</div>
          <div class="missed-when">${categoryChip(cat)} Was due ${escapeHtml(formatShort(r.remind_at))}</div>
        </div>
        <button type="button" class="btn-secondary" data-done="${escapeHtml(r.id)}">${icon("check")} Done</button>
      </div>`;
    })
    .join("");
  $("missedList").querySelectorAll("[data-done]").forEach((b) =>
    b.addEventListener("click", async () => {
      try {
        await run("toggle_reminder", { id: b.dataset.done });
        b.disabled = true;
        b.closest(".missed-item").classList.add("done");
        await loadReminders();
      } catch {
        /* shown */
      }
    })
  );
  openOverlay(missedOverlay, closeMissed, $("btnOkMissed"));
}

// ---- Global keyboard ----
function isTyping(el) {
  return el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.tagName === "SELECT");
}

window.addEventListener("keydown", async (e) => {
  // Alt+1..6 switch the schedule in the Edit dialog
  if (!editOverlay.classList.contains("hidden") && e.altKey && editKind === "reminder") {
    const map = { 1: "hourly", 2: "5min", 3: "today_evening", 4: "daily", 5: "weekly", 6: "custom" };
    if (map[e.key]) {
      e.preventDefault();
      setEditFrequency(map[e.key]);
      return;
    }
  }

  if (e.key === "Escape") {
    if (escapeTopOverlay()) {
      e.preventDefault();
      return;
    }
    if (document.activeElement === searchInput && searchInput.value) {
      btnClearSearch.click();
      return;
    }
    try {
      await flushNote();
      await invoke("hide_main_window");
    } catch (err) {
      console.error("Failed to hide main window:", err);
    }
    return;
  }

  if (anyOverlayOpen()) return;

  if (e.key === "F1") {
    e.preventDefault();
    openHelp();
    return;
  }
  if (e.ctrlKey && !e.altKey && !e.shiftKey) {
    switch (e.key) {
      case ",": e.preventDefault(); openSettings(); return;
      case "n": case "N": e.preventDefault(); triggerQuickEntry(); return;
      case "f": case "F":
        e.preventDefault();
        if (view !== "list") await setView("list");
        searchInput.focus();
        searchInput.select();
        return;
      case "1": e.preventDefault(); setView("list"); return;
      case "2": e.preventDefault(); setView("calendar"); return;
      case "3": e.preventDefault(); openTodayNote(); return;
    }
  }
  if (e.key === "/" && !isTyping(document.activeElement) && view === "list") {
    e.preventDefault();
    searchInput.focus();
  }
});

// ---- Settings ----
function applySettingsToUi() {
  document.querySelectorAll(".hotkey-label").forEach((el) => (el.textContent = settings.hotkey));
  drawCategoryFilter();
  renderAll();
}

initSettings(() => settings);
initCalendar(() => ({ reminders, notes: dailyNotes, settings }));

async function start() {
  settings = await bootSettings((s) => {
    settings = s;
    applySettingsToUi();
  });
  applySettingsToUi();
  await Promise.all([loadReminders(), loadNotes(), loadAppInfo(), checkLoadWarning()]);
  await checkMissed();
}
start();

if (tauri) {
  listen("reminder-updated", loadReminders);
  listen("notes-updated", loadNotes);
  window.addEventListener("focus", () => {
    checkMissed();
    renderReminders();
  });
}

// Refresh "Overdue" tags once a minute
setInterval(() => {
  if (view === "list") renderReminders();
}, 60000);
