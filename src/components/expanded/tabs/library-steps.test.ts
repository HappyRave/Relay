// The Library and Steps tabs, and the inline step editor.
import { beforeEach, describe, expect, test, vi } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import LibraryTab from "./LibraryTab.svelte";
import StepsTab from "./StepsTab.svelte";
import { core, freshStore, settle } from "../../../test/app";
import { browserBackend } from "../../../lib/ipc/backend";
import type { RelayStore } from "../../../lib/state/relay.svelte";
import type { Step } from "../../../lib/types";

let relay: RelayStore;
const [A, B, C] = [1, 2, 3].map((n) => `00000000-0000-0000-0000-00000000000${n}`);
beforeEach(async () => {
  relay = await freshStore();
});

const rows = () => screen.getAllByRole("button").filter((b) => b.classList.contains("item") || b.classList.contains("row"));

describe("Library tab", () => {
  test("lists every macro with its length, steps, runs, hotkey and last run", async () => {
    await relay.setTriggers({ hotkey: { enabled: true, combo: "Ctrl + Alt + 1" } });
    render(LibraryTab);
    const items = rows();
    expect(items).toHaveLength(4);
    expect(items[0]).toHaveTextContent("Export invoice to PDF");
    expect(items[0]).toHaveTextContent("10.2 s · 18 steps · 148 runs");
    expect(items[0]).toHaveTextContent("Ctrl + Alt + 1");
    expect(items[1]).toHaveTextContent("—"); // the samples' hotkeys are off
    expect(items[0]).toHaveClass("active");
  });

  test("turning a hotkey off takes it out of the list", async () => {
    await relay.setTriggers({ hotkey: { enabled: true, combo: "Ctrl + Alt + 1" } });
    render(LibraryTab);
    expect(rows()[0]).toHaveTextContent("Ctrl + Alt + 1");
    await relay.setTriggers({ hotkey: { enabled: false, combo: "Ctrl + Alt + 1" } });
    await settle();
    expect(rows()[0]).not.toHaveTextContent("Ctrl + Alt + 1");
    expect(rows()[0]).toHaveTextContent("—");
  });

  test("an empty library says how to make a macro", async () => {
    core.entries = [];
    await relay.refreshLibrary();
    render(LibraryTab);
    expect(screen.getByText("No macros yet — press Record (F9) to make one.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Import/ })).toBeEnabled();
  });

  test("clicking a macro opens it", async () => {
    render(LibraryTab);
    await userEvent.click(screen.getByText("Fill weekly timesheet"));
    await settle();
    expect(core.argsOf("load_macro")).toEqual([{ id: B }]);
    expect(relay.view?.id).toBe(B);
    expect(relay.tab).toBe("steps");
  });

  test("Enter or Space on a row opens it", async () => {
    render(LibraryTab);
    rows()[2].focus();
    await userEvent.keyboard("{Enter}");
    await settle();
    rows()[1].focus();
    await userEvent.keyboard(" ");
    await settle();
    expect(core.argsOf("load_macro")).toEqual([{ id: C }, { id: B }]);
  });

  test("Duplicate copies that macro, without opening the row first", async () => {
    render(LibraryTab);
    await userEvent.click(screen.getByRole("button", { name: "Duplicate Batch rename photos" }));
    await settle();
    expect(core.commands()).toEqual(["duplicate_macro", "list_macros", "load_macro", "screenshot", "get_triggers"]);
    expect(core.argsOf("duplicate_macro")).toEqual([{ id: C }]);
    expect(rows()).toHaveLength(5);
    expect(rows()[3]).toHaveTextContent("Batch rename photos (copy)"); // right after the original
    expect(rows()[3]).toHaveClass("active"); // and opened
    expect(relay.view?.name).toBe("Batch rename photos (copy)");
  });

  test("Delete moves that macro to the trash", async () => {
    render(LibraryTab);
    await userEvent.click(screen.getByRole("button", { name: "Delete Batch rename photos" }));
    await settle();
    expect(core.argsOf("delete_macro")).toEqual([{ id: C }]);
    expect(core.argsOf("load_macro")).toEqual([]);
    expect(rows()).toHaveLength(3);
    expect(screen.queryByText("Batch rename photos")).toBeNull();
  });

  test("Enter on Delete deletes instead of opening", async () => {
    render(LibraryTab);
    screen.getByRole("button", { name: "Delete Fill weekly timesheet" }).focus();
    await userEvent.keyboard("{Enter}");
    await settle();
    expect(core.argsOf("delete_macro")).toEqual([{ id: B }]);
    expect(core.argsOf("load_macro")).toEqual([]);
  });

  test("the open macro shows the name being typed", async () => {
    render(LibraryTab);
    relay.rename("Typing…");
    await settle();
    expect(rows()[0]).toHaveTextContent("Typing…");
  });

  test("Duplicate and Delete are hidden, and Import… is off, during a session", async () => {
    render(LibraryTab);
    core.emit({ type: "session", mode: "playing", macro_id: A });
    await settle();
    expect(screen.queryByRole("button", { name: /^Duplicate/ })).toBeNull();
    expect(screen.queryByRole("button", { name: /^Delete/ })).toBeNull();
    expect(screen.getByRole("button", { name: /Import/ })).toBeDisabled();
  });

  test("Import… asks for files and imports them", async () => {
    render(LibraryTab);
    core.dialog.open = ["C:\\macros\\Weekly report.rly"];
    await userEvent.click(screen.getByRole("button", { name: /Import/ }));
    await settle();
    expect(core.commands()).toEqual(["plugin:dialog|open", "import_macros", "list_macros", "load_macro", "screenshot", "get_triggers"]);
    expect(rows()[0]).toHaveTextContent("Weekly report"); // at the top
    expect(rows()[0]).toHaveClass("active");
    expect(relay.toast).toMatchObject({ kind: "info", message: "Imported 1 macro" });
  });

  test("the browser preview can't change the library", async () => {
    core.uninstall();
    try {
      await freshStore({ backend: browserBackend() });
      render(LibraryTab);
      expect(screen.queryByRole("button", { name: /^Duplicate/ })).toBeNull();
      expect(screen.queryByRole("button", { name: /Import/ })).toBeNull();
    } finally {
      core.install();
    }
  });
});

describe("Steps tab", () => {
  const stepRows = () => screen.getAllByRole("button").filter((b) => b.classList.contains("row"));

  test("describes each step", () => {
    render(StepsTab);
    const r = stepRows();
    expect(r).toHaveLength(18);
    expect(screen.getByText("18 steps")).toBeInTheDocument();
    expect(r[0]).toHaveTextContent("00:00.00 MOVE Move 960, 670 → 134, 70 px · 0.85 s");
    expect(r[1]).toHaveTextContent("00:00.85 CLICK Click · File menu 134, 70 px");
    expect(r[2]).toHaveTextContent("00:01.12 MOVE Move 134, 70 → 230, 324 px · 0.63 s");
    expect(r[4]).toHaveTextContent("WAIT Wait 0.7 s Dialog opens");
    expect(r[7]).toHaveTextContent("KEYS");
    expect(r[7]).toHaveTextContent("Key combination");
    expect(r[8]).toHaveTextContent("TYPE");
    expect(r[8]).toHaveTextContent("characters");
    expect(r[11]).toHaveTextContent("IF Wait for pixel 1248, 680 = #9B9797 Save button turns grey · timeout 5 s, else stop");
  });

  test.each<[string, Partial<Step>, string, string]>([
    ["double click", { kind: "click", btn: "Left", count: 2, label: "" }, "Double click", "10, 20 px"],
    ["triple click", { kind: "click", btn: "Left", count: 3, label: "" }, "Triple click", ""],
    ["quadruple click", { kind: "click", btn: "Left", count: 4, label: "" }, "4× click", ""],
    ["right click", { kind: "click", btn: "Right", count: 1, label: "Menu" }, "Right click · Menu", ""],
    ["drag", { kind: "drag", btn: "Left", to_x: 300, to_y: 400, label: "" }, "Drag", "10, 20 px → 300, 400"],
    ["right drag", { kind: "drag", btn: "Right", to_x: 1, to_y: 2, label: "Box" }, "Right drag · Box", ""],
    ["scroll down", { kind: "scroll", delta: -240, horizontal: false }, "Scroll down", "2 notches at 10, 20 px"],
    ["scroll up", { kind: "scroll", delta: 120, horizontal: false }, "Scroll up", "1 notch at 10, 20 px"],
    ["half-notch scroll", { kind: "scroll", delta: 60, horizontal: false }, "Scroll up", "0.5 notch at"],
    ["precise scroll", { kind: "scroll", delta: -200, horizontal: false }, "Scroll down", "1.7 notches at"],
    ["scroll right", { kind: "scroll", delta: 120, horizontal: true }, "Scroll right", ""],
    ["scroll left", { kind: "scroll", delta: -120, horizontal: true }, "Scroll left", ""],
    ["keys", { kind: "keys", combo: ["Ctrl", "Shift", "S"] }, "Ctrl + Shift + S", "Key combination"],
    ["type", { kind: "type", text: "hi", chars: [] }, "“hi”", "2 characters"],
    ["one char", { kind: "type", text: "x", chars: [] }, "“x”", "1 character"],
    ["move", { kind: "move", to_x: 300, to_y: 400, samples: 3 }, "Move", "10, 20 → 300, 400 px · 0.01 s"],
  ])("describes a %s", async (_n, step, detail, sub) => {
    relay.view = {
      ...relay.view!,
      steps: [{ t: 0, end: 10, pause: 0, items: [0], x: 10, y: 20, ...step } as Step],
    };
    render(StepsTab);
    expect(stepRows()[0]).toHaveTextContent(detail);
    if (sub) expect(stepRows()[0]).toHaveTextContent(sub);
  });

  test("in Window coordinates, positions are relative to the anchor window", async () => {
    await relay.setPlayback({ coord_mode: "window" });
    render(StepsTab);
    // The anchor window is at 48, 36.
    expect(stepRows()[1]).toHaveTextContent("+86, +34 in window");
    expect(stepRows()[2]).toHaveTextContent("+86, +34 → +182, +288 in window · 0.63 s");
  });

  test("in Window coordinates, a drag shows both ends in the window", async () => {
    await relay.setPlayback({ coord_mode: "window" });
    relay.view = {
      ...relay.view!,
      steps: [{ kind: "drag", t: 0, end: 10, pause: 0, items: [0, 1], x: 40, y: 50, to_x: 300, to_y: 400, btn: "Left", label: "" }],
    };
    render(StepsTab);
    // The anchor window is at 48, 36: a start left of it is negative.
    expect(stepRows()[0]).toHaveTextContent("-8, +14 → +252, +364 in window");
  });

  test("long pauses are marked above their step", async () => {
    render(StepsTab);
    // The sample's pauses are short: the cursor moves between its steps.
    expect(screen.queryByText(/s pause$/)).toBeNull();
    await relay.setPause(4, 1400);
    await settle();
    expect(screen.getByText("1.4 s pause")).toBeInTheDocument();
    expect(screen.getAllByText(/s pause$/)).toHaveLength(1);
  });

  test("a pause trimmed to exactly 1 s isn't marked any more", async () => {
    render(StepsTab);
    await relay.setPause(4, 1400);
    await relay.trimPauses();
    await settle();
    expect(relay.steps[4].pause).toBe(1000);
    expect(screen.queryByText(/s pause$/)).toBeNull();
  });

  test("the step under the playhead is highlighted, later ones dimmed", async () => {
    render(StepsTab);
    relay.seek(2000);
    await settle();
    const r = stepRows();
    expect(r[4]).toHaveClass("active");
    expect(r[3]).not.toHaveClass("future");
    expect(r[5]).toHaveClass("future");
  });

  test("clicking a step moves the playhead there and opens its editor; again closes it", async () => {
    render(StepsTab);
    await userEvent.click(stepRows()[4]);
    expect(relay.cur).toBe(2000);
    expect(stepRows()[4]).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("group", { name: "Edit step" })).toBeInTheDocument();
    await userEvent.click(stepRows()[4]);
    expect(screen.queryByRole("group", { name: "Edit step" })).toBeNull();
  });

  test("Enter opens a step's editor too", async () => {
    render(StepsTab);
    stepRows()[3].focus();
    await userEvent.keyboard("{Enter}");
    expect(screen.getByRole("group", { name: "Edit step" })).toBeInTheDocument();
  });

  test("Space opens a step's editor, and again closes it", async () => {
    render(StepsTab);
    stepRows()[6].focus();
    await userEvent.keyboard(" ");
    expect(stepRows()[6]).toHaveAttribute("aria-expanded", "true");
    expect(relay.cur).toBe(3500);
    await userEvent.keyboard(" ");
    expect(screen.queryByRole("group", { name: "Edit step" })).toBeNull();
  });

  test("Enter on a step's × deletes it instead of opening it", async () => {
    render(StepsTab);
    within(stepRows()[4]).getByRole("button", { name: "Delete step" }).focus();
    await userEvent.keyboard("{Enter}");
    await settle();
    expect(core.argsOf("edit_macro")).toEqual([{ id: A, op: { op: "delete_step", index: 4 } }]);
    expect(screen.queryByRole("group", { name: "Edit step" })).toBeNull();
  });

  test("× deletes that step (and doesn't open it)", async () => {
    render(StepsTab);
    await userEvent.click(within(stepRows()[4]).getByRole("button", { name: "Delete step" }));
    await settle();
    expect(core.argsOf("edit_macro")).toEqual([{ id: A, op: { op: "delete_step", index: 4 } }]);
    expect(stepRows()).toHaveLength(17);
    expect(screen.queryByRole("group", { name: "Edit step" })).toBeNull();
  });

  test("+ Wait, + Pixel check and Trim pauses", async () => {
    render(StepsTab);
    expect(screen.getByRole("button", { name: "Trim pauses" })).toBeDisabled(); // no pause over 1 s
    await relay.setPause(4, 2500);
    await settle();
    core.clearCalls();
    relay.seek(1000);
    await userEvent.click(screen.getByRole("button", { name: "+ Wait" }));
    await settle();
    await userEvent.click(screen.getByRole("button", { name: "+ Pixel check" }));
    await settle();
    await userEvent.click(screen.getByRole("button", { name: "Trim pauses" }));
    await settle();
    expect(core.argsOf("edit_macro").map((a) => (a.op as { op: string }).op)).toEqual([
      "insert_wait",
      "insert_pixel_wait",
      "cap_pauses",
    ]);
    expect(stepRows()).toHaveLength(20);
    expect(screen.getByRole("button", { name: "Trim pauses" })).toBeDisabled(); // nothing left to trim
  });

  test("editing is off while recording, and the live steps are shown", async () => {
    render(StepsTab);
    core.emit({ type: "session", mode: "recording", macro_id: null });
    await settle();
    for (const name of ["+ Wait", "+ Type text", "+ Pixel check", "+ Find image", "Trim pauses"]) expect(screen.getByRole("button", { name })).toBeDisabled();
    expect(screen.getByText("0 steps")).toBeInTheDocument();
    const step = core.view(A).steps[0];
    core.emit({ type: "rec_progress", elapsed_ms: 900, desktop: { x: 0, y: 0, w: 1920, h: 1080 }, moves: [], steps: [step] });
    await settle();
    expect(stepRows()).toHaveLength(1);
    expect(within(stepRows()[0]).getByRole("button", { name: "Delete step" })).toBeDisabled();
    expect(screen.queryByText(/s pause$/)).toBeNull();
  });

  test.each(["playing", "paused", "countdown"] as const)("editing is off while %s", async (mode) => {
    render(StepsTab);
    core.emit({ type: "session", mode, macro_id: A });
    await settle();
    for (const name of ["+ Wait", "+ Type text", "+ Pixel check", "+ Find image", "Trim pauses"]) expect(screen.getByRole("button", { name })).toBeDisabled();
    expect(within(stepRows()[0]).getByRole("button", { name: "Delete step" })).toBeDisabled();
  });

  test("with no macro open, there's nothing to add to or trim", async () => {
    relay.view = null;
    render(StepsTab);
    for (const name of ["+ Wait", "+ Type text", "+ Pixel check", "+ Find image", "Trim pauses"]) expect(screen.getByRole("button", { name })).toBeDisabled();
  });

  test("the editor closes when its step goes away", async () => {
    render(StepsTab);
    await userEvent.click(stepRows()[2]);
    await relay.edit({ op: "delete_step", index: 2 });
    await settle();
    expect(screen.queryByRole("group", { name: "Edit step" })).toBeNull();
  });

  test("the editor stays with its step when an earlier step is deleted", async () => {
    render(StepsTab);
    await userEvent.click(stepRows()[2]);
    await relay.edit({ op: "delete_step", index: 0 });
    await settle();
    expect(stepRows()[1]).toHaveAttribute("aria-expanded", "true");
  });

  test("the editor follows its step when a step is inserted before it, and through undo and redo", async () => {
    render(StepsTab);
    await userEvent.click(stepRows()[4]); // Wait 0.7 s · Dialog opens
    relay.seek(0);
    await relay.insertWait(); // after the first move and click
    await settle();
    expect(stepRows()[5]).toHaveAttribute("aria-expanded", "true");
    expect(stepRows()[5]).toHaveTextContent("Dialog opens");
    await relay.undo();
    await settle();
    expect(stepRows()[4]).toHaveAttribute("aria-expanded", "true");
    await relay.redo();
    await settle();
    expect(stepRows()[5]).toHaveAttribute("aria-expanded", "true");
  });

  test("editing the open step keeps it open, though it looks different now", async () => {
    render(StepsTab);
    await userEvent.click(stepRows()[4]);
    await relay.edit({ op: "set_label", index: 4, label: "Save dialog" });
    await settle();
    expect(stepRows()[4]).toHaveAttribute("aria-expanded", "true");
    expect(stepRows()[4]).toHaveTextContent("Save dialog");
  });

  test("opening another macro closes the editor", async () => {
    render(StepsTab);
    await userEvent.click(stepRows()[2]);
    await relay.loadMacro(B);
    await settle();
    expect(screen.queryByRole("group", { name: "Edit step" })).toBeNull();
  });

  test("no editor while playing", async () => {
    render(StepsTab);
    await userEvent.click(stepRows()[2]);
    core.emit({ type: "session", mode: "playing", macro_id: A });
    await settle();
    expect(screen.queryByRole("group", { name: "Edit step" })).toBeNull();
  });

  test("the browser preview shows steps read-only", async () => {
    core.uninstall();
    try {
      await freshStore({ backend: browserBackend() });
      render(StepsTab);
      expect(screen.getByRole("button", { name: "+ Wait" })).toBeDisabled();
      await userEvent.click(stepRows()[2]);
      expect(screen.queryByRole("group", { name: "Edit step" })).toBeNull();
    } finally {
      core.install();
    }
  });
});

describe("Step editor", () => {
  const stepRows = () => screen.getAllByRole("button").filter((b) => b.classList.contains("row"));
  async function open(i: number) {
    render(StepsTab);
    await userEvent.click(stepRows()[i]);
    return screen.getByRole("group", { name: "Edit step" });
  }
  const ops = () => core.argsOf("edit_macro").map((a) => a.op);
  const change = (el: HTMLElement, value: string) => fireEvent.change(el, { target: { value } });

  test("the pause before any step", async () => {
    const editor = await open(13);
    const pause = within(editor).getByLabelText(/Pause before/);
    expect(pause).toHaveValue(0.3);
    await change(pause, "2.5");
    await settle();
    await change(pause, "-1"); // refused: put back
    expect(pause).toHaveValue(2.5);
    await change(pause, ""); // cleared: put back
    expect(pause).toHaveValue(2.5);
    await settle();
    expect(ops()).toEqual([{ op: "set_pause", index: 13, dur: 2500 }]);
    expect(pause).toHaveValue(2.5); // the saved pause
  });

  test("a click's label", async () => {
    const editor = await open(1);
    const label = within(editor).getByLabelText("Label");
    expect(label).toHaveValue("File menu");
    expect(label).toHaveAttribute("placeholder", "e.g. Save button");
    await change(label, "The File menu");
    await settle();
    expect(ops()).toEqual([{ op: "set_label", index: 1, label: "The File menu" }]);
  });

  test("a wait's duration and label", async () => {
    const editor = await open(4);
    const dur = within(editor).getByLabelText(/Duration/);
    expect(dur).toHaveValue(0.7);
    await change(dur, "1.25");
    await settle();
    await change(dur, "-3");
    expect(dur).toHaveValue(1.25);
    await settle();
    await change(within(editor).getByLabelText("Label"), "Wait for dialog");
    await settle();
    expect(ops()).toEqual([
      { op: "set_wait_duration", index: 4, dur: 1250 },
      { op: "set_label", index: 4, label: "Wait for dialog" },
    ]);
  });

  test("keys and typing have no label", async () => {
    const editor = await open(7);
    expect(within(editor).queryByLabelText("Label")).toBeNull();
    expect(within(editor).getByLabelText(/Pause before/)).toBeInTheDocument();
  });

  describe("a move", () => {
    test("its duration: shorter is faster", async () => {
      const editor = await open(2); // 1116..1750
      const dur = within(editor).getByLabelText(/Duration/);
      expect(dur).toHaveValue(0.63);
      await change(dur, "0.3");
      await settle();
      await change(dur, "-1"); // refused: put back
      expect(dur).toHaveValue(0.3);
      await settle();
      expect(ops()).toEqual([{ op: "set_move_duration", index: 2, dur: 300 }]);
      expect(relay.steps[2]).toMatchObject({ t: 1116, end: 1416 });
      expect(within(editor).queryByLabelText("Label")).toBeNull();
      expect(within(editor).getByLabelText(/Pause before/)).toHaveValue(0.2);
    });

    test("Smooth and Straighten reshape it, and offer Undo", async () => {
      const editor = await open(2);
      await userEvent.click(within(editor).getByRole("button", { name: "Straighten" }));
      await settle();
      expect(relay.toast).toMatchObject({ message: "Straightened the move", action: { label: "Undo" } });
      await userEvent.click(within(editor).getByRole("button", { name: "Smooth" }));
      await settle();
      expect(ops()).toEqual([
        { op: "straighten_move", index: 2 },
        { op: "smooth_move", index: 2 },
      ]);
      // Already straight: nothing changed, so there's nothing to undo.
      expect(relay.toast).toBeNull();
      expect(stepRows()[2]).toHaveAttribute("aria-expanded", "true");
    });

    test("one sample is a jump: nothing to retime or reshape", async () => {
      await relay.loadMacro("00000000-0000-0000-0000-000000000004"); // Open standup tools
      const editor = await open(1);
      expect(within(editor).getByLabelText(/Duration/)).toBeDisabled();
      expect(within(editor).getByRole("button", { name: "Smooth" })).toBeDisabled();
      expect(within(editor).getByRole("button", { name: "Straighten" })).toBeDisabled();
    });
  });

  describe("a pixel check", () => {
    const base = { index: 11, x: 1248, y: 680, color: "#9B9797", tolerance: 8, timeout_ms: 5000 };

    test("shows its position, color, tolerance and timeout", async () => {
      const e = await open(11);
      expect(within(e).getByLabelText("X")).toHaveValue(1248);
      expect(within(e).getByLabelText("Y")).toHaveValue(680);
      expect(within(e).getByLabelText(/Color/)).toHaveValue("#9B9797");
      expect(within(e).getByLabelText("Tolerance")).toHaveValue(8);
      expect(within(e).getByLabelText(/Timeout/)).toHaveValue(5);
    });

    test("each field sends the whole check", async () => {
      const e = await open(11);
      await change(within(e).getByLabelText("X"), "100");
      await settle();
      await change(within(e).getByLabelText("Y"), "-50");
      await settle();
      await change(within(e).getByLabelText(/Color/), "#abcdef");
      await settle();
      await change(within(e).getByLabelText("Tolerance"), "300");
      await settle();
      await change(within(e).getByLabelText(/Timeout/), "2.5");
      await settle();
      expect(ops()).toEqual([
        { op: "update_pixel_wait", ...base, x: 100 },
        { op: "update_pixel_wait", ...base, x: 100, y: -50 },
        { op: "update_pixel_wait", ...base, x: 100, y: -50, color: "#ABCDEF" },
        { op: "update_pixel_wait", ...base, x: 100, y: -50, color: "#ABCDEF", tolerance: 255 },
        { op: "update_pixel_wait", ...base, x: 100, y: -50, color: "#ABCDEF", tolerance: 255, timeout_ms: 2500 },
      ]);
    });

    test("numbers are rounded to what Rust stores and clamped, and the field shows what's saved", async () => {
      const e = await open(11);
      const x = within(e).getByLabelText("X");
      await change(x, "12.7");
      expect(x).toHaveValue(13);
      await settle();
      const tolerance = within(e).getByLabelText("Tolerance");
      await change(tolerance, "300");
      await settle();
      await change(tolerance, "300"); // already 255: nothing to save, but the field says 255
      expect(tolerance).toHaveValue(255);
      await settle();
      const timeout = within(e).getByLabelText(/Timeout/);
      await change(timeout, "0.2");
      expect(timeout).toHaveValue(0.5);
      await settle();
      expect(ops()).toEqual([
        { op: "update_pixel_wait", ...base, x: 13 },
        { op: "update_pixel_wait", ...base, x: 13, tolerance: 255 },
        { op: "update_pixel_wait", ...base, x: 13, tolerance: 255, timeout_ms: 500 },
      ]);
    });

    test("a color that isn't #RRGGBB is put back", async () => {
      const e = await open(11);
      const color = within(e).getByLabelText(/Color/);
      await change(color, "red");
      expect(color).toHaveValue("#9B9797");
      await change(color, "#12345");
      expect(color).toHaveValue("#9B9797");
      expect(ops()).toEqual([]);
    });

    test("tolerance is kept within 0–255, and non-numbers are ignored", async () => {
      const e = await open(11);
      await change(within(e).getByLabelText("Tolerance"), "-4");
      await settle();
      expect(ops()).toEqual([{ op: "update_pixel_wait", ...base, tolerance: 0 }]);
      core.clearCalls();
      for (const field of [within(e).getByLabelText("X"), within(e).getByLabelText("Y"), within(e).getByLabelText("Tolerance"), within(e).getByLabelText(/Timeout/)]) {
        const before = (field as HTMLInputElement).value;
        await change(field, "");
        expect(field).toHaveProperty("value", before); // put back
      }
      await settle();
      expect(ops()).toEqual([]);
    });

    test("Pick counts down, then points the check at the pixel under the cursor", async () => {
      const e = await open(11);
      vi.useFakeTimers();
      core.hold("pick_pixel");
      await fireEvent.click(within(e).getByRole("button", { name: "Pick" }));
      await settle();
      expect(core.argsOf("pick_pixel")).toEqual([{ delayMs: 3000 }]);
      const pick = within(e).getByRole("button", { name: /Point at it/ });
      expect(pick).toHaveTextContent("Point at it… 3");
      expect(pick).toBeDisabled();
      await vi.advanceTimersByTimeAsync(1000);
      expect(pick).toHaveTextContent("Point at it… 2");
      core.held[0].resolve({ x: 5, y: 6, color: "#FF0000" });
      await settle();
      expect(within(e).getByRole("button", { name: "Pick" })).toBeEnabled();
      expect(ops()).toEqual([{ op: "update_pixel_wait", ...base, x: 5, y: 6, color: "#FF0000" }]);
    });
  });
});
