// The redesign's components: what they say to assistive technology, and how they
// answer the mouse and the keyboard.
import { describe, expect, test, vi } from "vitest";
import { createRawSnippet } from "svelte";
import { fireEvent, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import Key from "./Key.svelte";
import Chip from "./Chip.svelte";
import Keycaps from "./Keycaps.svelte";
import Glyph from "./Glyph.svelte";
import UnitField from "./UnitField.svelte";
import Switch from "./Switch.svelte";
import Checkbox from "./Checkbox.svelte";
import Choice from "./Choice.svelte";
import Tabs from "./Tabs.svelte";
import Section from "./Section.svelte";
import Menu from "./Menu.svelte";
import Lcd from "./Lcd.svelte";
import Leds from "./Leds.svelte";
import Divider from "./Divider.svelte";
import Icon, { ICON_NAMES } from "../ui/Icon.svelte";
import { category } from "../../lib/categories";

const text = (s: string) => createRawSnippet(() => ({ render: () => `<span>${s}</span>` }));

describe("Key", () => {
  test("is a button named by its text, that clicks", async () => {
    const onclick = vi.fn();
    render(Key, { children: text("Play"), onclick, variant: "ink" });
    const key = screen.getByRole("button", { name: "Play" });
    expect(key).toHaveClass("key", "ink", "md");
    expect(key).toHaveAttribute("type", "button");
    await userEvent.click(key);
    expect(onclick).toHaveBeenCalledOnce();
  });

  test("an icon-only key is named by its aria-label", () => {
    render(Key, { children: text(""), iconOnly: true, "aria-label": "Undo (Ctrl + Z)", size: "sm" });
    expect(screen.getByRole("button", { name: "Undo (Ctrl + Z)" })).toHaveClass("icon-only", "sm");
  });

  test("a disabled key doesn't click", async () => {
    const onclick = vi.fn();
    render(Key, { children: text("Play"), onclick, disabled: true });
    await userEvent.click(screen.getByRole("button"));
    expect(onclick).not.toHaveBeenCalled();
  });

  test("the record key breathes while recording", () => {
    render(Key, { children: text(""), variant: "red", round: true, live: true, "aria-label": "Stop recording" });
    expect(screen.getByRole("button", { name: "Stop recording" })).toHaveClass("red", "round", "live");
  });
});

describe("Chip", () => {
  test("a chip that does something is a button with its full name", async () => {
    const onclick = vi.fn();
    render(Chip, { children: text("×3 · 1×"), onclick, label: "Repeats 3 times, normal speed" });
    const chip = screen.getByRole("button", { name: "Repeats 3 times, normal speed" });
    expect(chip).toHaveAttribute("title", "Repeats 3 times, normal speed");
    await userEvent.click(chip);
    expect(onclick).toHaveBeenCalledOnce();
  });

  test("otherwise it's text", () => {
    render(Chip, { children: text("customers.csv") });
    expect(screen.queryByRole("button")).toBeNull();
    expect(screen.getByText("customers.csv")).toBeInTheDocument();
  });
});

describe("Keycaps", () => {
  test("reads as one combination", () => {
    render(Keycaps, { combo: ["Ctrl", "Alt", "1"] });
    expect(screen.getByRole("img", { name: "Ctrl + Alt + 1" })).toBeInTheDocument();
    expect(document.querySelectorAll(".kc")).toHaveLength(3);
  });

  test("no keys says None", () => {
    render(Keycaps, { combo: [] });
    expect(screen.getByText("None")).toBeInTheDocument();
  });
});

describe("Glyph and categories", () => {
  test("each kind of step has its category", () => {
    expect(category("click")).toBe("pointer");
    expect(category("drag")).toBe("pointer");
    expect(category("scroll")).toBe("pointer");
    expect(category("move")).toBe("move");
    expect(category("keys")).toBe("keys");
    expect(category("type")).toBe("keys");
    expect(category("text")).toBe("keys");
    expect(category("wait")).toBe("logic");
    expect(category("pixel_wait")).toBe("logic");
    expect(category("find_image")).toBe("logic");
  });

  test("the glyph is decoration: the row says what the step is", () => {
    const { container } = render(Glyph, { category: "logic", small: true });
    const g = container.querySelector(".glyph")!;
    expect(g).toHaveAttribute("aria-hidden", "true");
    expect(g).toHaveClass("logic", "small");
  });
});

describe("UnitField", () => {
  const field = () => screen.getByRole("textbox", { name: "Pause before in seconds" });

  test("is named with its unit, and shows the value with its decimals", () => {
    render(UnitField, { value: 0.3, label: "Pause before", unit: "s", step: 0.05, oncommit: () => {} });
    expect(field()).toHaveValue("0.30");
  });

  test("commits a typed value, with a comma or a dot, clamped", async () => {
    const oncommit = vi.fn();
    render(UnitField, { value: 0.3, label: "Pause before", unit: "s", min: 0, max: 5, oncommit });
    await userEvent.clear(field());
    await userEvent.type(field(), "1,25");
    await fireEvent.change(field());
    expect(oncommit).toHaveBeenLastCalledWith(1.25);
    await userEvent.clear(field());
    await userEvent.type(field(), "9");
    await fireEvent.change(field());
    expect(oncommit).toHaveBeenLastCalledWith(5);
    expect(field()).toHaveValue("5.00");
  });

  test("puts back what it showed when the text isn't a number", async () => {
    const oncommit = vi.fn();
    render(UnitField, { value: 0.3, label: "Pause before", unit: "s", oncommit });
    await userEvent.clear(field());
    await userEvent.type(field(), "soon");
    await fireEvent.change(field());
    expect(oncommit).not.toHaveBeenCalled();
    expect(field()).toHaveValue("0.30");
  });

  test("the arrow keys step it, Shift ten steps", async () => {
    const oncommit = vi.fn();
    render(UnitField, { value: 8, label: "Tolerance", step: 1, decimals: 0, min: 0, max: 255, oncommit });
    const f = screen.getByRole("textbox", { name: "Tolerance" });
    f.focus();
    await userEvent.keyboard("{ArrowUp}");
    expect(oncommit).toHaveBeenLastCalledWith(9);
    await userEvent.keyboard("{Shift>}{ArrowDown}{/Shift}");
    expect(oncommit).toHaveBeenLastCalledWith(0);
  });

  test("dragging sideways scrubs, then commits once", async () => {
    const oncommit = vi.fn();
    const onscrub = vi.fn();
    render(UnitField, { value: 1, label: "Duration", unit: "s", step: 0.05, min: 0, oncommit, onscrub });
    const wrap = document.querySelector(".field")!;
    await fireEvent.pointerDown(wrap, { clientX: 100, button: 0, pointerId: 1 });
    await fireEvent.pointerMove(wrap, { clientX: 140, pointerId: 1 });
    expect(onscrub).toHaveBeenLastCalledWith(1.5);
    expect(oncommit).not.toHaveBeenCalled();
    await fireEvent.pointerUp(wrap, { pointerId: 1 });
    expect(oncommit).toHaveBeenCalledExactlyOnceWith(1.5);
  });

  test("a click without dragging focuses it for typing", async () => {
    render(UnitField, { value: 1, label: "Duration", unit: "s", step: 0.05, oncommit: () => {} });
    const wrap = document.querySelector(".field")!;
    await fireEvent.pointerDown(wrap, { clientX: 100, button: 0, pointerId: 1 });
    await fireEvent.pointerUp(wrap, { pointerId: 1 });
    expect(screen.getByRole("textbox", { name: "Duration in seconds" })).toHaveFocus();
  });

  test("a visible prefix doesn't change its name", () => {
    render(UnitField, { value: 1248, label: "X", pre: "X", decimals: 0, oncommit: () => {} });
    expect(screen.getByRole("textbox", { name: "X" })).toHaveValue("1248");
    expect(document.querySelector(".pre")).toHaveAttribute("aria-hidden", "true");
  });
});

describe("Switch and Checkbox", () => {
  test("a switch asks for the other state", async () => {
    const onchange = vi.fn();
    render(Switch, { on: true, label: "Vary timing", onchange });
    const sw = screen.getByRole("switch", { name: "Vary timing" });
    expect(sw).toHaveAttribute("aria-checked", "true");
    await userEvent.click(sw);
    expect(onchange).toHaveBeenCalledExactlyOnceWith(false);
  });

  test("a disabled switch doesn't change", async () => {
    const onchange = vi.fn();
    render(Switch, { on: false, label: "Frosted glass", disabled: true, onchange });
    await userEvent.click(screen.getByRole("switch"));
    expect(onchange).not.toHaveBeenCalled();
  });

  test("a checkbox", async () => {
    const onchange = vi.fn();
    render(Checkbox, { checked: false, label: "Select Export invoice to PDF", onchange });
    const cb = screen.getByRole("checkbox", { name: "Select Export invoice to PDF" });
    expect(cb).toHaveAttribute("aria-checked", "false");
    await userEvent.click(cb);
    expect(onchange).toHaveBeenCalledExactlyOnceWith(true);
  });
});

describe("Choice", () => {
  const options: [number, string][] = [
    [0.5, "0.5×"],
    [1, "1×"],
    [2, "2×"],
  ];

  test("is a radio group with one tab stop, on the chosen option", () => {
    render(Choice, { options, value: 1, label: "Speed", onchange: () => {} });
    expect(screen.getByRole("radiogroup", { name: "Speed" })).toBeInTheDocument();
    const radios = screen.getAllByRole("radio");
    expect(radios.map((r) => r.getAttribute("tabindex"))).toEqual(["-1", "0", "-1"]);
    expect(screen.getByRole("radio", { name: "1×" })).toHaveAttribute("aria-checked", "true");
  });

  test("the arrow keys choose and move the focus, wrapping", async () => {
    const onchange = vi.fn();
    render(Choice, { options, value: 2, label: "Speed", onchange });
    screen.getByRole("radio", { name: "2×" }).focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(onchange).toHaveBeenLastCalledWith(0.5);
    expect(screen.getByRole("radio", { name: "0.5×" })).toHaveFocus();
    await userEvent.keyboard("{ArrowLeft}");
    expect(onchange).toHaveBeenLastCalledWith(2);
  });
});

describe("Tabs", () => {
  test("a tablist whose tabs control the panel, with the arrow keys", async () => {
    const onchange = vi.fn();
    render(Tabs, { tabs: [["step", "Step"], ["macro", "Macro", "Macro (Ctrl + M)"]], value: "step", label: "Inspector", panel: "insp", onchange });
    expect(screen.getByRole("tablist", { name: "Inspector" })).toBeInTheDocument();
    const step = screen.getByRole("tab", { name: "Step" });
    expect(step).toHaveAttribute("aria-selected", "true");
    expect(step).toHaveAttribute("aria-controls", "insp");
    expect(screen.getByRole("tab", { name: "Macro" })).toHaveAttribute("tabindex", "-1");
    step.focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(onchange).toHaveBeenLastCalledWith("macro");
    expect(screen.getByRole("tab", { name: "Macro" })).toHaveFocus();
  });
});

describe("Section", () => {
  test("says whether it's open, what it controls, and what's in it", async () => {
    const onclick = vi.fn();
    render(Section, { label: "Runs automatically", summary: "Ctrl + Alt + 1 · Mon–Fri at 09:00", open: false, controls: "sec-runs", onclick });
    const b = screen.getByRole("button", { name: /Runs automatically/ });
    expect(b).toHaveAttribute("aria-expanded", "false");
    expect(b).toHaveAttribute("aria-controls", "sec-runs");
    expect(b).toHaveTextContent("Mon–Fri at 09:00");
    await userEvent.click(b);
    expect(onclick).toHaveBeenCalledOnce();
  });
});

describe("Menu", () => {
  function setup(extra: Partial<{ onselect: (id: string) => void; onclose: () => void }> = {}) {
    const opener = document.createElement("button");
    opener.textContent = "More";
    document.body.append(opener);
    const onselect = extra.onselect ?? vi.fn();
    const onclose = extra.onclose ?? vi.fn();
    render(Menu, {
      opener,
      label: "Macro",
      onselect,
      onclose,
      items: [
        { id: "export", label: "Export…", icon: "export" },
        { id: "dup", label: "Duplicate", icon: "duplicate", disabled: true },
        { id: "moves", label: "Show moves", checked: false },
        "separator",
        { id: "delete", label: "Delete macro", icon: "trash", danger: true, shortcut: "Del" },
      ],
    });
    return { opener, onselect, onclose };
  }

  test("opens on its first item, and skips disabled ones", async () => {
    setup();
    expect(screen.getByRole("menu", { name: "Macro" })).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "Export…" })).toHaveFocus();
    await userEvent.keyboard("{ArrowDown}");
    expect(screen.getByRole("menuitemcheckbox", { name: "Show moves" })).toHaveFocus();
    expect(screen.getByRole("menuitemcheckbox", { name: "Show moves" })).toHaveAttribute("aria-checked", "false");
    await userEvent.keyboard("{End}");
    expect(screen.getByRole("menuitem", { name: /Delete macro/ })).toHaveFocus();
    await userEvent.keyboard("{ArrowDown}");
    expect(screen.getByRole("menuitem", { name: "Export…" })).toHaveFocus();
  });

  test("Enter chooses, closes, and gives the focus back", async () => {
    const { opener, onselect, onclose } = setup();
    await userEvent.keyboard("{Enter}");
    expect(onselect).toHaveBeenCalledExactlyOnceWith("export");
    expect(onclose).toHaveBeenCalledOnce();
    expect(opener).toHaveFocus();
  });

  test("Esc closes and gives the focus back", async () => {
    const { opener, onselect, onclose } = setup();
    await userEvent.keyboard("{Escape}");
    expect(onselect).not.toHaveBeenCalled();
    expect(onclose).toHaveBeenCalledOnce();
    expect(opener).toHaveFocus();
  });

  test("a click outside closes it", async () => {
    const { onclose } = setup();
    await fireEvent.pointerDown(document.body);
    expect(onclose).toHaveBeenCalledOnce();
  });
});

