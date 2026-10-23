// Relay's redesign, as a clickable prototype. Static data, simulated sessions:
// nothing here touches the real app. Open prototype.html in a browser.
(() => {
const { icon, ICONS } = window.RelayIcons;

// ── Data ────────────────────────────────────────────────────────────────────

const CAT = { click: "pointer", dclick: "pointer", drag: "pointer", scroll: "pointer", move: "move", keys: "keys", type: "keys", text: "keys", wait: "logic", pixel: "logic", find: "logic" };
const KIND_NAME = { click: "Click", dclick: "Double click", drag: "Drag", scroll: "Scroll", move: "Move", keys: "Keys", type: "Type", text: "Text", wait: "Wait", pixel: "Wait for", find: "Find image" };

let nextId = 1;
const s = (o) => ({ id: nextId++, pause: 0.2, label: "", ...o });

function invoiceSteps() {
  return [
    s({ kind: "click", t: 0.85, x: 134, y: 70, label: "File menu", move: { dur: 0.85, from: [960, 670] } }),
    s({ kind: "click", t: 1.75, x: 230, y: 324, label: "Export as PDF", move: { dur: 0.63, from: [134, 70] } }),
    s({ kind: "wait", t: 2.4, dur: 0.7, label: "" }),
    s({ kind: "type", t: 3.1, text: "invoice_0924", dur: 1.4 }),
    s({ kind: "keys", t: 4.8, combo: ["Ctrl", "S"], dur: 0.3 }),
    s({ kind: "click", t: 6.02, x: 1248, y: 680, label: "Save", pause: 0.3, move: { dur: 0.73, from: [812, 402] } }),
    s({ kind: "pixel", t: 6.27, x: 1248, y: 680, color: "#9B9797", tol: 8, timeout: 5, label: "Save button turns grey", dur: 0.8 }),
    s({ kind: "keys", t: 7.17, combo: ["Ctrl", "W"], dur: 0.3 }),
    s({ kind: "click", t: 8.6, x: 1640, y: 420, label: "Invoices folder", pause: 1.4, move: { dur: 0.52, from: [1248, 680] } }),
    s({ kind: "find", t: 9.3, label: "Upload button", match: 85, timeout: 5, btn: "Left", dur: 0.8, cx: 0.5, cy: 0.5 }),
    s({ kind: "keys", t: 10.0, combo: ["Enter"], dur: 0.15 }),
  ];
}
function claimsSteps() {
  return [
    s({ kind: "click", t: 0.6, x: 300, y: 140, label: "New claim", move: { dur: 0.6, from: [900, 600] } }),
    s({ kind: "pixel", t: 1.3, x: 640, y: 210, color: "#2D6CDF", tol: 8, timeout: 5, label: "Form is open", dur: 0.8 }),
    s({ kind: "click", t: 2.2, x: 640, y: 260, label: "Policy number", move: { dur: 0.5, from: [300, 140] } }),
    s({ kind: "text", t: 2.6, text: "{col:PolicyNo}", dur: 0.6 }),
    s({ kind: "keys", t: 3.3, combo: ["Tab"], dur: 0.1 }),
    s({ kind: "text", t: 3.5, text: "{col:Amount}", dur: 0.6 }),
    s({ kind: "keys", t: 4.2, combo: ["Tab"], dur: 0.1 }),
    s({ kind: "text", t: 4.4, text: "{col:Customer}", dur: 0.7 }),
    s({ kind: "click", t: 5.6, x: 980, y: 620, label: "Submit", move: { dur: 0.7, from: [640, 260] } }),
    s({ kind: "pixel", t: 6.4, x: 700, y: 120, color: "#1DB46A", tol: 16, timeout: 8, label: "Confirmation banner", dur: 0.8 }),
  ];
}
function refundSteps() {
  return [
    s({ kind: "click", t: 0.3, x: 820, y: 700, label: "Reply box", move: { dur: 0.3, from: [700, 500] } }),
    s({ kind: "text", t: 0.6, text: "Hi {clipboard},\nwe've refunded part of your order today ({date}).", dur: 1.0 }),
    s({ kind: "keys", t: 1.9, combo: ["Ctrl", "Enter"], dur: 0.2 }),
  ];
}
function keyboardOnlySteps() {
  return [
    s({ kind: "text", t: 0.2, text: "Thanks for reaching out, {clipboard}!", dur: 0.9 }),
    s({ kind: "keys", t: 1.3, combo: ["Tab"], dur: 0.1 }),
    s({ kind: "keys", t: 1.5, combo: ["Ctrl", "Enter"], dur: 0.2 }),
  ];
}

const PLAY = (o = {}) => ({ repeat: 1, forever: false, speed: 1, vary: true, jitter: 40, stopOnKey: true, coords: "screen", ...o });

const MACROS = [
  { id: "invoice", name: "Export invoice to PDF", steps: invoiceSteps(), play: PLAY({ repeat: 3 }), data: null,
    triggers: [{ type: "hotkey", on: true, combo: ["Ctrl", "Alt", "1"] }, { type: "schedule", on: true, days: [1, 1, 1, 1, 1, 0, 0], time: "09:00" }],
    last: { ok: true, text: "Today 09:00" }, made: 5 },
  { id: "timesheet", name: "Fill weekly timesheet", steps: invoiceSteps().slice(0, 8), play: PLAY(), data: null,
    triggers: [{ type: "schedule", on: true, days: [0, 0, 0, 0, 1, 0, 0], time: "17:00" }],
    last: { ok: false, text: "Fri 17:00" }, made: 4 },
  { id: "claims", name: "Claims from customers.csv", steps: claimsSteps(), play: PLAY(), data: { file: "customers.csv", rows: 200, cols: ["PolicyNo", "Amount", "Customer"] },
    triggers: [{ type: "hotkey", on: true, combo: ["F13"] }],
    last: { ok: true, text: "Yesterday" }, made: 6 },
  { id: "refund", name: "Refund — partial (EU)", steps: refundSteps(), play: PLAY({ vary: false }), data: null,
    triggers: [{ type: "hotkey", on: true, combo: ["Ctrl", "Alt", "7"] }],
    last: { ok: true, text: "12:41" }, made: 3 },
  { id: "thanks", name: "Reply: thanks", steps: keyboardOnlySteps(), play: PLAY({ vary: false }), data: null,
    triggers: [{ type: "hotkey", on: true, combo: ["F14"] }], last: { ok: true, text: "12:02" }, made: 7 },
  { id: "photos", name: "Batch rename photos", steps: invoiceSteps().slice(0, 6), play: PLAY(), data: null,
    triggers: [{ type: "app", on: true, exe: "EXCEL.EXE", delay: 2 }], last: null, made: 2 },
  { id: "standup", name: "Open standup tools", steps: invoiceSteps().slice(0, 7), play: PLAY(), data: null,
    triggers: [{ type: "image", on: true, match: 85 }], last: { ok: true, text: "Thu 21:42" }, made: 1 },
];

const RUNS = [
  { macro: "invoice", by: "Schedule", when: "Today, 09:00", ok: true, end: "Completed", loops: 3, dur: "31.8 s", checks: ["Step 7 · Pixel check matched after 0.6 s", "Step 10 · Image found at 812, 440 (97%) after 0.4 s"] },
  { macro: "claims", by: "Hotkey", when: "Yesterday, 16:02", ok: false, end: "Pixel check timed out at step 10, row 112", sub: "PolicyNo 88-1042 · stopped after 111 rows", dur: "14 min 22 s", step: 10, checks: ["Rows 1–111 · Confirmation banner matched after 0.4–1.9 s", "Row 112 · Confirmation banner never matched (8 s)"] },
  { macro: "timesheet", by: "Schedule", when: "Fri, 17:00", ok: false, end: "Image not found at step 5", sub: "Best match was 61%", dur: "9.4 s", step: 5, checks: ["Step 5 · Image not found: the best match was 61%"] },
  { macro: "standup", by: "Image trigger", when: "Thu, 21:42", ok: true, end: "Completed", dur: "5.2 s", checks: [] },
  { macro: "invoice", by: "Schedule", when: "Thu, 09:00", ok: null, end: "Skipped: Relay was busy", dur: "", checks: [] },
  { macro: "refund", by: "Hotkey", when: "Thu, 12:41", ok: true, end: "Completed", dur: "2.1 s", checks: [] },
];

// AltGr characters, by layout: Windows treats Ctrl + Alt as AltGr.
const ALTGR = {
  us: {},
  "be-azerty": { Digit1: "|", Digit2: "@", Digit3: "#", Digit6: "^", Digit9: "{", Digit0: "}", KeyE: "€", BracketLeft: "[", BracketRight: "]" },
  "fr-azerty": { Digit2: "~", Digit3: "#", Digit4: "{", Digit5: "[", Digit6: "|", Digit7: "`", Digit8: "\\", Digit9: "^", Digit0: "@", Minus: "]", Equal: "}", KeyE: "€" },
  "de-qwertz": { KeyQ: "@", KeyE: "€", Digit2: "²", Digit3: "³", Digit7: "{", Digit8: "[", Digit9: "]", Digit0: "}", Minus: "\\", KeyM: "µ", BracketRight: "~" },
  "sv-qwerty": { Digit2: "@", Digit3: "£", Digit4: "$", Digit7: "{", Digit8: "[", Digit9: "]", Digit0: "}", Minus: "\\", KeyE: "€", KeyM: "µ" },
};
const LAYOUT_NAME = { us: "US", "be-azerty": "Belgian AZERTY", "fr-azerty": "French AZERTY", "de-qwertz": "German QWERTZ", "sv-qwerty": "Swedish" };

const TAKEN = {
  "F9": "F9 recalculates in Excel and updates fields in Word. While Relay runs, they won't get it.",
  "F10": "F10 opens the menu bar in most apps and steps over in debuggers. While Relay runs, they won't get it.",
  "Ctrl+Shift+F9": "Ctrl + Shift + F9 unlinks fields in Word, a rare command. Word won't get it while Relay runs.",
  "Ctrl+Alt+End": "Remote Desktop sends this as Ctrl + Alt + Del. In a remote session, Relay won't get it.",
  "Ctrl+Alt+Home": "Remote Desktop uses this for its connection bar. In a remote session, Relay won't get it.",
  "Ctrl+Alt+Pause": "Remote Desktop uses this to switch to full screen. In a remote session, Relay won't get it.",
  "Ctrl+Alt+Insert": "VMware Horizon sends this as Ctrl + Alt + Del. In a Horizon session, Relay won't get it.",
  "Ctrl+F1": "Citrix Workspace uses this for Ctrl + Alt + Del. In a Citrix session, Relay won't get it.",
  "Ctrl+Alt+Minus": "Remote Desktop uses this for a screenshot. In a remote session, Relay won't get it.",
};

// ── State ───────────────────────────────────────────────────────────────────

const q = new URLSearchParams(location.search);
const st = {
  screen: q.get("screen") ?? "library",
  firstStep: Number(q.get("first") ?? 0),
  macroId: q.get("macro") ?? "invoice",
  // Relay's settings
  theme: q.get("theme") ?? "light", // light | dark | system
  glass: q.has("glass") ? q.get("glass") === "on" : null, // null: the default for this Windows
  spacing: q.get("spacing") ?? "comfortable",
  textSize: Number(q.get("text") ?? 100),
  playerSize: q.get("player") ?? "compact",
  reduceMotion: q.get("motion") ?? "system", // system | on
  sounds: q.has("sounds"),
  hideRecording: true,
  hidePlaying: false,
  keepOnTop: "always",
  closeToTray: true,
  startWithWindows: false,
  clickLabels: true,
  pathMode: "full",
  recording: { moves: true, keys: true, screenshot: true, countdown: true, escStops: true, ignoreSimulated: true },
  keys: { record: ["Ctrl", "Shift", "F9"], play: ["Ctrl", "Shift", "F10"], player: ["Ctrl", "Shift", "M"], kill: ["Ctrl", "Alt", "Backspace"] },
  // The machine Relay runs on
  os: q.get("os") ?? "win11",
  transparency: !q.has("notransparency"),
  systemMotion: q.has("sysreduce") ? "reduce" : "full",
  systemDark: q.has("sysdark"),
  layout: q.get("layout") ?? "us",
  winSize: q.get("size") ?? "default",
  failRow: true,
  failed: null, // { macro, step }: the step that failed in the last run
  // UI state
  libFilter: "all",
  libSort: "name",
  libSearch: "",
  libSel: new Set(),
  actFilter: q.get("act") ?? "all",
  openRun: null,
  settingsSection: q.get("section") ?? "recording",
  tab: q.get("tab") ?? "step",
  sel: new Set(),
  cursor: null,
  anchor: null,
  showMoves: q.has("moves"),
  find: "",
  viewOpen: null, // null: by the macro (closed for keyboard-only macros)
  inspOpen: q.has("inspector"),
  menu: null, // { id, x, y, ... }
  dialog: q.get("dialog"),
  exportFmt: "rly",
  capture: null, // { for: "record" | "trigger:0", combo, msg }
  testResult: null,
  picking: null,
  toasts: [],
  session: "idle", // idle | countdown | recording | playing | paused
  count: 3,
  recSteps: 0,
  t: 0,
  row: 1,
  playerMacro: null,
  error: null,
  done: false,
  undo: [],
  redo: [],
  saved: "saved",
  firstChoice: "ctrlshift",
  frame: q.get("frame"),
  playerMode: q.get("mode") === "player",
  macroSection: q.get("msec") ?? "play", // play | runs: the Macro tab's open section
};
if (q.get("step")) { const m = MACROS.find((x) => x.id === st.macroId); const st0 = m?.steps[Number(q.get("step")) - 1]; if (st0) { st.sel = new Set([st0.id]); st.cursor = st0.id; } }
if (q.has("playing")) { st.session = "playing"; st.playerMacro = st.macroId; st.t = Number(q.get("playing")) || 6.1; st.row = 37; }
if (q.has("recording")) { st.session = "recording"; st.playerMacro = st.macroId; st.recSteps = 14; }
if (q.has("error")) { st.playerMacro = st.macroId; st.error = { text: "Pixel check timed out · step 10 · row 112", step: 10 }; st.failed = { macro: st.macroId, step: 10 }; }

const macro = () => MACROS.find((m) => m.id === st.macroId) ?? MACROS[0];
const glassOn = () => st.transparency && (st.glass ?? st.os === "win11");
const isDark = () => (st.theme === "system" ? st.systemDark : st.theme === "dark");
const calm = () => st.reduceMotion === "on" || st.systemMotion === "reduce";
const WIN = { min: [760, 520], default: [944, 612], large: [1280, 800], small: [1368 * 0.6, 640] };
const zoom = () => st.textSize / 100;
// A bigger text size makes the window bigger too, as far as the screen allows.
const winPx = () => { const [w, h] = WIN[st.winSize]; return [Math.min(w * zoom(), innerWidth - 32), Math.min(h * zoom(), innerHeight - 40 - 56 - 12)]; };
const effWidth = () => winPx()[0] / zoom();
const effHeight = () => winPx()[1] / zoom();
const narrow = () => effWidth() < 860;
const tight = () => effWidth() < 700;
// The player shows when the window doesn't: while recording (the window hides), in player mode, or when asked for.
const windowAway = () => st.playerMode || ((st.session === "recording" || st.session === "countdown") && st.hideRecording) || ((st.session === "playing" || st.session === "paused") && st.hidePlaying);
const viewDefault = (m) => hasPointer(m) && effHeight() >= 470;

// ── Helpers ─────────────────────────────────────────────────────────────────

const esc = (v) => String(v ?? "").replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);
const fmt = (t) => { const m = Math.floor(t / 60); return `${String(m).padStart(2, "0")}:${(t - m * 60).toFixed(2).padStart(5, "0")}`; };
const fmtShort = (t) => { const m = Math.floor(t / 60); return `${String(m).padStart(2, "0")}:${String(Math.floor(t - m * 60)).padStart(2, "0")}`; };
const duration = (m) => { const last = m.steps[m.steps.length - 1]; return last ? +(last.t + (last.dur ?? 0.15)).toFixed(2) : 0; };
const kc = (combo) => (combo?.length ? `<span class="keycaps">${combo.map((k) => `<span class="kc">${esc(k)}</span>`).join("")}</span>` : `<span class="muted">None</span>`);
const comboText = (combo) => (combo?.length ? combo.join(" + ") : "none");
const comboKey = (combo) => combo.join("+");
const glyph = (kind, sm = false) => `<span class="glyph ${CAT[kind]}${sm ? " sm" : ""}" aria-hidden="true"><i></i></span>`;
const plural = (n, w, ws = w + "s") => `${n} ${n === 1 ? w : ws}`;
const hasPointer = (m) => m.steps.filter((x) => CAT[x.kind] === "pointer").length > 1;

function stepDetail(x) {
  switch (x.kind) {
    case "click": case "dclick": return x.label || `${x.x}, ${x.y} px`;
    case "keys": return x.combo.join(" + ");
    case "type": return `“${x.text}”`;
    case "text": return `“${x.text.replace(/\n/g, " ⏎ ")}”`;
    case "wait": return `${x.dur.toFixed(1)} s${x.label ? " · " + x.label : ""}`;
    case "pixel": return `pixel ${x.x}, ${x.y}${x.label ? " · " + x.label : ""}`;
    case "find": return x.label || "an image";
    case "move": return `${x.from[0]}, ${x.from[1]} → ${x.x}, ${x.y}`;
    default: return "";
  }
}
function stepEnd(x) {
  switch (x.kind) {
    case "click": return x.move ? `${x.move.dur.toFixed(2)} s` : "";
    case "type": return plural(x.text.length, "char");
    case "pixel": case "find": return `≤ ${x.timeout} s`;
    case "move": return `${x.dur.toFixed(2)} s`;
    default: return "";
  }
}
function triggerSentence(tr) {
  switch (tr.type) {
    case "hotkey": return `Runs from any app`;
    case "schedule": {
      const names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
      const on = tr.days.map((d, i) => (d ? names[i] : null)).filter(Boolean);
      const days = on.length === 5 && tr.days.slice(0, 5).every(Boolean) ? "Mon–Fri" : on.join(", ") || "Pick a day";
      return `${days} at ${tr.time}`;
    }
    case "app": return `When ${tr.exe} starts, after ${tr.delay} s`;
    case "pixel": return `When ${tr.x}, ${tr.y} becomes ${tr.color}`;
    case "image": return `When the image appears (${tr.match}% match)`;
  }
}
function alsoRuns(m) {
  return m.triggers.filter((t) => t.type !== "hotkey" && t.on).map((t) =>
    t.type === "schedule" ? triggerSentence(t) : t.type === "app" ? `When ${t.exe} starts` : t.type === "image" ? "When an image appears" : "When a pixel changes").join(" · ");
}
const hotkeyOf = (m) => m.triggers.find((t) => t.type === "hotkey" && t.on)?.combo;

let liveText = "";
function announce(text) { liveText = text; const el = document.getElementById("live"); if (el) { el.textContent = ""; setTimeout(() => (el.textContent = text), 30); } }

function toast(msg, opts = {}) {
  const id = Math.random().toString(36).slice(2);
  st.toasts = [...st.toasts.slice(-2), { id, msg, kind: opts.kind ?? "info", action: opts.action }];
  if (opts.kind !== "error") setTimeout(() => { st.toasts = st.toasts.filter((t) => t.id !== id); render(); }, opts.action ? 8000 : 5000);
  announce(msg);
}

function snapshot() {
  const m = macro();
  st.undo.push({ id: m.id, steps: structuredClone(m.steps), name: m.name, play: structuredClone(m.play), triggers: structuredClone(m.triggers), sel: [...st.sel], cursor: st.cursor });
  st.redo = [];
  st.saved = "saving";
  setTimeout(() => { st.saved = "saved"; render(); }, 450);
}
function restore(from, to) {
  const snap = from.pop(); if (!snap) return;
  const m = MACROS.find((x) => x.id === snap.id);
  to.push({ id: m.id, steps: structuredClone(m.steps), name: m.name, play: structuredClone(m.play), triggers: structuredClone(m.triggers), sel: [...st.sel], cursor: st.cursor });
  Object.assign(m, { steps: snap.steps, name: snap.name, play: snap.play, triggers: snap.triggers });
  st.macroId = m.id;
  if (snap.sel) { st.sel = new Set(snap.sel); st.cursor = snap.cursor; }
}

// ── Sounds ──────────────────────────────────────────────────────────────────

let audio = null;
function blip(freqs, len = 0.05) {
  if (!st.sounds) return;
  audio ??= new AudioContext();
  let t = audio.currentTime;
  for (const f of freqs) {
    const o = audio.createOscillator(), g = audio.createGain();
    o.type = "square"; o.frequency.value = f;
    g.gain.setValueAtTime(0.0001, t); g.gain.exponentialRampToValueAtTime(0.06, t + 0.005); g.gain.exponentialRampToValueAtTime(0.0001, t + len);
    o.connect(g).connect(audio.destination); o.start(t); o.stop(t + len + 0.02); t += len + 0.03;
  }
}
const SOUND = { record: () => blip([660, 990]), stop: () => blip([990, 660]), done: () => blip([784, 1047, 1319], 0.06), error: () => blip([220, 196], 0.09), tick: () => blip([1400], 0.02) };

// ── Rendering ───────────────────────────────────────────────────────────────

function envBar() {
  const opt = (v, cur, label) => `<option value="${v}"${v === cur ? " selected" : ""}>${label}</option>`;
  return `<div class="env" role="region" aria-label="Prototype controls (not part of Relay)">
    <strong>PROTOTYPE</strong>
    <label>Screen <select data-env="screen">${["firstrun", "library", "activity", "editor", "settings"].map((x) => opt(x, st.screen, x === "firstrun" ? "first run" : x)).join("")}</select></label>
    <label>Windows <select data-env="os">${opt("win11", st.os, "11")}${opt("win10", st.os, "10")}</select></label>
    <label>Transparency effects <select data-env="transparency">${opt("on", st.transparency ? "on" : "off", "on")}${opt("off", st.transparency ? "on" : "off", "off (or Remote Desktop)")}</select></label>
    <label>Windows reduce motion <select data-env="systemMotion">${opt("full", st.systemMotion, "off")}${opt("reduce", st.systemMotion, "on")}</select></label>
    <label>Keyboard <select data-env="layout">${Object.keys(LAYOUT_NAME).map((k) => opt(k, st.layout, LAYOUT_NAME[k])).join("")}</select></label>
    <label>Window <select data-env="winSize">${opt("min", st.winSize, "760 × 520 (minimum)")}${opt("default", st.winSize, "944 × 612 (default)")}${opt("large", st.winSize, "1280 × 800")}</select></label>
    <button type="button" data-act="env-reset">Reset</button>
  </div>`;
}

function topWinCtl() {
  return `<span class="winctl">
    <button class="key ghost small icon-only" type="button" aria-label="Minimize" data-fid="win-min" title="Minimize">${icon("minimize", 14)}</button>
    <button class="key ghost small icon-only" type="button" aria-label="${st.closeToTray ? "Hide to the tray" : "Quit"}" data-fid="win-close" title="${st.closeToTray ? "Hide to the tray" : "Quit"}">${icon("close", 14)}</button>
  </span>`;
}

function sideNav(active) {
  const problems = RUNS.filter((r) => r.ok === false).length;
  const n = (f) => MACROS.filter(f).length;
  const item = (id, label, count, extra = "") => `<button class="navi" type="button" data-act="nav" data-to="${id}" data-fid="nav-${id}"${active === id ? ' aria-current="page"' : ""}>${label}${count != null ? `<span class="n num">${count}</span>` : ""}${extra}</button>`;
  return `<nav class="side" aria-label="Library">
    <span class="label" id="lbl-macros">Macros</span>
    ${item("all", "All", MACROS.length)}
    ${item("triggers", "With triggers", n((m) => m.triggers.some((t) => t.on)))}
    ${item("recent", "Recently run", n((m) => m.last))}
    <span class="label" style="margin-top:8px">Activity</span>
    ${item("runs", "All runs", null)}
    ${item("problems", "Problems", null, problems ? `<span class="badge" aria-label="${problems} problems">${problems}</span>` : "")}
    <span class="spacer"></span>
    <button class="navi" type="button" data-act="import" data-fid="nav-import"><span style="display:flex;gap:8px;align-items:center">${icon("import")}Import…</span></button>
  </nav>`;
}

function libraryScreen() {
  const filterFn = { all: () => true, triggers: (m) => m.triggers.some((t) => t.on), recent: (m) => m.last }[st.libFilter] ?? (() => true);
  let list = MACROS.filter(filterFn).filter((m) => m.name.toLowerCase().includes(st.libSearch.toLowerCase()));
  list = [...list].sort(st.libSort === "name" ? (a, b) => a.name.localeCompare(b.name) : (a, b) => b.made - a.made);
  const sel = st.libSel;
  const rows = list.map((m) => {
    const hk = hotkeyOf(m);
    const last = m.last ? (m.last.ok ? `<span class="ok">${icon("check", 14)}${esc(m.last.text)}</span>` : `<span class="bad">${icon("error", 14)}${esc(m.last.text)}</span>`) : `<span class="muted" style="font-size:13px">Never</span>`;
    return `<tr class="${sel.has(m.id) ? "sel" : ""}">
      <td><button class="check" role="checkbox" aria-checked="${sel.has(m.id)}" aria-label="Select ${esc(m.name)}" type="button" data-act="lib-check" data-id="${m.id}" data-fid="chk-${m.id}">${sel.has(m.id) ? icon("check", 12) : ""}</button></td>
      <td><button class="mname" type="button" data-act="open" data-id="${m.id}" data-fid="open-${m.id}"><b>${esc(m.name)}</b><span class="num">${duration(m).toFixed(1)} s · ${plural(m.steps.length, "step")}${m.data ? ` · ${m.data.rows} rows` : ""}</span></button></td>
      <td>${hk ? kc(hk) : ""}</td>
      <td class="muted" style="font-size:13px">${alsoRuns(m) ? `<button class="linkish" type="button" data-act="open-runs" data-id="${m.id}" data-fid="runs-${m.id}" aria-label="${esc(alsoRuns(m))}: open its triggers">${esc(alsoRuns(m))}</button>` : ""}</td>
      <td>${last}</td>
      <td style="width:44px"><button class="key ghost small icon-only" type="button" aria-label="More for ${esc(m.name)}" aria-haspopup="menu" data-act="menu" data-menu="row" data-id="${m.id}" data-fid="more-${m.id}">${icon("more")}</button></td>
    </tr>`;
  }).join("");
  const bulk = sel.size ? `<div class="bulk"><b>${plural(sel.size, "macro")} selected</b><span class="spacer"></span>
      <button class="key small" type="button" data-act="bulk-export" data-fid="bulk-export">${icon("export", 14)}Export…</button>
      <button class="key small danger" type="button" data-act="bulk-delete" data-fid="bulk-delete">${icon("trash", 14)}Delete</button>
      <button class="key ghost small" type="button" data-act="bulk-clear" data-fid="bulk-clear">Clear</button></div>` : "";
  return `<div class="window lib glass${winCls()}" role="application" aria-label="Relay">
    <div class="topbar">
      <span class="brand"><span class="mark" aria-hidden="true"></span>Relay</span>
      <span class="spacer"></span>
      ${recordKey(true)}
      ${playerKey()}
      <button class="key small" type="button" aria-label="Settings" title="Settings" data-act="go" data-to="settings" data-fid="go-settings">${icon("sliders")}<span class="hide-tight">Settings</span></button>
      ${topWinCtl()}
    </div>
    ${sideNav(st.libFilter)}
    <div class="plate table">
      <div class="table-tools">
        <div class="search">${icon("search", 14)}<input class="field" type="search" placeholder="Search macros" aria-label="Search macros" value="${esc(st.libSearch)}" data-input="libSearch" data-fid="lib-search"></div>
        <div class="seg" role="radiogroup" aria-label="Sort by">${["name", "made"].map((k) => `<button type="button" role="radio" aria-checked="${st.libSort === k}" data-act="lib-sort" data-v="${k}" data-fid="sort-${k}">${k === "name" ? "Name" : "Newest"}</button>`).join("")}</div>
      </div>
      <div class="rows">${list.length ? `<table class="macros" aria-label="Macros">
        <thead><tr><th><span class="sr-only">Select</span></th><th class="label">Name</th><th class="label">Hotkey</th><th class="label">Also runs</th><th class="label">Last run</th><th><span class="sr-only">Actions</span></th></tr></thead>
        <tbody>${rows}</tbody></table>` : `<div class="empty"><b>No macros match “${esc(st.libSearch)}”</b><span>Try another word, or clear the search.</span></div>`}</div>
      ${bulk}
    </div>
    ${toastsHtml()}
  </div>`;
}

function activityScreen() {
  const runs = RUNS.filter((r) => st.libFilter !== "problems" || r.ok === false);
  const item = (r, i) => {
    const m = MACROS.find((x) => x.id === r.macro);
    const ic = r.ok === true ? `<span class="ok" aria-label="Completed">${icon("check")}</span>` : r.ok === false ? `<span class="bad" aria-label="Failed">${icon("error")}</span>` : `<span class="muted" aria-label="Skipped">${icon("info")}</span>`;
    const open = st.openRun === i;
    return `<div class="run${open ? " open" : ""}">
      <button class="run-head" type="button" aria-expanded="${open}" data-act="run" data-i="${i}" data-fid="run-${i}">
        ${ic}<span><b>${esc(m.name)}</b><span class="sub">${esc(r.by)} · ${esc(r.when)}${r.loops ? ` · ${r.loops} loops` : ""}</span></span>
        <span class="${r.ok === false ? "bad" : r.ok ? "ok" : "muted"}" style="font-size:13px">${esc(r.end)}</span>
        <span class="muted num" style="font-size:12px;min-width:70px;text-align:right">${esc(r.dur)}</span>
      </button>
      ${open ? `<div class="run-detail">${r.sub ? `<div class="line"><b>${esc(r.sub)}</b></div>` : ""}${r.checks.map((c) => `<div class="line">${glyph("pixel", true)}${esc(c)}</div>`).join("")}
        ${r.step ? `<div class="btnrow"><button class="key small" type="button" data-act="open-at" data-id="${m.id}" data-step="${r.step}" data-fid="openat-${i}">Open at step ${r.step}</button></div>` : ""}</div>` : ""}
    </div>`;
  };
  return `<div class="window lib glass${winCls()}" role="application" aria-label="Relay">
    <div class="topbar">
      <span class="brand"><span class="mark" aria-hidden="true"></span>Relay</span>
      <span class="spacer"></span>
      ${recordKey(true)}
      ${playerKey()}
      <button class="key small" type="button" aria-label="Settings" title="Settings" data-act="go" data-to="settings" data-fid="go-settings">${icon("sliders")}<span class="hide-tight">Settings</span></button>
      ${topWinCtl()}
    </div>
    ${sideNav(st.libFilter)}
    <div class="plate table">
      <div class="table-tools"><span class="page-title">${st.libFilter === "problems" ? "Problems" : "All runs"}</span><span class="spacer"></span><span class="muted" style="font-size:12px">Relay keeps the last 200 runs.</span></div>
      <div class="runs">${runs.map((r) => item(r, RUNS.indexOf(r))).join("") || `<div class="empty"><b>No problems</b><span>Every run ended as planned.</span></div>`}</div>
    </div>
    ${toastsHtml()}
  </div>`;
}

function recordKey(withLabel) {
  const live = st.session === "recording" || st.session === "countdown";
  const k = comboText(st.keys.record);
  return withLabel
    ? `<button class="key red${live ? " live" : ""}" type="button" data-act="record" data-fid="rec-lib" aria-label="${live ? "Stop recording" : "Record"} (${k})" title="${live ? "Stop recording" : "Record"} (${k})"><span class="rec-dot"></span>${live ? "Stop" : "Record"}</button>`
    : `<button class="key red round${live ? " live" : ""}" type="button" data-act="record" data-fid="rec" aria-label="${live ? "Stop recording" : "Record a new macro"} (${k})" title="${live ? "Stop recording" : "Record a new macro"} (${k})"><span class="rec-dot"></span></button>`;
}

function editorScreen() {
  const m = macro();
  const playing = st.session === "playing" || st.session === "paused";
  const pk = comboText(st.keys.play);
  const chips = [];
  chips.push(m.data ? `Each row (${m.data.rows})` : m.play.forever ? "∞" : `×${m.play.repeat}`);
  chips.push(`${m.play.speed}×`);
  if (m.play.vary) chips.push(`±${m.play.jitter} ms`);
  const chipName = `${m.data ? `Once per row of ${m.data.file}` : m.play.forever ? "Loops until stopped" : `Repeats ${plural(m.play.repeat, "time")}`}, ${m.play.speed === 1 ? "normal speed" : `${m.play.speed} times as fast`}${m.play.vary ? `, timing varies by up to ${m.play.jitter} ms` : ""}${m.play.stopOnKey ? ", any key stops it" : ""}. Open the play options.`;
  const viewOpen = st.viewOpen ?? viewDefault(m);
  return `<div class="window editor glass${winCls()}" role="application" aria-label="Relay: ${esc(m.name)}">
    <div class="topbar">
      <button class="key ghost small" type="button" data-act="go" data-to="library" data-fid="back" aria-label="Back to the Library (Alt + ←)" title="Back to the Library (Alt + ←)">${icon("back")}<span class="hide-narrow">Library</span></button>
      <input class="title-input" aria-label="Macro name" value="${esc(m.name)}" data-input="name" data-fid="name" spellcheck="false">
      <span class="saved${st.saved === "saved" ? " fade" : ""}" role="status">${st.saved === "saving" ? `<span class="hide-narrow">Saving…</span>` : `${icon("check", 12)}<span class="hide-narrow">Saved</span>`}</span>
      <span class="spacer"></span>
      <button class="key small icon-only" type="button" aria-label="Undo (Ctrl + Z)" title="Undo (Ctrl + Z)" data-act="undo" data-fid="undo"${st.undo.length && !playing ? "" : " disabled"}>${icon("undo")}</button>
      <button class="key small icon-only" type="button" aria-label="Redo (Ctrl + Y)" title="Redo (Ctrl + Y)" data-act="redo" data-fid="redo"${st.redo.length && !playing ? "" : " disabled"}>${icon("redo")}</button>
      <span style="width:4px"></span>
      ${recordKey(false)}
      <button class="key ink" type="button" data-act="play" data-fid="play" aria-label="${st.session === "playing" ? "Pause" : "Play"} (${pk})" title="${st.session === "playing" ? "Pause" : "Play from the playhead"} (${pk})">${icon(st.session === "playing" ? "pause" : "play", 14)}<span class="hide-tight">${st.session === "playing" ? "Pause" : "Play"}</span></button>
      ${playing ? `<button class="key small icon-only" type="button" aria-label="Stop (Esc)" title="Stop (Esc)" data-act="stop" data-fid="stop">${icon("stop", 14)}</button>` : ""}
      <button class="chip" type="button" data-act="chips" data-fid="chips" aria-label="${esc(chipName)}" title="${esc(chipName)}">${chips.map((c) => `<span>${esc(c)}</span>`).join('<span aria-hidden="true" class="muted">·</span>')}${m.play.stopOnKey ? "" : `<span aria-hidden="true" class="muted">·</span><span style="color:var(--red-text)">keys don't stop it</span>`}</button>
      <button class="key ghost small icon-only" type="button" aria-label="More" title="More" aria-haspopup="menu" data-act="menu" data-menu="macro" data-fid="macro-more">${icon("more")}</button>
      ${playerKey()}
      ${topWinCtl()}
    </div>
    <div class="main">
      ${stepsPlate(m)}
      ${inspector(m)}
    </div>
    ${viewPlate(m, viewOpen)}
    ${toastsHtml()}
  </div>`;
}

function visibleRows(m) {
  const rows = [];
  const f = st.find.trim().toLowerCase();
  for (const x of m.steps) {
    if (st.showMoves && x.move) rows.push({ kind: "move", id: `m${x.id}`, of: x.id, t: +(x.t - x.move.dur).toFixed(2), dur: x.move.dur, from: x.move.from, x: x.x, y: x.y });
    rows.push(x);
  }
  if (!f) return rows;
  return rows.filter((x) => `${KIND_NAME[x.kind]} ${stepDetail(x)}`.toLowerCase().includes(f));
}

function stepsPlate(m) {
  const rows = visibleRows(m);
  const cursor = st.cursor ?? m.steps[m.steps.length - 1]?.id;
  const failedStep = st.failed?.macro === m.id ? m.steps[st.failed.step - 1]?.id : null;
  const html = [];
  rows.forEach((x, i) => {
    const prev = m.steps[m.steps.indexOf(x) - 1];
    if (x.kind !== "move" && x.pause >= 1 && !st.find) html.push(`<div class="pause" aria-hidden="true">${x.pause.toFixed(1)} s pause</div>`);
    const idx = m.steps.indexOf(x);
    const num = x.kind === "move" ? "" : `${idx + 1}`;
    const selected = st.sel.has(x.id);
    const played = (st.session === "playing" || st.session === "paused" || st.t > 0) && x.t <= st.t;
    const now = st.session === "playing" && x.t <= st.t && st.t - x.t < 0.5;
    html.push(`<div class="step ${x.kind === "move" ? "move-row" : CAT[x.kind]}${selected ? "" : ""}${played ? " played" : ""}${now ? " now" : ""}${x.id === failedStep ? " failed" : ""}${x.id === cursor ? " cursor" : ""}"
      role="option" id="row-${x.id}" aria-selected="${selected}" data-row="${x.id}" aria-label="Step ${num}: ${esc(KIND_NAME[x.kind])}, ${esc(stepDetail(x))}${x.id === failedStep ? ", failed in the last run" : ""}">
      <span class="t num">${fmt(x.t)}</span>${glyph(x.kind)}<span class="what"><b>${esc(KIND_NAME[x.kind])}</b> <span>· ${esc(stepDetail(x))}</span></span><span class="end num">${x.id === failedStep ? "Timed out" : esc(stepEnd(x))}</span>
    </div>`);
    if (x.id === cursor && !st.find) html.push(`<div class="insert" aria-hidden="true">+ insert here</div>`);
    else if (!st.find && i < rows.length - 1) html.push(`<div class="gap-plus" aria-hidden="true"><button type="button" tabindex="-1" aria-label="Insert after step ${num}" data-act="gap" data-after="${x.id}">${icon("plus", 12)}</button></div>`);
  });
  const activeId = st.cursor ? (rows.find((r) => r.id === st.cursor) ? `row-${st.cursor}` : "") : "";
  return `<div class="plate steps">
    <div class="steps-head">
      <span class="label" id="steps-label">Steps <span class="num">${m.steps.length}</span></span>
      <div class="search">${icon("search", 14)}<input class="field" type="search" placeholder="Find a step" aria-label="Find a step (Ctrl + F)" value="${esc(st.find)}" data-input="find" data-fid="find"></div>
      <button class="key small" type="button" aria-haspopup="menu" data-act="menu" data-menu="add" data-fid="add" title="Add a step at the insertion point (Insert)"${st.session !== "idle" ? " disabled" : ""}>${icon("plus", 14)}Add</button>
      <button class="key ghost small icon-only" type="button" aria-label="Steps options" title="Steps options" aria-haspopup="menu" data-act="menu" data-menu="list" data-fid="list-more">${icon("more")}</button>
    </div>
    <div class="list" role="listbox" aria-multiselectable="true" aria-labelledby="steps-label" tabindex="0" data-fid="list"${activeId ? ` aria-activedescendant="${activeId}"` : ""}>
      ${html.join("") || `<div class="empty"><b>No steps match “${esc(st.find)}”</b></div>`}
    </div>
  </div>`;
}

function inspector(m) {
  const all = visibleRows(m);
  const sel = [...st.sel].map((id) => m.steps.find((x) => x.id === id) ?? all.find((x) => x.id === id)).filter(Boolean);
  const isMacro = st.tab === "macro";
  let body;
  const playingNow = st.session === "playing" || st.session === "paused";
  if (isMacro) body = macroPane(m);
  else if (playingNow) body = `<div id="now-pane" style="display:flex;flex-direction:column;gap:12px">${nowPane(m)}</div>`;
  else if (sel.length > 1) body = multiPane(m, sel);
  else if (sel.length === 1) body = stepPane(m, sel[0]);
  else body = `<div class="empty" style="padding:24px 0"><b>No step selected</b><span>Select a step to edit it, or open the Macro tab for how the macro plays and when it runs.</span></div>`;
  const locked = st.session !== "idle" && !isMacro ? `<div class="notice">${icon("lock", 14)}Stop playback to edit steps. Speed and the other play options can still change.</div>` : "";
  return `<div class="plate inspector${st.inspOpen ? " open" : ""}" role="region" aria-label="Inspector">
    <div style="display:flex;gap:8px;align-items:center">
      <div class="tabs${isMacro ? " second" : ""}" role="tablist" aria-label="Inspector" style="flex:1">
        <span class="thumb" aria-hidden="true"></span>
        <button role="tab" type="button" aria-selected="${!isMacro}" aria-controls="insp-panel" data-act="tab" data-v="step" data-fid="tab-step" tabindex="${isMacro ? -1 : 0}">Step</button>
        <button role="tab" type="button" aria-selected="${isMacro}" aria-controls="insp-panel" data-act="tab" data-v="macro" data-fid="tab-macro" tabindex="${isMacro ? 0 : -1}" title="Macro (Ctrl + M)">Macro</button>
      </div>
      <button class="key ghost small icon-only insp-close" type="button" aria-label="Close the inspector" data-act="insp-close" data-fid="insp-close">${icon("close", 14)}</button>
    </div>
    <div id="insp-panel" role="tabpanel" style="display:flex;flex-direction:column;gap:12px;flex:1;min-height:0">${locked}${body}</div>
  </div>`;
}

function currentStep(m) { return [...m.steps].reverse().find((x) => x.t <= st.t) ?? m.steps[0]; }
function nowPane(m) {
  const x = currentStep(m); if (!x) return "";
  const n = m.steps.indexOf(x) + 1;
  return `<div class="insp-title"><span class="label" style="display:flex;gap:6px;align-items:center">${glyph(x.kind, true)}Now · step ${n} of ${m.steps.length}</span><span class="name">${esc(KIND_NAME[x.kind])} · ${esc(stepDetail(x))}</span></div>
    ${m.data ? `<div class="spec"><span class="k">Row</span><span class="num">${st.row} of ${m.data.rows}</span><span class="k">PolicyNo</span><span class="num">88-${String(1000 + st.row).padStart(4, "0")}</span></div>` : ""}
    ${x.kind === "pixel" ? `<span class="hint">Waits until ${x.x}, ${x.y} matches ${x.color}, up to ${x.timeout} s.</span>` : ""}`;
}

function unitField(id, value, unit, label, opts = {}) {
  return `<span class="unit-field${opts.scrub ? " scrub" : ""}" ${opts.scrub ? `data-scrub="${id}" data-step="${opts.step ?? 0.05}" data-min="${opts.min ?? 0}" data-max="${opts.max ?? 9999}" data-dec="${opts.dec ?? 2}"` : ""}>
    ${opts.pre ? `<span class="pre" aria-hidden="true">${opts.pre}</span>` : ""}<input class="num" inputmode="decimal" aria-label="${esc(label)}${unit ? ` in ${unit === "s" ? "seconds" : unit}` : ""}" value="${esc(value)}" data-field="${id}" data-fid="f-${id}"${st.session !== "idle" ? " disabled" : ""}>
    ${unit ? `<span class="unit">${unit}</span>` : ""}${opts.scrub ? '<span class="ticks" aria-hidden="true"></span>' : ""}</span>`;
}

function stepPane(m, x) {
  const idx = x.kind === "move" ? `${m.steps.findIndex((y) => y.id === x.of) + 1}, its move` : m.steps.indexOf(x) + 1;
  const head = `<div class="insp-title"><span class="label" style="display:flex;gap:6px;align-items:center">${glyph(x.kind, true)}${esc(KIND_NAME[x.kind])} · step ${idx}</span><span class="name">${esc(x.label || stepDetail(x))}</span></div>`;
  const pause = `<span class="k">Pause before</span>${unitField("pause", x.pause.toFixed(2), "s", "Pause before", { scrub: true, step: 0.05, dec: 2 })}`;
  const label = (ph = "Optional") => `<span class="k">Label</span><input class="field" aria-label="Label" placeholder="${ph}" value="${esc(x.label)}" data-field="label" data-fid="f-label">`;
  const failed = st.failed?.macro === m.id && m.steps[st.failed.step - 1]?.id === x.id ? `<div class="notice" role="note" style="color:var(--red-text)">${icon("warning", 14)}<span><b>Timed out in the last run</b> at row 112: the pixel never matched in 8 s. Raise the timeout, or pick the pixel again.</span></div>` : "";
  let spec = "", extra = "";
  switch (x.kind) {
    case "click":
      spec = `${label("e.g. Save button")}${pause}<span class="k">Position</span><span style="display:flex;gap:6px">${unitField("px", x.x, "", "X", { pre: "X" })}${unitField("py", x.y, "", "Y", { pre: "Y" })}</span>
        <span></span><span><button class="key small" type="button" data-act="pick" data-fid="pick">${icon("pick", 14)}Pick from screen</button></span>
        ${x.move ? `<span class="k">Moving there</span>${unitField("movedur", x.move.dur.toFixed(2), "s", "Moving there", { scrub: true, step: 0.05 })}
        <span class="full btnrow"><button class="key small" type="button" data-act="smooth" data-fid="smooth">${icon("smooth", 14)}Smooth the path</button><button class="key small" type="button" data-act="straighten" data-fid="straighten">${icon("straighten", 14)}Straighten</button></span>` : ""}`;
      break;
    case "keys":
      spec = `<span class="k">Keys</span>${kc(x.combo)}${pause}`;
      break;
    case "type":
      spec = `<span class="k">Types</span><span class="field" style="height:auto;min-height:var(--field-h);padding:6px 10px">“${esc(x.text)}”</span>${pause}`;
      extra = `<div class="btnrow"><button class="key small" type="button" data-act="make-editable" data-fid="make-editable">${icon("text", 14)}Make editable</button></div><span class="hint">An editable text can fill in the date, the time, the clipboard or a data column each run, and types the same whatever the keyboard layout.</span>`;
      break;
    case "text": {
      const cols = m.data?.cols ?? [];
      const sample = x.text.replace(/\{col:(\w+)\}/g, (_, c) => ({ PolicyNo: "88-1001", Amount: "1250,00", Customer: "ACME Ltd" })[c] ?? "").replace("{clipboard}", "Ana").replace("{date}", "2026-10-23").replace("{time}", "14:05:09").replace("{n}", "1");
      spec = `<span class="k full" style="margin-bottom:-4px">Text</span><textarea class="field full" rows="3" aria-label="Text" data-field="text" data-fid="f-text" spellcheck="false">${esc(x.text)}</textarea>
        <div class="placeholders full" role="group" aria-label="Insert a placeholder">${["{date}", "{time}", "{clipboard}", "{n}", ...cols.map((c) => `{col:${c}}`)].map((p) => `<button class="chip" type="button" data-act="ph" data-v="${p}" data-fid="ph-${p}">${esc(p)}</button>`).join("")}</div>
        <div class="types-now full" role="status">Types now: “${esc(sample)}”</div>${pause}`;
      break;
    }
    case "wait":
      spec = `<span class="k">Duration</span>${unitField("dur", x.dur.toFixed(2), "s", "Duration", { scrub: true, step: 0.05 })}${label()}${pause}`;
      break;
    case "pixel":
      spec = `<span class="k">Position</span><span style="display:flex;gap:6px">${unitField("px", x.x, "", "X", { pre: "X" })}${unitField("py", x.y, "", "Y", { pre: "Y" })}</span>
        <span></span><span><button class="key small" type="button" data-act="pick" data-fid="pick">${icon("pick", 14)}Pick from screen</button></span>
        <span class="k">Color</span><span class="unit-field"><span style="width:16px;height:16px;border-radius:4px;background:${x.color};box-shadow:0 0 0 1px var(--line-strong)" aria-hidden="true"></span><input aria-label="Color" value="${x.color}" data-field="color" data-fid="f-color" spellcheck="false"></span>
        <span class="k">Tolerance</span>${unitField("tol", x.tol, "", "Tolerance", { scrub: true, step: 1, dec: 0, max: 255 })}
        <span class="k">Give up after</span>${unitField("timeout", x.timeout, "s", "Give up after", { scrub: true, step: 0.5, dec: 1, min: 0.5 })}
        ${label()}${pause}`;
      break;
    case "find":
      spec = `<span class="full"><button class="shot" type="button" aria-label="The image to find. Click where it should be clicked." data-act="clickpoint" data-fid="shot"><span class="btn-img">Upload ↑</span><span class="pt" style="left:${x.cx * 100}%;top:${x.cy * 100}%"></span></button></span>
        <span class="full btnrow"><button class="key small" type="button" data-fid="snip">${icon("snip", 14)}Snip</button><button class="key small" type="button" data-fid="paste">${icon("paste", 14)}Paste</button><button class="key small" type="button" data-fid="file">${icon("file", 14)}File…</button><button class="key small" type="button" data-act="test" data-fid="test">${icon("target", 14)}Test</button>${st.testResult ? `<button class="key small" type="button" data-fid="show">${icon("eye", 14)}Show</button>` : ""}</span>
        ${st.testResult ? `<span class="hint ok full" role="status">${icon("check", 12)} ${esc(st.testResult)}</span>` : ""}
        <span class="k">Match</span>${unitField("match", x.match, "%", "Match", { scrub: true, step: 1, dec: 0, min: 50, max: 100 })}
        <span class="k">Give up after</span>${unitField("timeout", x.timeout, "s", "Give up after", { scrub: true, step: 0.5, dec: 1, min: 0.5 })}
        <span class="k">Click</span><span class="seg" role="radiogroup" aria-label="Button to click">${["Left", "Right", "Middle"].map((b) => `<button type="button" role="radio" aria-checked="${x.btn === b}" data-act="btn" data-v="${b}" data-fid="btn-${b}">${b}</button>`).join("")}</span>
        ${label()}`;
      break;
    case "move":
      spec = `<span class="k">Duration</span>${unitField("dur", x.dur.toFixed(2), "s", "Duration", { scrub: true })}`;
      break;
  }
  return `${head}${failed}<div class="spec">${spec}</div>${extra}
    <div class="insp-foot"><button class="key small danger" type="button" data-act="delete" data-fid="del-step">${icon("trash", 14)}Delete step</button><span class="spacer"></span><span class="hint">Del</span></div>`;
}

function multiPane(m, sel) {
  const typed = sel.filter((x) => x.kind === "type").length;
  return `<div class="insp-title"><span class="label">${plural(sel.length, "step")} selected</span><span class="name">${[...new Set(sel.map((x) => KIND_NAME[x.kind]))].join(", ")}</span></div>
    <div class="spec"><span class="k">Pause before</span>${unitField("pause-all", "", "s", "Pause before, for all selected", {})}</div>
    <span class="hint">Leave empty to keep each step's own pause.</span>
    <div class="btnrow">${typed ? `<button class="key small" type="button" data-act="make-editable" data-fid="make-editable">${icon("text", 14)}Make ${plural(typed, "text")} editable</button>` : ""}</div>
    <div class="insp-foot"><button class="key small danger" type="button" data-act="delete" data-fid="del-step">${icon("trash", 14)}Delete ${plural(sel.length, "step")}</button></div>`;
}

function macroPane(m) {
  const p = m.play;
  const card = (tr, i) => {
    const icons = { hotkey: "keycap", schedule: "clock", app: "app", pixel: "pixel", image: "image" };
    const title = { hotkey: "Hotkey", schedule: "Schedule", app: "When an app starts", pixel: "When a pixel changes", image: "When an image appears" }[tr.type];
    let body = "";
    if (tr.type === "hotkey") {
      const cap = st.capture?.for === `trigger:${i}`;
      body = `<div style="display:flex;gap:8px;align-items:center">${cap ? `<span class="key small capture listening" aria-live="polite">Press keys…</span>` : kc(tr.combo)}<span class="spacer"></span><button class="key small" type="button" data-act="capture" data-for="trigger:${i}" data-fid="cap-trigger-${i}">${cap ? "Cancel" : "Change"}</button></div>${captureHint(`trigger:${i}`)}`;
    } else if (tr.type === "schedule") {
      const full = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];
      body = `<div class="days" role="group" aria-label="Days">${full.map((d, j) => `<button type="button" aria-pressed="${!!tr.days[j]}" aria-label="${d}" data-act="day" data-i="${i}" data-d="${j}" data-fid="day-${i}-${j}">${d.slice(0, 2)}</button>`).join("")}</div>
        <div style="display:flex;gap:8px;align-items:center"><span class="hint">At</span><span class="unit-field" style="width:90px"><input aria-label="Time" value="${tr.time}" data-fid="time-${i}"></span></div>`;
    } else if (tr.type === "app") body = `<div class="spec"><span class="k">Program</span><input class="field" aria-label="Program" value="${esc(tr.exe)}"><span class="k">Wait</span>${unitField("delay", tr.delay, "s", "Wait after it starts")}</div>`;
    else if (tr.type === "image") body = `<div style="display:flex;gap:8px;align-items:center"><span class="shot" style="width:64px;height:36px"><span class="btn-img" style="padding:3px 8px;font-size:12px">Later</span></span><span class="spacer"></span><button class="key small" type="button">${icon("snip", 14)}Snip</button><button class="key small" type="button">${icon("target", 14)}Test</button></div>`;
    else if (tr.type === "pixel") body = `<div style="display:flex;gap:8px;align-items:center"><span class="muted num" style="font-size:13px">${tr.x}, ${tr.y} · ${tr.color}</span><span class="spacer"></span><button class="key small" type="button" data-act="pick-trigger" data-fid="pick-trigger">${icon("pick", 14)}Pick from screen</button></div>`;
    return `<div class="card"><div class="card-head">${icon(icons[tr.type])}<b>${title}</b><button class="switch" role="switch" aria-checked="${tr.on}" aria-label="${title}" type="button" data-act="trig-on" data-i="${i}" data-fid="trig-on-${i}"></button><button class="key ghost small icon-only" type="button" aria-label="Remove ${title}" data-act="trig-remove" data-i="${i}" data-fid="trig-rm-${i}">${icon("close", 12)}</button></div>
      <span class="hint">${esc(triggerSentence(tr))}${tr.type === "schedule" && tr.on ? " · next: tomorrow" : ""}</span>${body}</div>`;
  };
  const playSum = `${m.data ? `Each row (${m.data.rows})` : p.forever ? "Until stopped" : `×${p.repeat}`} · ${p.speed}×${p.vary ? ` · ±${p.jitter} ms` : ""}${p.stopOnKey ? "" : " · keys don't stop it"}`;
  const on = m.triggers.filter((t) => t.on);
  const runsSum = on.length ? on.map((t) => (t.type === "hotkey" ? comboText(t.combo) : triggerSentence(t))).join(" · ") : "Only when you play it";
  const sec = st.macroSection;
  const head = (v, label, sum) => `<button class="acc" type="button" aria-expanded="${sec === v}" aria-controls="sec-${v}" data-act="msec" data-v="${v}" data-fid="msec-${v}"><span class="label" style="color:var(--text)">${label}</span><span class="acc-sum">${esc(sum)}</span>${icon("down", 14)}</button>`;
  return `${head("play", "Play options", playSum)}
    ${sec === "play" ? `<div class="acc-body" id="sec-play"><div class="spec">
      <span class="k">Repeat</span>${m.data ? `<span class="muted" style="font-size:13px">Once per row (${m.data.rows})</span>` : `<span style="display:flex;gap:6px;align-items:center">${unitField("repeat", p.forever ? "∞" : p.repeat, "times", "Repeat", { scrub: !p.forever, step: 1, dec: 0, min: 1, max: 99 })}<button class="key small${p.forever ? " ink" : ""}" type="button" aria-pressed="${p.forever}" data-act="forever" data-fid="forever" title="Until stopped">∞</button></span>`}
      <span class="k">Speed</span><span class="seg" role="radiogroup" aria-label="Speed">${[0.5, 1, 2, 4].map((v) => `<button type="button" role="radio" aria-checked="${p.speed === v}" data-act="speed" data-v="${v}" data-fid="speed-${v}">${v}×</button>`).join("")}</span>
      <span class="k">Data file</span>${m.data ? `<span style="display:flex;gap:6px;align-items:center;min-width:0"><span class="chip">${icon("data", 12)}${esc(m.data.file)} · ${m.data.rows} rows</span><button class="key ghost small" type="button" data-act="data-remove" data-fid="data-remove">Remove</button></span>` : `<span><button class="key small" type="button" data-act="data-choose" data-fid="data-choose">${icon("file", 14)}Choose…</button></span>`}
      <span class="k">Vary timing</span><span style="display:flex;gap:8px;align-items:center"><button class="switch" role="switch" aria-checked="${p.vary}" type="button" aria-label="Vary timing" data-act="vary" data-fid="vary"></button>${p.vary ? unitField("jitter", p.jitter, "ms", "Vary timing by up to", { scrub: true, step: 5, dec: 0, max: 200 }) : ""}</span>
      <span class="k">Stop on key</span><span style="display:flex;gap:8px;align-items:center"><button class="switch" role="switch" aria-checked="${p.stopOnKey}" type="button" aria-label="Any key stops playback" data-act="stopkey" data-fid="stopkey"></button><span class="hint">${p.stopOnKey ? "Any key stops it" : `Only Esc and ${comboText(st.keys.kill)}`}</span></span>
      <span class="k">Clicks land</span><span style="display:flex;flex-direction:column;gap:4px"><span class="seg" role="radiogroup" aria-label="Clicks land" style="align-self:flex-start">${[["screen", "Screen"], ["window", "Window"]].map(([v, l]) => `<button type="button" role="radio" aria-checked="${p.coords === v}" data-act="coords" data-v="${v}" data-fid="coords-${v}">${l}</button>`).join("")}</span><span class="hint">${p.coords === "screen" ? "Where you clicked on the screen." : "At the same place in the window, if it moved."}</span></span>
    </div>
    <span class="hint">Saved with this macro.</span></div>` : ""}
    ${head("runs", "Runs automatically", runsSum)}
    ${sec === "runs" ? `<div class="acc-body" id="sec-runs">${m.triggers.length ? m.triggers.map(card).join("") : `<span class="hint">Only when you play it. Add a hotkey, a schedule, a program starting, a pixel changing or an image appearing.</span>`}
      <span><button class="key small" type="button" aria-haspopup="menu" data-act="menu" data-menu="trigger" data-fid="add-trigger">${icon("plus", 14)}Add a trigger</button></span></div>` : ""}`;
}

function viewPlate(m, open) {
  const dur = duration(m) || 1;
  const pct = (t) => `${Math.min(100, (t / dur) * 100)}%`;
  const clicks = m.steps.filter((x) => x.kind === "click");
  const ptr = [];
  m.steps.forEach((x) => {
    if (x.move) ptr.push(`<div class="blk mv" style="left:${pct(x.t - x.move.dur)};width:calc(${pct(x.move.dur)} - 3px)"></div>`);
    if (x.kind === "click") ptr.push(`<div class="blk ck${x.t > st.t ? " unplayed" : ""}" style="left:${pct(x.t)}"></div>`);
  });
  const tw = effWidth() * (narrow() ? 0.58 : 0.5);
  const fits = (d, label) => (d / dur) * tw - 10 >= Math.min(label.length * 7, 70);
  const keysL = m.steps.filter((x) => CAT[x.kind] === "keys").map((x) => { const d = Math.max(x.dur ?? 0.3, 0.3); const l = x.kind === "keys" ? x.combo.join(" ") : x.kind === "type" ? "“" + x.text + "”" : "Text"; return `<div class="blk ky" title="${esc(l)}" style="left:${pct(x.t)};width:calc(${pct(d)} - 2px)">${fits(d, l) ? `<span class="bl">${esc(l)}</span>` : ""}</div>`; }).join("");
  const logicL = m.steps.filter((x) => CAT[x.kind] === "logic").map((x) => { const d = Math.max(x.dur ?? 0.5, 0.5); const l = x.kind === "wait" ? "Wait" : x.kind === "pixel" ? "Wait for" : "Find"; return `<div class="blk lg" title="${l}" style="left:${pct(x.t)};width:calc(${pct(d)} - 2px)">${fits(d, l) ? `<span class="bl">${l}</span>` : ""}</div>`; }).join("");
  const pauses = m.steps.filter((x) => x.pause >= 1).map((x) => `<div class="blk ht" style="left:${pct(x.t - x.pause)};width:${pct(x.pause)}"></div>`).join("");
  const cur = st.t || (m.steps.find((x) => x.id === st.cursor)?.t ?? 0);
  const curStep = [...m.steps].reverse().find((x) => x.t <= cur);
  // Preview: balloons on clicks, a path through them.
  const sx = (v) => 18 + (v / 1920) * 264, sy = (v) => 10 + (v / 1080) * 130;
  const pts = clicks.map((c) => [sx(c.x), sy(c.y), c]);
  const start = clicks[0]?.move?.from ?? [960, 600];
  const spline = (ps) => {
    if (ps.length < 2) return ps.length ? `M${ps[0][0]} ${ps[0][1]}` : "";
    let out = `M${ps[0][0].toFixed(1)} ${ps[0][1].toFixed(1)}`;
    for (let i = 0; i < ps.length - 1; i++) {
      const p0 = ps[i - 1] ?? ps[i], p1 = ps[i], p2 = ps[i + 1], p3 = ps[i + 2] ?? p2;
      const c1 = [p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6], c2 = [p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6];
      out += ` C${c1[0].toFixed(1)} ${c1[1].toFixed(1)} ${c2[0].toFixed(1)} ${c2[1].toFixed(1)} ${p2[0].toFixed(1)} ${p2[1].toFixed(1)}`;
    }
    return out;
  };
  const allPts = [[sx(start[0]), sy(start[1])], ...pts.map(([x, y]) => [x, y])];
  const playedPts = pts.filter(([, , c]) => c.t <= cur);
  const d = spline(allPts);
  const dp = spline([[sx(start[0]), sy(start[1])], ...playedPts.map(([x, y]) => [x, y])]);
  // Callouts beside their click, nudged apart so none covers another, and kept inside the frame.
  const spots = [];
  pts.forEach(([x, y, c], i) => {
    const left = x > 190;
    let bx = Math.min(284, Math.max(14, x + (left ? -18 : 18))), by = Math.min(138, Math.max(13, y + (i % 2 ? 16 : -16)));
    const dir = by > 75 ? -1 : 1;
    for (let tries = 0; tries < 8 && spots.some((o) => Math.abs(o.bx - bx) < 26 && Math.abs(o.by - by) < 26); tries++) {
      by += dir * 26; if (by > 138 || by < 13) { by = Math.min(138, Math.max(13, by)); bx += left ? -26 : 26; }
    }
    spots.push({ x, y, bx, by, c, left });
  });
  const leaders = spots.map(({ x, y, bx, by }) => `<line x1="${x}" y1="${y}" x2="${bx}" y2="${by}" stroke="rgba(244,243,239,.55)" stroke-width="1" vector-effect="non-scaling-stroke"/><circle cx="${x}" cy="${y}" r="1.6" fill="#f4f3ef"/>`).join("");
  const balloonsHtml = spots.map(({ bx, by, c, left }) => {
    const n = m.steps.indexOf(c) + 1, sel = st.sel.has(c.id), played = c.t <= cur;
    return `<span class="balloon${left ? " left" : ""}${sel ? " sel" : played ? "" : " unplayed"}" style="left:${(bx / 300) * 100}%;top:${(by / 150) * 100}%"><i>${n}</i>${st.clickLabels && c.label && sel ? `<span>${esc(c.label)}</span>` : ""}</span>`;
  }).join("");
  const last = playedPts[playedPts.length - 1] ?? [sx(start[0]), sy(start[1])];
  const why = !hasPointer(m) ? "This macro uses only the keyboard, so there's nothing to draw." : "";
  return `<div class="plate view${open ? "" : " collapsed"}">
    <div class="view-head">
      <button class="key ghost small" type="button" aria-expanded="${open}" data-act="view" data-fid="view-toggle">${icon(open ? "down" : "next", 14)}<span class="label" style="color:inherit">View</span></button>
      <span class="lcd" aria-label="Playhead at ${fmt(cur)} of ${fmt(dur)}"><span>${fmt(cur)}</span><span class="of">/ ${dur.toFixed(2)}</span></span>
      ${open ? `<span class="seg" role="radiogroup" aria-label="Draw over">${["Screen", "Outline"].map((v, i) => `<button type="button" role="radio" aria-checked="${i === 0}" data-act="segnop" data-fid="bg-${v}">${v}</button>`).join("")}</span>` : `<span class="why">${why || (curStep ? `Step ${m.steps.indexOf(curStep) + 1} · ${KIND_NAME[curStep.kind]} · ${esc(stepDetail(curStep))}` : "")}</span>`}
      <span class="spacer"></span>
      <button class="key ghost small icon-only" type="button" aria-label="Previous step (←)" title="Previous step" data-act="prev" data-fid="prev">${icon("prev", 14)}</button>
      <button class="key ghost small icon-only" type="button" aria-label="Next step (→)" title="Next step" data-act="next" data-fid="next">${icon("next", 14)}</button>
      ${open ? `<button class="key ghost small" type="button" data-fid="fit">Fit</button>` : ""}
    </div>
    <div class="stage" role="img" aria-label="The mouse path over the screen, with ${plural(clicks.length, "click")} numbered in order">
      <svg viewBox="0 0 300 150" preserveAspectRatio="none" aria-hidden="true">
        <defs><pattern id="dots" width="10" height="10" patternUnits="userSpaceOnUse"><circle cx="1" cy="1" r=".6" fill="rgba(244,243,239,.18)"/></pattern></defs>
        <rect width="300" height="150" fill="url(#dots)"/>
        <rect x="18" y="10" width="264" height="130" fill="none" stroke="rgba(244,243,239,.35)"/>
        <path d="M12 10h4M18 4v4M288 10h-4M282 4v4M12 140h4M18 146v-4M288 140h-4M282 146v-4" stroke="rgba(244,243,239,.6)"/>
        ${st.pathMode === "full" ? `<path d="${d}" fill="none" stroke="#6f8fff" stroke-width="1.4" stroke-dasharray="4 4" opacity=".75" vector-effect="non-scaling-stroke"/>` : ""}
        <path d="${dp}" fill="none" stroke="#6f8fff" stroke-width="2.4" stroke-linejoin="round" vector-effect="non-scaling-stroke"/>
        ${leaders}
        <path d="M${last[0]} ${last[1]} l0 9 l2.4 -2.2 l1.8 3.6 l1.6 -.8 l-1.8 -3.4 l3.2 0 z" fill="#f4f3ef" stroke="#141415" stroke-width=".6"/>
      </svg>
      ${balloonsHtml}
    </div>
    <div class="timeline" role="slider" tabindex="0" aria-label="Playhead" aria-valuemin="0" aria-valuemax="${dur}" aria-valuenow="${cur.toFixed(2)}" aria-valuetext="${fmt(cur)} of ${fmt(dur)}${curStep ? `, step ${m.steps.indexOf(curStep) + 1}, ${KIND_NAME[curStep.kind]}, ${esc(stepDetail(curStep))}` : ""}" data-act="seek" data-fid="timeline">
      <div class="ruler"><div class="dimension"><span class="num">${dur.toFixed(2)} s</span></div></div>
      <div class="lane"><span class="tag">${glyph("click", true)}</span>${pauses}${ptr.join("")}</div>
      <div class="lane"><span class="tag">${glyph("keys", true)}</span>${keysL}</div>
      <div class="lane"><span class="tag">${glyph("wait", true)}</span>${logicL}</div>
      <div class="playhead" style="left:${pct(cur)}"></div>
    </div>
    <div class="titleblock" aria-label="Macro details">
      <div>Steps</div><div class="num">${m.steps.length}</div>
      <div>Length</div><div class="num">${dur.toFixed(2)} s</div>
      <div>Screen</div><div class="num">1920 × 1080</div>
      <div>Scale</div><div class="num">1 : 6.4</div>
    </div>
  </div>`;
}

function settingsScreen() {
  const sw = (key, label, on, hint = "", disabled = false) => `<div class="setting"><span class="what"><b id="s-${key}">${label}</b>${hint ? `<span>${hint}</span>` : ""}</span><button class="switch" role="switch" type="button" aria-checked="${on}" aria-labelledby="s-${key}" data-act="set" data-k="${key}" data-fid="set-${key}"${disabled ? " disabled" : ""}></button></div>`;
  const seg = (key, label, opts, cur, hint = "") => `<div class="setting"><span class="what"><b id="s-${key}">${label}</b>${hint ? `<span>${hint}</span>` : ""}</span><span class="seg" role="radiogroup" aria-labelledby="s-${key}">${opts.map(([v, l]) => `<button type="button" role="radio" aria-checked="${String(cur) === String(v)}" data-act="setv" data-k="${key}" data-v="${v}" data-fid="setv-${key}-${v}">${l}</button>`).join("")}</span></div>`;
  const keyRow = (k, label, hint) => {
    const cap = st.capture?.for === k;
    return `<div class="setting"><span class="what"><b>${label}</b><span>${hint}</span></span>
      <span style="display:flex;gap:8px;align-items:center">${cap ? `<span class="key small capture listening" aria-live="polite">Press keys…</span>` : kc(st.keys[k])}<button class="key small" type="button" data-act="capture" data-for="${k}" data-fid="cap-${k}">${cap ? "Cancel" : "Change"}</button></span>
      ${captureHint(k) ? `<div class="full">${captureHint(k)}</div>` : ""}</div>`;
  };
  const glassDefault = st.os === "win11";
  const sections = {
    recording: `<h2 class="label" id="sec-recording">Recording</h2>
      ${sw("rec-moves", "Capture the mouse path", st.recording.moves, "Off records only where you clicked.")}
      ${sw("rec-keys", "Capture keystrokes", st.recording.keys)}
      ${sw("rec-screenshot", "Screenshot", st.recording.screenshot, "A picture of the screen as recording starts, for the view. Kept on this PC only.")}
      ${sw("rec-countdown", "Count down 3, 2, 1", st.recording.countdown)}
      ${sw("rec-esc", "Esc stops recording", st.recording.escStops, "Turn off to record Esc.")}
      ${sw("rec-sim", "Ignore simulated input", st.recording.ignoreSimulated, "Turn off for Remote Desktop and KVM tools: they send your typing as simulated input.")}`,
    keys: `<h2 class="label" id="sec-keys">Keys</h2>
      <span class="hint" style="padding:4px 0 2px">These work from any app. Set one to None to use only the buttons.</span>
      ${keyRow("record", "Record", "Start and stop a recording")}
      ${keyRow("play", "Play / pause", "Play the open macro from the playhead")}
      ${keyRow("player", "Show the player", "Switch between the window and the player")}
      ${keyRow("kill", "Stop everything", "Stops any recording or playback and pauses triggers. Can't be turned off.")}`,
    window: `<h2 class="label" id="sec-window">Window</h2>
      ${seg("keepOnTop", "Keep the player on top", [["always", "Always"], ["sessions", "While working"], ["never", "Never"]], st.keepOnTop)}
      ${sw("hideRecording", "Hide the window while recording", st.hideRecording, "The player shows instead, so Relay never covers what you record.")}
      ${sw("hidePlaying", "Hide the window while playing", st.hidePlaying, "Off, the window stays where it is, behind your apps, so you can watch the steps.")}
      ${sw("closeToTray", "Close to the tray", st.closeToTray, "Hotkeys and triggers keep running.")}
      ${sw("startWithWindows", "Start with Windows", st.startWithWindows, "Starts in the tray, so triggers run after you sign in.")}`,
    appearance: `<h2 class="label" id="sec-appearance">Appearance</h2>
      ${seg("theme", "Theme", [["system", "Like Windows"], ["light", "Light"], ["dark", "Dark"]], st.theme)}
      ${sw("glass", "Frosted glass", glassOn(), !st.transparency ? "Windows' transparency effects are off, so Relay is solid." : glassDefault ? "On by default on Windows 11." : "Off by default on Windows 10, where it can slow down moving the window.", !st.transparency)}
      ${seg("spacing", "Spacing", [["compact", "Compact"], ["comfortable", "Comfortable"], ["large", "Large"]], st.spacing)}
      ${seg("textSize", "Text size", [[100, "100%"], [125, "125%"], [150, "150%"], [200, "200%"]], st.textSize)}
      ${seg("playerSize", "Player size", [["compact", "Compact"], ["large", "Large"]], st.playerSize)}
      ${seg("reduceMotion", "Reduce motion", [["system", "Like Windows"], ["on", "Always"]], st.reduceMotion)}
      ${sw("clickLabels", "Show click labels", st.clickLabels, "Next to the numbered clicks in the view.")}
      ${seg("pathMode", "Mouse path", [["full", "Whole path"], ["trail", "Played part"]], st.pathMode)}`,
    sound: `<h2 class="label" id="sec-sound">Sound</h2>
      ${sw("sounds", "Sounds", st.sounds, "Short clicks when recording starts and stops, and when a macro finishes or fails.")}
      <div class="setting"><span class="what"><b>Try them</b></span><span class="btnrow">${["record", "stop", "done", "error"].map((k) => `<button class="key small" type="button" data-act="try-sound" data-v="${k}" data-fid="try-${k}"${st.sounds ? "" : " disabled"}>${icon("sound", 14)}${k[0].toUpperCase() + k.slice(1)}</button>`).join("")}</span></div>`,
  };
  const navItem = (k, l) => `<button class="navi" type="button" data-act="settings-nav" data-v="${k}" data-fid="snav-${k}"${st.settingsSection === k ? ' aria-current="true"' : ""}>${l}</button>`;
  return `<div class="window settings glass${winCls()}" role="application" aria-label="Relay settings">
    <div class="topbar">
      <button class="key ghost small" type="button" data-act="go" data-to="library" data-fid="back">${icon("back")}Library</button>
      <span class="page-title">Settings</span>
      <span class="spacer"></span>${topWinCtl()}
    </div>
    <nav class="side" aria-label="Settings sections">${navItem("recording", "Recording")}${navItem("keys", "Keys")}${navItem("window", "Window")}${navItem("appearance", "Appearance")}${navItem("sound", "Sound")}</nav>
    <div class="plate settings-body" data-fid="settings-body">${sections[st.settingsSection]}</div>
    ${toastsHtml()}
  </div>`;
}

function captureHint(forKey) {
  if (!st.capture || st.capture.for !== forKey || !st.capture.msg) return "";
  const { msg, ok } = st.capture;
  return `<span class="hint ${ok ? "ok" : "warn"}" role="status">${icon(ok ? "check" : "warning", 14)}<span>${esc(msg)}</span></span>`;
}

function firstRunScreen() {
  const step = st.firstStep;
  const dots = `<span class="stepper" aria-label="Step ${step + 1} of 3">${[0, 1, 2].map((i) => `<i class="${i <= step ? "on" : ""}"></i>`).join("")}</span>`;
  let copy = "";
  if (step === 0) copy = `${dots}<h1>Do it once.<br>Relay does it again.</h1>
    <p>Relay watches what you do with the mouse and keyboard, turns it into steps you can read and change, and plays them back whenever you like: on a key, on a schedule, or when something happens on screen.</p>
    <p>It never connects to the internet. Your macros stay on this PC.</p>
    <div class="first-actions"><button class="key ink" type="button" data-act="first-next" data-fid="first-next">Next</button><span class="spacer"></span><button class="key ghost" type="button" data-act="first-skip" data-fid="first-skip">Skip</button></div>`;
  if (step === 1) {
    const card = (v, title, sub) => `<button class="radio-card" type="button" role="radio" aria-checked="${st.firstChoice === v}" data-act="first-choice" data-v="${v}" data-fid="fc-${v}"><span class="mark" aria-hidden="true"></span><span><b>${title}</b><span class="sub">${sub}</span></span></button>`;
    copy = `${dots}<h1>Your keys</h1>
      <p>Record and play from any app, without opening Relay.</p>
      <div class="choices" role="radiogroup" aria-label="Keys for record and play">
        ${card("ctrlshift", "Ctrl + Shift + F9 and Ctrl + Shift + F10", "Other apps keep F9 and F10. Recommended.")}
        ${card("plain", "F9 and F10", "One key each. Excel, Word and debuggers lose F9 and F10 while Relay runs.")}
        ${card("own", "I'll choose my own", "In Settings → Keys. For keyboards without F keys.")}
      </div>
      <div class="notice">${icon("stop", 14)}<span><b>To stop:</b> Esc, the player's ■, or ${kc(st.keys.kill)} for everything at once. It works over Remote Desktop too.</span></div>
      <div class="first-actions"><button class="key ghost" type="button" data-act="first-back" data-fid="first-back">${icon("back")}Back</button><span class="spacer"></span><button class="key ink" type="button" data-act="first-next" data-fid="first-next">Next</button></div>`;
  }
  if (step === 2) copy = `${dots}<h1>Your first macro</h1>
    <ol style="margin:0;padding-left:20px;display:flex;flex-direction:column;gap:6px;font-size:15px">
      <li>Open Notepad and click inside it.</li>
      <li>Press <b>Record</b> or ${kc(st.keys.record)}. The player counts down 3, 2, 1.</li>
      <li>Type a line, and press Enter.</li>
      <li>Press <b>■</b> on the player, or ${kc(st.keys.record)} again.</li>
    </ol>
    <p>Relay opens your macro so you can name it, read its steps, and play it.</p>
    <div class="first-actions"><button class="key ghost" type="button" data-act="first-back" data-fid="first-back">${icon("back")}Back</button><span class="spacer"></span><button class="key ghost" type="button" data-act="first-skip" data-fid="first-later">Later</button><button class="key red" type="button" data-act="record" data-fid="first-record"><span class="rec-dot"></span>Record now</button></div>`;
  return `<div class="window first glass${winCls()}" role="application" aria-label="Welcome to Relay">
    <div class="topbar"><span class="brand"><span class="mark" aria-hidden="true"></span>Relay</span><span class="spacer"></span>${topWinCtl()}</div>
    <div class="first-body">
      <div class="plate first-copy">${copy}</div>
      <div class="first-art" aria-hidden="true">${firstArt(step)}</div>
    </div>
  </div>`;
}

function firstArt(step) {
  if (step === 1) return `<div style="display:flex;flex-direction:column;gap:14px;align-items:center;color:#f4f3ef">
    <div style="display:flex;gap:8px">${["Ctrl", "Shift", "F9"].map((k) => `<span class="kc" style="height:44px;min-width:56px;font-size:16px;background:#2a2a2c;color:#f4f3ef;box-shadow:0 0 0 1px #4a4a4e,0 3px 0 #000">${k}</span>`).join("")}</div>
    <span style="font-size:13px;color:#a8a6a0">record</span>
    <div style="display:flex;gap:8px">${["Ctrl", "Shift", "F10"].map((k) => `<span class="kc" style="height:44px;min-width:56px;font-size:16px;background:#2a2a2c;color:#f4f3ef;box-shadow:0 0 0 1px #4a4a4e,0 3px 0 #000">${k}</span>`).join("")}</div>
    <span style="font-size:13px;color:#a8a6a0">play</span></div>`;
  return `<svg viewBox="0 0 300 200" width="88%" style="max-height:88%">
    <rect x="20" y="20" width="260" height="160" fill="none" stroke="rgba(244,243,239,.35)"/>
    <path d="M14 20h4M20 14v4M286 20h-4M280 14v4M14 180h4M20 186v-4M286 180h-4M280 186v-4" stroke="rgba(244,243,239,.6)"/>
    <path d="M60 150 C 80 80, 120 60, 150 70 S 210 120, 240 60" fill="none" stroke="#6f8fff" stroke-width="2.2"/>
    ${[[60, 150, 1], [150, 70, 2], [240, 60, 3]].map(([x, y, n]) => `<line x1="${x}" y1="${y}" x2="${x + 14}" y2="${y - 14}" stroke="rgba(244,243,239,.6)"/><circle cx="${x + 18}" cy="${y - 18}" r="9" fill="#2450e6"/><text x="${x + 18}" y="${y - 14}" font-size="11" font-weight="800" text-anchor="middle" fill="#fff" font-family="Archivo">${n}</text>`).join("")}
    <rect x="40" y="170" width="60" height="8" rx="2" fill="#ffc21a"/><rect x="110" y="170" width="40" height="8" rx="2" fill="#1db46a"/><rect x="160" y="170" width="80" height="8" rx="2" fill="#2450e6"/>
  </svg>`;
}

function playerHtml() {
  const m = MACROS.find((x) => x.id === (st.playerMacro ?? st.macroId)) ?? macro();
  const show = windowAway() || st.frame === "player";
  const dur = duration(m) || 1;
  let state = `<span class="state">READY</span>`, right = `${fmtShort(0)} / ${fmtShort(dur)}`;
  if (st.session === "countdown") { state = `<span class="state live">GET READY</span>`; right = `<span class="digit" style="font-size:16px">${st.count}</span>`; }
  if (st.session === "recording") { state = `<span class="state live">REC</span>`; right = `<span class="count"><span class="roll">${st.recSteps}</span></span> steps`; }
  if (st.session === "playing") { state = `<span class="state live">PLAYING</span>`; right = m.data ? `Row ${st.row} / ${m.data.rows}` : `${fmtShort(st.t)} / ${fmtShort(dur)}`; }
  if (st.session === "paused") { state = `<span class="state">PAUSED</span>`; right = m.data ? `Row ${st.row} / ${m.data.rows}` : `${fmtShort(st.t)} / ${fmtShort(dur)}`; }
  if (st.done) state = `<span class="state done">DONE ✓</span>`;
  if (st.error && st.session === "idle") state = `<span class="state">STOPPED</span>`;
  const clicksAt = new Set(m.steps.filter((x) => x.kind === "click").map((x) => Math.round((x.t / dur) * 47)));
  const lit = st.session === "recording" ? -1 : Math.round((st.t / dur) * 47);
  const leds = Array.from({ length: 48 }, (_, i) => `<i class="${clicksAt.has(i) ? "tick" : ""}${i <= lit && (st.t > 0 || st.session === "playing") ? " on" : ""}"></i>`).join("");
  const line2 = st.error
    ? `<div class="top"><span class="err">${icon("warning", 12)}</span><span class="err">${esc(st.error.text)}</span><button class="dismiss" type="button" aria-label="Dismiss" data-act="dismiss-error" data-fid="p-dismiss">${icon("close", 12)}</button></div>`
    : st.session === "recording" ? `<div class="top" style="color:var(--alu-300);font-size:12px">Last: ${esc(["Click · 812, 402", "Type · “Hello from Relay”", "Keys · Enter", "Click · Save", "Keys · Ctrl + S"][st.recSteps % 5])}</div>`
    : `<div class="leds" aria-hidden="true">${leds}</div>`;
  const live = st.session === "recording" || st.session === "countdown";
  return `<div class="player glass${st.playerSize === "large" ? " big" : ""}${show ? "" : " hidden"}" role="group" aria-label="Relay player" ${show ? "" : 'aria-hidden="true"'}>
    <span class="grip" aria-hidden="true">${icon("grip", 14)}</span>
    <button class="key red${live ? " live" : ""}" type="button" data-act="record" data-fid="p-rec" aria-label="${live ? "Stop recording" : "Record"} (${comboText(st.keys.record)})" title="${live ? "Stop recording" : "Record"}"><span class="rec-dot"></span></button>
    <button class="key ink" type="button" data-act="play" data-fid="p-play" aria-label="${st.session === "playing" ? "Pause" : "Play"} (${comboText(st.keys.play)})" title="${st.session === "playing" ? "Pause" : "Play"}"${live ? " disabled" : ""}>${icon(st.session === "playing" ? "pause" : "play")}</button>
    <button class="key" type="button" data-act="stop" data-fid="p-stop" aria-label="Stop (Esc)" title="Stop (Esc)">${icon("stop", 14)}</button>
    <div class="screen${st.done ? " flash" : ""}">
      <div class="top">${state}<button class="nm" type="button" aria-haspopup="menu" aria-label="Macro: ${esc(m.name)}. Switch macros" data-act="menu" data-menu="switch" data-fid="p-switch"><span>${esc(st.session === "recording" || st.session === "countdown" ? "New recording" : m.name)}</span>${icon("down", 12)}</button><span class="rt" aria-live="off">${right}</span></div>
      ${line2}
    </div>
    <button class="key" type="button" data-act="open-window" data-fid="p-open" aria-label="Open the window (${comboText(st.keys.player)})" title="Open the window">${icon("expand")}</button>
  </div>`;
}

function exportedHtml() {
  if (st.frame !== "exported") return "";
  return `<div class="exported glass" role="group" aria-label="Exported program: Export invoice to PDF">
    <button class="key ink" type="button" aria-label="Pause">${icon("pause")}</button>
    <button class="key" type="button" aria-label="Stop">${icon("stop", 14)}</button>
    <div class="screen"><div class="top"><span class="state live">PLAYING</span><span class="nm" style="cursor:default"><span>Export invoice to PDF</span></span></div>
      <div class="top" style="gap:10px"><div class="leds" aria-hidden="true" style="flex:1">${Array.from({ length: 48 }, (_, i) => `<i class="${[4, 8, 28, 40].includes(i) ? "tick" : ""}${i <= 28 ? " on" : ""}"></i>`).join("")}</div><span class="rt" style="margin-left:0">00:06 / 00:10</span></div></div>
  </div>`;
}

function toastsHtml() {
  if (!st.toasts.length) return "";
  return `<div class="toasts">${st.toasts.map((t) => `<div class="toast ${t.kind === "error" ? "error" : ""}" role="${t.kind === "error" ? "alert" : "status"}">
    <span class="msg">${esc(t.msg)}</span>${t.action ? `<button class="key small" type="button" data-act="toast-action" data-id="${t.id}" data-fid="toast-${t.id}">${esc(t.action.label)}</button>` : ""}
    <button class="key ghost small icon-only" type="button" aria-label="Dismiss" data-act="toast-close" data-id="${t.id}" data-fid="toastx-${t.id}" style="--key-fg:inherit">${icon("close", 12)}</button></div>`).join("")}</div>`;
}

function menuHtml() {
  const mn = st.menu; if (!mn) return "";
  const item = (act, label, ic, extra = "", sc = "") => `<button role="menuitem" type="button" data-act="${act}" ${extra} tabindex="-1">${ic ?? ""}<span>${label}</span>${sc ? `<span class="sc">${sc}</span>` : ""}</button>`;
  let items = "";
  if (mn.id === "add") items = item("add-step", "Wait", glyph("wait", true), 'data-v="wait"') + item("add-step", "Text", glyph("text", true), 'data-v="text"') + item("add-step", "Wait for pixel", glyph("pixel", true), 'data-v="pixel"') + item("add-step", "Find image", glyph("find", true), 'data-v="find"');
  if (mn.id === "list") items = item("toggle-moves", `${st.showMoves ? "Hide" : "Show"} moves`, icon("move")) + item("trim", "Shorten pauses over 1 s", icon("wait")) + item("select-all", "Select all", icon("check"), "", "Ctrl + A");
  if (mn.id === "macro") items = item("export", "Export…", icon("export")) + item("duplicate", "Duplicate", icon("duplicate")) + item("runs-of", "Runs", icon("history")) + "<hr>" + item("delete-macro", '<span class="danger">Delete macro</span>', `<span class="danger">${icon("trash")}</span>`);
  if (mn.id === "row") items = item("open", "Open", icon("expand"), `data-id="${mn.data}"`, "Enter") + item("play-macro", "Play", icon("play"), `data-id="${mn.data}"`) + item("duplicate", "Duplicate", icon("duplicate"), `data-id="${mn.data}"`) + item("export", "Export…", icon("export"), `data-id="${mn.data}"`) + "<hr>" + item("delete-macro", '<span class="danger">Delete</span>', `<span class="danger">${icon("trash")}</span>`, `data-id="${mn.data}"`, "Del");
  if (mn.id === "trigger") { const have = new Set(macro().triggers.map((t) => t.type)); items = [["hotkey", "Hotkey", "keycap"], ["schedule", "Schedule", "clock"], ["app", "When an app starts", "app"], ["pixel", "When a pixel changes", "pixel"], ["image", "When an image appears", "image"]].filter(([t]) => !have.has(t)).map(([t, l, i]) => item("add-trigger", l, icon(i), `data-v="${t}"`)).join("") || `<span class="hint" style="padding:6px 10px;display:block">Every kind is already here.</span>`; }
  if (mn.id === "switch") items = MACROS.map((m) => item("switch-macro", esc(m.name), m.id === (st.playerMacro ?? st.macroId) ? icon("check") : '<span style="width:16px"></span>', `data-id="${m.id}"`, hotkeyOf(m) ? comboText(hotkeyOf(m)) : "")).join("");
  return `<div class="menu" role="menu" aria-label="${esc(mn.label)}" style="left:${mn.x}px;top:${mn.y}px">${items}</div>`;
}

function exportDialog() {
  if (st.dialog !== "export") return "";
  const m = macro();
  const typed = m.steps.filter((x) => x.kind === "type" || x.kind === "text").reduce((n, x) => n + x.text.length, 0);
  const fmts = [["rly", "Relay macro", ".rly · editable, keeps timing"], ["exe", "Standalone program", ".exe · plays on any Windows PC, in a small player"], ["ahk", "AutoHotkey v2", ".ahk · a readable script"], ["json", "JSON events", ".json · the raw events, for your tools"]];
  const ext = { rly: "rly", exe: "exe", ahk: "ahk", json: "json" }[st.exportFmt];
  return `<div class="dlg-wrap" style="position:absolute;inset:0;z-index:35;display:flex;align-items:center;justify-content:center;background:rgba(20,20,21,.35);border-radius:var(--r-xl)">
    <div class="dlg-body glass" role="dialog" aria-modal="true" aria-labelledby="dlg-title" style="width:min(460px,92%)">
      <div class="dlg-title" id="dlg-title">Export “${esc(m.name)}”</div>
      <div role="radiogroup" aria-label="Format" style="display:flex;flex-direction:column;gap:6px">${fmts.map(([v, t, sub]) => `<button class="radio-card" type="button" role="radio" aria-checked="${st.exportFmt === v}" data-act="fmt" data-v="${v}" data-fid="fmt-${v}"><span class="mark" aria-hidden="true"></span><span><b>${t}</b><span class="sub">${sub}</span></span></button>`).join("")}</div>
      ${typed ? `<div class="notice">${icon("warning", 14)}<span>This macro types ${plural(typed, "character")}. Check its Type and Text steps before you share it.</span></div>` : ""}
      <div class="muted num" style="font-size:13px">${esc(m.name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, ""))}.${ext}</div>
      <div style="display:flex;gap:8px;justify-content:flex-end"><button class="key" type="button" data-act="dialog-close" data-fid="dlg-cancel">Cancel</button><button class="key ink" type="button" data-act="do-export" data-fid="dlg-save">Save…</button></div>
    </div></div>`;
}

function pickerHtml() {
  if (!st.picking) return "";
  return `<div class="picker" data-act="pick-confirm" role="dialog" aria-label="Pick a pixel from the screen">
    <div class="help glass">${icon("pick", 14)}<span>Click the pixel, or move with the arrow keys and press Enter. Esc cancels.</span></div>
    <div class="loupe" id="loupe" style="left:${st.picking.x}px;top:${st.picking.y + 40}px"><canvas id="loupe-c" width="11" height="11"></canvas></div>
    <div class="loupe-tag" id="loupe-tag" style="left:${st.picking.x}px;top:${st.picking.y + 40}px">${st.picking.x}, ${st.picking.y} · ${colorHex(st.picking.x, st.picking.y)}</div>
  </div>`;
}

const winCls = () => (windowAway() ? " away" : "");
const playerKey = () => `<button class="key small icon-only" type="button" aria-label="Switch to the player (${comboText(st.keys.player)})" title="Switch to the player (${comboText(st.keys.player)})" data-act="to-player" data-fid="to-player">${icon("collapse")}</button>`;

function render() {
  const fid = document.activeElement?.dataset?.fid;
  const caret = document.activeElement?.matches?.("input, textarea") ? [document.activeElement.selectionStart, document.activeElement.selectionEnd] : null;
  const root = document.documentElement;
  root.classList.toggle("dark", isDark());
  root.classList.toggle("dark-desktop", isDark());
  root.classList.toggle("solid", !glassOn());
  root.classList.toggle("calm", calm());
  root.classList.toggle("compact", st.spacing === "compact");
  root.classList.toggle("large", st.spacing === "large");
  root.classList.toggle("narrow", narrow());
  root.classList.toggle("picking", !!st.picking);
  const [ww, wh] = winPx();
  root.style.setProperty("--win-w", `${ww}px`);
  root.style.setProperty("--win-h", `${wh}px`);
  root.classList.toggle("tight", tight());
  root.style.setProperty("--zoom", zoom());
  root.style.setProperty("--pzoom", Math.min(zoom(), 1.5));
  let win = "";
  if (st.frame === "icons") win = iconSheet();
  else if (st.frame === "player" || st.frame === "exported") win = "";
  else win = { firstrun: firstRunScreen, library: libraryScreen, activity: activityScreen, editor: editorScreen, settings: settingsScreen }[st.screen]?.() ?? "";
  const dlg = exportDialog();
  if (dlg) win = win.replace(/<\/div>\s*$/, `${dlg}</div>`);
  const menu = menuHtml();
  document.getElementById("app").innerHTML = `
    ${st.frame ? "" : envBar()}
    <div class="desktop">
      <div class="app-behind" aria-hidden="true"><div class="bar">Invoices.xlsx</div><span class="cell" style="left:12px;top:40px">ACME Ltd</span><span class="cell" style="left:140px;top:40px">F-1042</span><span class="cell" style="left:12px;top:62px">Globex</span><span class="cell" style="left:140px;top:62px">F-1043</span></div>
      ${win && st.frame !== "icons" ? `<div class="win-pos">${win}</div>` : win}
      ${st.frame === "icons" ? "" : `<div class="player-pos">${playerHtml()}</div>`}
      ${exportedHtml()}
      ${pickerHtml()}
      <div class="taskbar" aria-hidden="true"></div>
    </div>
    ${menu}
    <div id="live" class="sr-only" role="status" aria-live="polite">${esc(liveText)}</div>`;
  if (st.picking) drawLoupe();
  if (fid) { const el = document.querySelector(`[data-fid="${CSS.escape(fid)}"]`); el?.focus({ preventScroll: true }); if (caret && el?.setSelectionRange) try { el.setSelectionRange(...caret); } catch {} }
  if (st.menu?.focusFirst) { document.querySelector(".menu [role=menuitem]")?.focus(); st.menu.focusFirst = false; }
  const cur = document.querySelector(".step.cursor");
  cur?.scrollIntoView({ block: "nearest" });
}

function iconSheet() {
  return `<div class="icon-sheet glass" role="list" aria-label="Relay's icons">${Object.keys(ICONS).map((k) => `<div class="icon-cell" role="listitem">${icon(k, 24)}<span>${k}</span></div>`).join("")}</div>`;
}

// ── Picking ─────────────────────────────────────────────────────────────────

function colorHex(x, y) {
  const [r, g, b] = colorAt(x, y);
  return "#" + [r, g, b].map((v) => Math.round(v).toString(16).padStart(2, "0")).join("").toUpperCase();
}
function colorAt(x, y) {
  const app = document.querySelector(".app-behind")?.getBoundingClientRect();
  const Y = y + 40;
  if (app && x >= app.left && x <= app.right && Y >= app.top && Y <= app.bottom) {
    if (Y - app.top < 30) return [31, 122, 69];
    const gx = (x - app.left) % 64 < 1, gy = (Y - app.top - 30) % 22 < 1;
    return gx || gy ? [227, 230, 234] : [255, 255, 255];
  }
  const W = innerWidth, H = innerHeight;
  const blob = (cx, cy, rx, ry, c) => { const d = Math.hypot((x - cx * W) / (rx * W), (Y - cy * H) / (ry * H)); const a = Math.max(0, 1 - d / 0.7); return [c, a]; };
  let col = [201, 205, 209];
  for (const [c, a] of [blob(0.18, 0.22, 0.4, 0.5, [255, 122, 61]), blob(0.82, 0.3, 0.45, 0.55, [61, 107, 255]), blob(0.6, 0.9, 0.5, 0.6, [47, 207, 138])]) col = col.map((v, i) => v * (1 - a) + c[i] * a);
  return col;
}
function drawLoupe() {
  const c = document.getElementById("loupe-c"); if (!c) return;
  const ctx = c.getContext("2d");
  for (let i = -5; i <= 5; i++) for (let j = -5; j <= 5; j++) { const [r, g, b] = colorAt(st.picking.x + i, st.picking.y + j); ctx.fillStyle = `rgb(${r},${g},${b})`; ctx.fillRect(i + 5, j + 5, 1, 1); }
}
function movePicker(x, y) {
  st.picking.x = Math.round(x); st.picking.y = Math.round(y);
  const l = document.getElementById("loupe"), t = document.getElementById("loupe-tag");
  if (l) { l.style.left = `${st.picking.x}px`; l.style.top = `${st.picking.y + 40}px`; t.style.left = l.style.left; t.style.top = l.style.top; t.textContent = `${st.picking.x}, ${st.picking.y} · ${colorHex(st.picking.x, st.picking.y)}`; drawLoupe(); }
}
function confirmPick() {
  const m = macro(), x = m.steps.find((s) => s.id === st.picking.stepId);
  const col = colorHex(st.picking.x, st.picking.y);
  if (x) { snapshot(); x.x = Math.round(st.picking.x * 1920 / innerWidth); x.y = Math.round(st.picking.y * 1080 / innerHeight); if (x.kind === "pixel") x.color = col; }
  st.picking = null;
  toast(`Picked ${x?.x}, ${x?.y}${x?.kind === "pixel" ? ` · ${col}` : ""}`, { action: { label: "Undo", run: () => restore(st.undo, st.redo) } });
  render();
}

// ── Sessions ────────────────────────────────────────────────────────────────

let timer = null;
function clearTimer() { clearInterval(timer); clearTimeout(timer); timer = null; }

function startRecording() {
  if (st.session === "playing" || st.session === "paused") return;
  clearTimer(); st.error = null; st.done = false; st.menu = null;
  st.playerMacro = st.macroId;
  const go = () => {
    st.session = "recording"; st.recSteps = 0; SOUND.record(); announce("Recording started. Press the record key or Stop to finish.");
    render();
    timer = setInterval(() => { st.recSteps++; SOUND.tick(); render(); }, 900);
  };
  if (st.recording.countdown) {
    st.session = "countdown"; st.count = 3; announce("Get ready. Recording starts in 3 seconds."); render();
    timer = setInterval(() => { st.count--; if (st.count <= 0) { clearTimer(); go(); } else render(); }, 1000);
  } else go();
}
function stopRecording() {
  clearTimer();
  const n = Math.max(st.recSteps, 3);
  const steps = [s({ kind: "click", t: 0.4, x: 812, y: 402, label: "", move: { dur: 0.4, from: [960, 700] } }), s({ kind: "type", t: 1.1, text: "Hello from Relay", dur: 1.6 }), s({ kind: "keys", t: 3.0, combo: ["Enter"], dur: 0.1 })];
  for (let i = 3; i < n; i++) steps.push(s({ kind: i % 2 ? "keys" : "click", t: 3.4 + i * 0.6, combo: ["Ctrl", "S"], x: 900 + i * 20, y: 300, move: i % 2 ? undefined : { dur: 0.4, from: [800, 400] } }));
  const id = `rec${Date.now()}`;
  MACROS.unshift({ id, name: `Recording ${MACROS.filter((m) => m.name.startsWith("Recording")).length + 1}`, steps, play: PLAY(), data: null, triggers: [], last: null, made: 99 });
  st.session = "idle"; st.recSteps = 0; st.macroId = id; st.screen = "editor"; st.sel = new Set(); st.cursor = steps[steps.length - 1].id; st.tab = "step"; st.t = 0;
  SOUND.stop();
  announce(`Recording stopped, ${plural(steps.length, "step")}. Name this macro.`);
  render();
  const nm = document.querySelector('[data-fid="name"]'); nm?.focus(); nm?.select();
}
function play() {
  if (st.session === "recording" || st.session === "countdown") return;
  if (st.session === "playing") { st.session = "paused"; clearTimer(); announce("Paused"); render(); return; }
  const m = MACROS.find((x) => x.id === (st.playerMacro ?? st.macroId)) ?? macro();
  st.playerMacro = m.id; st.error = null; st.done = false; st.menu = null;
  if (st.session !== "paused") { st.t = 0; st.row = 1; }
  st.session = "playing"; announce(`Playing ${m.name}`); render();
  const dur = duration(m), t0 = performance.now(), from = st.t;
  timer = setInterval(() => {
    st.t = Math.min(dur, from + ((performance.now() - t0) / 1000) * 1.6 * m.play.speed);
    if (m.data && st.t >= dur) {
      st.row++;
      if (st.failRow && st.row === 112) { fail(m, 10, "Pixel check timed out · step 10 · row 112"); return; }
      st.t = 0;
      return playLoop(m);
    }
    if (st.t >= dur) { finish(m); return; }
    tickPlayback(m);
  }, 50);
}
function playLoop(m) {
  clearTimer();
  const dur = duration(m);
  // Rows go fast in the prototype, so a long data run is quick to try.
  timer = setInterval(() => {
    st.row += 3;
    if (st.failRow && st.row >= 112) { st.row = 112; fail(m, 10, "Pixel check timed out · step 10 · row 112"); return; }
    if (st.row >= m.data.rows) { st.row = m.data.rows; finish(m); return; }
    st.t = (st.t + dur / 6) % dur;
    tickPlayback(m);
  }, 60);
}
function tickPlayback(m) {
  // Light updates while playing: the player, the rows and the playhead.
  const dur = duration(m) || 1;
  const p = document.querySelector(".player .rt");
  if (p) p.textContent = m.data ? `Row ${st.row} / ${m.data.rows}` : `${fmtShort(st.t)} / ${fmtShort(dur)}`;
  document.querySelectorAll(".player .leds i").forEach((d, i) => d.classList.toggle("on", i <= (st.t / dur) * 47));
  if (st.screen === "editor" && st.macroId === m.id) {
    m.steps.forEach((x) => { const r = document.getElementById(`row-${x.id}`); if (r) { r.classList.toggle("played", x.t <= st.t); r.classList.toggle("now", x.t <= st.t && st.t - x.t < 0.5); } });
    const ph = document.querySelector(".playhead"); if (ph) ph.style.left = `${(st.t / dur) * 100}%`;
    const lcd = document.querySelector(".view .lcd span"); if (lcd) lcd.textContent = fmt(st.t);
    const now = document.querySelector(".step.now"); now?.scrollIntoView({ block: "nearest" });
    const np = document.getElementById("now-pane"); const cur = currentStep(m);
    if (np && (np.dataset.step !== String(cur?.id) || m.data)) { np.dataset.step = String(cur?.id); np.innerHTML = nowPane(m); }
  }
}
function finish(m) {
  clearTimer(); st.session = "idle"; st.done = true; st.t = duration(m);
  m.last = { ok: true, text: "Just now" };
  RUNS.unshift({ macro: m.id, by: "Play", when: "Just now", ok: true, end: "Completed", dur: `${duration(m).toFixed(1)} s`, checks: [] });
  SOUND.done(); announce(`${m.name} finished.`);
  render();
  setTimeout(() => { st.done = false; render(); }, 1600);
}
function fail(m, step, text) {
  clearTimer(); st.session = "idle"; st.error = { text, step }; st.failed = { macro: m.id, step }; st.t = m.steps[step - 1]?.t ?? st.t;
  m.last = { ok: false, text: "Just now" };
  RUNS.unshift({ macro: m.id, by: "Play", when: "Just now", ok: false, end: `Pixel check timed out at step ${step}, row ${st.row}`, sub: "PolicyNo 88-1042", dur: "", step, checks: [] });
  SOUND.error(); announce(`${m.name} stopped: ${text}`);
  if (!windowAway()) toast(`${m.name} stopped: ${text.replace(/ · /g, ", ")}`, { kind: "error" });
  render();
  if (!calm()) { document.querySelector(".player")?.classList.add("shake"); document.querySelector(".step.failed")?.classList.add("shake"); }
}
function stop() {
  const was = st.session; clearTimer();
  if (was === "recording" || was === "countdown") { if (was === "countdown") { st.session = "idle"; announce("Recording cancelled."); render(); return; } stopRecording(); return; }
  st.session = "idle"; st.t = 0; st.done = false; if (was !== "idle") { SOUND.stop(); announce("Stopped."); }
  render();
}

// ── Editing ─────────────────────────────────────────────────────────────────

function selectRow(id, { extend = false, toggle = false } = {}) {
  const m = macro(); const rows = visibleRows(m).map((r) => r.id);
  if (extend && st.anchor != null) {
    const a = rows.indexOf(st.anchor), b = rows.indexOf(id);
    st.sel = new Set(rows.slice(Math.min(a, b), Math.max(a, b) + 1).filter((r) => typeof r === "number"));
  } else if (toggle) { st.sel.has(id) ? st.sel.delete(id) : st.sel.add(id); st.anchor = id; }
  else { st.sel = new Set([id]); st.anchor = id; }
  st.cursor = id;
  st.tab = "step";
  const x = m.steps.find((s) => s.id === id); if (x && st.session === "idle") st.t = 0;
  st.testResult = null;
}
function deleteSelected() {
  const m = macro(); if (st.session !== "idle" || !st.sel.size) return;
  snapshot();
  const n = st.sel.size, idx = m.steps.findIndex((x) => st.sel.has(x.id));
  m.steps.forEach((x) => { if (st.sel.has(`m${x.id}`)) delete x.move; });
  m.steps = m.steps.filter((x) => !st.sel.has(x.id));
  st.sel = new Set(); const next = m.steps[Math.min(idx, m.steps.length - 1)]; st.cursor = next?.id ?? null; if (next) st.sel.add(next.id);
  toast(`Deleted ${plural(n, "step")}`, { action: { label: "Undo", run: () => restore(st.undo, st.redo) } });
}
function insertStep(kind, afterId) {
  const m = macro(); if (st.session !== "idle") return;
  snapshot();
  const after = m.steps.findIndex((x) => x.id === (afterId ?? st.cursor ?? m.steps[m.steps.length - 1]?.id));
  const base = m.steps[after];
  const t = +(base ? base.t + (base.dur ?? 0.2) + 0.05 : 0).toFixed(2);
  const dur = { wait: 0.5, text: 0.6, pixel: 0.8, find: 0.8 }[kind];
  const x = s({ kind, t, dur, text: kind === "text" ? "" : undefined, x: 960, y: 540, color: "#2450E6", tol: 8, timeout: 5, match: 85, btn: "Left", cx: 0.5, cy: 0.5, label: "" });
  m.steps.forEach((y, i) => { if (i > after) y.t = +(y.t + dur).toFixed(2); });
  m.steps.splice(after + 1, 0, x);
  st.sel = new Set([x.id]); st.cursor = x.id; st.anchor = x.id; st.tab = "step"; st.menu = null;
  if (narrow()) st.inspOpen = true;
  toast(`Added ${KIND_NAME[kind] === "Wait for" ? "a pixel check" : `a ${KIND_NAME[kind].toLowerCase()} step`} after step ${after + 1}`, { action: { label: "Undo", run: () => restore(st.undo, st.redo) } });
  render();
  setTimeout(() => document.querySelector('#insp-panel input, #insp-panel textarea')?.focus(), 0);
}

// ── Key capture ─────────────────────────────────────────────────────────────

const MODS = ["Control", "Shift", "Alt", "Meta"];
function keyLabel(e) {
  const c = e.code;
  if (/^Key[A-Z]$/.test(c)) return c.slice(3);
  if (/^Digit\d$/.test(c)) return c.slice(5);
  if (/^F\d{1,2}$/.test(c)) return c;
  return { Backspace: "Backspace", Enter: "Enter", Space: "Space", Tab: "Tab", End: "End", Home: "Home", Insert: "Insert", Delete: "Delete", Pause: "Pause", Minus: "Minus", Equal: "Equal", PageUp: "PageUp", PageDown: "PageDown", ArrowLeft: "←", ArrowRight: "→", ArrowUp: "↑", ArrowDown: "↓", BracketLeft: "[", BracketRight: "]", Escape: "Esc" }[c] ?? e.key.toUpperCase();
}
function captureKey(e) {
  const cap = st.capture;
  if (e.key === "Escape" && !e.ctrlKey && !e.altKey) { st.capture = null; announce("Cancelled."); render(); return; }
  if (MODS.includes(e.key)) return;
  const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Win"].filter(Boolean);
  const k = keyLabel(e);
  const isRelay = !cap.for.startsWith("trigger:");
  if (e.code === "Backspace" && !mods.length) {
    if (cap.for === "kill") { cap.msg = "Stop everything can't be turned off. Press another combination."; cap.ok = false; render(); return; }
    setCombo(cap.for, []); st.capture = null; toast(isRelay ? "Set to None: use the buttons instead." : "Hotkey cleared."); render(); return;
  }
  const combo = [...mods, k];
  const typing = /^[A-Z0-9]$|^(Space|Enter|Minus|Equal|\[|\])$/.test(k);
  let refuse = null;
  if (mods.includes("Win")) refuse = "Windows keeps key combinations with Win for itself. Choose one without it.";
  else if (typing && !mods.some((x) => x === "Ctrl" || x === "Alt")) refuse = mods.includes("Shift") ? `Add Ctrl or Alt: Shift + ${k} is ordinary typing.` : `Add Ctrl or Alt, so ${k} still types normally.`;
  const used = Object.entries(st.keys).find(([key, c]) => key !== cap.for && comboKey(c) === comboKey(combo));
  const usedMacro = MACROS.find((m) => m.triggers.some((t, i) => t.type === "hotkey" && t.combo?.length && comboKey(t.combo) === comboKey(combo) && !(m.id === st.macroId && `trigger:${i}` === cap.for)));
  if (!refuse && used) refuse = `${comboText(combo)} is already Relay's “${{ record: "Record", play: "Play / pause", player: "Show the player", kill: "Stop everything" }[used[0]]}” key.`;
  if (!refuse && usedMacro) refuse = `${comboText(combo)} already runs “${usedMacro.name}”.`;
  if (refuse) { cap.msg = refuse; cap.ok = false; render(); return; }
  setCombo(cap.for, combo);
  const ch = mods.includes("Ctrl") && mods.includes("Alt") && !mods.includes("Shift") ? ALTGR[st.layout]?.[e.code] : null;
  const warn = ch ? `${comboText(combo)} types “${ch}” on your ${LAYOUT_NAME[st.layout]} keyboard (AltGr + ${k}). You couldn't type “${ch}” while this is set.` : TAKEN[comboKey(combo)];
  st.capture = { for: cap.for, msg: warn ?? (cap.for === "kill" ? `${comboText(combo)} works over Remote Desktop.` : `Set to ${comboText(combo)}.`), ok: !warn };
  announce(st.capture.msg);
  render();
  setTimeout(() => { if (st.capture?.for === cap.for && st.capture.ok) { st.capture = null; render(); } }, 3500);
}
function setCombo(forKey, combo) {
  if (forKey.startsWith("trigger:")) { snapshot(); const t = macro().triggers[Number(forKey.split(":")[1])]; t.combo = combo; t.on = combo.length > 0; }
  else st.keys[forKey] = combo;
}
const matches = (e, combo) => combo?.length && comboKey([e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Win", keyLabel(e)].filter(Boolean)) === comboKey(combo);

// ── Events ──────────────────────────────────────────────────────────────────

function openMenu(btn, id, data) {
  const r = btn.getBoundingClientRect();
  const labels = { add: "Add a step", list: "Steps options", macro: "Macro", row: "Macro", trigger: "Add a trigger", switch: "Switch macros" };
  const w = 230;
  let x = Math.min(innerWidth - w - 8, r.left), y = r.bottom + 6;
  if (id === "switch") y = r.top - 6 - MACROS.length * 32 - 10;
  st.menu = { id, data, x, y, label: labels[id], from: btn.dataset.fid, focusFirst: true };
}
function closeMenu(refocus = true) {
  const from = st.menu?.from; st.menu = null; render();
  if (refocus && from) document.querySelector(`[data-fid="${from}"]`)?.focus();
}

document.addEventListener("click", (e) => {
  const uf = e.target.closest(".unit-field");
  if (uf && e.target === uf && !uf.dataset.scrub) uf.querySelector("input")?.focus();
  const t = e.target.closest("[data-act], [data-row]");
  if (st.menu && !e.target.closest(".menu") && !t?.dataset.menu) { st.menu = null; render(); if (!t) return; }
  if (!t) return;
  if (t.dataset.row) {
    const id = isNaN(t.dataset.row) ? t.dataset.row : Number(t.dataset.row);
    selectRow(id, { extend: e.shiftKey, toggle: e.ctrlKey || e.metaKey });
    if (narrow() && !e.shiftKey && !e.ctrlKey) st.inspOpen = true;
    render(); document.querySelector('[data-fid="list"]')?.focus({ preventScroll: true }); return;
  }
  const a = t.dataset.act, m = macro();
  const doRender = () => render();
  switch (a) {
    case "nav":
      st.libFilter = t.dataset.to; st.screen = ["runs", "problems"].includes(t.dataset.to) ? "activity" : "library"; st.openRun = null; return doRender();
    case "go": st.screen = t.dataset.to; st.capture = null; st.inspOpen = false; return doRender();
    case "open": st.macroId = t.dataset.id; st.screen = "editor"; st.sel = new Set(); st.cursor = null; st.tab = "step"; st.menu = null; st.t = 0; st.viewOpen = null; return doRender();
    case "open-at": { st.macroId = t.dataset.id; st.screen = "editor"; const x = macro().steps[Number(t.dataset.step) - 1]; st.sel = new Set([x.id]); st.cursor = x.id; st.tab = "step"; st.failed = { macro: macro().id, step: Number(t.dataset.step) }; return doRender(); }
    case "lib-check": { const id = t.dataset.id; st.libSel.has(id) ? st.libSel.delete(id) : st.libSel.add(id); return doRender(); }
    case "lib-sort": st.libSort = t.dataset.v; return doRender();
    case "bulk-clear": st.libSel.clear(); return doRender();
    case "bulk-delete": { const n = st.libSel.size; const removed = MACROS.filter((x) => st.libSel.has(x.id)); removed.forEach((x) => MACROS.splice(MACROS.indexOf(x), 1)); st.libSel.clear(); toast(`Moved ${plural(n, "macro")} to the trash`, { action: { label: "Undo", run: () => MACROS.push(...removed) } }); return doRender(); }
    case "bulk-export": toast(`Exported ${plural(st.libSel.size, "macro")} to Documents\\Relay exports`); st.libSel.clear(); return doRender();
    case "import": toast("Imported 3 macros"); return doRender();
    case "run": st.openRun = st.openRun === Number(t.dataset.i) ? null : Number(t.dataset.i); return doRender();
    case "record": st.session === "recording" || st.session === "countdown" ? stop() : startRecording(); return;
    case "play": return play();
    case "stop": return stop();
    case "dismiss-error": st.error = null; return doRender();
    case "open-window": st.playerMode = false; if (st.session === "recording" || st.session === "countdown") st.hideRecording = st.hideRecording; return doRender();
    case "to-player": st.playerMode = true; announce("Switched to the player."); doRender(); document.querySelector('[data-fid="p-play"]')?.focus(); return;
    case "undo": restore(st.undo, st.redo); toast("Undone"); return doRender();
    case "redo": restore(st.redo, st.undo); toast("Redone"); return doRender();
    case "msec": st.macroSection = st.macroSection === t.dataset.v ? (t.dataset.v === "play" ? "runs" : "play") : t.dataset.v; return doRender();
    case "open-runs": st.macroId = t.dataset.id; st.screen = "editor"; st.sel = new Set(); st.cursor = null; st.tab = "macro"; st.macroSection = "runs"; st.t = 0; st.viewOpen = null; return doRender();
    case "chips": st.tab = "macro"; st.macroSection = "play"; if (narrow()) st.inspOpen = true; doRender(); document.querySelector('[data-fid="tab-macro"]')?.focus(); return;
    case "tab": st.tab = t.dataset.v; return doRender();
    case "insp-close": st.inspOpen = false; doRender(); document.querySelector('[data-fid="list"]')?.focus(); return;
    case "menu": if (st.menu?.id === t.dataset.menu) return closeMenu(); openMenu(t, t.dataset.menu, t.dataset.id); return doRender();
    case "add-step": return insertStep(t.dataset.v);
    case "gap": st.cursor = Number(t.dataset.after); openMenu(t, "add"); return doRender();
    case "toggle-moves": st.showMoves = !st.showMoves; st.menu = null; toast(st.showMoves ? "Showing moves as steps" : "Moves are folded into the step they lead to"); return doRender();
    case "trim": { snapshot(); let n = 0; m.steps.forEach((x) => { if (x.pause > 1) { x.pause = 1; n++; } }); st.menu = null; toast(n ? `Shortened ${plural(n, "pause")} to 1 s` : "No pause is over 1 s", n ? { action: { label: "Undo", run: () => restore(st.undo, st.redo) } } : {}); return doRender(); }
    case "select-all": st.sel = new Set(m.steps.map((x) => x.id)); st.menu = null; return doRender();
    case "export": st.menu = null; if (t.dataset.id) st.macroId = t.dataset.id; st.dialog = "export"; doRender(); document.querySelector('[data-fid="fmt-' + st.exportFmt + '"]')?.focus(); return;
    case "fmt": st.exportFmt = t.dataset.v; return doRender();
    case "dialog-close": st.dialog = null; doRender(); document.querySelector('[data-fid="macro-more"]')?.focus(); return;
    case "do-export": { const ext = st.exportFmt; st.dialog = null; toast(`Saved ${m.name.toLowerCase().replace(/[^a-z0-9]+/g, "-")}.${ext} to Documents`); return doRender(); }
    case "duplicate": { st.menu = null; const src = t.dataset.id ? MACROS.find((x) => x.id === t.dataset.id) : m; const c = structuredClone(src); c.id = `${src.id}-copy${Date.now()}`; c.name = `${src.name} (copy)`; c.triggers = []; c.last = null; MACROS.splice(MACROS.indexOf(src) + 1, 0, c); toast(`Duplicated as “${c.name}”, without its triggers`); return doRender(); }
    case "delete-macro": { st.menu = null; const tgt = t.dataset.id ? MACROS.find((x) => x.id === t.dataset.id) : m; const at = MACROS.indexOf(tgt); MACROS.splice(at, 1); if (st.macroId === tgt.id) st.screen = "library"; toast(`Moved “${tgt.name}” to the trash`, { action: { label: "Undo", run: () => MACROS.splice(at, 0, tgt) } }); return doRender(); }
    case "play-macro": st.menu = null; st.playerMacro = t.dataset.id; return play();
    case "switch-macro": st.playerMacro = t.dataset.id; st.macroId = t.dataset.id; closeMenu(); toast(`Player: ${macro().name}`); return;
    case "runs-of": st.menu = null; st.screen = "activity"; st.libFilter = "runs"; return doRender();
    case "toast-action": { const tt = st.toasts.find((x) => x.id === t.dataset.id); tt?.action?.run(); st.toasts = st.toasts.filter((x) => x.id !== t.dataset.id); announce("Undone"); return doRender(); }
    case "toast-close": st.toasts = st.toasts.filter((x) => x.id !== t.dataset.id); return doRender();
    case "delete": deleteSelected(); return doRender();
    case "make-editable": { snapshot(); let n = 0; m.steps.forEach((x) => { if (st.sel.has(x.id) && x.kind === "type") { x.kind = "text"; n++; } }); toast(`Made ${plural(n, "text")} editable`, { action: { label: "Undo", run: () => restore(st.undo, st.redo) } }); return doRender(); }
    case "smooth": case "straighten": snapshot(); toast(a === "smooth" ? "Smoothed the path" : "Straightened the path", { action: { label: "Undo", run: () => restore(st.undo, st.redo) } }); return doRender();
    case "pick": case "pick-trigger": st.picking = { stepId: [...st.sel][0], x: Math.round(innerWidth * 0.3), y: Math.round(innerHeight * 0.35) }; announce("Picking a pixel. Move with the arrow keys and press Enter, or click. Escape cancels."); return doRender();
    case "pick-confirm": movePicker(e.clientX, e.clientY - 40); return confirmPick();
    case "clickpoint": { const r = t.getBoundingClientRect(); const x = m.steps.find((s) => st.sel.has(s.id)); if (x && e.detail) { snapshot(); x.cx = (e.clientX - r.left) / r.width; x.cy = (e.clientY - r.top) / r.height; } return doRender(); }
    case "test": st.testResult = "Found at 812, 440 · 97% match, well above 85%"; announce(st.testResult); return doRender();
    case "btn": { snapshot(); m.steps.find((s) => st.sel.has(s.id)).btn = t.dataset.v; return doRender(); }
    case "ph": { const ta = document.querySelector('[data-fid="f-text"]'); const x = m.steps.find((s) => st.sel.has(s.id)); if (ta && x) { snapshot(); const p = ta.selectionStart ?? ta.value.length; x.text = ta.value.slice(0, p) + t.dataset.v + ta.value.slice(ta.selectionEnd ?? p); } return doRender(); }
    case "forever": snapshot(); m.play.forever = !m.play.forever; return doRender();
    case "speed": snapshot(); m.play.speed = Number(t.dataset.v); return doRender();
    case "vary": snapshot(); m.play.vary = !m.play.vary; return doRender();
    case "stopkey": snapshot(); m.play.stopOnKey = !m.play.stopOnKey; return doRender();
    case "coords": snapshot(); m.play.coords = t.dataset.v; return doRender();
    case "data-choose": snapshot(); m.data = { file: "customers.csv", rows: 200, cols: ["PolicyNo", "Amount", "Customer"] }; toast("customers.csv: 200 rows. The macro plays once per row."); return doRender();
    case "data-remove": snapshot(); m.data = null; return doRender();
    case "trig-on": { snapshot(); const tr = m.triggers[Number(t.dataset.i)]; tr.on = !tr.on; return doRender(); }
    case "trig-remove": { snapshot(); const [tr] = m.triggers.splice(Number(t.dataset.i), 1); toast(`Removed the ${tr.type} trigger`, { action: { label: "Undo", run: () => restore(st.undo, st.redo) } }); return doRender(); }
    case "add-trigger": { snapshot(); const v = t.dataset.v; const tr = { hotkey: { type: "hotkey", on: false, combo: [] }, schedule: { type: "schedule", on: true, days: [1, 1, 1, 1, 1, 0, 0], time: "09:00" }, app: { type: "app", on: false, exe: "", delay: 2 }, pixel: { type: "pixel", on: false, x: 0, y: 0, color: "#000000" }, image: { type: "image", on: false, match: 85 } }[v]; m.triggers.push(tr); st.menu = null; if (v === "hotkey") st.capture = { for: `trigger:${m.triggers.length - 1}` }; return doRender(); }
    case "day": { snapshot(); const tr = m.triggers[Number(t.dataset.i)]; tr.days[Number(t.dataset.d)] = tr.days[Number(t.dataset.d)] ? 0 : 1; return doRender(); }
    case "capture": st.capture = st.capture?.for === t.dataset.for ? null : { for: t.dataset.for }; if (st.capture) announce("Press the key combination. Escape cancels, Backspace clears."); return doRender();
    case "view": st.viewOpen = !(st.viewOpen ?? viewDefault(m)); return doRender();
    case "prev": case "next": { const i = m.steps.findIndex((x) => x.id === st.cursor); const j = Math.max(0, Math.min(m.steps.length - 1, (i < 0 ? 0 : i) + (a === "next" ? 1 : -1))); selectRow(m.steps[j].id); return doRender(); }
    case "seek": { const lanes = t.getBoundingClientRect(); const f = (e.clientX - lanes.left) / lanes.width; const tt = f * duration(m); const x = [...m.steps].reverse().find((s) => s.t <= tt) ?? m.steps[0]; selectRow(x.id); return doRender(); }
    case "settings-nav": st.settingsSection = t.dataset.v; st.capture = null; return doRender();
    case "set": {
      const k = t.dataset.k;
      const map = { "rec-moves": ["recording", "moves"], "rec-keys": ["recording", "keys"], "rec-screenshot": ["recording", "screenshot"], "rec-countdown": ["recording", "countdown"], "rec-esc": ["recording", "escStops"], "rec-sim": ["recording", "ignoreSimulated"] };
      if (map[k]) st[map[k][0]][map[k][1]] = !st[map[k][0]][map[k][1]];
      else if (k === "glass") st.glass = !glassOn();
      else st[k] = !st[k];
      return doRender();
    }
    case "setv": { const k = t.dataset.k; st[k] = k === "textSize" ? Number(t.dataset.v) : t.dataset.v; return doRender(); }
    case "try-sound": SOUND[t.dataset.v](); return;
    case "first-next": st.firstStep++; if (st.firstStep === 2) { st.keys.record = st.firstChoice === "plain" ? ["F9"] : ["Ctrl", "Shift", "F9"]; st.keys.play = st.firstChoice === "plain" ? ["F10"] : ["Ctrl", "Shift", "F10"]; } return doRender();
    case "first-back": st.firstStep--; return doRender();
    case "first-skip": st.screen = "library"; return doRender();
    case "first-choice": st.firstChoice = t.dataset.v; return doRender();
    case "env-reset": location.search = ""; return;
    case "segnop": return;
  }
});

document.addEventListener("change", (e) => {
  const env = e.target.dataset.env;
  if (env) {
    const v = e.target.value;
    if (env === "screen") { st.screen = v; if (v === "firstrun") st.firstStep = 0; }
    else if (env === "transparency") st.transparency = v === "on";
    else st[env] = v;
    render(); return;
  }
  const f = e.target.dataset.field; if (!f) return;
  const m = macro(), x = m.steps.find((s) => st.sel.has(s.id)), v = e.target.value;
  snapshot();
  if (f === "label" && x) x.label = v;
  else if (f === "text" && x) x.text = v;
  else if (f === "color" && x) { if (/^#[0-9a-f]{6}$/i.test(v.trim())) x.color = v.trim().toUpperCase(); }
  else if (f === "pause-all") { const n = parseFloat(v.replace(",", ".")); if (!isNaN(n)) m.steps.forEach((s) => { if (st.sel.has(s.id)) s.pause = n; }); }
  else setNumber(f, v);
  render();
});
function setNumber(f, v) {
  const m = macro(), x = m.steps.find((s) => st.sel.has(s.id)); const n = parseFloat(String(v).replace(",", ".")); if (isNaN(n)) return;
  const p = m.play;
  ({ pause: () => (x.pause = Math.max(0, n)), dur: () => (x.dur = Math.max(0, n)), movedur: () => (x.move.dur = Math.max(0, n)), tol: () => (x.tol = Math.round(n)), timeout: () => (x.timeout = Math.max(0.5, n)), match: () => (x.match = Math.round(n)), px: () => (x.x = Math.round(n)), py: () => (x.y = Math.round(n)), repeat: () => (p.repeat = Math.max(1, Math.min(99, Math.round(n)))), jitter: () => (p.jitter = Math.round(n)), delay: () => {} }[f] ?? (() => {}))();
}
document.addEventListener("input", (e) => {
  const k = e.target.dataset.input; if (!k) return;
  if (k === "name") { const m = macro(); if (!e.target._snap) { snapshot(); e.target._snap = true; } m.name = e.target.value; st.saved = "saving"; clearTimeout(e.target._t); e.target._t = setTimeout(() => { st.saved = "saved"; render(); }, 500); return; }
  st[k] = e.target.value;
  render();
});

// Scrub: drag sideways on a number, like turning an encoder.
let scrub = null;
document.addEventListener("pointerdown", (e) => {
  const el = e.target.closest("[data-scrub]");
  if (!el || document.activeElement === el.querySelector("input") || st.session !== "idle") return;
  const input = el.querySelector("input");
  scrub = { el, input, x: e.clientX, v: parseFloat(input.value.replace(",", ".")) || 0, moved: false };
  el.setPointerCapture(e.pointerId);
});
document.addEventListener("pointermove", (e) => {
  if (st.picking && e.target.closest(".picker")) return movePicker(e.clientX, e.clientY - 40);
  if (!scrub) return;
  const dx = e.clientX - scrub.x; if (Math.abs(dx) < 3 && !scrub.moved) return;
  if (!scrub.moved) { scrub.moved = true; snapshot(); scrub.el.classList.add("dragging"); }
  const step = Number(scrub.el.dataset.step), dec = Number(scrub.el.dataset.dec);
  const v = Math.max(Number(scrub.el.dataset.min), Math.min(Number(scrub.el.dataset.max), scrub.v + Math.round(dx / 4) * step));
  scrub.input.value = v.toFixed(dec);
  scrub.el.querySelector(".ticks").style.backgroundPosition = `${dx}px 0`;
  setNumber(scrub.input.dataset.field, v);
});
document.addEventListener("pointerup", () => {
  if (!scrub) return;
  const { moved, input } = scrub; scrub = null;
  if (moved) render(); else { input.focus(); input.select(); }
});

document.addEventListener("keydown", (e) => {
  const tgt = e.target;
  if (st.capture) { e.preventDefault(); return captureKey(e); }
  // Relay's own keys, from anywhere.
  if (matches(e, st.keys.kill)) { e.preventDefault(); stop(); toast("Stopped everything. Triggers are paused."); return; }
  if (matches(e, st.keys.record)) { e.preventDefault(); st.session === "recording" || st.session === "countdown" ? stop() : startRecording(); return; }
  if (matches(e, st.keys.play)) { e.preventDefault(); play(); return; }
  if (matches(e, st.keys.player)) { e.preventDefault(); return; }
  if (st.picking) {
    const d = e.shiftKey ? 10 : 1;
    const mv = { ArrowLeft: [-d, 0], ArrowRight: [d, 0], ArrowUp: [0, -d], ArrowDown: [0, d] }[e.key];
    if (mv) { e.preventDefault(); return movePicker(st.picking.x + mv[0], st.picking.y + mv[1]); }
    if (e.key === "Enter") { e.preventDefault(); return confirmPick(); }
    if (e.key === "Escape") { st.picking = null; announce("Cancelled."); render(); document.querySelector('[data-fid="pick"]')?.focus(); return; }
    return;
  }
  if (e.key === "Escape") {
    if (st.menu) { e.preventDefault(); return closeMenu(); }
    if (st.dialog) { st.dialog = null; render(); document.querySelector('[data-fid="macro-more"]')?.focus(); return; }
    if (st.session !== "idle") { stop(); return; }
    if (narrow() && st.inspOpen) { st.inspOpen = false; render(); document.querySelector('[data-fid="list"]')?.focus(); return; }
    if (st.sel.size > 1) { st.sel = new Set([st.cursor]); render(); return; }
  }
  // Menus: arrows move, Enter chooses.
  if (st.menu && tgt.closest(".menu")) {
    const items = [...document.querySelectorAll(".menu [role=menuitem]")]; const i = items.indexOf(tgt);
    if (e.key === "ArrowDown") { e.preventDefault(); items[(i + 1) % items.length]?.focus(); }
    if (e.key === "ArrowUp") { e.preventDefault(); items[(i - 1 + items.length) % items.length]?.focus(); }
    if (e.key === "Tab") { e.preventDefault(); closeMenu(); }
    return;
  }
  // The export dialog keeps the focus inside.
  if (st.dialog && e.key === "Tab") {
    const f = [...document.querySelectorAll('[role=dialog] button')]; const i = f.indexOf(tgt);
    if (e.shiftKey && i <= 0) { e.preventDefault(); f[f.length - 1].focus(); } else if (!e.shiftKey && i === f.length - 1) { e.preventDefault(); f[0].focus(); }
    return;
  }
  // Radio groups and tabs: arrows.
  const group = tgt.closest('[role=radiogroup], [role=tablist]');
  if (group && ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(e.key)) {
    const items = [...group.querySelectorAll('[role=radio], [role=tab]')]; const i = items.indexOf(tgt); if (i < 0) return;
    e.preventDefault(); const next = items[(i + (e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : -1) + items.length) % items.length];
    next.click(); setTimeout(() => document.querySelector(`[data-fid="${CSS.escape(next.dataset.fid)}"]`)?.focus(), 0); return;
  }
  const inField = tgt.matches("input, textarea");
  const m = macro();
  if (!inField && (e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z") { e.preventDefault(); restore(st.undo, st.redo); toast("Undone"); render(); return; }
  if (!inField && (e.ctrlKey || e.metaKey) && (e.key.toLowerCase() === "y" || (e.shiftKey && e.key.toLowerCase() === "z"))) { e.preventDefault(); restore(st.redo, st.undo); toast("Redone"); render(); return; }
  if (st.screen === "editor") {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") { e.preventDefault(); document.querySelector('[data-fid="find"]')?.focus(); return; }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "m") { e.preventDefault(); st.tab = "macro"; if (narrow()) st.inspOpen = true; render(); document.querySelector('[data-fid="tab-macro"]')?.focus(); return; }
    if (e.altKey && e.key === "ArrowLeft") { e.preventDefault(); st.screen = "library"; render(); return; }
    if (tgt.dataset.fid === "list") {
      const rows = visibleRows(m); const i = rows.findIndex((r) => r.id === st.cursor);
      const go = (j) => { j = Math.max(0, Math.min(rows.length - 1, j)); const id = rows[j].id; if (e.shiftKey) { st.anchor ??= st.cursor; selectRow(id, { extend: true }); } else selectRow(id); render(); };
      if (e.key === "ArrowDown") { e.preventDefault(); return go(i + 1); }
      if (e.key === "ArrowUp") { e.preventDefault(); return go(i - 1); }
      if (e.key === "Home") { e.preventDefault(); return go(0); }
      if (e.key === "End") { e.preventDefault(); return go(rows.length - 1); }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") { e.preventDefault(); st.sel = new Set(m.steps.map((x) => x.id)); render(); return; }
      if (e.key === "Delete") { e.preventDefault(); deleteSelected(); render(); return; }
      if (e.key === "Insert" || e.key === "+") { e.preventDefault(); openMenu(document.querySelector('[data-fid="add"]'), "add"); render(); return; }
      if (e.key === "Enter") { e.preventDefault(); if (narrow()) st.inspOpen = true; st.tab = "step"; render(); document.querySelector('#insp-panel input, #insp-panel textarea, #insp-panel button')?.focus(); return; }
      if (e.key === " ") { e.preventDefault(); play(); return; }
    }
    if (tgt.dataset.fid === "timeline") {
      const dur = duration(m); const cur = m.steps.find((x) => x.id === st.cursor)?.t ?? 0;
      if (e.key === "ArrowRight" || e.key === "ArrowLeft") { e.preventDefault(); const i = m.steps.findIndex((x) => x.id === st.cursor); const j = Math.max(0, Math.min(m.steps.length - 1, (i < 0 ? 0 : i) + (e.key === "ArrowRight" ? 1 : -1))); selectRow(m.steps[j].id); render(); return; }
      if (e.key === "Home" || e.key === "End") { e.preventDefault(); selectRow((e.key === "Home" ? m.steps[0] : m.steps[m.steps.length - 1]).id); render(); return; }
      if (e.key === " ") { e.preventDefault(); play(); return; }
      void dur; void cur;
    }
  }
  if (st.screen === "library" && tgt.dataset.fid?.startsWith("open-")) {
    const id = tgt.dataset.fid.slice(5);
    const ids = [...document.querySelectorAll("[data-fid^=open-]")].map((b) => b.dataset.fid);
    const i = ids.indexOf(tgt.dataset.fid);
    if (e.key === "ArrowDown" || e.key === "ArrowUp") { e.preventDefault(); document.querySelector(`[data-fid="${ids[i + (e.key === "ArrowDown" ? 1 : -1)] ?? ids[i]}"]`)?.focus(); return; }
    if (e.key === "Delete") { e.preventDefault(); const tgtM = MACROS.find((x) => x.id === id); const at = MACROS.indexOf(tgtM); MACROS.splice(at, 1); toast(`Moved “${tgtM.name}” to the trash`, { action: { label: "Undo", run: () => MACROS.splice(at, 0, tgtM) } }); render(); document.querySelector(`[data-fid="${ids[i + 1] ?? ids[i - 1]}"]`)?.focus(); return; }
  }
});

window.relayProto = { st, MACROS, render, play, startRecording, stop };
render();
})();
