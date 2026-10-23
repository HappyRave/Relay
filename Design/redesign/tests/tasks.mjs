// The panel's tasks, done for real in the prototype: each step acts, then checks.
// node tasks.mjs <round-name>, with the prototype served on port 4310.
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { launch, MOD } from "./cdp.mjs";

const here = path.dirname(new URL(import.meta.url).pathname).replace(/^\/(\w:)/, "$1");
const round = process.argv[2] ?? "tasks1";
const out = path.join(here, "out", round);
fs.mkdirSync(out, { recursive: true });
const BASE = "http://127.0.0.1:4310/prototype.html";
const b = await launch({ port: 9334, profile: path.join(os.tmpdir(), "relay-proto-edge-tasks") });

const st = (expr) => b.eval(`(() => { const p = relayProto.st, M = relayProto.MACROS, m = M.find((x) => x.id === p.macroId); return ${expr}; })()`);
const q = (sel) => b.eval(`(() => { const e = document.querySelector(${JSON.stringify(sel)}); return e ? { text: e.innerText ?? e.textContent, cls: e.className?.baseVal ?? e.className, vis: !!e.getClientRects().length && getComputedStyle(e).visibility !== "hidden" && !e.closest(".hidden, .away") } : null; })()`);
const focus = () => b.eval(`document.activeElement?.dataset?.fid ?? document.activeElement?.tagName`);
const live = () => b.eval(`document.getElementById("live")?.textContent ?? ""`);
const typeText = async (t) => { for (const c of t) await b.key(c, { text: c, code: /[a-z]/i.test(c) ? `Key${c.toUpperCase()}` : c === " " ? "Space" : undefined }); };

const results = [];
async function task(persona, name, fn) {
  const log = [];
  let actions = 0, ok = true, note = "";
  const act = async (label, f) => { actions++; await f(); log.push(`→ ${label}`); };
  const check = async (label, cond) => { const v = await cond(); log.push(`${v ? "✓" : "✗"} ${label}`); if (!v) ok = false; return v; };
  try { await fn({ act, check, note: (n) => (note = n) }); } catch (e) { ok = false; log.push(`! ${e.message}`); }
  results.push({ persona, name, ok, actions, note, log });
  console.log(`${ok ? "PASS" : "FAIL"}  ${persona.padEnd(7)} ${name}  (${actions} actions)${note ? " — " + note : ""}`);
  if (!ok) console.log("      " + log.join("\n      "));
  await b.shot(path.join(out, `${persona}-${name.replace(/[^a-z0-9]+/gi, "-").toLowerCase()}.png`));
}

// P11 Linda: first run, first macro, name it, play it
await task("P11", "first run to a first macro", async ({ act, check }) => {
  await b.goto(`${BASE}?screen=firstrun&first=0`);
  await act("Next", () => b.clickSel('[data-fid="first-next"]'));
  await check("Ctrl + Shift + F9/F10 preselected", async () => (await b.eval(`document.querySelector('[data-fid="fc-ctrlshift"]').getAttribute("aria-checked")`)) === "true");
  await act("Next", () => b.clickSel('[data-fid="first-next"]'));
  await check("step 3 says how to record", async () => (await q(".first-copy h1"))?.text === "Your first macro");
  await act("Record now", () => b.clickSel('[data-fid="first-record"]'));
  await check("window steps aside, player counts down", async () => (await b.eval(`!!document.querySelector(".window.away")`)) && (await q(".player"))?.vis && /GET READY/.test((await q(".player .state"))?.text ?? ""));
  await check("countdown announced", async () => /Get ready/.test(await live()));
  await b.wait(3300);
  await check("recording", async () => /REC/.test((await q(".player .state"))?.text ?? ""));
  await b.wait(1900);
  await act("■ stops", () => b.clickSel('[data-fid="p-stop"]'));
  await check("back in the editor on the new macro", async () => (await st("p.screen")) === "editor" && /^Recording/.test(await st("m.name")));
  await check("name focused for typing", async () => (await focus()) === "name");
  await check("stop announced", async () => /Recording stopped/.test(await live()));
  await check("player hidden again", async () => !(await q(".player"))?.vis);
  await act("types a name", async () => { await b.key("a", { code: "KeyA", mods: MOD.Ctrl }); await typeText("Say hello"); await b.key("Tab"); });
  await check("named", async () => (await st("m.name")) === "Say hello");
  await act("Ctrl + Shift + F10 plays", () => b.key("F10", { code: "F10", mods: MOD.Ctrl | MOD.Shift }));
  await b.wait(500);
  await check("playing, window stays, player hidden", async () => (await st("p.session")) === "playing" && !(await q(".player"))?.vis);
  await b.wait(400);
});

