// The macro editor: its top bar and its steps list, and what each sends.
import { beforeEach, describe, expect, test, vi } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import EditorBar from "./EditorBar.svelte";
import StepList from "./StepList.svelte";
import { core, freshStore, png, settle } from "../../test/app";
import { browserBackend } from "../../lib/ipc/backend";
import type { RelayStore } from "../../lib/state/relay.svelte";

let relay: RelayStore;
const A = "00000000-0000-0000-0000-000000000001";
beforeEach(async () => {
  relay = await freshStore();
  relay.place = "macro";
});

const button = (name: string) => screen.getByRole("button", { name });

describe("the top bar", () => {
  test("Back goes to the Library, except during a session", async () => {
    render(EditorBar);
    await userEvent.click(button("Back to the Library"));
    expect(relay.place).toBe("library");
    core.emit({ type: "session", mode: "playing", macro_id: A, source: null });
    await settle();
    expect(button("Back to the Library")).toBeDisabled();
  });

  test("the name is edited in place", async () => {
    vi.useFakeTimers();
    render(EditorBar);
    await fireEvent.input(screen.getByRole("textbox", { name: "Macro name" }), { target: { value: "Invoice" } });
    await vi.advanceTimersByTimeAsync(300);
    await settle();
    expect(core.argsOf("edit_macro")).toEqual([{ id: A, op: { op: "rename", name: "Invoice" } }]);
  });

  test("Saved shows for a moment after a change", async () => {
    vi.useFakeTimers();
    render(EditorBar);
    expect(screen.getByRole("status")).toHaveTextContent("");
    await relay.edit({ op: "set_label", index: 1, label: "Menu" });
    await settle();
    expect(screen.getByRole("status")).toHaveTextContent("Saved");
    await vi.advanceTimersByTimeAsync(2000);
    await settle();
    expect(screen.getByRole("status")).toHaveTextContent("");
  });

  test("a change that couldn't be saved says so until its message is dismissed", async () => {
    render(EditorBar);
    core.saveError = "disk full";
    await relay.edit({ op: "set_label", index: 1, label: "Menu" });
    await settle();
    expect(screen.getByRole("status")).toHaveTextContent("Couldn't save");
    expect(relay.toast).toMatchObject({ kind: "error", message: "Couldn't save the change: disk full. It's kept until you quit." });
    relay.dismissToast(relay.toast!.id);
    await settle();
    expect(screen.getByRole("status")).not.toHaveTextContent("Couldn't save");
  });

  test("Undo and Redo", async () => {
    await relay.edit({ op: "delete_step", index: 0 });
    render(EditorBar);
    expect(button("Redo")).toBeDisabled();
    await userEvent.click(button("Undo"));
    await settle();
    await userEvent.click(button("Redo"));
    await settle();
    expect(core.argsOf("undo_edit")).toEqual([
      { id: A, redo: false },
      { id: A, redo: true },
    ]);
  });

  test("Record records a new macro, and is off while playing", async () => {
    render(EditorBar);
    await userEvent.click(button("Record a new macro"));
    expect(core.commands()).toEqual(["toggle_record"]);
    core.emit({ type: "session", mode: "recording", macro_id: null, source: null });
    await settle();
    expect(button("Stop recording")).toBeInTheDocument();
    core.emit({ type: "session", mode: "playing", macro_id: A, source: null });
    await settle();
    expect(screen.getByRole("button", { name: /record/i })).toBeDisabled();
  });

  test("Play plays from the playhead; while playing, Pause and Stop", async () => {
    relay.seek(1500);
    render(EditorBar);
    await userEvent.click(button("Play"));
    expect(core.lastArgs("toggle_play")).toEqual({ from: 1500 });
    core.emit({ type: "session", mode: "playing", macro_id: A, source: null });
    await settle();
    expect(button("Pause")).toHaveAttribute("title", "Pause (F10)");
    await userEvent.click(button("Stop"));
    expect(core.commands()).toContain("stop_session");
  });

  test("the play summary says how it will play, and opens the play options", async () => {
    relay.inspector = "step";
    relay.macroSection = "triggers";
    render(EditorBar);
    const summary = button("Repeats 3 times, normal speed, timing varies by up to 40 ms, any key stops it. Open the play options.");
    expect(summary).toHaveTextContent("×3·1×·±40 ms");
    await userEvent.click(summary);
    expect([relay.inspector, relay.macroSection]).toEqual(["macro", "play"]);
  });

  test("More: Export, Duplicate and Delete", async () => {
    render(EditorBar);
    await userEvent.click(button("More"));
    const menu = within(screen.getByRole("menu", { name: "More" }));
    expect(menu.getAllByRole("menuitem").map((m) => m.textContent?.trim())).toEqual(["Export…", "Duplicate", "Runs", "Delete"]);
    await userEvent.click(menu.getByRole("menuitem", { name: "Export…" }));
    expect(relay.exportOpen).toBe(true);

    await userEvent.click(button("More"));
    await userEvent.click(screen.getByRole("menuitem", { name: "Duplicate" }));
    await settle();
    expect(core.lastArgs("duplicate_macro")).toEqual({ id: A });
    expect(relay.name).toBe("Export invoice to PDF (copy)");
    expect(relay.place).toBe("macro");

    const copy = relay.view!.id;
    await userEvent.click(button("More"));
    await userEvent.click(screen.getByRole("menuitem", { name: "Delete" }));
    await settle();
    expect(core.lastArgs("delete_macro")).toEqual({ id: copy });
    expect(relay.place).toBe("library");
  });

  test("More → Runs shows this macro's runs in the Library", async () => {
    render(EditorBar);
    await userEvent.click(button("More"));
    await userEvent.click(screen.getByRole("menuitem", { name: "Runs" }));
    await settle();
    expect([relay.place, relay.libraryShows, relay.runsFilter]).toEqual(["library", "runs", A]);
  });

  test("a new recording opens with its name selected, ready to type", async () => {
    render(EditorBar);
    core.emit({ type: "saved", id: A });
    await settle();
    const name = screen.getByRole("textbox", { name: "Macro name" }) as HTMLInputElement;
    expect(document.activeElement).toBe(name);
    expect([name.selectionStart, name.selectionEnd]).toEqual([0, name.value.length]);
    expect(relay.naming).toBeNull();
  });

  test("the player key switches to the player", async () => {
    render(EditorBar);
    await userEvent.click(button("Switch to the player"));
    await settle();
    expect(core.lastArgs("set_player_mode")).toEqual({ on: true });
    expect(relay.playerMode).toBe(true);
  });

  test("while a trigger plays another macro, this one's Play is off and it isn't drawn as playing", async () => {
    render(EditorBar);
    core.emit({ type: "session", mode: "playing", macro_id: "00000000-0000-0000-0000-000000000003", source: "schedule" });
    await settle();
    expect(button("Play")).toBeDisabled();
    expect(screen.queryByRole("button", { name: "Stop" })).toBeNull();
    expect(screen.queryByRole("button", { name: "Pause" })).toBeNull();
  });
});

