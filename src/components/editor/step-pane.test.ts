// The inspector's Step tab: every field of every kind of step, what several
// selected steps share, the step playing, and a step that timed out.
import { beforeEach, describe, expect, test, vi } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import StepPane from "./StepPane.svelte";
import { core, freshStore, png, settle } from "../../test/app";
import type { RelayStore } from "../../lib/state/relay.svelte";

let relay: RelayStore;
const A = "00000000-0000-0000-0000-000000000001";
beforeEach(async () => {
  relay = await freshStore();
  relay.place = "macro";
});

const ops = () => core.argsOf("edit_macro").map((a) => a.op);
const change = (el: HTMLElement, value: string) => fireEvent.change(el, { target: { value } });
const field = (name: string | RegExp) => screen.getByRole("textbox", { name });
const button = (name: string | RegExp) => screen.getByRole("button", { name });

/** Selects step `index` and shows the Step tab. */
async function open(index: number) {
  relay.select([index]);
  render(StepPane);
  await settle();
  core.clearCalls();
}

describe("the Step tab", () => {
  test("with nothing selected, says what to do", () => {
    render(StepPane);
    expect(screen.getByText("No step selected")).toBeInTheDocument();
  });

  test("says what the step is and where it is in the list", async () => {
    await open(10);
    expect(screen.getByText("Click · step 7")).toBeInTheDocument();
    expect(screen.getByText("Click · Save")).toBeInTheDocument();
    expect(screen.getByText("1248, 680 px")).toBeInTheDocument();
  });

  test("a click's label", async () => {
    await open(1);
    const label = field("Label");
    expect(label).toHaveValue("File menu");
    expect(label).toHaveAttribute("placeholder", "e.g. Save button");
    await change(label, "The File menu");
    await settle();
    expect(ops()).toEqual([{ op: "set_label", index: 1, label: "The File menu" }]);
  });

  test("the pause before a step goes before its move, if it has one", async () => {
    await open(13 + 1); // Right click · Invoices folder, after its move (13)
    const pause = field("Pause before in seconds");
    expect(pause).toHaveValue("0.34");
    await change(pause, "2.5");
    await settle();
    await change(pause, "-1"); // clamped to 0
    await settle();
    await change(pause, ""); // not a number: put back
    expect(pause).toHaveValue("0.00");
    expect(ops()).toEqual([
      { op: "set_pause", index: 13, dur: 2500 },
      { op: "set_pause", index: 13, dur: 0 },
    ]);
  });

  test("a wait's length and label", async () => {
    await open(4);
    const waits = field("Waits in seconds");
    expect(waits).toHaveValue("0.7");
    await change(waits, "1.25");
    await settle();
    await change(field("Label"), "Wait for dialog");
    await settle();
    expect(ops()).toEqual([
      { op: "set_wait_duration", index: 4, dur: 1250 },
      { op: "set_label", index: 4, label: "Wait for dialog" },
    ]);
  });

  test("keys and typing have no label", async () => {
    await open(7);
    expect(screen.queryByRole("textbox", { name: "Label" })).toBeNull();
    expect(screen.getByRole("img", { name: "Ctrl + A" })).toBeInTheDocument();
    expect(field("Pause before in seconds")).toBeInTheDocument();
  });

  describe("the move to a step", () => {
    test("how long it takes: shorter is faster", async () => {
      await open(3); // Click · Export as PDF; its move is step 2, 1116..1750
      const there = field("Moving there in seconds");
      expect(there).toHaveValue("0.63");
      await change(there, "0.3");
      await settle();
      expect(ops()).toEqual([{ op: "set_move_duration", index: 2, dur: 300 }]);
      expect(relay.steps[2]).toMatchObject({ t: 1116, end: 1416 });
    });

    test("Smooth and Straighten reshape it, and offer Undo", async () => {
      await open(3);
      await userEvent.click(button("Straighten"));
      await settle();
      expect(relay.toast).toMatchObject({ message: "Straightened the move", action: { label: "Undo" } });
      await userEvent.click(button("Smooth the path"));
      await settle();
      expect(ops()).toEqual([
        { op: "straighten_move", index: 2 },
        { op: "smooth_move", index: 2 },
      ]);
    });

    test("one sample is a jump: nothing to retime or reshape", async () => {
      await relay.loadMacro("00000000-0000-0000-0000-000000000004"); // Open standup tools
      await relay.updateSettings({ show_moves: true });
      await open(1);
      expect(screen.getByText("Move before step 2")).toBeInTheDocument(); // numbered as with moves folded: Keys is step 1
      expect(field("Takes in seconds")).toBeDisabled();
      expect(button("Smooth the path")).toBeDisabled();
      expect(button("Straighten")).toBeDisabled();
    });
  });

  describe("a pixel check", () => {
    const base = { index: 11, x: 1248, y: 680, color: "#9B9797", tolerance: 8, timeout_ms: 5000 };

    test("shows its position, color, tolerance and timeout", async () => {
      await open(11);
      expect(field("X")).toHaveValue("1248");
      expect(field("Y")).toHaveValue("680");
      expect(field("Color")).toHaveValue("#9B9797");
      expect(field("Tolerance")).toHaveValue("8");
      expect(field("Gives up after in seconds")).toHaveValue("5.0");
    });

    test("each field sends the whole check, rounded and clamped to what Rust stores", async () => {
      await open(11);
      await change(field("X"), "100.4");
      await settle();
      await change(field("Y"), "-50");
      await settle();
      await change(field("Color"), "#abcdef");
      await settle();
      await change(field("Tolerance"), "300");
      await settle();
      await change(field("Gives up after in seconds"), "0.2");
      await settle();
      expect(ops()).toEqual([
        { op: "update_pixel_wait", ...base, x: 100 },
        { op: "update_pixel_wait", ...base, x: 100, y: -50 },
        { op: "update_pixel_wait", ...base, x: 100, y: -50, color: "#ABCDEF" },
        { op: "update_pixel_wait", ...base, x: 100, y: -50, color: "#ABCDEF", tolerance: 255 },
        { op: "update_pixel_wait", ...base, x: 100, y: -50, color: "#ABCDEF", tolerance: 255, timeout_ms: 500 },
      ]);
      expect(field("Tolerance")).toHaveValue("255");
    });

    test("a color that isn't #RRGGBB is put back", async () => {
      await open(11);
      await change(field("Color"), "red");
      expect(field("Color")).toHaveValue("#9B9797");
      await change(field("Color"), "#12345");
      expect(field("Color")).toHaveValue("#9B9797");
      expect(ops()).toEqual([]);
    });

    test("Pick from screen opens the picker, then points the check at the pixel picked", async () => {
      await open(11);
      core.hold("pick_pixel");
      await fireEvent.click(button("Pick from screen"));
      await settle();
      expect(core.argsOf("pick_pixel")).toEqual([{}]);
      expect(button("Picking…")).toBeDisabled();
      core.held[0].resolve({ x: 5, y: 6, color: "#FF0000", frozen: false });
      await settle();
      expect(ops()).toEqual([{ op: "update_pixel_wait", ...base, x: 5, y: 6, color: "#FF0000" }]);
    });

    test("when it timed out in the last run, it says what happened", async () => {
      relay.failedStep = 11;
      await open(11);
      expect(screen.getByRole("alert")).toHaveTextContent(
        "Timed out in the last run: the pixel at 1248, 680 px wasn't #9B9797 within 5 s, so the macro stopped.",
      );
    });
  });

  describe("a Text step", () => {
    const text = () => screen.getByRole("textbox", { name: "Text" }) as HTMLTextAreaElement;
    async function withText(t: string) {
      await relay.edit({ op: "insert_text", at: 0, text: t });
      await open(2);
    }
    async function typeText(t: string) {
      await fireEvent.input(text(), { target: { value: t } });
      await fireEvent.change(text());
      await settle();
    }

    test("shows what it types now, and saves it when the field is left", async () => {
      await withText("No. {n}");
      expect(screen.getByRole("status")).toHaveTextContent("Types now: “No. 1”");
      core.clipboardText = "ACME";
      await typeText("{clipboard} on {date}");
      expect(ops()).toEqual([{ op: "update_text", index: 2, text: "{clipboard} on {date}" }]);
      expect(screen.getByRole("status")).toHaveTextContent("Types now: “ACME on 2026-10-01”");
    });

    test("a text with a mistake says what's wrong and isn't saved", async () => {
      await withText("No. {n}");
      await typeText("Hello {name}");
      expect(screen.getByRole("status")).toHaveClass("wrong");
      expect(screen.getByRole("status")).toHaveTextContent("{name} isn't a placeholder");
      expect(ops()).toEqual([]);
    });

    test("placeholder keys go where the caret is, and the data file's columns are some too", async () => {
      await withText("Invoice  done");
      text().setSelectionRange(8, 8);
      await userEvent.click(button("{date}"));
      await settle();
      expect(ops()).toEqual([{ op: "update_text", index: 2, text: "Invoice {date} done" }]);
      core.csv.set("C:\\customers.csv", { columns: ["Customer", "", "a{b}"], rows: [["ACME"]] });
      core.dialog.open = "C:\\customers.csv";
      await relay.chooseDataFile();
      await settle();
      expect(button("{col:Customer}")).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "{col:}" })).toBeNull();
    });

    test("how long it lasts, never shorter than its typing", async () => {
      await withText("No. {n}"); // 50 ms of typing
      const lasts = field("Lasts in seconds");
      await change(lasts, "2");
      await settle();
      expect(ops()).toEqual([{ op: "set_wait_duration", index: 2, dur: 2000 }]);
      await change(lasts, "0");
      await settle();
      expect(lasts).toHaveValue("0.1"); // what Rust kept, rounded to a tenth
    });

    test("has no label", async () => {
      await withText("x");
      expect(screen.queryByRole("textbox", { name: "Label" })).toBeNull();
    });
  });

  test("Make editable turns typing into a Text step, and stays on it", async () => {
    await open(8); // “invoice_0924”
    await userEvent.click(button("Make editable"));
    await settle();
    expect(ops()).toEqual([{ op: "make_editable", index: 8 }]);
    expect((screen.getByRole("textbox", { name: "Text" }) as HTMLTextAreaElement).value).toBe("invoice_0924");
  });

  describe("a Find image step", () => {
    const IMAGE = png(40, 20);
    const inserted = { image: IMAGE, click_x: 20, click_y: 10, btn: "Left", threshold: 85, timeout_ms: 5000, area: null };
    async function withImage() {
      core.snip = IMAGE;
      relay.seek(1000);
      await relay.insertFindImage();
      await settle();
      const index = relay.steps.findIndex((s) => s.kind === "find_image");
      await open(index);
      return index;
    }

    test("its image, match, timeout and button, each sending the whole step", async () => {
      const index = await withImage();
      expect(screen.getByRole("img", { name: "What to find" })).toHaveAttribute("src", `data:image/png;base64,${IMAGE}`);
      await change(field("Match in percent"), "120");
      await settle();
      await change(field("Gives up after in seconds"), "0.2");
      await settle();
      await userEvent.click(screen.getByRole("radio", { name: "Right" }));
      await settle();
      const base = { op: "update_find_image", index, ...inserted };
      expect(ops()).toEqual([
        { ...base, threshold: 100 },
        { ...base, threshold: 100, timeout_ms: 500 },
        { ...base, threshold: 100, timeout_ms: 500, btn: "Right" },
      ]);
    });

    test("clicking the image sets where to click it; a key press doesn't", async () => {
      const index = await withImage();
      const shot = button(/What to find/);
      shot.getBoundingClientRect = () => ({ left: 10, top: 20, width: 80, height: 40, right: 90, bottom: 60, x: 10, y: 20, toJSON: () => null });
      await fireEvent.click(shot, { clientX: 70, clientY: 50, detail: 1 });
      await settle();
      await fireEvent.click(shot, { detail: 0 });
      await settle();
      expect(ops()).toEqual([{ op: "update_find_image", index, ...inserted, click_x: 30, click_y: 15 }]);
    });

    test("Paste replaces the image; a snip can be cancelled", async () => {
      const index = await withImage();
      core.clipboard = png(10, 6);
      await userEvent.click(button("Paste"));
      await settle();
      expect(ops()).toEqual([{ op: "update_find_image", index, ...inserted, image: png(10, 6), click_x: 5, click_y: 3 }]);
      core.hold("snip_image");
      await userEvent.click(button("Snip"));
      await settle();
      expect(screen.getByRole("status")).toHaveTextContent("Snip the image…");
      await userEvent.click(button("Cancel"));
      await settle();
      expect(core.commands().at(-1)).toBe("cancel_snip");
    });

    test("Test says where it found the image, and Show marks it", async () => {
      await withImage();
      core.found = { x: 300, y: -200, w: 60, h: 30, score: 92 };
      await userEvent.click(button("Test"));
      await settle();
      expect(screen.getByRole("status")).toHaveTextContent("Found at 300, -200 (92%)");
      await userEvent.click(button("Show"));
      await settle();
      expect(core.argsOf("show_match")).toEqual([{ area: { x: 300, y: -200, w: 60, h: 30 }, dotX: 330, dotY: -185 }]);
    });

    test("with several screens, where to look", async () => {
      const second = { name: "\\\\.\\DISPLAY2", rect: { x: 1920, y: 0, w: 2560, h: 1440 }, work: { x: 1920, y: 0, w: 2560, h: 1400 }, dpi: 144, primary: false };
      core.entries[0].view.recording.monitors.push(second);
      await relay.loadMacro(A);
      const index = await withImage();
      await userEvent.click(screen.getByRole("radio", { name: "Screen 2" }));
      await settle();
      expect(ops()).toEqual([{ op: "update_find_image", index, ...inserted, area: second.rect }]);
    });

    test("with one screen there's nothing to choose", async () => {
      await withImage();
      expect(screen.queryByRole("radiogroup", { name: "Looks on" })).toBeNull();
    });
  });

  describe("several steps", () => {
    beforeEach(async () => {
      relay.select([4, 7, 8], 8); // a wait, keys and typing
      render(StepPane);
      await settle();
      core.clearCalls();
    });

    test("say how many, and set their pause in one edit", async () => {
      expect(screen.getByText("3 steps selected")).toBeInTheDocument();
      await change(field("Pause before in seconds"), "0.5");
      await settle();
      expect(ops()).toEqual([{ op: "batch", ops: [4, 7, 8].map((index) => ({ op: "set_pause", index, dur: 500 })) }]);
    });

    test("a label for those that take one", async () => {
      await change(field("Label"), "Done");
      await settle();
      expect(ops()).toEqual([{ op: "set_label", index: 4, label: "Done" }]);
    });

    test("Make editable for the typed texts, and Delete for them all", async () => {
      await userEvent.click(button("Make editable"));
      await settle();
      expect(ops()).toEqual([{ op: "make_editable", index: 8 }]);
      await userEvent.click(button("Delete 3 steps"));
      await settle();
      expect(ops().at(-1)).toEqual({ op: "batch", ops: [8, 7, 4].map((index) => ({ op: "delete_step", index })) });
    });
  });

  test("Delete step deletes it with its move", async () => {
    await open(3);
    await userEvent.click(button("Delete step"));
    await settle();
    expect(ops()).toEqual([{ op: "batch", ops: [3, 2].map((index) => ({ op: "delete_step", index })) }]);
  });

  test("while playing, the step playing; fields are off, and it says why", async () => {
    await open(1);
    relay.seek(relay.steps[4].t + 10);
    core.emit({ type: "session", mode: "playing", macro_id: A, source: null });
    await settle();
    expect(screen.getByText("Now · step 3 of 12")).toBeInTheDocument();
    expect(screen.getByText(/Stop playback to edit steps/)).toBeInTheDocument();
    core.emit({ type: "session", mode: "idle", macro_id: A, source: null });
    await settle();
    expect(within(document.body).queryByText(/Stop playback/)).toBeNull();
  });
});