// P1 Aisha: find typed text, make it editable; a data run that fails at row 112
await task("P1", "find typed text and make it editable", async ({ act, check }) => {
  await b.goto(`${BASE}?screen=editor`);
  await act("Ctrl + F", () => b.key("f", { code: "KeyF", mods: MOD.Ctrl }));
  await check("find focused", async () => (await focus()) === "find");
  await act("types 'type'", () => typeText("type"));
  await check("one row left", async () => (await b.eval(`document.querySelectorAll(".list .step").length`)) === 1);
  await act("clicks it", () => b.clickSel(".list .step"));
  await act("Make editable", () => b.clickSel('[data-fid="make-editable"]'));
  await check("now a Text step", async () => (await st("m.steps.find((x) => x.text === 'invoice_0924')?.kind")) === "text");
  await check("undo offered", async () => /Undo/.test((await q(".toast"))?.text ?? ""));
});
await task("P1", "a data run fails at row 112", async ({ act, check }) => {
  await b.goto(`${BASE}?screen=editor&macro=claims`);
  await check("play summary says each row", async () => /Each row \(200\)/.test((await q('[data-fid="chips"]'))?.text ?? ""));
  await act("Play", () => b.clickSel('[data-fid="play"]'));
  await b.wait(700);
  await check("inspector says the row", async () => /Row/.test((await q("#now-pane"))?.text ?? ""));
  await b.wait(8000);
  await check("stopped with an error that stays", async () => /row 112/.test((await q(".toast.error"))?.text ?? ""));
  await check("failed step marked", async () => !!(await q(".step.failed")));
  await act("clicks the failed step", () => b.clickSel(".step.failed"));
  await check("its inspector explains", async () => /Timed out in the last run/.test((await q("#insp-panel"))?.text ?? ""));
  await act("Library", () => b.clickSel('[data-fid="back"]'));
  await act("Problems", () => b.clickSel('[data-fid="nav-problems"]'));
  await check("the run is first, with the row", async () => /row 112/.test((await q(".run"))?.text ?? ""));
});

// P2 Joris: watch the steps while it plays; turn off varied timing
await task("P2", "watch a run, then make it deterministic", async ({ act, check }) => {
  await b.goto(`${BASE}?screen=editor`);
  await act("Play", () => b.clickSel('[data-fid="play"]'));
  await b.wait(1600);
  await check("window stays, rows fill in", async () => !(await st("p.playerMode")) && (await b.eval(`document.querySelectorAll(".step.played").length`)) >= 2);
  await act("Esc stops", () => b.key("Escape"));
  await act("summary → Macro tab", () => b.clickSel('[data-fid="chips"]'));
  await check("Macro tab open", async () => (await st("p.tab")) === "macro");
  await act("Vary timing off", () => b.clickSel('[data-fid="vary"]'));
  await check("summary drops ±40 ms", async () => !/±/.test((await q('[data-fid="chips"]'))?.text ?? "x±"));
});

// P3 Kenji: the player as a remote: switch macros, play, stop; play key to None
await task("P3", "the player as a remote", async ({ act, check }) => {
  await b.goto(`${BASE}?screen=editor`);
  await act("switch to the player", () => b.clickSel('[data-fid="to-player"]'));
  await check("window away, player shown", async () => (await q(".player"))?.vis && !!(await q(".window.away")));
  await act("macro menu", () => b.clickSel('[data-fid="p-switch"]'));
  await check("menu lists macros", async () => (await b.eval(`document.querySelectorAll(".menu [role=menuitem]").length`)) >= 7);
  await act("picks Reply: thanks", () => b.clickSel('.menu [data-id="thanks"]'));
  await check("player on Reply: thanks", async () => /Reply: thanks/.test((await q(".player .nm"))?.text ?? ""));
  await act("▶", () => b.clickSel('[data-fid="p-play"]'));
  await b.wait(300);
  await check("playing", async () => (await st("p.session")) === "playing");
  await act("■", () => b.clickSel('[data-fid="p-stop"]'));
  await check("stopped", async () => (await st("p.session")) === "idle");
  await act("back to the window", () => b.clickSel('[data-fid="p-open"]'));
  await check("window back", async () => !(await q(".window.away")));
});
await task("P3", "play key set to None", async ({ act, check }) => {
  await b.goto(`${BASE}?screen=settings&section=keys`);
  await act("Change play", () => b.clickSel('[data-fid="cap-play"]'));
  await act("Backspace", () => b.key("Backspace"));
  await check("play key is None", async () => (await st("p.keys.play.length")) === 0);
  await check("says what to use", async () => /buttons/.test((await q(".toast"))?.text ?? ""));
});