describe("Lcd and Leds", () => {
  test("the LCD can name what it shows", () => {
    render(Lcd, { children: text("00:06.02"), label: "Playhead at 00:06.02 of 00:10.15" });
    expect(screen.getByRole("img", { name: "Playhead at 00:06.02 of 00:10.15" })).toHaveTextContent("00:06.02");
  });

  test("LEDs light up to the progress, with a mark for each click", () => {
    const { container } = render(Leds, { value: 0.5, ticks: [0, 1], count: 11 });
    const leds = [...container.querySelectorAll(".leds i")];
    expect(leds).toHaveLength(11);
    expect(leds.filter((l) => l.classList.contains("on"))).toHaveLength(6);
    expect(leds[0]).toHaveClass("tick", "on");
    expect(leds[10]).toHaveClass("tick");
    expect(leds[10]).not.toHaveClass("on");
  });

  test("nothing is lit at the start", () => {
    const { container } = render(Leds, { value: 0 });
    expect(container.querySelectorAll(".leds i.on")).toHaveLength(0);
  });
});

describe("Icons", () => {
  test.each(ICON_NAMES)("draws %s on the 16 px grid", (name) => {
    const { container } = render(Icon, { name, size: 20 });
    const svg = container.querySelector("svg")!;
    expect(svg.getAttribute("viewBox")).toBe("0 0 16 16");
    expect(svg.getAttribute("width")).toBe("20");
    expect(svg).toHaveAttribute("aria-hidden", "true");
    expect(svg.innerHTML.length).toBeGreaterThan(10);
  });

  test.each(["x", "copy", "square"] as const)("still draws the old name %s", (name) => {
    const { container } = render(Icon, { name });
    expect(container.querySelector("svg")!.innerHTML.length).toBeGreaterThan(10);
  });
});

