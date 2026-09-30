// The desktop preview and the four-lane timeline: what they draw for each
// mode and playhead position, and seeking by clicking or dragging.
import { beforeEach, describe, expect, test, vi } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import Preview from "./Preview.svelte";
import Timeline from "./Timeline.svelte";
import CompactBar from "../CompactBar.svelte";
import { core, freshStore, nextFrame, settle } from "../../test/app";
import type { RelayStore } from "../../lib/state/relay.svelte";

let relay: RelayStore;
const A = "00000000-0000-0000-0000-000000000001";
beforeEach(async () => {
  relay = await freshStore();
});

async function at(t: number) {
  relay.seek(t);
  await settle();
}

describe("Preview", () => {
  test.each([
    ["idle", "Preview"],
    ["countdown", "Get ready"],
    ["recording", "● Rec"],
    ["playing", "Playing"],
    ["paused", "Paused"],
  ] as const)("the badge in %s mode says %s", async (mode, text) => {
    const { container } = render(Preview);
    core.emit({ type: "session", mode, macro_id: A });
    await settle();
    expect(container.querySelector(".badge")).toHaveTextContent(text);
    expect(container.querySelector(".badge")!.classList.contains("rec")).toBe(mode === "countdown" || mode === "recording");
  });

  test("while playing, it shows the loop and speed", async () => {
    const { container } = render(Preview);
    core.emit({ type: "session", mode: "playing", macro_id: A });
    core.emit({ type: "play_tick", t: 100, advancing: true, speed: 1, loop_idx: 1, loops: 3 });
    await settle();
    expect(container.querySelector(".loop")).toHaveTextContent("Loop 2 / 3 · 1×");
    // Saved options don't change the run in progress; the engine's ticks say what it plays.
    await relay.setPlayback({ repeat: "forever", speed: 2 });
    await settle();
    expect(container.querySelector(".loop")).toHaveTextContent("Loop 2 / 3 · 1×");
    core.emit({ type: "play_tick", t: 200, advancing: true, speed: 2, loop_idx: 1, loops: null });
    await settle();
    expect(container.querySelector(".loop")).toHaveTextContent("Loop 2 / ∞ · 2×");
    core.emit({ type: "session", mode: "idle", macro_id: A });
    await settle();
    expect(container.querySelector(".loop")).toBeNull();
  });

  test("the countdown shows whole seconds left", async () => {
    const { container } = render(Preview);
    core.emit({ type: "session", mode: "countdown", macro_id: null });
    core.emit({ type: "countdown", left_ms: 2400 });
    await settle();
    expect(container.querySelector(".countdown")).toHaveTextContent("3");
    core.emit({ type: "countdown", left_ms: 900 });
    await settle();
    expect(container.querySelector(".countdown")).toHaveTextContent("1");
  });

  test("the cursor's coordinates, 4 digits", async () => {
    const { container } = render(Preview);
    expect(container.querySelector(".bar .coords")).toHaveTextContent(/^X 0960\s+Y 0670$/);
    await at(850);
    expect(container.querySelector(".bar .coords")).toHaveTextContent(/^X 0134\s+Y 0070$/);
  });

  test("the bar above the drawing holds the text, so nothing covers the drawing", async () => {
    const { container } = render(Preview);
    await at(4400);
    const bar = container.querySelector(".bar")!;
    for (const part of [".badge", ".keys", ".info", ".coords"]) expect(bar.querySelector(part)).not.toBeNull();
    expect(container.querySelector(".stage")!.children).toHaveLength(1);
    expect(container.querySelector(".stage svg")).toHaveAttribute("height", "100%");
  });

  describe("the screenshot", () => {
    const shot = (c: Element) => c.querySelector("svg image.shot");
    const frames = (c: Element) => c.querySelectorAll("svg > g:first-of-type rect").length;

    test("is drawn over the desktop it shows, in place of the outlines", async () => {
      core.screens.set(A, new Uint8Array([1]));
      await relay.loadMacro(A);
      const { container } = render(Preview);
      const img = shot(container)!;
      expect(img.getAttribute("href")).toBe(relay.screenUrl);
      expect(["x", "y", "width", "height"].map((a) => img.getAttribute(a))).toEqual(["0", "0", "1920", "1080"]);
      expect(frames(container)).toBe(0);
      const bg = within(screen.getByRole("radiogroup", { name: "Background" }));
      expect(bg.getByRole("radio", { name: "Screen" })).toHaveAttribute("aria-checked", "true");
    });

    test("Sketch shows the outlines instead, and is remembered", async () => {
      core.screens.set(A, new Uint8Array([1]));
      await relay.loadMacro(A);
      const { container } = render(Preview);
      await fireEvent.click(screen.getByRole("radio", { name: "Sketch" }));
      await settle();
      expect(core.lastArgs("update_settings")).toMatchObject({ settings: { preview_background: "sketch" } });
      expect(shot(container)).toBeNull();
      expect(frames(container)).toBe(2);
      await fireEvent.click(screen.getByRole("radio", { name: "Screen" }));
      await settle();
      expect(shot(container)).not.toBeNull();
    });

    test("without one, the switch is off and shows the sketch", () => {
      const { container } = render(Preview);
      expect(shot(container)).toBeNull();
      expect(frames(container)).toBe(2);
      const bg = within(screen.getByRole("radiogroup", { name: "Background" }));
      expect(bg.getByRole("radio", { name: "Sketch" })).toHaveAttribute("aria-checked", "true");
      expect(bg.getByRole("radio", { name: "Screen" })).toBeDisabled();
      expect(container.querySelector(".bar .bg")).toHaveAttribute("title", "No screenshot: this macro was recorded without one");
    });

    test("isn't shown while recording, which draws the live desktop", async () => {
      core.screens.set(A, new Uint8Array([1]));
      await relay.loadMacro(A);
      const { container } = render(Preview);
      core.emit({ type: "session", mode: "recording", macro_id: null });
      await settle();
      expect(shot(container)).toBeNull();
      expect(screen.queryByRole("radiogroup", { name: "Background" })).toBeNull();
    });
  });

  describe("the view", () => {
    const viewBox = (c: Element) => c.querySelector("svg")!.getAttribute("viewBox")!.split(" ").map(Number);
    /** The drawing laid out at 600 × 302 (jsdom has no layout). */
    const layOut = (c: Element) => {
      const stage = c.querySelector(".stage") as HTMLElement;
      stage.getBoundingClientRect = () => ({ left: 0, top: 36, width: 600, height: 302, right: 600, bottom: 338 }) as DOMRect;
      return stage;
    };

    test("shows the whole screen, fitted to the drawing, so none of it is hidden", () => {
      const { container } = render(Preview);
      // The sample's 1920 × 1080 monitor in a 600 × 302 drawing: its full height, centered.
      const [x, y, w, h] = viewBox(container);
      expect([y, h]).toEqual([0, 1080]);
      expect(x).toBeLessThan(0);
      expect(x + w / 2).toBeCloseTo(960);
      expect(screen.queryByRole("button", { name: /Fit/ })).toBeNull();
    });

    test("the wheel zooms in around the pointer; Fit shows it all again", async () => {
      const { container } = render(Preview);
      const stage = layOut(container);
      const [fx, , fw] = viewBox(container);
      // The pointer a quarter of the way across the drawing; six notches in (1.25⁶ ≈ 3.8×).
      await fireEvent.wheel(stage, { deltaY: -600, clientX: 150, clientY: 36 + 151 });
      const [x, , w] = viewBox(container);
      expect(w).toBeCloseTo(fw / 1.25 ** 6);
      expect(x + w / 4).toBeCloseTo(fx + fw / 4); // the point under the pointer stayed there
      await fireEvent.click(screen.getByRole("button", { name: "381% · Fit" }));
      expect(viewBox(container)[2]).toBeCloseTo(fw);
      expect(screen.queryByRole("button", { name: /Fit/ })).toBeNull();
    });

    test("zoomed in, dragging moves the view; a double-click shows it all", async () => {
      const { container } = render(Preview);
      const stage = layOut(container);
      await fireEvent.wheel(stage, { deltaY: -400, clientX: 300, clientY: 187 });
      const [x0, y0, w] = viewBox(container);
      await fireEvent.pointerDown(stage, { button: 0, pointerId: 1, clientX: 300, clientY: 187 });
      await fireEvent.pointerMove(stage, { pointerId: 1, clientX: 250, clientY: 177 });
      await fireEvent.pointerUp(stage, { pointerId: 1, clientX: 250, clientY: 177 });
      const [x1, y1] = viewBox(container);
      const per = w / 600; // desktop px per drawing px
      expect(x1 - x0).toBeCloseTo(50 * per);
      expect(y1 - y0).toBeCloseTo(10 * per);
      await fireEvent.dblClick(stage);
      expect(viewBox(container)[3]).toBe(1080);
    });

    test("at the whole screen, a drag doesn't move it, and zooming out does nothing", async () => {
      const { container } = render(Preview);
      const stage = layOut(container);
      const before = viewBox(container);
      await fireEvent.pointerDown(stage, { button: 0, pointerId: 1, clientX: 300, clientY: 187 });
      await fireEvent.pointerMove(stage, { pointerId: 1, clientX: 100, clientY: 100 });
      await fireEvent.wheel(stage, { deltaY: 300, clientX: 300, clientY: 187 });
      expect(viewBox(container)).toEqual(before);
    });

    test("markers keep their size on screen when zooming in", async () => {
      const { container } = render(Preview);
      const stage = layOut(container);
      const size = () => {
        const r = container.querySelector("svg > g:not(:first-of-type) > rect")!;
        return Number(r.getAttribute("width")) / (viewBox(container)[2] / 600); // drawing px
      };
      const at1 = size();
      await fireEvent.wheel(stage, { deltaY: -400, clientX: 300, clientY: 187 });
      expect(size()).toBeCloseTo(at1);
    });

    test("a recording starts with the whole screen", async () => {
      const { container } = render(Preview);
      await fireEvent.wheel(layOut(container), { deltaY: -400, clientX: 300, clientY: 187 });
      core.emit({ type: "session", mode: "recording", macro_id: null });
      await settle();
      expect(screen.queryByRole("button", { name: /Fit/ })).toBeNull();
      // The sample's desktop, fitted: its full height.
      expect(viewBox(container)[3]).toBe(1080);
    });

    test("another macro opens with the whole screen", async () => {
      const { container } = render(Preview);
      await fireEvent.wheel(layOut(container), { deltaY: -400, clientX: 300, clientY: 187 });
      expect(screen.getByRole("button", { name: /Fit/ })).toBeInTheDocument();
      await relay.loadMacro("00000000-0000-0000-0000-000000000002");
      await settle();
      expect(screen.queryByRole("button", { name: /Fit/ })).toBeNull();
    });
  });

  test("the bar names the step under the playhead, as the steps list does", async () => {
    const { container } = render(Preview);
    // The cursor moves to the first click from the start.
    expect(container.querySelector(".info")).toHaveTextContent("Step 1 · Move");
    await at(900);
    expect(container.querySelector(".info")).toHaveTextContent("Step 2 · Click · File menu");
    await at(3600);
    expect(container.querySelector(".info")).toHaveTextContent("Step 7 · Double click · Filename field");
    await at(3760);
    expect(container.querySelector(".info")).toHaveTextContent("Step 8 · Ctrl + A");
  });

  test("the open move's path is highlighted, from where the cursor was before it", async () => {
    const { container } = render(Preview);
    expect(container.querySelector(".open-move")).toBeNull();
    relay.selectStep(2); // 134, 70 (the first click) → 230, 324
    await settle();
    const d = container.querySelector(".open-move")!.getAttribute("d")!;
    expect(d.startsWith("M134 70 L")).toBe(true);
    expect(d.endsWith(" L230 324")).toBe(true);
    expect(d.split(" L")).toHaveLength(1 + 41 + 1); // the start, its samples, and the click at its end
    relay.selectStep(1); // a click
    await settle();
    expect(container.querySelector(".open-move")).toBeNull();
  });

  test("numbered markers for each click, with their labels", () => {
    const { container } = render(Preview);
    const texts = [...container.querySelectorAll("svg text")].map((t) => t.textContent);
    expect(texts).toEqual(expect.arrayContaining(["1", "2", "3", "4", "5", "6", "File menu", "Save", "Upload to portal"]));
  });

  test("click labels can be turned off", async () => {
    await relay.updateSettings({ show_click_labels: false });
    const { container } = render(Preview);
    const texts = [...container.querySelectorAll("svg text")].map((t) => t.textContent);
    expect(texts).toEqual(["1", "2", "3", "4", "5", "6"]);
  });

  test("markers the playhead passed are filled", async () => {
    const { container } = render(Preview);
    const filled = () => [...container.querySelectorAll("svg g > rect")].filter((r) => r.getAttribute("fill") === "var(--color-accent)").length;
    expect(filled()).toBe(0);
    await at(3600);
    expect(filled()).toBe(3);
  });

  test("a ring grows for half a second around the click just reached", async () => {
    const { container } = render(Preview);
    const ring = () => container.querySelector('svg > rect[fill="none"]');
    await at(900);
    expect(ring()).not.toBeNull();
    const early = Number(ring()!.getAttribute("width"));
    await at(1200);
    expect(Number(ring()!.getAttribute("width"))).toBeGreaterThan(early);
    await at(1400);
    expect(ring()).toBeNull();
  });

  test("the full path is dashed, the part already played is solid", async () => {
    const { container } = render(Preview);
    const paths = () => [...container.querySelectorAll("svg > path")];
    expect(paths()).toHaveLength(1);
    await at(2000);
    expect(paths()).toHaveLength(2);
    expect(paths()[1].getAttribute("stroke")).toBe("var(--color-accent)");
  });

  test("Trail only hides the path ahead", async () => {
    await relay.updateSettings({ path_mode: "trail" });
    const { container } = render(Preview);
    expect(container.querySelectorAll("svg > path")).toHaveLength(0);
    await at(2000);
    expect(container.querySelectorAll("svg > path")).toHaveLength(1);
  });

  test("outlines the monitors and the anchor window", () => {
    const { container } = render(Preview);
    const frames = [...container.querySelectorAll("svg > g:first-child rect")].map((r) => r.getAttribute("width"));
    expect(frames).toEqual(["1920", "1416"]);
  });

  test("key combos and typing appear as they play", async () => {
    const { container } = render(Preview);
    await at(3760);
    expect(container.querySelector(".keys")).toHaveTextContent("Keys");
    expect(container.querySelector(".keys .kind")).toHaveTextContent("Keys");
    await at(4400);
    expect(container.querySelector(".keys .kind")).toHaveTextContent("Typing");
    expect(container.querySelector(".keys .key")!.textContent).toMatch(/^invo.*_$/);
    await at(5025 + 950);
    expect(container.querySelector(".keys")).toBeNull();
  });

  test("a pixel check being waited for is highlighted", async () => {
    const { container } = render(Preview);
    await at(6500);
    expect(container.querySelector(".cond")).toHaveTextContent("Waiting for pixel 1248, 680");
    await at(7200);
    expect(container.querySelector(".cond")).toBeNull();
  });

  test("recording shows the whole desktop and the live path", async () => {
    const { container } = render(Preview);
    core.emit({ type: "session", mode: "recording", macro_id: null });
    core.emit({
      type: "rec_progress",
      elapsed_ms: 500,
      desktop: { x: -1280, y: 0, w: 3200, h: 1080 },
      moves: [
        { t: 0, x: 0, y: 0 },
        { t: 100, x: 50, y: 50 },
      ],
      steps: [],
    });
    await settle();
    // The whole live desktop, fitted to the drawing: its full width, centered vertically.
    const [x, y, w, h] = container.querySelector("svg")!.getAttribute("viewBox")!.split(" ").map(Number);
    expect([x, w]).toEqual([-1280, 3200]);
    expect(y).toBeLessThan(0);
    expect(y + h / 2).toBeCloseTo(540);
    expect(container.querySelectorAll("svg > g:first-child rect")).toHaveLength(0);
  });

  test("humanize jiggles the cursor while playing", async () => {
    const { container } = render(Preview);
    const cursor = () => container.querySelector("svg > g:last-child")!.getAttribute("transform");
    core.emit({ type: "session", mode: "playing", macro_id: A });
    core.emit({ type: "play_tick", t: 3000, advancing: false, speed: 1, loop_idx: 0, loops: 3 });
    await settle();
    const jiggled = cursor();
    await relay.setPlayback({ humanize: false });
    await settle();
    expect(cursor()).not.toBe(jiggled);
  });
});

