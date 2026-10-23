// Takes the prototype through every screen and state, screenshots each, and audits it.
// node run.mjs <round-name> [filter], with the prototype served on port 4310 (see docs/design/visual-redesign.md).
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { launch, MOD } from "./cdp.mjs";

const here = path.dirname(new URL(import.meta.url).pathname).replace(/^\/(\w:)/, "$1");
const round = process.argv[2] ?? "round1";
const only = process.argv[3];
const out = path.join(here, "out", round);
fs.mkdirSync(out, { recursive: true });
const AUDIT = fs.readFileSync(path.join(here, "audit-page.js"), "utf8");
const BASE = "http://127.0.0.1:4310/prototype.html";
const P = (q) => `${BASE}?${q}`;
const setSel = (ids) => `(() => { const p = relayProto; const m = p.MACROS.find((x) => x.id === p.st.macroId); p.st.sel = new Set(${JSON.stringify(ids)}.map((i) => m.steps[i - 1].id)); p.st.cursor = m.steps[${ids[ids.length - 1]} - 1].id; p.render(); })()`;

const S = [
  ["first-0", "screen=firstrun&first=0"],
  ["first-1", "screen=firstrun&first=1"],
  ["first-2", "screen=firstrun&first=2"],
  ["library", "screen=library"],
  ["library-search", "screen=library", async (b) => { await b.clickSel('[data-fid="lib-search"]'); for (const c of "inv") await b.key(c, { text: c }); }],
  ["library-select", "screen=library", async (b) => { await b.clickSel('[data-fid="chk-invoice"]'); await b.clickSel('[data-fid="chk-claims"]'); }],
  ["library-rowmenu", "screen=library", async (b) => { await b.clickSel('[data-fid="more-claims"]'); }],
  ["activity-problems", "screen=library", async (b) => { await b.clickSel('[data-fid="nav-problems"]'); await b.clickSel('[data-fid="run-1"]'); }],
  ["editor-click", "screen=editor&step=6"],
  ["editor-moves", "screen=editor&step=6&moves"],
  ["editor-macro", "screen=editor&tab=macro"],
  ["editor-macro-runs", "screen=editor&tab=macro&msec=runs"],
  ["editor-macro-runs-dark", "screen=editor&tab=macro&msec=runs&theme=dark"],
  ["editor-multi", "screen=editor", async (b) => b.eval(setSel([3, 4, 5]))],
  ["editor-find", "screen=editor&step=10"],
  ["editor-pixel", "screen=editor&step=7"],
  ["editor-type", "screen=editor&step=4"],
  ["editor-text", "screen=editor&macro=claims&step=4"],
  ["editor-keyboard-only", "screen=editor&macro=thanks&step=1"],
  ["editor-add-menu", "screen=editor&step=6", async (b) => b.clickSel('[data-fid="add"]')],
  ["editor-export", "screen=editor&dialog=export"],
  ["editor-narrow", "screen=editor&step=6&size=min&inspector"],
  ["editor-narrow-closed", "screen=editor&step=6&size=min"],
  ["editor-text150", "screen=editor&step=6&text=150&inspector"],
  ["editor-text200", "screen=editor&step=6&text=200&inspector"],
  ["editor-large-window", "screen=editor&step=6&size=large"],
  ["editor-compact", "screen=editor&step=6&spacing=compact"],
  ["editor-large-spacing", "screen=editor&step=6&spacing=large"],
  ["editor-dark", "screen=editor&step=6&theme=dark"],
  ["editor-dark-macro", "screen=editor&tab=macro&theme=dark"],
  ["editor-win10", "screen=editor&step=6&os=win10"],
  ["editor-notransparency", "screen=editor&step=6&notransparency"],
  ["editor-failed", "screen=editor&macro=claims&step=10&error"],
  ["editor-playing", "screen=editor&playing=6.4"],
  ["settings-recording", "screen=settings&section=recording"],
  ["settings-keys", "screen=settings&section=keys"],
  ["settings-window", "screen=settings&section=window"],
  ["settings-appearance", "screen=settings&section=appearance"],
  ["settings-appearance-win10", "screen=settings&section=appearance&os=win10"],
  ["settings-sound", "screen=settings&section=sound"],
  ["keys-altgr", "screen=settings&section=keys&layout=sv-qwerty", async (b) => { await b.clickSel('[data-fid="cap-record"]'); await b.key("8", { code: "Digit8", mods: MOD.Ctrl | MOD.Alt }); }],
  ["keys-f9", "screen=settings&section=keys", async (b) => { await b.clickSel('[data-fid="cap-record"]'); await b.key("F9", { code: "F9" }); }],
  ["keys-kill-end", "screen=settings&section=keys", async (b) => { await b.clickSel('[data-fid="cap-kill"]'); await b.key("End", { code: "End", mods: MOD.Ctrl | MOD.Alt }); }],
  ["keys-refused", "screen=settings&section=keys", async (b) => { await b.clickSel('[data-fid="cap-play"]'); await b.key("a", { code: "KeyA", mods: MOD.Shift }); }],
  ["player-playing-row", "screen=editor&macro=claims&playing=3&mode=player"],
  ["player-recording", "screen=library&recording"],
  ["player-error", "screen=editor&macro=claims&error&step=10&mode=player"],
  ["player-large", "screen=editor&playing=4&player=large&mode=player"],
  ["player-dark", "screen=editor&playing=4&theme=dark&mode=player"],
  ["player-switch-menu", "screen=editor&playing=4&mode=player", async (b) => { await b.clickSel('[data-fid="p-switch"]'); }],
  ["player-mode", "screen=editor&mode=player"],
  ["player-mode-large-dark", "screen=library&mode=player&player=large&theme=dark"],
  ["editor-win10-dark", "screen=editor&step=6&os=win10&theme=dark"],
  ["triggered-run", "screen=editor&step=6&triggered"],
  ["triggered-run-library-dark", "screen=library&triggered&theme=dark"],
  ["exported", "frame=exported"],
  ["icons", "frame=icons"],
  ["picker", "screen=editor&step=7", async (b) => { await b.clickSel('[data-fid="pick"]'); }],
  ["calm-recording", "screen=library&recording&motion=on", null, { calmCheck: true }],
  ["forced-colors", "screen=editor&step=6", null, { media: [{ name: "forced-colors", value: "active" }] }],
  ["protanopia", "screen=editor&playing=6.4", null, { vision: "protanopia" }],
  ["deuteranopia", "screen=editor&playing=6.4", null, { vision: "deuteranopia" }],
  ["blurred-150", "screen=editor&step=6&text=150&inspector", null, { vision: "blurredVision" }],
  ["achromatopsia", "screen=library", null, { vision: "achromatopsia" }],
];

