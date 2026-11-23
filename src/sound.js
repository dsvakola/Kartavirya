// Alert sounds, made with the Web Audio API (no sound files needed).
import { invoke } from "./common.js";

export const SOUND_CHOICES = [
  ["chime", "Chime"],
  ["bell", "Bell"],
  ["digital", "Digital beep"],
  ["soft", "Soft tone"],
  ["windows", "Windows sound"],
  ["none", "Silent"]
];

let ctx = null;

function audio() {
  const AudioCtx = window.AudioContext || window.webkitAudioContext;
  if (!AudioCtx) return null;
  if (!ctx) ctx = new AudioCtx();
  if (ctx.state === "suspended") ctx.resume();
  return ctx;
}

function tone(c, master, { freq, start, length, type = "sine", peak = 0.35 }) {
  const osc = c.createOscillator();
  const gain = c.createGain();
  osc.type = type;
  osc.frequency.setValueAtTime(freq, c.currentTime + start);
  gain.gain.setValueAtTime(0.0001, c.currentTime + start);
  gain.gain.exponentialRampToValueAtTime(peak, c.currentTime + start + 0.015);
  gain.gain.exponentialRampToValueAtTime(0.0001, c.currentTime + start + length);
  osc.connect(gain);
  gain.connect(master);
  osc.start(c.currentTime + start);
  osc.stop(c.currentTime + start + length + 0.05);
}

/** Plays a sound. `volume` is 0 to 100. */
export async function playSound(name, volume) {
  const level = Math.max(0, Math.min(100, Number(volume))) / 100;
  if (name === "none" || level === 0) return;
  if (name === "windows") {
    try {
      await invoke("play_system_sound");
    } catch {
      /* ignore */
    }
    return;
  }
  try {
    const c = audio();
    if (!c) return;
    const master = c.createGain();
    master.gain.value = level;
    master.connect(c.destination);
    switch (name) {
      case "bell":
        [[880, 1.0], [2430, 0.35], [4750, 0.15]].forEach(([freq, g]) =>
          tone(c, master, { freq, start: 0, length: 1.3, peak: 0.3 * g })
        );
        [[880, 1.0], [2430, 0.35]].forEach(([freq, g]) =>
          tone(c, master, { freq, start: 0.7, length: 1.1, peak: 0.25 * g })
        );
        break;
      case "digital":
        [0, 0.22, 0.44].forEach((t) =>
          tone(c, master, { freq: 1040, start: t, length: 0.12, type: "square", peak: 0.18 })
        );
        break;
      case "soft":
        tone(c, master, { freq: 523.25, start: 0, length: 1.0, peak: 0.3 });
        tone(c, master, { freq: 783.99, start: 0.25, length: 1.0, peak: 0.2 });
        break;
      default:
        [587.33, 739.99, 880.0].forEach((freq, i) =>
          tone(c, master, { freq, start: i * 0.14, length: 0.55 })
        );
    }
  } catch (err) {
    console.warn("Sound error:", err);
  }
}