describe("the steps list", () => {
  const list = () => screen.getByRole("listbox", { name: /Steps/ });
  const options = () => within(list()).getAllByRole("option");
  const selectedNames = () => options().filter((o) => o.getAttribute("aria-selected") === "true").map((o) => o.getAttribute("aria-label"));

  test("one row per step, moves folded into the step they lead to", () => {
    render(StepList);
    expect(options()).toHaveLength(12);
    expect(options()[0]).toHaveAccessibleName("Click, File menu");
    expect(options()[0]).toHaveTextContent("00:00.85");
    expect(options()[0]).toHaveTextContent("0.85 s");
    expect(screen.getByText("12", { selector: ".num" })).toBeInTheDocument();
  });

  test("a click selects a row and moves the playhead to it; Ctrl and Shift select several", async () => {
    render(StepList);
    await userEvent.click(options()[1]);
    expect(relay.selection).toEqual([3]);
    expect(relay.cur).toBe(relay.steps[3].t);
    expect(list()).toHaveAttribute("aria-activedescendant", "step-3");
    await fireEvent.click(options()[3], { ctrlKey: true });
    expect(relay.selection).toEqual([3, 6]);
    await fireEvent.click(options()[5], { shiftKey: true });
    // From the row clicked last to this one.
    expect(selectedNames()).toEqual(["Double click, Filename field", "Keys, Ctrl + A", "Type, “invoice_0924”"]);
  });

  test("the arrow keys move the selection, Shift extends it, Home, End and Ctrl + A", async () => {
    render(StepList);
    list().focus();
    await userEvent.keyboard("{ArrowDown}");
    expect(relay.selection).toEqual([1]);
    await userEvent.keyboard("{ArrowDown}{ArrowRight}");
    expect(relay.selection).toEqual([4]); // the wait, two rows down
    await userEvent.keyboard("{Shift>}{ArrowUp}{/Shift}");
    expect(relay.selection).toEqual([3, 4]);
    await userEvent.keyboard("{End}");
    expect(relay.selection).toEqual([17]);
    await userEvent.keyboard("{Home}");
    expect(relay.selection).toEqual([1]);
    await userEvent.keyboard("{Control>}a{/Control}");
    expect(relay.selection).toHaveLength(12);
  });

  test("Del deletes the selected rows and their moves, in one edit", async () => {
    render(StepList);
    relay.select([1, 6]);
    list().focus();
    await userEvent.keyboard("{Delete}");
    await settle();
    expect(core.lastArgs("edit_macro")).toEqual({
      id: A,
      op: { op: "batch", ops: [6, 5, 1, 0].map((index) => ({ op: "delete_step", index })) },
    });
    expect(relay.toast?.message).toBe("Deleted 2 steps");
  });

  test("the insertion line is after the step under the playhead; Insert adds a step there", async () => {
    relay.seek(relay.steps[4].t + 100); // the wait
    render(StepList);
    const line = screen.getByText("+ insert here");
    expect(line.previousElementSibling).toHaveAccessibleName("Wait, 0.70 s · Dialog opens");
    list().focus();
    await userEvent.keyboard("{Insert}");
    const menu = within(screen.getByRole("menu", { name: "Add a step" }));
    expect(menu.getAllByRole("menuitem").map((m) => m.textContent?.trim())).toEqual(["Wait", "Text", "Wait for pixel", "Find image"]);
    await userEvent.click(menu.getByRole("menuitem", { name: "Wait" }));
    await settle();
    expect(core.lastArgs("edit_macro")?.op).toEqual({ op: "insert_wait", at: relay.steps[4].t + 100, dur: 500, label: "Inserted" });
  });

  test("the + between two rows inserts right there", async () => {
    render(StepList);
    const plus = screen.getAllByRole("button", { name: "Insert after this step" });
    // The first row has the insertion line under it (the playhead is on it), so the first + is under the second.
    await userEvent.click(plus[1]); // under the third row, the wait
    expect(relay.cur).toBe(relay.steps[4].t);
    await userEvent.click(screen.getByRole("menuitem", { name: "Text" }));
    await settle();
    expect(core.lastArgs("edit_macro")?.op).toEqual({ op: "insert_text", at: relay.steps[4].t, text: "" });
  });

  test("Find a step filters the rows, and Ctrl + F goes there", async () => {
    render(StepList);
    await userEvent.keyboard("{Control>}f{/Control}");
    const find = screen.getByRole("searchbox", { name: "Find a step" });
    expect(find).toHaveFocus();
    await userEvent.type(find, "keys");
    expect(options().map((o) => o.getAttribute("aria-label"))).toEqual(["Keys, Ctrl + A", "Keys, Ctrl + W", "Keys, Enter"]);
    expect(screen.queryByText("+ insert here")).toBeNull();
    await userEvent.clear(find);
    await userEvent.type(find, "zebra");
    expect(screen.getByText("No steps match “zebra”")).toBeInTheDocument();
  });

  test("Show moves unfolds them, and is remembered", async () => {
    render(StepList);
    await userEvent.click(button("Steps options"));
    await userEvent.click(screen.getByRole("menuitemcheckbox", { name: "Show moves" }));
    await settle();
    expect(core.lastArgs("update_settings")?.settings).toMatchObject({ show_moves: true });
    expect(options()).toHaveLength(18);
    expect(options()[0].getAttribute("aria-label")).toMatch(/^Move, /);
  });

  test("Shorten pauses is in the list's menu", async () => {
    await relay.setPause(4, 2500);
    render(StepList);
    await userEvent.click(button("Steps options"));
    const shorten = screen.getByRole("menuitem", { name: "Shorten pauses" });
    expect(shorten).toBeEnabled();
    await userEvent.click(shorten);
    await settle();
    expect(core.lastArgs("edit_macro")?.op).toEqual({ op: "cap_pauses", max: 1000 });
  });

  test("the step a run timed out on says so", async () => {
    render(StepList);
    relay.failedStep = 11;
    await settle();
    const row = options()[7];
    expect(row).toHaveClass("failed");
    expect(row).toHaveTextContent("Timed out");
    expect(row).toHaveAccessibleName("Wait for, pixel 1248, 680 · Save button turns grey, timed out in the last run");
  });

  test("Enter goes to the inspector", async () => {
    const onenter = vi.fn();
    render(StepList, { onenter });
    list().focus();
    await userEvent.keyboard("{Enter}");
    expect(onenter).not.toHaveBeenCalled(); // nothing selected yet
    await userEvent.keyboard("{ArrowDown}{Enter}");
    expect(onenter).toHaveBeenCalledOnce();
  });

  test("long pauses are marked above their step, until they're shortened to 1 s", async () => {
    render(StepList);
    // The sample's pauses are short: the cursor moves between its steps.
    expect(screen.queryByText(/s pause$/)).toBeNull();
    await relay.setPause(4, 1400);
    await settle();
    expect(screen.getByText("1.4 s pause")).toBeInTheDocument();
    await relay.trimPauses();
    await settle();
    expect(screen.queryByText(/s pause$/)).toBeNull();
  });

  test("while recording, the live steps, and nothing can be added", async () => {
    render(StepList);
    core.emit({ type: "session", mode: "recording", macro_id: null, source: null });
    await settle();
    expect(button("Add")).toBeDisabled();
    expect(screen.getByText("No steps yet")).toBeInTheDocument();
    const step = core.view(A).steps[1];
    core.emit({ type: "rec_progress", elapsed_ms: 900, desktop: { x: 0, y: 0, w: 1920, h: 1080 }, moves: [], steps: [step] });
    await settle();
    expect(options()).toHaveLength(1);
  });

  test.each(["paused", "countdown"] as const)("nothing can be added while %s", async (mode) => {
    render(StepList);
    core.emit({ type: "session", mode, macro_id: A, source: null });
    await settle();
    expect(button("Add")).toBeDisabled();
  });

  test("the browser preview shows the steps read-only", async () => {
    core.uninstall();
    try {
      await freshStore({ backend: browserBackend() });
      render(StepList);
      expect(options()).toHaveLength(12);
      expect(button("Add")).toBeDisabled();
    } finally {
      core.install();
    }
  });

  test("while a session runs, nothing can be added", async () => {
    render(StepList);
    core.emit({ type: "session", mode: "playing", macro_id: A, source: null });
    await settle();
    expect(button("Add")).toBeDisabled();
    expect(screen.queryByText("+ insert here")).toBeNull();
    expect(screen.queryAllByRole("button", { name: "Insert after this step" })).toEqual([]);
  });
});

