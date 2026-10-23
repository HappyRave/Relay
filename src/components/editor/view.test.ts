// The view under the editor's list: its bar, the preview's callouts, the
// timeline as a slider, and the title block.
import { beforeEach, describe, expect, test } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import ViewPane from "./ViewPane.svelte";
import ViewTimeline from "./view/ViewTimeline.svelte";
import ViewPreview from "./view/ViewPreview.svelte";
import MacroPage from "./MacroPage.svelte";
import { core, freshStore, nextFrame, settle } from "../../test/app";
import type { RelayStore } from "../../lib/state/relay.svelte";

let relay: RelayStore;
const A = "00000000-0000-0000-0000-000000000001";
beforeEach(async () => {
  relay = await freshStore();
  relay.place = "macro";
});

describe("the view's bar", () => {
  test("opens and closes the view; closed, it says what's under the playhead", async () => {
    render(ViewPane);
    const toggle = screen.getByRole("button", { name: "View" });
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByRole("slider", { name: "Playhead" })).toBeInTheDocument();
    relay.seek(relay.steps[4].t + 10);
    await userEvent.click(toggle);
    expect(relay.viewOpen).toBe(false);
    expect(screen.queryByRole("slider")).toBeNull();
    expect(screen.getByText("Step 3 · Wait · 0.70 s · Dialog opens")).toBeInTheDocument();
  });

  test("the time, as a device would show it", () => {
    relay.seek(6020);
    render(ViewPane);
    expect(screen.getByRole("img", { name: "Playhead at 00:06.02 of 00:10.15" })).toHaveTextContent("00:06.02/ 10.15");
  });

  test("previous and next step", async () => {
    render(ViewPane);
    await userEvent.click(screen.getByRole("button", { name: "Next step" }));
    expect(relay.cur).toBe(relay.steps[1].t);
    await userEvent.click(screen.getByRole("button", { name: "Previous step" }));
    expect(relay.cur).toBe(0);
  });

  test("Screen or Outline, when the macro has a screenshot", async () => {
    render(ViewPane);
    const draw = within(screen.getByRole("radiogroup", { name: "Draw over" }));
    expect(draw.getByRole("radio", { name: "Outline" })).toBeDisabled(); // no screenshot
    core.screens.set(A, new Uint8Array([1]));
    await relay.loadMacro(A);
    await settle();
    await userEvent.click(draw.getByRole("radio", { name: "Outline" }));
    await settle();
    expect(core.lastArgs("update_settings")?.settings).toMatchObject({ preview_background: "sketch" });
  });

  test("a keyboard-only macro says there's nothing to draw", async () => {
    relay.view = { ...relay.view!, steps: relay.steps.filter((s) => s.kind === "keys") };
    relay.viewOpen = false;
    render(ViewPane);
    expect(screen.getByText("This macro uses only the keyboard, so there's nothing to draw.")).toBeInTheDocument();
  });

  test("the title block: steps, length, screen", () => {
    render(ViewPane);
    const block = within(screen.getByLabelText("Macro details"));
    expect(block.getByText("12")).toBeInTheDocument();
    expect(block.getByText("10.15 s")).toBeInTheDocument();
    expect(block.getByText("1920 × 1080")).toBeInTheDocument();
  });
});