describe("Timeline", () => {
  test("a ruler tick every second for a short macro", () => {
    const { container } = render(Timeline);
    const ticks = [...container.querySelectorAll(".ruler .tick")].map((t) => t.textContent);
    expect(ticks).toEqual(["0s", "1s", "2s", "3s", "4s", "5s", "6s", "7s", "8s", "9s", "10s"]);
  });

  test("lanes for mouse moves, clicks, keys and logic", () => {
    const { container } = render(Timeline);
    expect(container.querySelectorAll(".move").length).toBeGreaterThan(0);
    expect(container.querySelectorAll(".click")).toHaveLength(6);
    expect([...container.querySelectorAll(".chip")].map((c) => c.getAttribute("title"))).toEqual([
      "Ctrl + A",
      "invoice_0924",
      "Ctrl + W",
      "Enter",
    ]);
    expect(container.querySelectorAll(".wait")).toHaveLength(1);
    expect(container.querySelector(".cond")).toHaveTextContent("IF");
    for (const label of ["Mouse", "Clicks", "Keys", "Logic"]) expect(screen.getByText(label)).toBeInTheDocument();
  });

  test("what the playhead passed is marked", async () => {
    const { container } = render(Timeline);
    await at(4000);
    expect(container.querySelectorAll(".click.past")).toHaveLength(3);
    expect(container.querySelectorAll(".chip.past")).toHaveLength(1);
    expect(container.querySelectorAll(".cond.past")).toHaveLength(0);
    await at(7000);
    expect(container.querySelectorAll(".cond.past")).toHaveLength(1);
    const head = container.querySelector(".playhead") as HTMLElement;
    expect(parseFloat(head.style.left)).toBeCloseTo((7000 / relay.duration) * 100, 3);
  });

  describe("seeking", () => {
    function lanes() {
      const { container } = render(Timeline);
      const el = container.querySelector(".lanes") as HTMLElement;
      vi.spyOn(el, "getBoundingClientRect").mockReturnValue({ left: 100, width: 1000, top: 0, height: 100 } as DOMRect);
      return el;
    }

    test("clicking seeks there", async () => {
      const el = lanes();
      await fireEvent.pointerDown(el, { button: 0, clientX: 600, pointerId: 1 });
      expect(relay.cur).toBeCloseTo(relay.duration / 2);
      await nextFrame();
      expect(core.argsOf("seek")).toEqual([{ t: relay.duration / 2 }]);
    });

    test("dragging follows the pointer until it's released, within the macro", async () => {
      const el = lanes();
      await fireEvent.pointerDown(el, { button: 0, clientX: 100, pointerId: 1 });
      expect(relay.cur).toBe(0);
      await fireEvent.pointerMove(el, { clientX: 350, pointerId: 1 });
      expect(relay.cur).toBeCloseTo(relay.duration / 4);
      await fireEvent.pointerMove(el, { clientX: 5000, pointerId: 1 });
      expect(relay.cur).toBe(relay.duration);
      await fireEvent.pointerUp(el, { pointerId: 1 });
      await fireEvent.pointerMove(el, { clientX: 350, pointerId: 1 });
      expect(relay.cur).toBe(relay.duration);
    });

    test("losing the pointer ends the drag too", async () => {
      const el = lanes();
      await fireEvent.pointerDown(el, { button: 0, clientX: 100, pointerId: 1 });
      el.dispatchEvent(new Event("lostpointercapture"));
      await fireEvent.pointerMove(el, { clientX: 600, pointerId: 1 });
      expect(relay.cur).toBe(0);
    });

    test("only the left button seeks, and not while recording", async () => {
      const el = lanes();
      await fireEvent.pointerDown(el, { button: 2, clientX: 600, pointerId: 1 });
      expect(relay.cur).toBe(0);
      core.emit({ type: "session", mode: "recording", macro_id: null });
      await settle();
      await fireEvent.pointerDown(el, { button: 0, clientX: 600, pointerId: 1 });
      expect(relay.cur).toBe(0);
    });
  });
});

