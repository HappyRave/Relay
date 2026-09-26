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
  test("lists every macro with its length, steps, runs, hotkey and last run", () => {
    render(LibraryTab);
    const items = rows();
    expect(items).toHaveLength(4);
    expect(items[0]).toHaveTextContent("Export invoice to PDF");
    expect(items[0]).toHaveTextContent("10.2 s · 12 steps · 148 runs");
    expect(items[0]).toHaveTextContent("Ctrl + Alt + 1");
    expect(items[2]).toHaveTextContent("—"); // no hotkey
    expect(items[0]).toHaveClass("active");
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
    expect(core.commands()).toEqual(["duplicate_macro", "list_macros", "load_macro", "get_triggers"]);
    expect(core.argsOf("duplicate_macro")).toEqual([{ id: C }]);
    expect(rows()).toHaveLength(5);
    expect(screen.getByText("Batch rename photos (copy)")).toBeInTheDocument();
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

  test("Duplicate and Delete are hidden during a session", async () => {
    render(LibraryTab);
    core.emit({ type: "session", mode: "playing", macro_id: A });
    await settle();
    expect(screen.queryByRole("button", { name: /^Duplicate/ })).toBeNull();
    expect(screen.queryByRole("button", { name: /^Delete/ })).toBeNull();
  });

  test("Import… asks for files and imports them", async () => {
    render(LibraryTab);
    core.dialog.open = ["C:\\macros\\Weekly report.rly"];
    await userEvent.click(screen.getByRole("button", { name: /Import/ }));
    await settle();
    expect(core.commands()).toEqual(["plugin:dialog|open", "import_macros", "list_macros", "load_macro", "get_triggers"]);
    expect(screen.getByText("Weekly report")).toBeInTheDocument();
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
    expect(r).toHaveLength(12);
    expect(screen.getByText("12 steps")).toBeInTheDocument();
    expect(r[0]).toHaveTextContent("00:00.85 CLICK Click · File menu 134, 70 px");
    expect(r[2]).toHaveTextContent("WAIT Wait 0.7 s Dialog opens");
    expect(r[4]).toHaveTextContent("KEYS");
    expect(r[4]).toHaveTextContent("Key combination");
    expect(r[5]).toHaveTextContent("TYPE");
    expect(r[5]).toHaveTextContent("characters");
    expect(r[7]).toHaveTextContent("IF Wait for pixel 1248, 680 = #9B9797 Save button turns grey · timeout 5 s, else stop");
  });

  test.each<[string, Partial<Step>, string, string]>([
    ["double click", { kind: "click", btn: "Left", count: 2, label: "" }, "Double click", "10, 20 px"],
    ["triple click", { kind: "click", btn: "Left", count: 3, label: "" }, "Triple click", ""],
    ["quadruple click", { kind: "click", btn: "Left", count: 4, label: "" }, "4× click", ""],
    ["right click", { kind: "click", btn: "Right", count: 1, label: "Menu" }, "Right click · Menu", ""],
    ["drag", { kind: "drag", btn: "Left", to_x: 300, to_y: 400, label: "" }, "Drag", "10, 20 px → 300, 400"],
    ["right drag", { kind: "drag", btn: "Right", to_x: 1, to_y: 2, label: "Box" }, "Right drag · Box", ""],
    ["scroll down", { kind: "scroll", delta: -240, horizontal: false }, "Scroll down", "2 notches at 10, 20 px"],
    ["scroll up", { kind: "scroll", delta: 120, horizontal: false }, "Scroll up", "1 notches"],
    ["scroll right", { kind: "scroll", delta: 120, horizontal: true }, "Scroll right", ""],
    ["scroll left", { kind: "scroll", delta: -120, horizontal: true }, "Scroll left", ""],
    ["keys", { kind: "keys", combo: ["Ctrl", "Shift", "S"] }, "Ctrl + Shift + S", "Key combination"],
    ["type", { kind: "type", text: "hi", chars: [] }, "“hi”", "2 characters"],
    ["one char", { kind: "type", text: "x", chars: [] }, "“x”", "1 character"],
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
    expect(stepRows()[0]).toHaveTextContent("+86, +34 in window");
  });

  test("long pauses are marked above their step", () => {
    render(StepsTab);
    expect(screen.getByText("1.4 s pause")).toBeInTheDocument();
    expect(screen.getAllByText(/s pause$/)).toHaveLength(1);
  });

  test("the step under the playhead is highlighted, later ones dimmed", async () => {
    render(StepsTab);
    relay.seek(2000);
    await settle();
    const r = stepRows();
    expect(r[2]).toHaveClass("active");
    expect(r[1]).not.toHaveClass("future");
    expect(r[3]).toHaveClass("future");
  });

  test("clicking a step moves the playhead there and opens its editor; again closes it", async () => {
    render(StepsTab);
    await userEvent.click(stepRows()[2]);
    expect(relay.cur).toBe(2000);
    expect(stepRows()[2]).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("group", { name: "Edit step" })).toBeInTheDocument();
    await userEvent.click(stepRows()[2]);
    expect(screen.queryByRole("group", { name: "Edit step" })).toBeNull();
  });

  test("Enter opens a step's editor too", async () => {
    render(StepsTab);
    stepRows()[3].focus();
    await userEvent.keyboard("{Enter}");
    expect(screen.getByRole("group", { name: "Edit step" })).toBeInTheDocument();
  });

  test("× deletes that step (and doesn't open it)", async () => {
    render(StepsTab);
    await userEvent.click(within(stepRows()[4]).getByRole("button", { name: "Delete step" }));
    await settle();
    expect(core.argsOf("edit_macro")).toEqual([{ id: A, op: { op: "delete_step", index: 4 } }]);
    expect(stepRows()).toHaveLength(11);
    expect(screen.queryByRole("group", { name: "Edit step" })).toBeNull();
  });

  test("+ Wait, + Pixel check and Trim pauses", async () => {
    render(StepsTab);
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
    expect(stepRows()).toHaveLength(14);
    expect(screen.getByRole("button", { name: "Trim pauses" })).toBeDisabled(); // nothing left to trim
  });

  test("editing is off while recording, and the live steps are shown", async () => {
    render(StepsTab);
    core.emit({ type: "session", mode: "recording", macro_id: null });
    await settle();
    for (const name of ["+ Wait", "+ Pixel check", "Trim pauses"]) expect(screen.getByRole("button", { name })).toBeDisabled();
    expect(screen.getByText("0 steps")).toBeInTheDocument();
    const step = core.view(A).steps[0];
    core.emit({ type: "rec_progress", elapsed_ms: 900, desktop: { x: 0, y: 0, w: 1920, h: 1080 }, moves: [], steps: [step] });
    await settle();
    expect(stepRows()).toHaveLength(1);
    expect(within(stepRows()[0]).getByRole("button", { name: "Delete step" })).toBeDisabled();
    expect(screen.queryByText(/s pause$/)).toBeNull();
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
    const editor = await open(9);
    const pause = within(editor).getByLabelText(/Pause before/);
    expect(pause).toHaveValue(1.4);
    await change(pause, "2.5");
    await change(pause, "-1"); // ignored
    await change(pause, ""); // cleared: ignored and put back
    await settle();
    expect(ops()).toEqual([{ op: "set_pause", index: 9, dur: 2500 }]);
    expect(pause).toHaveValue(2.5); // the saved pause
  });

  test("a click's label", async () => {
    const editor = await open(0);
    const label = within(editor).getByLabelText("Label");
    expect(label).toHaveValue("File menu");
    expect(label).toHaveAttribute("placeholder", "e.g. Save button");
    await change(label, "The File menu");
    await settle();
    expect(ops()).toEqual([{ op: "set_label", index: 0, label: "The File menu" }]);
  });

  test("a wait's duration and label", async () => {
    const editor = await open(2);
    const dur = within(editor).getByLabelText(/Duration/);
    expect(dur).toHaveValue(0.7);
    await change(dur, "1.25");
    await change(dur, "-3");
    await settle();
    await change(within(editor).getByLabelText("Label"), "Wait for dialog");
    await settle();
    expect(ops()).toEqual([
      { op: "set_wait_duration", index: 2, dur: 1250 },
      { op: "set_label", index: 2, label: "Wait for dialog" },
    ]);
  });

  test("keys and typing have no label", async () => {
    const editor = await open(4);
    expect(within(editor).queryByLabelText("Label")).toBeNull();
    expect(within(editor).getByLabelText(/Pause before/)).toBeInTheDocument();
  });

  describe("a pixel check", () => {
    const base = { index: 7, x: 1248, y: 680, color: "#9B9797", tolerance: 8, timeout_ms: 5000 };

    test("shows its position, color, tolerance and timeout", async () => {
      const e = await open(7);
      expect(within(e).getByLabelText("X")).toHaveValue(1248);
      expect(within(e).getByLabelText("Y")).toHaveValue(680);
      expect(within(e).getByLabelText(/Color/)).toHaveValue("#9B9797");
      expect(within(e).getByLabelText("Tolerance")).toHaveValue(8);
      expect(within(e).getByLabelText(/Timeout/)).toHaveValue(5);
    });

    test("each field sends the whole check", async () => {
      const e = await open(7);
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

    test("a color that isn't #RRGGBB is put back", async () => {
      const e = await open(7);
      const color = within(e).getByLabelText(/Color/);
      await change(color, "red");
      expect(color).toHaveValue("#9B9797");
      await change(color, "#12345");
      expect(color).toHaveValue("#9B9797");
      expect(ops()).toEqual([]);
    });

    test("tolerance is kept within 0–255, and non-numbers are ignored", async () => {
      const e = await open(7);
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
      const e = await open(7);
      vi.useFakeTimers();
      core.hold("pick_pixel");
      await fireEvent.click(within(e).getByRole("button", { name: "Pick" }));
      await settle();
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