const b = await launch({ profile: path.join(os.tmpdir(), "relay-proto-edge") });
const report = {};
for (const [name, query, setup, opts = {}] of S) {
  if (only && !name.includes(only)) continue;
  b.errors.length = 0;
  await b.media(opts.media ?? []);
  await b.vision(opts.vision ?? "none");
  await b.goto(P(query));
  if (setup) await setup(b);
  await b.wait(opts.calmCheck ? 1200 : 450);
  await b.shot(path.join(out, `${name}.png`));
  let audit = null;
  try { audit = await b.eval(AUDIT); } catch (e) { audit = { error: String(e) }; }
  const ax = await b.ax();
  const unnamed = ax.filter((n) => !n.ignored && ["button", "textbox", "checkbox", "radio", "switch", "tab", "slider", "menuitem", "option", "combobox", "searchbox", "spinbutton"].includes(n.role?.value) && !n.name?.value?.trim()).map((n) => `${n.role.value}`);
  report[name] = { query, counts: audit.counts, issues: audit.issues, truncated: audit.truncated, running: audit.running, unnamedAX: unnamed, errors: [...b.errors] };
  const c = audit.counts ?? {};
  console.log(`${name.padEnd(28)} contrast ${c.contrast ?? "?"} small ${c.small ?? "?"} names ${c.names ?? "?"} targets ${c.targets ?? "?"} clipped ${c.clipped ?? "?"} overflow ${c.overflow ?? "?"} ax ${unnamed.length} err ${b.errors.length}${opts.calmCheck ? ` running ${audit.running?.length}` : ""}`);
}

// Keyboard: a Tab walk through the editor, and the list's keys.
if (!only || only.includes("kbd")) {
  await b.media([]); await b.vision("none");
  await b.goto(P("screen=editor&step=6"));
  const walk = [];
  for (let i = 0; i < 45; i++) {
    await b.key("Tab");
    walk.push(await b.eval(`(() => { const e = document.activeElement; const cs = getComputedStyle(e); const ring = cs.outlineStyle !== "none" && parseFloat(cs.outlineWidth) > 0 || /inset 0px 0px 0px 1.5px/.test(cs.boxShadow); return (e.getAttribute("aria-label") || e.textContent.trim().slice(0, 30) || e.tagName) + (ring ? "" : "  [NO RING]") + (e.closest(".env") ? "  [ENV]" : ""); })()`));
  }
  report.tabWalk = walk;
  await b.goto(P("screen=editor&step=6"));
  await b.eval(`document.querySelector("[data-fid=list]").focus()`);
  const log = [];
  const state = () => b.eval(`(() => { const p = relayProto.st; const m = relayProto.MACROS.find((x) => x.id === p.macroId); return { sel: [...p.sel].map((id) => m.steps.findIndex((s) => s.id === id) + 1), steps: m.steps.length, menu: p.menu?.id ?? null, focus: document.activeElement.dataset.fid ?? document.activeElement.tagName }; })()`);
  log.push(["start", await state()]);
  await b.key("ArrowDown"); log.push(["↓", await state()]);
  await b.key("ArrowDown", { mods: MOD.Shift }); log.push(["Shift+↓", await state()]);
  await b.key("Delete"); log.push(["Del", await state()]);
  await b.key("z", { code: "KeyZ", mods: MOD.Ctrl }); log.push(["Ctrl+Z", await state()]);
  await b.key("Insert"); log.push(["Insert", await state()]);
  await b.key("ArrowDown"); await b.key("Enter"); log.push(["↓ Enter (Text)", await state()]);
  await b.key("Escape"); log.push(["Esc", await state()]);
  await b.key("m", { code: "KeyM", mods: MOD.Ctrl }); log.push(["Ctrl+M", await state()]);
  report.listKeys = log;
  await b.shot(path.join(out, "kbd-end.png"));
  console.log("tab walk:", walk.join(" → "));
  console.log("list keys:", JSON.stringify(log));
}
fs.writeFileSync(path.join(out, "report.json"), JSON.stringify(report, null, 2));
await b.close();
