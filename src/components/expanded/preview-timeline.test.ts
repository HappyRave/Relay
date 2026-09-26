// The desktop preview and the four-lane timeline: what they draw for each
// mode and playhead position, and seeking by clicking or dragging.
import { beforeEach, describe, expect, test, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/svelte";
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
    await relay.setPlayback({ repeat: "forever", speed: 2 });
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
    render(Preview);
    expect(screen.getByText(/X 0960/)).toHaveTextContent(/^X 0960\s+Y 0670$/);
    await at(850);
    expect(screen.getByText(/X 0134/)).toHaveTextContent(/^X 0134\s+Y 0070$/);
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
    expect(container.querySelector("svg")!.getAttribute("viewBox")).toBe("-1280 0 3200 1080");
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