describe("adding steps", () => {
  const IMAGE = png(40, 20);
  const inserted = { image: IMAGE, click_x: 20, click_y: 10, btn: "Left", threshold: 85, timeout_ms: 5000, area: null };
  const ops = () => core.argsOf("edit_macro").map((a) => a.op);
  async function add(name: string) {
    await userEvent.click(button("Add"));
    await userEvent.click(within(screen.getByRole("menu", { name: "Add a step" })).getByRole("menuitem", { name }));
    await settle();
  }

  test("Text inserts an empty Text step at the playhead, and selects it", async () => {
    render(StepList);
    relay.seek(1000);
    await add("Text");
    expect(ops()).toEqual([{ op: "insert_text", at: 1000, text: "" }]);
    expect(relay.selection).toEqual([2]);
    expect(screen.getByRole("option", { name: "Text, “”" })).toHaveAttribute("aria-selected", "true");
  });

  test("Find image snips, then inserts a step at the playhead that clicks the image's middle", async () => {
    render(StepList);
    core.snip = IMAGE;
    relay.seek(1000);
    await add("Find image");
    expect(core.commands()).toContain("snip_image");
    expect(ops()).toEqual([{ op: "insert_find_image", at: 1000, dur: 800, ...inserted, label: "" }]);
    expect(screen.getByRole("option", { name: "Find, image" })).toHaveTextContent("≤ 5 s");
  });

  test("a cancelled snip inserts nothing", async () => {
    render(StepList);
    await add("Find image");
    expect(core.commands()).toContain("snip_image");
    expect(ops()).toEqual([]);
  });

  test("while the snip waits, Paste or File… instead, or Cancel", async () => {
    render(StepList);
    core.hold("snip_image");
    await add("Find image");
    const status = screen.getByRole("status");
    expect(status).toHaveTextContent("Snip the image to find, or");
    expect(button("Add")).toBeDisabled();
    core.clipboard = png(30, 30);
    await userEvent.click(within(status).getByRole("button", { name: "Paste" }));
    await settle();
    const images = ["snip_image", "cancel_snip", "paste_image", "edit_macro"];
    expect(core.commands().filter((c) => images.includes(c))).toEqual(images);
    expect(ops()).toEqual([{ op: "insert_find_image", at: 0, dur: 800, ...inserted, image: png(30, 30), click_x: 15, click_y: 15, label: "" }]);
    // The cancelled snip lands later: nothing more is inserted.
    core.held[0].resolve(IMAGE);
    await settle();
    expect(ops()).toHaveLength(1);

    core.release("snip_image");
    core.hold("snip_image");
    await add("Find image");
    await userEvent.click(within(screen.getByRole("status")).getByRole("button", { name: "Cancel" }));
    await settle();
    expect(core.commands().at(-1)).toBe("cancel_snip");
    expect(screen.queryByRole("status")).toBeNull();
  });

  test("File… reads the chosen image", async () => {
    render(StepList);
    core.hold("snip_image");
    await add("Find image");
    core.dialog.open = "C:\shots\ok.png";
    core.images.set("C:\shots\ok.png", IMAGE);
    await userEvent.click(within(screen.getByRole("status")).getByRole("button", { name: "File…" }));
    await settle();
    expect(core.lastArgs("load_image")).toEqual({ path: "C:\shots\ok.png" });
    expect(ops()).toEqual([{ op: "insert_find_image", at: 0, dur: 800, ...inserted, label: "" }]);
  });

  test("an image that can't be used says why, and inserts nothing", async () => {
    render(StepList);
    core.hold("snip_image");
    await add("Find image");
    await userEvent.click(within(screen.getByRole("status")).getByRole("button", { name: "Paste" }));
    await settle();
    expect(relay.toast).toMatchObject({ kind: "error", message: "There's no picture on the clipboard. Copy or snip one first." });
    expect(ops()).toEqual([]);
  });

  test("another macro opened during the snip gets nothing", async () => {
    render(StepList);
    core.hold("snip_image");
    await add("Find image");
    await relay.loadMacro("00000000-0000-0000-0000-000000000002");
    await settle();
    core.held[0].resolve(IMAGE);
    await settle();
    expect(ops()).toEqual([]);
  });

  test("Wait for pixel waits for the color at where the cursor was, there", async () => {
    render(StepList);
    relay.seek(1000);
    const { x, y } = relay.cursorAt(1000);
    await add("Wait for pixel");
    expect(core.lastArgs("sample_pixel")).toEqual({ x: Math.round(x), y: Math.round(y) });
    expect(ops()).toEqual([expect.objectContaining({ op: "insert_pixel_wait", at: 1000, x: Math.round(x), y: Math.round(y), tolerance: 8, timeout_ms: 5000 })]);
  });
});