// P4 Dee: Windows 10 is solid; transparency off; the kill switch warning
await task("P4", "solid on Windows 10, and the kill switch", async ({ act, check }) => {
  await b.goto(`${BASE}?screen=settings&section=appearance&os=win10`);
  await check("solid by default on Windows 10", async () => b.eval(`document.documentElement.classList.contains("solid")`));
  await check("says why", async () => /Off by default on Windows 10/.test((await q(".settings-body"))?.text ?? ""));
  await act("turns glass on", () => b.clickSel('[data-fid="set-glass"]'));
  await check("glass on", async () => !(await b.eval(`document.documentElement.classList.contains("solid")`)));
  await b.goto(`${BASE}?screen=settings&section=appearance&notransparency`);
  await check("no transparency: switch off and disabled", async () => b.eval(`document.querySelector('[data-fid="set-glass"]').disabled && document.documentElement.classList.contains("solid")`));
  await b.goto(`${BASE}?screen=settings&section=keys`);
  await check("default kill switch is Ctrl + Alt + Backspace", async () => (await st("p.keys.kill.join('+')")) === "Ctrl+Alt+Backspace");
  await act("Change kill switch", () => b.clickSel('[data-fid="cap-kill"]'));
  await act("Ctrl + Alt + End", () => b.key("End", { code: "End", mods: MOD.Ctrl | MOD.Alt }));
  await check("Remote Desktop warning", async () => /Remote Desktop/.test((await q(".hint.warn"))?.text ?? ""));
});

// P5 Ruth: 150% text, large player
await task("P5", "150% text and a large player", async ({ act, check }) => {
  await b.goto(`${BASE}?screen=editor&step=6&text=150`);
  await check("still two columns", async () => !(await b.eval(`document.documentElement.classList.contains("narrow")`)));
  await check("smallest text at least 18 px", async () => (await b.eval(`Math.min(...[...document.querySelectorAll(".window *")].filter((e) => [...e.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim()) && e.getClientRects().length).map((e) => parseFloat(getComputedStyle(e).fontSize) * (parseFloat(getComputedStyle(document.querySelector(".window")).zoom) || 1)))`)) >= 17.9);
  await b.goto(`${BASE}?screen=editor&mode=player&player=large`);
  await check("large player", async () => !!(await q(".player.big")));
});

// P6 Sam: everything by keyboard, including picking a pixel
await task("P6", "pick a pixel by keyboard", async ({ act, check }) => {
  await b.goto(`${BASE}?screen=editor&step=6`);
  await b.eval(`document.querySelector("[data-fid=list]").focus()`);
  await act("↓ to the pixel check", () => b.key("ArrowDown"));
  await check("pixel check selected", async () => (await st("m.steps.find((x) => p.sel.has(x.id))?.kind")) === "pixel");
  await act("Enter into the inspector", () => b.key("Enter"));
  let tabs = 0;
  while ((await focus()) !== "pick" && tabs < 12) { await b.key("Tab"); tabs++; }
  await act(`Tab ×${tabs} to Pick from screen`, async () => {});
  await act("Enter", () => b.key("Enter"));
  await check("picker open", async () => !!(await q(".picker")));
  await act("→ ×5, ↓ ×3", async () => { for (let i = 0; i < 5; i++) await b.key("ArrowRight"); for (let i = 0; i < 3; i++) await b.key("ArrowDown"); });
  await act("Enter picks", () => b.key("Enter"));
  await check("picked, with undo", async () => /Picked/.test((await q(".toast"))?.text ?? "") && !(await q(".picker")));
});
await task("P6", "delete and undo by keyboard", async ({ act, check }) => {
  await b.goto(`${BASE}?screen=editor&step=3`);
  await b.eval(`document.querySelector("[data-fid=list]").focus()`);
  await act("Del", () => b.key("Delete"));
  await check("10 steps", async () => (await st("m.steps.length")) === 10);
  await act("Ctrl + Z", () => b.key("z", { code: "KeyZ", mods: MOD.Ctrl }));
  await check("11 steps, the step selected again", async () => (await st("m.steps.length")) === 11 && (await st("m.steps.findIndex((x) => p.sel.has(x.id))")) === 2);
  await act("Insert → Wait", async () => { await b.key("Insert"); await b.key("Enter"); });
  await check("a wait added after step 3", async () => (await st("m.steps[3].kind")) === "wait" && (await st("m.steps.length")) === 12);
});