describe("the timeline", () => {
  const slider = () => screen.getByRole("slider", { name: "Playhead" });

  test("is a slider saying where the playhead is, and on which step", () => {
    relay.seek(relay.steps[10].t + 10);
    render(ViewTimeline);
    expect(slider()).toHaveAttribute("aria-valuetext", "00:06.03 of 00:10.15, step 7 of 12, Click · Save");
    expect(slider()).toHaveAttribute("aria-valuemax", "10.15");
  });

  test("the arrow keys go from step to step, a tenth of a second with Shift; Home and End", async () => {
    render(ViewTimeline);
    slider().focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(relay.cur).toBe(relay.steps[1].t);
    await userEvent.keyboard("{Shift>}{ArrowRight}{/Shift}");
    expect(relay.cur).toBe(relay.steps[1].t + 100);
    await userEvent.keyboard("{End}");
    expect(relay.cur).toBe(relay.duration);
    await userEvent.keyboard("{Home}");
    expect(relay.cur).toBe(0);
  });

  test("Space plays or pauses", async () => {
    render(ViewTimeline);
    slider().focus();
    await userEvent.keyboard(" ");
    expect(core.commands()).toContain("toggle_play");
  });

  test("a click moves the playhead there", async () => {
    render(ViewTimeline);
    slider().getBoundingClientRect = () => ({ left: 0, top: 0, width: 1000, height: 100, right: 1000, bottom: 100, x: 0, y: 0, toJSON: () => null });
    await fireEvent.pointerDown(slider(), { button: 0, clientX: 500, pointerId: 1 });
    expect(relay.cur).toBeCloseTo(relay.duration / 2, 0);
    await nextFrame();
    expect(core.lastArgs("seek")).toEqual({ t: relay.cur });
  });

  test("a lane for each kind of step, with what's played filled in", () => {
    relay.seek(2000);
    const { container } = render(ViewTimeline);
    expect(container.querySelectorAll(".lane")).toHaveLength(3);
    const points = container.querySelectorAll(".point");
    expect(points).toHaveLength(6); // the clicks
    expect([...points].filter((p) => !p.classList.contains("unplayed"))).toHaveLength(2);
    expect(container.querySelectorAll(".blk.keys")).toHaveLength(4);
    expect(container.querySelectorAll(".blk.logic")).toHaveLength(2);
    expect(container.querySelector(".dimension")).toHaveTextContent("10.15 s");
  });
});

describe("the preview", () => {
  test("says what it shows", () => {
    render(ViewPreview);
    expect(screen.getByRole("img", { name: "The mouse path over the screen, with 6 clicks numbered in order" })).toBeInTheDocument();
  });

  test("the selected click's callout is white, and shows its label", async () => {
    const box = { width: 300, height: 150 };
    Object.defineProperty(HTMLElement.prototype, "clientWidth", { configurable: true, get: () => box.width });
    Object.defineProperty(HTMLElement.prototype, "clientHeight", { configurable: true, get: () => box.height });
    try {
      relay.select([10]);
      const { container } = render(ViewPreview);
      await settle();
      const callouts = [...container.querySelectorAll(".callout")];
      expect(callouts.map((c) => c.querySelector("i")?.textContent)).toEqual(["1", "2", "4", "7", "10", "11"]);
      const selected = container.querySelector(".callout.selected")!;
      expect(selected).toHaveTextContent("7Save");
    } finally {
      // jsdom's own: 0.
      Object.defineProperty(HTMLElement.prototype, "clientWidth", { configurable: true, get: () => 0 });
      Object.defineProperty(HTMLElement.prototype, "clientHeight", { configurable: true, get: () => 0 });
    }
  });

  test("+ and − zoom, 0 fits", async () => {
    render(ViewPane);
    const stage = screen.getByRole("img", { name: /mouse path/ });
    stage.focus();
    await userEvent.keyboard("+");
    expect(screen.getByRole("button", { name: "125% · Fit" })).toBeInTheDocument();
    await userEvent.keyboard("0");
    expect(screen.queryByRole("button", { name: /Fit/ })).toBeNull();
  });
});

describe("the page", () => {
  test("a click on a step, Enter, the play summary and Ctrl + M open the inspector (it slides over the list in a narrow window)", async () => {
    render(MacroPage);
    const list = screen.getByRole("listbox", { name: /Steps/ });
    await userEvent.click(within(list).getAllByRole("option")[1]);
    expect(relay.inspectorOpen).toBe(true);
    relay.inspectorOpen = false;
    list.focus();
    await userEvent.keyboard("{Enter}");
    expect(relay.inspectorOpen).toBe(true);
    relay.inspectorOpen = false;
    await userEvent.click(screen.getByRole("button", { name: /Open the play options/ }));
    expect([relay.inspectorOpen, relay.inspector]).toEqual([true, "macro"]);
    await userEvent.click(screen.getByRole("button", { name: "Close the inspector" }));
    expect(relay.inspectorOpen).toBe(false);
  });

  test("the divider between the list and the inspector resizes it, saved once it's let go; double-click resets it", async () => {
    render(MacroPage);
    const divider = screen.getByRole("separator", { name: "Resize the inspector" });
    expect(divider).toHaveAttribute("aria-valuenow", "340");
    divider.focus();
    await fireEvent.keyDown(divider, { key: "ArrowLeft", shiftKey: true });
    await settle();
    expect(core.lastArgs("save_panes")).toEqual({ panes: { inspector_w: 372 } });
    await fireEvent.dblClick(divider);
    await settle();
    expect(core.lastArgs("save_panes")).toEqual({ panes: { inspector_w: null } });
  });
});
