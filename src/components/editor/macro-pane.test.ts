// The inspector's Macro tab: its two sections, every play option, and the
// triggers' cards, with what each control saves.
import { beforeEach, describe, expect, test, vi } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import MacroPane from "./MacroPane.svelte";
import { core, freshStore, png, settle } from "../../test/app";
import { defaultTriggers } from "../../test/fake-core";
import { browserBackend } from "../../lib/ipc/backend";
import type { RelayStore } from "../../lib/state/relay.svelte";
import type { MacroTriggers } from "../../lib/types";

let relay: RelayStore;
const [A, B] = [1, 2].map((n) => `00000000-0000-0000-0000-00000000000${n}`);
beforeEach(async () => {
  relay = await freshStore();
  relay.place = "macro";
  relay.inspector = "macro";
});

const change = (el: HTMLElement, value: string) => fireEvent.change(el, { target: { value } });
const field = (name: string | RegExp) => screen.getByRole("textbox", { name });
const button = (name: string | RegExp) => screen.getByRole("button", { name });
const toggle = (name: string) => screen.getByRole("switch", { name });

describe("the sections", () => {
  test("Play options and Runs automatically, one open at a time, each summed up", async () => {
    render(MacroPane);
    const play = button(/Play options/);
    const runs = button(/Runs automatically/);
    expect(play).toHaveAttribute("aria-expanded", "true");
    expect(play).toHaveTextContent("×3 · 1× · ±40 ms");
    expect(runs).toHaveAttribute("aria-expanded", "false");
    expect(runs).toHaveTextContent("Only when you play it");
    await userEvent.click(runs);
    expect(relay.macroSection).toBe("triggers");
    expect(button(/Play options/)).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByRole("textbox", { name: "Repeat in times" })).toBeNull();
  });
});

describe("play options", () => {
  const options = () => core.lastArgs("set_playback_options")?.options as Record<string, unknown>;

  test("Repeat is a number to type; Forever loops, and comes back to the count", async () => {
    render(MacroPane);
    const repeat = field("Repeat in times");
    expect(repeat).toHaveValue("3");
    await change(repeat, "12.4");
    await settle();
    expect(options().repeat).toEqual({ count: 12 });
    await userEvent.click(button("Forever"));
    await settle();
    expect(options().repeat).toBe("forever");
    expect(button("Forever")).toHaveAttribute("aria-pressed", "true");
    expect(field("Repeat in times")).toBeDisabled();
    await userEvent.click(button("Forever"));
    await settle();
    expect(options().repeat).toEqual({ count: 12 });
  });

  test("Speed", async () => {
    render(MacroPane);
    const speed = within(screen.getByRole("radiogroup", { name: "Speed" }));
    expect(speed.getAllByRole("radio").map((r) => r.textContent)).toEqual(["0.5×", "1×", "2×", "4×"]);
    await userEvent.click(speed.getByRole("radio", { name: "2×" }));
    await settle();
    expect(options().speed).toBe(2);
  });

  test("Vary timing and by how much", async () => {
    render(MacroPane);
    const by = field("By up to in milliseconds");
    expect(by).toHaveValue("40");
    await change(by, "120");
    await settle();
    expect(options().jitter_ms).toBe(120);
    await userEvent.click(toggle("Vary timing"));
    await settle();
    expect(options().humanize).toBe(false);
    expect(field("By up to in milliseconds")).toBeDisabled();
  });

  test("Stop on key, and what it means", async () => {
    render(MacroPane);
    expect(screen.getByText("Any key stops playback")).toBeInTheDocument();
    await userEvent.click(toggle("Stop on key"));
    await settle();
    expect(options().stop_on_key).toBe(false);
    expect(screen.getByText("Keys don't stop it: Esc and Stop still do")).toBeInTheDocument();
  });

  test("Clicks land on the screen or in the window", async () => {
    render(MacroPane);
    await userEvent.click(screen.getByRole("radio", { name: "Window" }));
    await settle();
    expect(core.lastArgs("set_playback_options")).toMatchObject({ id: A, options: { coord_mode: "window" } });
  });

  test("they can change while it plays", async () => {
    render(MacroPane);
    core.emit({ type: "session", mode: "playing", macro_id: A, source: null });
    await settle();
    await userEvent.click(screen.getByRole("radio", { name: "4×" }));
    await settle();
    expect(options().speed).toBe(4);
  });

  describe("the data file", () => {
    const CSV = "C:\\Data\\customers.csv";
    beforeEach(() => {
      core.csv.set(CSV, { columns: ["Customer", "Total"], rows: [["ACME", "12"], ["Globex", "7"]] });
    });

    test("Choose… links a CSV: it then plays once per row", async () => {
      render(MacroPane);
      await settle();
      expect(screen.getByText(/A CSV file: the macro plays once per row/)).toBeInTheDocument();
      core.dialog.open = CSV;
      await userEvent.click(button("Choose…"));
      await settle();
      expect(core.lastArgs("set_data_file")).toEqual({ id: A, path: CSV });
      expect(screen.getByText("customers.csv · 2 rows")).toHaveAttribute("title", CSV);
      expect(screen.getByText("Once per row of the data file (2 rows)")).toBeInTheDocument();
      expect(screen.queryByRole("textbox", { name: "Repeat in times" })).toBeNull();
      expect(button(/Play options/)).toHaveTextContent("Each row (2) · 1× · ±40 ms");
    });

    test("Remove unlinks it; a cancelled dialog changes nothing", async () => {
      render(MacroPane);
      await settle();
      core.dialog.open = null;
      await userEvent.click(button("Choose…"));
      await settle();
      expect(core.argsOf("set_data_file")).toEqual([]);
      core.dialog.open = CSV;
      await relay.chooseDataFile();
      await settle();
      await userEvent.click(button("Remove"));
      await settle();
      expect(core.lastArgs("set_data_file")).toEqual({ id: A, path: null });
      expect(button("Choose…")).toBeInTheDocument();
    });

    test("what's wrong with it shows, read again when the tab opens", async () => {
      core.dialog.open = CSV;
      await relay.chooseDataFile();
      await relay.edit({ op: "insert_text", at: 0, text: "{col:Due}" });
      core.clearCalls();
      render(MacroPane);
      await settle();
      expect(core.argsOf("get_data_file")).toEqual([{ id: A }]);
      expect(screen.getByRole("alert")).toHaveTextContent("customers.csv has no column “Due”.");
    });
  });
});

