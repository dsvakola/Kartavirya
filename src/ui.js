// Dialogs, toast and error handling for the main window.
import { invoke } from "./common.js";

const stack = []; // open dialogs, last = on top: { el, close, opener }

const FOCUSABLE =
  'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

/** Opens a modal dialog. `close` is called when Esc is pressed. */
export function openOverlay(el, close, focusEl) {
  const opener = document.activeElement;
  el.classList.remove("hidden");
  stack.push({ el, close, opener });
  setTimeout(() => {
    const target = focusEl || el.querySelector(FOCUSABLE);
    if (target) target.focus();
  }, 30);
}

export function closeOverlay(el) {
  el.classList.add("hidden");
  const i = stack.findIndex((o) => o.el === el);
  if (i < 0) return;
  const [entry] = stack.splice(i, 1);
  if (entry.opener && entry.opener.isConnected && typeof entry.opener.focus === "function") {
    entry.opener.focus();
  }
}

export function anyOverlayOpen() {
  return stack.length > 0;
}

export function topOverlay() {
  return stack.length ? stack[stack.length - 1] : null;
}

/** Esc closes the top dialog. Returns true when a dialog was closed. */
export function escapeTopOverlay() {
  const top = topOverlay();
  if (!top) return false;
  top.close();
  return true;
}

/** Keeps Tab inside the top dialog, so the main window cannot be reached. */
window.addEventListener("keydown", (e) => {
  if (e.key !== "Tab") return;
  const top = topOverlay();
  if (!top) return;
  const items = [...top.el.querySelectorAll(FOCUSABLE)].filter((n) => n.offsetParent !== null);
  if (items.length === 0) return;
  const first = items[0];
  const last = items[items.length - 1];
  if (!top.el.contains(document.activeElement)) {
    e.preventDefault();
    first.focus();
  } else if (e.shiftKey && document.activeElement === first) {
    e.preventDefault();
    last.focus();
  } else if (!e.shiftKey && document.activeElement === last) {
    e.preventDefault();
    first.focus();
  }
});

// ---- Confirmation ----
const confirmOverlay = document.getElementById("confirmOverlay");
const confirmTitle = document.getElementById("confirmTitle");
const confirmMessage = document.getElementById("confirmMessage");
const btnConfirmOk = document.getElementById("btnConfirmOk");
const btnConfirmCancel = document.getElementById("btnConfirmCancel");
let confirmResolve = null;

/** Shows a modal Yes/No dialog. Resolves to true when the user confirms. */
export function askConfirm(title, message, okLabel) {
  confirmTitle.textContent = title;
  confirmMessage.textContent = message;
  btnConfirmOk.textContent = okLabel || "OK";
  openOverlay(confirmOverlay, () => closeConfirm(false), btnConfirmCancel);
  return new Promise((resolve) => {
    confirmResolve = resolve;
  });
}

function closeConfirm(result) {
  closeOverlay(confirmOverlay);
  if (confirmResolve) {
    const r = confirmResolve;
    confirmResolve = null;
    r(result);
  }
}
btnConfirmOk.addEventListener("click", () => closeConfirm(true));
btnConfirmCancel.addEventListener("click", () => closeConfirm(false));

// ---- Toast ----
const toast = document.getElementById("toast");
const toastText = document.getElementById("toastText");
const btnToastAction = document.getElementById("btnToastAction");
let toastActionHandler = null;
let toastTimer = null;

/** Shows a short message at the bottom, with an optional action button (Undo). */
export function showToast(message, actionLabel, onAction) {
  toastText.textContent = message;
  toastActionHandler = onAction || null;
  if (onAction) {
    btnToastAction.textContent = actionLabel || "Undo";
    btnToastAction.classList.remove("hidden");
  } else {
    btnToastAction.classList.add("hidden");
  }
  toast.classList.remove("hidden");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => toast.classList.add("hidden"), onAction ? 8000 : 6000);
}

btnToastAction.addEventListener("click", async () => {
  const handler = toastActionHandler;
  toastActionHandler = null;
  toast.classList.add("hidden");
  if (handler) await handler();
});

/** Runs a backend command; on failure shows the message to the user and rethrows. */
export async function run(cmd, args) {
  try {
    return await invoke(cmd, args);
  } catch (err) {
    showToast(String(err));
    throw err;
  }
}
