// Find image steps: inserting one from a snip, the clipboard or a file, and
// editing it (image, click point, button, match, timeout, where to look, Test).
import { beforeEach, describe, expect, test } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import StepsTab from "./StepsTab.svelte";
import Timeline from "../Timeline.svelte";
import Preview from "../Preview.svelte";
import { core, freshStore, png, settle } from "../../../test/app";
import type { RelayStore } from "../../../lib/state/relay.svelte";

let relay: RelayStore;
beforeEach(async () => {
  relay = await freshStore();
});

const ops = () => core.argsOf("edit_macro").map((a) => a.op);
const stepRows = () => screen.getAllByRole("button").filter((b) => b.classList.contains("row"));
const IMAGE = png(40, 20);
/** What + Find image inserts, with IMAGE. */
const inserted = { image: IMAGE, click_x: 20, click_y: 10, btn: "Left", threshold: 85, timeout_ms: 5000, area: null };

describe("+ Find image", () => {
  test("snips, then inserts a step at the playhead that clicks the image's middle", async () => {
    render(StepsTab);
    core.snip = IMAGE;
    relay.seek(1000);
    await userEvent.click(screen.getByRole("button", { name: "+ Find image" }));
    await settle();
    expect(core.commands()).toContain("snip_image");
    expect(ops()).toEqual([{ op: "insert_find_image", at: 1000, dur: 800, ...inserted, label: "" }]);
    const row = stepRows().find((r) => r.textContent?.includes("Find image"))!;
    expect(row).toHaveTextContent("FIND");
    expect(row).toHaveTextContent("Left click when found · timeout 5 s, else stop");
    expect(screen.queryByRole("status")).toBeNull();
  });

  test("a cancelled snip inserts nothing", async () => {
    render(StepsTab);
    await userEvent.click(screen.getByRole("button", { name: "+ Find image" }));
    await settle();
    expect(core.commands()).toContain("snip_image");
    expect(ops()).toEqual([]);
  });

  test("while the snip waits, Paste or File… instead, or Cancel", async () => {
    render(StepsTab);
    core.hold("snip_image");
    await userEvent.click(screen.getByRole("button", { name: "+ Find image" }));
    await settle();
    const status = screen.getByRole("status");
    expect(status).toHaveTextContent("Snip the image to find, or");
    expect(screen.getByRole("button", { name: "+ Find image" })).toBeDisabled();

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
    await userEvent.click(screen.getByRole("button", { name: "+ Find image" }));
    await settle();
    await userEvent.click(within(screen.getByRole("status")).getByRole("button", { name: "Cancel" }));
    await settle();
    expect(core.commands().at(-1)).toBe("cancel_snip");
    expect(screen.queryByRole("status")).toBeNull();
    core.held[1].resolve(IMAGE);
    await settle();
    expect(ops()).toHaveLength(1);
  });

  test("File… reads the chosen image", async () => {
    render(StepsTab);
    core.hold("snip_image");
    await userEvent.click(screen.getByRole("button", { name: "+ Find image" }));
    await settle();
    core.dialog.open = "C:\\shots\\ok.png";
    core.images.set("C:\\shots\\ok.png", IMAGE);
    await userEvent.click(within(screen.getByRole("status")).getByRole("button", { name: "File…" }));
    await settle();
    expect(core.lastArgs("load_image")).toEqual({ path: "C:\\shots\\ok.png" });
    expect(ops()).toEqual([{ op: "insert_find_image", at: 0, dur: 800, ...inserted, label: "" }]);
  });

  test("an image that can't be used says why, and inserts nothing", async () => {
    render(StepsTab);
    core.hold("snip_image");
    await userEvent.click(screen.getByRole("button", { name: "+ Find image" }));
    await settle();
    await userEvent.click(within(screen.getByRole("status")).getByRole("button", { name: "Paste" }));
    await settle();
    expect(relay.toast).toMatchObject({ kind: "error", message: "There's no picture on the clipboard. Copy or snip one first." });
    const why = { code: "image", message: "This image is too plain to find reliably. Pick a part with more detail." };
    core.images.set("C:\\plain.png", why);
    core.dialog.open = "C:\\plain.png";
    await relay.insertFindImage("file");
    await settle();
    expect(relay.toast).toMatchObject({ kind: "error", message: why.message });
    expect(ops()).toEqual([]);
  });

  test("another macro opened during the snip gets nothing", async () => {
    render(StepsTab);
    core.hold("snip_image");
    await userEvent.click(screen.getByRole("button", { name: "+ Find image" }));
    await settle();
    await relay.loadMacro("00000000-0000-0000-0000-000000000002");
    await settle();
    core.held[0].resolve(IMAGE);
    await settle();
    expect(ops()).toEqual([]);
  });

  test("shows as FIND on the timeline's Logic lane", async () => {
    core.snip = IMAGE;
    await relay.insertFindImage();
    await settle();
    render(Timeline);
    expect(screen.getByText("FIND")).toBeInTheDocument();
  });
});