describe("Compact player", () => {
  test("shows the mode, the macro, the time and a seek bar with the clicks", () => {
    const { container } = render(CompactBar);
    expect(container.querySelector(".badge")).toHaveTextContent("Preview");
    expect(screen.getByText("Export invoice to PDF")).toBeInTheDocument();
    expect(container.querySelector(".time")).toHaveTextContent("00:00.00 / 00:10.15");
    expect(container.querySelectorAll(".seek .tick")).toHaveLength(6);
  });

  test("while recording, the time is open-ended", async () => {
    const { container } = render(CompactBar);
    core.emit({ type: "session", mode: "recording", macro_id: null });
    await settle();
    expect(container.querySelector(".time")).toHaveTextContent("/ recording");
    expect(container.querySelector(".name")).toHaveTextContent("New recording");
    expect(container.querySelector(".badge")).toHaveClass("rec");
  });

  test("the fill follows the playhead", async () => {
    const { container } = render(CompactBar);
    await at(relay.duration / 2);
    expect((container.querySelector(".fill") as HTMLElement).style.width).toBe("50%");
  });

  test("Record, Play and Expand", async () => {
    render(CompactBar);
    await fireEvent.click(screen.getByRole("button", { name: "Record" }));
    await fireEvent.click(screen.getByRole("button", { name: "Play" }));
    await settle();
    expect(core.commands()).toEqual(["toggle_record", "toggle_play"]);
    relay.expanded = false;
    await fireEvent.click(screen.getByRole("button", { name: "Expand" }));
    expect(relay.expanded).toBe(true);
  });

  test("the seek bar seeks", async () => {
    const { container } = render(CompactBar);
    const bar = container.querySelector(".seek") as HTMLElement;
    vi.spyOn(bar, "getBoundingClientRect").mockReturnValue({ left: 0, width: 400 } as DOMRect);
    await fireEvent.pointerDown(bar, { button: 0, clientX: 100, pointerId: 1 });
    expect(relay.cur).toBeCloseTo(relay.duration / 4);
  });
});