describe("Divider", () => {
  const props = (over: Record<string, unknown> = {}) => ({
    orientation: "vertical" as const,
    label: "Resize preview",
    value: 600,
    min: 360,
    max: 618,
    onchange: vi.fn(),
    oncommit: vi.fn(),
    onreset: vi.fn(),
    ...over,
  });
  const drag = async (el: Element, axis: "clientX" | "clientY", from: number, to: number) => {
    await fireEvent.pointerDown(el, { button: 0, pointerId: 1, [axis]: from });
    await fireEvent.pointerMove(el, { pointerId: 1, [axis]: to });
    await fireEvent.pointerUp(el, { pointerId: 1, [axis]: to });
  };

  test("is a labeled, focusable separator with its value and limits", () => {
    render(Divider, props());
    const s = screen.getByRole("separator", { name: "Resize preview" });
    expect(s).toHaveAttribute("aria-orientation", "vertical");
    expect([s.getAttribute("aria-valuenow"), s.getAttribute("aria-valuemin"), s.getAttribute("aria-valuemax")]).toEqual([
      "600",
      "360",
      "618",
    ]);
    expect(s).toHaveAttribute("tabindex", "0");
  });

  test("dragging moves it within its limits, and the end of the drag saves once", async () => {
    const p = props();
    render(Divider, p);
    const s = screen.getByRole("separator");
    await drag(s, "clientX", 100, 60);
    expect(p.onchange).toHaveBeenLastCalledWith(560);
    expect(p.oncommit).toHaveBeenCalledOnce();
    await drag(s, "clientX", 100, 900);
    expect(p.onchange).toHaveBeenLastCalledWith(618);
    await fireEvent.pointerMove(s, { pointerId: 1, clientX: 50 });
    expect(p.onchange).toHaveBeenCalledTimes(2); // no drag, no move
  });

  test("a pane after the divider grows as it goes up", async () => {
    const p = props({ orientation: "horizontal", label: "Resize timeline", value: 146, min: 146, max: 266, invert: true });
    render(Divider, p);
    await drag(screen.getByRole("separator"), "clientY", 400, 350);
    expect(p.onchange).toHaveBeenLastCalledWith(196);
  });

  test("the keys: arrows (Shift for more), Home and End, each saved", async () => {
    const p = props();
    render(Divider, p);
    const s = screen.getByRole("separator");
    await fireEvent.keyDown(s, { key: "ArrowLeft" });
    expect(p.onchange).toHaveBeenLastCalledWith(592);
    await fireEvent.keyDown(s, { key: "ArrowLeft", shiftKey: true });
    expect(p.onchange).toHaveBeenLastCalledWith(568);
    await fireEvent.keyDown(s, { key: "End" });
    expect(p.onchange).toHaveBeenLastCalledWith(618);
    await fireEvent.keyDown(s, { key: "Home" });
    expect(p.onchange).toHaveBeenLastCalledWith(360);
    expect(p.oncommit).toHaveBeenCalledTimes(4);
    await fireEvent.keyDown(s, { key: "a" });
    expect(p.oncommit).toHaveBeenCalledTimes(4);
  });

  test("the timeline's keys go up and down; double-click resets", async () => {
    const p = props({ orientation: "horizontal", value: 146, min: 146, max: 266, invert: true });
    render(Divider, p);
    const s = screen.getByRole("separator");
    await fireEvent.keyDown(s, { key: "ArrowUp" });
    expect(p.onchange).toHaveBeenLastCalledWith(154);
    await fireEvent.dblClick(s);
    expect(p.onreset).toHaveBeenCalledOnce();
  });
});