describe("runs automatically", () => {
  const sent = () => core.lastArgs("set_triggers")?.triggers as MacroTriggers;
  async function openTriggers() {
    relay.macroSection = "triggers";
    render(MacroPane);
    await settle();
  }
  async function add(name: string) {
    await userEvent.click(button("Add"));
    await userEvent.click(within(screen.getByRole("menu", { name: "Add a trigger" })).getByRole("menuitem", { name }));
    await settle();
  }
  const card = (name: string) => within(screen.getByRole("group", { name }));

  test("only the triggers the macro has, and + Add offers the others", async () => {
    await openTriggers();
    expect(screen.getAllByRole("group").map((g) => g.getAttribute("aria-label"))).toEqual(["Hotkey"]);
    await userEvent.click(button("Add"));
    const menu = within(screen.getByRole("menu", { name: "Add a trigger" }));
    expect(menu.getAllByRole("menuitem").map((m) => m.textContent?.trim())).toEqual([
      "Schedule",
      "When an app starts",
      "When a pixel changes",
      "When an image appears",
    ]);
  });

  test("the running programs are the app trigger's suggestions", async () => {
    const { container } = render(MacroPane);
    await settle();
    expect(core.commands()).toContain("list_processes");
    relay.macroSection = "triggers";
    await settle();
    await add("When an app starts");
    expect([...container.querySelectorAll("#relay-processes option")].map((o) => o.getAttribute("value"))).toEqual(["chrome.exe", "EXCEL.EXE", "notepad.exe"]);
  });

  test("triggers that couldn't be loaded can be retried", async () => {
    core.fail("get_triggers", "library.json is locked");
    await relay.loadMacro(B);
    await openTriggers();
    expect(screen.getByRole("alert")).toHaveTextContent("Couldn't load the triggers");
    core.succeed("get_triggers");
    await userEvent.click(button("Retry"));
    await settle();
    expect(core.lastArgs("get_triggers")).toEqual({ id: B });
    expect(toggle("Hotkey on")).toBeInTheDocument();
  });

  describe("the hotkey", () => {
    test("its keys, a note, and its switch (the samples' are off)", async () => {
      await openTriggers();
      expect(card("Hotkey").getByText("Ctrl + Alt + 1", { selector: ".sentence" })).toBeInTheDocument();
      expect(card("Hotkey").getByText("Runs from any app")).toBeInTheDocument();
      expect(toggle("Hotkey on")).toHaveAttribute("aria-checked", "false");
      await userEvent.click(toggle("Hotkey on"));
      await settle();
      expect(sent().hotkey).toEqual({ enabled: true, combo: "Ctrl + Alt + 1" });
    });

    test("new keys turn it on; Backspace clears them", async () => {
      await openTriggers();
      await userEvent.click(card("Hotkey").getByRole("button", { name: /Ctrl \+ Alt \+ 1/ }));
      await fireEvent.keyDown(window, { key: "7", code: "Digit7", ctrlKey: true, altKey: true });
      await settle();
      expect(core.argsOf("set_triggers")).toEqual([{ id: A, triggers: { ...defaultTriggers(), hotkey: { enabled: true, combo: "Ctrl + Alt + 7" } } }]);
      await userEvent.click(card("Hotkey").getByRole("button", { name: /Ctrl \+ Alt \+ 7/ }));
      await fireEvent.keyDown(window, { key: "Backspace", code: "Backspace" });
      await settle();
      expect(sent().hotkey).toEqual({ enabled: false, combo: "" });
    });

    test("keys that are refused are put back, and explained", async () => {
      await openTriggers();
      core.fail("set_triggers", "F9 is one of Relay's own hotkeys", "hotkey");
      await userEvent.click(card("Hotkey").getByRole("button", { name: /Ctrl \+ Alt \+ 1/ }));
      await fireEvent.keyDown(window, { key: "F9", code: "F9" });
      await settle();
      expect(card("Hotkey").getByRole("button", { name: /Ctrl \+ Alt \+ 1/ })).toBeInTheDocument();
      expect(relay.error).toBe("F9 is one of Relay's own hotkeys");
    });

    test("keys another app owns are explained", async () => {
      core.triggers.get(A)!.hotkey.enabled = true;
      core.hotkeyErrors.set(A, "Ctrl + Alt + 1 is taken by another app");
      await relay.loadMacro(A);
      await openTriggers();
      expect(card("Hotkey").getByText("Ctrl + Alt + 1 is taken by another app")).toHaveClass("wrong");
    });

    test("Remove takes it away", async () => {
      await openTriggers();
      await userEvent.click(card("Hotkey").getByRole("button", { name: "Remove" }));
      await settle();
      expect(sent().hotkey).toEqual({ enabled: false, combo: "" });
      expect(screen.queryByRole("group", { name: "Hotkey" })).toBeNull();
    });
  });

  describe("the schedule", () => {
    test("added, it's on: weekdays at 09:00, with the next run", async () => {
      await openTriggers();
      await add("Schedule");
      expect(sent().schedule).toEqual({ enabled: true, schedule: { days: [true, true, true, true, true, false, false], time: "09:00" } });
      expect(card("Schedule").getByText("Mon–Fri at 09:00")).toBeInTheDocument();
      expect(card("Schedule").getByText(/^Next run: /)).toBeInTheDocument();
      expect(button(/Runs automatically/)).toHaveTextContent("Mon–Fri at 09:00");
    });

    test("each day toggles, named in full; on with none, it asks for one", async () => {
      await relay.setTriggers({ schedule: { enabled: true, schedule: { days: [true, false, false, false, false, false, false], time: "09:00" } } });
      await openTriggers();
      const days = within(screen.getByRole("group", { name: "Days" }));
      expect(days.getAllByRole("button").map((d) => d.getAttribute("aria-label"))).toEqual(["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"]);
      await userEvent.click(days.getByRole("button", { name: "Saturday" }));
      await settle();
      expect(sent().schedule.schedule.days).toEqual([true, false, false, false, false, true, false]);
      await userEvent.click(days.getByRole("button", { name: "Monday" }));
      await userEvent.click(days.getByRole("button", { name: "Saturday" }));
      await settle();
      expect(card("Schedule").getByText("Pick a day")).toBeInTheDocument();
    });

    test("the time; a cleared one is put back", async () => {
      await relay.setTriggers({ schedule: { ...defaultTriggers().schedule, enabled: true } });
      await openTriggers();
      const time = screen.getByLabelText("Time");
      await change(time, "17:45");
      await settle();
      expect(sent().schedule.schedule.time).toBe("17:45");
      core.clearCalls();
      await change(time, "");
      await settle();
      expect(core.argsOf("set_triggers")).toEqual([]);
      expect(time).toHaveValue("17:45");
    });
  });

  describe("when an app starts", () => {
    test("its program, trimmed, turns it on; the switch needs one", async () => {
      await openTriggers();
      await add("When an app starts");
      expect(toggle("When an app starts on")).toBeDisabled();
      await change(screen.getByRole("combobox", { name: "Program" }), "  EXCEL.EXE ");
      await settle();
      expect(sent().app_launch).toEqual({ enabled: true, exe: "EXCEL.EXE", delay_ms: 2000 });
      expect(card("When an app starts").getByText("2 s after EXCEL.EXE starts")).toBeInTheDocument();
    });

    test("clearing the program turns it off", async () => {
      await relay.setTriggers({ app_launch: { enabled: true, exe: "notepad.exe", delay_ms: 0 } });
      await openTriggers();
      await change(screen.getByRole("combobox", { name: "Program" }), "   ");
      await settle();
      expect(sent().app_launch).toEqual({ enabled: false, exe: "", delay_ms: 0 });
    });

    test("the delay, in seconds", async () => {
      await relay.setTriggers({ app_launch: { enabled: true, exe: "notepad.exe", delay_ms: 2000 } });
      await openTriggers();
      const delay = field("Delay in seconds");
      await change(delay, "3.5");
      await settle();
      expect(sent().app_launch.delay_ms).toBe(3500);
      await change(delay, "-2");
      await settle();
      expect(sent().app_launch.delay_ms).toBe(0);
    });
  });

  describe("when a pixel changes", () => {
    test("its position and color", async () => {
      await openTriggers();
      await add("When a pixel changes");
      await userEvent.click(toggle("When a pixel changes on"));
      await settle();
      await change(field("X"), "-100.4");
      await settle();
      await change(field("Y"), "250");
      await settle();
      await change(field("Color"), "#a1b2c3");
      await settle();
      expect(sent().pixel).toEqual({ enabled: true, x: -100, y: 250, color: "#A1B2C3", tolerance: 8 });
      expect(card("When a pixel changes").getByText("When -100, 250 turns #A1B2C3")).toBeInTheDocument();
    });

    test("a color that isn't #RRGGBB is put back", async () => {
      await openTriggers();
      await add("When a pixel changes");
      await change(field("Color"), "blue");
      expect(field("Color")).toHaveValue("#EC3013");
      expect(core.argsOf("set_triggers")).toEqual([]);
    });

    test("Pick from screen watches the pixel under the cursor after a countdown", async () => {
      await openTriggers();
      await add("When a pixel changes");
      vi.useFakeTimers();
      core.hold("pick_pixel");
      await fireEvent.click(button("Pick from screen"));
      await settle();
      expect(core.argsOf("pick_pixel")).toEqual([{ delayMs: 3000 }]);
      core.held[0].resolve({ x: 11, y: 22, color: "#FFFFFF" });
      await settle();
      expect(sent().pixel).toMatchObject({ x: 11, y: 22, color: "#FFFFFF" });
    });
  });

  describe("when an image appears", () => {
    const IMAGE = png(40, 20);

    test("off until it has an image; the first one turns it on", async () => {
      await openTriggers();
      await add("When an image appears");
      expect(toggle("When an image appears on")).toBeDisabled();
      expect(button("Test")).toBeDisabled();
      core.snip = IMAGE;
      await userEvent.click(button("Snip"));
      await settle();
      expect(sent().image).toEqual({ enabled: true, image: IMAGE, threshold: 85, area: null });
      expect(screen.getByRole("img", { name: "What to watch for" })).toHaveAttribute("src", `data:image/png;base64,${IMAGE}`);
    });

    test("the match, Test and Show", async () => {
      await relay.setTriggers({ image: { enabled: true, image: IMAGE, threshold: 85, area: null } });
      await openTriggers();
      await change(field("Match in percent"), "40");
      await settle();
      expect(sent().image.threshold).toBe(50);
      core.found = { x: -300, y: 20, w: 40, h: 20, score: 72 };
      await userEvent.click(button("Test"));
      await settle();
      expect(screen.getByRole("status")).toHaveTextContent("Found at -300, 20 (72%)");
      // The trigger doesn't click: the dot marks the middle.
      await userEvent.click(button("Show"));
      await settle();
      expect(core.argsOf("show_match")).toEqual([{ area: { x: -300, y: 20, w: 40, h: 20 }, dotX: -280, dotY: 30 }]);
    });

    test("with several screens, where to look", async () => {
      const second = { name: "\\\\.\\DISPLAY2", rect: { x: 1920, y: 0, w: 2560, h: 1440 }, work: { x: 1920, y: 0, w: 2560, h: 1400 }, dpi: 144, primary: false };
      core.entries[0].view.recording.monitors.push(second);
      await relay.loadMacro(A);
      await relay.setTriggers({ image: { enabled: true, image: IMAGE, threshold: 85, area: null } });
      await openTriggers();
      await userEvent.click(screen.getByRole("radio", { name: "Screen 2" }));
      await settle();
      expect(sent().image.area).toEqual(second.rect);
    });
  });

  test("paused by the kill switch, Resume turns triggers back on", async () => {
    core.emit({ type: "triggers_paused", paused: true });
    await openTriggers();
    expect(screen.getByText("Triggers are paused (the kill switch).")).toBeInTheDocument();
    await userEvent.click(button("Resume"));
    await settle();
    expect(core.argsOf("set_triggers_paused")).toEqual([{ paused: false }]);
  });

  test("the browser preview can't change them", async () => {
    core.uninstall();
    try {
      const r = await freshStore({ backend: browserBackend() });
      r.macroSection = "triggers";
      render(MacroPane);
      await settle();
      expect(toggle("Hotkey on")).toBeDisabled();
      expect(button("Remove")).toBeDisabled();
      expect(button("Add")).toBeDisabled();
    } finally {
      core.install();
    }
  });
});