describe("a Find image step's editor", () => {
  async function open() {
    core.snip = IMAGE;
    relay.seek(1000);
    await relay.insertFindImage();
    await settle();
    core.clearCalls();
    const index = relay.steps.findIndex((s) => s.kind === "find_image");
    render(StepsTab);
    await userEvent.click(stepRows()[index]);
    return { e: screen.getByRole("group", { name: "Edit step" }), index };
  }
  const change = (el: HTMLElement, value: string) => fireEvent.change(el, { target: { value } });

  test("shows the image, the match, the timeout and the button, and sends the whole step for each", async () => {
    const { e, index } = await open();
    expect(within(e).getByRole("img", { name: "What to find" })).toHaveAttribute("src", `data:image/png;base64,${IMAGE}`);
    const match = within(e).getByLabelText("Match %");
    expect(match).toHaveValue(85);
    expect(within(e).getByLabelText(/Timeout/)).toHaveValue(5);
    expect(within(e).getByRole("radio", { name: "Left" })).toHaveAttribute("aria-checked", "true");
    await change(match, "120");
    expect(match).toHaveValue(100);
    await settle();
    await change(within(e).getByLabelText(/Timeout/), "0.2");
    await settle();
    await userEvent.click(within(e).getByRole("radio", { name: "Right" }));
    await settle();
    const base = { op: "update_find_image", index, ...inserted };
    expect(ops()).toEqual([
      { ...base, threshold: 100 },
      { ...base, threshold: 100, timeout_ms: 500 },
      { ...base, threshold: 100, timeout_ms: 500, btn: "Right" },
    ]);
  });

  test("clicking the image sets where to click it", async () => {
    const { e, index } = await open();
    const shot = within(e).getByRole("button", { name: /What to find/ });
    shot.getBoundingClientRect = () => ({ left: 10, top: 20, width: 80, height: 40, right: 90, bottom: 60, x: 10, y: 20, toJSON: () => null });
    await fireEvent.click(shot, { clientX: 70, clientY: 50, detail: 1 });
    await settle();
    expect(ops()).toEqual([{ op: "update_find_image", index, ...inserted, click_x: 30, click_y: 15 }]);
    // Enter or Space: no position to take.
    await fireEvent.click(shot, { detail: 0 });
    await settle();
    expect(ops()).toHaveLength(1);
  });

  test("Snip, Paste and File… replace the image, clicked in its middle", async () => {
    const { e, index } = await open();
    core.clipboard = png(10, 6);
    await userEvent.click(within(e).getByRole("button", { name: "Paste" }));
    await settle();
    expect(ops()).toEqual([{ op: "update_find_image", index, ...inserted, image: png(10, 6), click_x: 5, click_y: 3 }]);
    core.hold("snip_image");
    await userEvent.click(within(e).getByRole("button", { name: "Snip" }));
    await settle();
    expect(within(e).getByRole("status")).toHaveTextContent("Snip the image…");
    await userEvent.click(within(e).getByRole("button", { name: "Cancel" }));
    await settle();
    expect(core.commands().at(-1)).toBe("cancel_snip");
    core.held[0].resolve(png(50, 50));
    await settle();
    expect(ops()).toHaveLength(1);
  });

  test("Test says where it found the image, or how close it came", async () => {
    const { e } = await open();
    const test = within(e).getByRole("button", { name: "Test" });
    core.found = { x: 5, y: 6, w: 40, h: 20, score: 92 };
    await userEvent.click(test);
    await settle();
    expect(core.lastArgs("test_find_image")).toEqual({ image: IMAGE, threshold: 85, area: null });
    expect(within(e).getByRole("status")).toHaveTextContent("Found at 5, 6 (92%)");
    core.found = { x: 5, y: 6, w: 40, h: 20, score: 61 };
    await userEvent.click(test);
    await settle();
    expect(within(e).getByRole("status")).toHaveTextContent("Not found: the best match is 61%");
    core.found = null;
    await userEvent.click(test);
    await settle();
    expect(within(e).getByRole("status")).toHaveTextContent("Not found");
    expect(within(e).queryByRole("button", { name: "Show" })).toBeNull(); // nothing to show
    // A change of settings makes the answer stale.
    await change(within(e).getByLabelText("Match %"), "70");
    await settle();
    expect(within(e).queryByRole("status")).toBeNull();
  });

  test("Show marks the match on screen, with a dot where the step would click", async () => {
    const { e } = await open();
    // Found at 1.5 times its size: the click point (20, 10) scales with it.
    core.found = { x: 300, y: -200, w: 60, h: 30, score: 64 };
    await userEvent.click(within(e).getByRole("button", { name: "Test" }));
    await settle();
    await userEvent.click(within(e).getByRole("button", { name: "Show" }));
    await settle();
    expect(core.argsOf("show_match")).toEqual([{ area: { x: 300, y: -200, w: 60, h: 30 }, dotX: 330, dotY: -185 }]);
  });

  test("with several screens, where to look: all of them or one", async () => {
    const second = { name: "\\\\.\\DISPLAY2", rect: { x: 1920, y: 0, w: 2560, h: 1440 }, work: { x: 1920, y: 0, w: 2560, h: 1400 }, dpi: 144, primary: false };
    core.entries[0].view.recording.monitors.push(second);
    const { e, index } = await open();
    expect(within(e).getByRole("radio", { name: "All screens" })).toHaveAttribute("aria-checked", "true");
    await userEvent.click(within(e).getByRole("radio", { name: "Screen 2" }));
    await settle();
    expect(ops()).toEqual([{ op: "update_find_image", index, ...inserted, area: second.rect }]);
    expect(within(e).getByRole("radio", { name: "Screen 2" })).toHaveAttribute("aria-checked", "true");
  });

  test("with one screen there's nothing to choose", async () => {
    const { e } = await open();
    expect(within(e).queryByRole("radiogroup", { name: "Where to look" })).toBeNull();
  });

  test("takes a label", async () => {
    const { e, index } = await open();
    await change(within(e).getByLabelText("Label"), "OK button");
    await settle();
    expect(ops()).toEqual([{ op: "set_label", index, label: "OK button" }]);
    expect(stepRows()[index]).toHaveTextContent("Find image · OK button");
  });
});

test("the preview says it's looking for the image, and outlines where", async () => {
  core.snip = IMAGE;
  relay.seek(1000);
  await relay.insertFindImage();
  await settle();
  const index = relay.steps.findIndex((s) => s.kind === "find_image");
  const step = relay.steps[index];
  const { container } = render(Preview);
  relay.seek(step.t + 100);
  await settle();
  expect(container.querySelector(".cond")).toHaveTextContent("Looking for the image");
  expect(container.querySelector("rect[stroke-dasharray]")).toBeNull(); // anywhere: no outline
  await relay.updateFindImage(index, { area: { x: 100, y: 50, w: 400, h: 300 } });
  relay.seek(step.t + 100);
  await settle();
  expect(container.querySelector("rect[stroke-dasharray]")).toHaveAttribute("width", "400");
  relay.seek(step.end + 1);
  await settle();
  expect(container.querySelector(".cond")).toBeNull();
});