// P7 Priya: names, announcements, focus
await task("P7", "screen reader basics", async ({ act, check }) => {
  await b.goto(`${BASE}?screen=editor&step=6`);
  const ax = await b.ax();
  const roles = (r) => ax.filter((n) => n.role?.value === r && !n.ignored);
  await check("the steps are a listbox of options with names", async () => roles("listbox").length >= 1 && roles("option").length >= 10 && roles("option").every((n) => n.name?.value));
  await check("the timeline is a slider that names the step", async () => roles("slider").some((n) => /step 6, Click/.test(JSON.stringify(n.properties ?? []) + (n.value?.value ?? "")) ) || (await b.eval(`/step 6, Click/.test(document.querySelector('[role=slider]').getAttribute("aria-valuetext"))`)));
  await check("every button has a name", async () => roles("button").every((n) => n.name?.value?.trim()));
  await act("opens the ⋯ menu", () => b.clickSel('[data-fid="macro-more"]'));
  await check("focus on the first item", async () => (await b.eval(`document.activeElement.getAttribute("role")`)) === "menuitem");
  await act("Esc", () => b.key("Escape"));
  await check("focus back on ⋯", async () => (await focus()) === "macro-more");
  await act("Ctrl + Shift + F9 records", () => b.key("F9", { code: "F9", mods: MOD.Ctrl | MOD.Shift }));
  await check("announced", async () => /Get ready|Recording/.test(await live()));
  await act("Ctrl + Shift + F9 stops (during the countdown it cancels)", () => b.key("F9", { code: "F9", mods: MOD.Ctrl | MOD.Shift }));
  await check("cancel announced", async () => /cancelled/i.test(await live()));
});

// P8 Mateo: the minimum window
await task("P8", "edit in the minimum window", async ({ act, check }) => {
  await b.goto(`${BASE}?screen=editor&size=min`);
  await check("narrow layout", async () => b.eval(`document.documentElement.classList.contains("narrow")`));
  const rowId = await st("m.steps[5].id");
  await act("taps a step", () => b.clickSel("#row-" + rowId));
  await check("inspector slides over", async () => !!(await q(".inspector.open")));
  await act("Esc", () => b.key("Escape"));
  await check("closed, focus on the list", async () => !(await q(".inspector.open")) && (await focus()) === "list");
});

// P9 Viktor: keyboard power
await task("P9", "keyboard power", async ({ act, check }) => {
  await b.goto(`${BASE}?screen=editor&step=1`);
  await b.eval(`document.querySelector("[data-fid=list]").focus()`);
  await act("Ctrl + A", () => b.key("a", { code: "KeyA", mods: MOD.Ctrl }));
  await check("all 11 selected", async () => (await st("p.sel.size")) === 11);
  await check("inspector shows the shared fields", async () => /11 steps selected/i.test((await q("#insp-panel"))?.text ?? ""));
  await act("Esc", () => b.key("Escape"));
  await check("back to one", async () => (await st("p.sel.size")) === 1);
  await act("Show moves", async () => { await b.clickSel('[data-fid="list-more"]'); await b.clickSel('.menu [data-act="toggle-moves"]'); });
  await check("move rows shown", async () => (await b.eval(`document.querySelectorAll(".step.move-row").length`)) >= 4);
  await act("Alt + ←", () => b.key("ArrowLeft", { mods: MOD.Alt }));
  await check("Library", async () => (await st("p.screen")) === "library");
});

// P12 Hannah: what runs when
await task("P12", "what runs when", async ({ act, check }) => {
  await b.goto(`${BASE}?screen=library`);
  await check("schedule in the Library row", async () => /Mon–Fri at 09:00/.test((await q("table.macros"))?.text ?? ""));
  await check("Problems badge says 2", async () => /2/.test((await q('[data-fid="nav-problems"] .badge'))?.text ?? ""));
  await act("opens Export invoice", () => b.clickSel('[data-fid="open-invoice"]'));
  await act("Ctrl + M", () => b.key("m", { code: "KeyM", mods: MOD.Ctrl }));
  await act("opens Runs automatically", () => b.clickSel('[data-fid="msec-runs"]'));
  await act("turns Saturday on", () => b.clickSel('[data-fid="day-1-5"]'));
  await check("sentence updates", async () => /Mon, Tue, Wed, Thu, Fri, Sat at 09:00/.test((await q("#insp-panel"))?.text ?? ""));
});

// P13 Grace: find among many, export two
await task("P13", "find and export several", async ({ act, check }) => {
  await b.goto(`${BASE}?screen=library`);
  await act("search 'ref'", async () => { await b.clickSel('[data-fid="lib-search"]'); await typeText("ref"); });
  await check("one macro", async () => (await b.eval(`document.querySelectorAll("table.macros tbody tr").length`)) === 1);
  await check("hotkey as keycaps", async () => (await b.eval(`document.querySelectorAll("table.macros .kc").length`)) === 3);
  await act("clears the search", async () => { await b.key("a", { code: "KeyA", mods: MOD.Ctrl }); await b.key("Backspace"); });
  await act("selects two", async () => { await b.clickSel('[data-fid="chk-refund"]'); await b.clickSel('[data-fid="chk-thanks"]'); });
  await check("bulk bar", async () => /2 macros selected/.test((await q(".bulk"))?.text ?? ""));
});

fs.writeFileSync(path.join(out, "tasks.json"), JSON.stringify(results, null, 2));
console.log(`${results.filter((r) => r.ok).length}/${results.length} tasks pass`);
await b.close();
