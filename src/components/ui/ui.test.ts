// The small controls every tab is built from.
import { describe, expect, test, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import Toggle from "./Toggle.svelte";
import Segmented from "./Segmented.svelte";
import Kbd from "./Kbd.svelte";
import HotkeyCapture from "./HotkeyCapture.svelte";
import Icon from "./Icon.svelte";

describe("Toggle", () => {
  test("is a labeled switch that asks for the opposite state", async () => {
    const onchange = vi.fn();
    render(Toggle, { on: false, onchange, label: "Humanize" });
    const sw = screen.getByRole("switch", { name: "Humanize" });
    expect(sw).toHaveAttribute("aria-checked", "false");
    await userEvent.click(sw);
    expect(onchange).toHaveBeenCalledExactlyOnceWith(true);
  });

  test("on asks for off", async () => {
    const onchange = vi.fn();
    render(Toggle, { on: true, onchange, label: "X" });
    expect(screen.getByRole("switch")).toHaveAttribute("aria-checked", "true");
    expect(screen.getByRole("switch")).toHaveClass("on");
    await userEvent.click(screen.getByRole("switch"));
    expect(onchange).toHaveBeenCalledWith(false);
  });

  test("works from the keyboard", async () => {
    const onchange = vi.fn();
    render(Toggle, { on: false, onchange, label: "X" });
    screen.getByRole("switch").focus();
    await userEvent.keyboard("{Enter}");
    await userEvent.keyboard(" ");
    expect(onchange).toHaveBeenCalledTimes(2);
  });
});

describe("Segmented", () => {
  const options: [number, string][] = [
    [0.5, "0.5×"],
    [1, "1×"],
    [2, "2×"],
  ];

  test("is a radio group with the current option checked", () => {
    render(Segmented, { options, value: 1, onchange: () => {}, label: "Speed" });
    expect(screen.getByRole("radiogroup", { name: "Speed" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "1×" })).toHaveAttribute("aria-checked", "true");
    expect(screen.getByRole("radio", { name: "2×" })).toHaveAttribute("aria-checked", "false");
    expect(screen.getAllByRole("radio")).toHaveLength(3);
  });

  test("each option sends its value", async () => {
    const onchange = vi.fn();
    render(Segmented, { options, value: 1, onchange, label: "Speed" });
    for (const [v, text] of options) {
      await userEvent.click(screen.getByRole("radio", { name: text }));
      expect(onchange).toHaveBeenLastCalledWith(v);
    }
  });

  test("sizes", () => {
    const { container } = render(Segmented, { options, value: 1, onchange: () => {}, label: "S", size: "md" });
    expect(container.querySelector(".opt.md")).not.toBeNull();
  });
});

describe("Kbd", () => {
  test("shows the combo, optionally muted", () => {
    const { container } = render(Kbd, { combo: "Ctrl + Alt + 1", muted: true });
    expect(screen.getByText("Ctrl + Alt + 1")).toHaveClass("kbd", "muted");
    expect(container.querySelectorAll(".kbd")).toHaveLength(1);
  });
});

describe("Icon", () => {
  test.each(["record", "play", "pause", "stop", "square", "prev", "next", "loop", "undo", "redo", "export", "import", "copy", "trash", "x", "expand", "collapse", "grip"] as const)(
    "draws %s",
    (name) => {
      const { container } = render(Icon, { name, size: 20 });
      const svg = container.querySelector("svg")!;
      expect(svg).not.toBeNull();
      expect(svg.getAttribute("width")).toBe("20");
      expect(svg.innerHTML.length).toBeGreaterThan(10);
    },
  );
});

describe("HotkeyCapture", () => {
  const press = (key: string, init: KeyboardEventInit = {}) =>
    fireEvent.keyDown(window, { key, ...init });

  function setup(combo = "Ctrl + Alt + 1", disabled = false) {
    const onchange = vi.fn();
    render(HotkeyCapture, { combo, onchange, disabled });
    return { onchange, button: screen.getByRole("button") };
  }

  test("shows the combo, or Set… when there is none", () => {
    setup("");
    expect(screen.getByText("Set…")).toBeInTheDocument();
  });

  test("click, then press the keys", async () => {
    const { onchange, button } = setup();
    expect(button).toHaveTextContent("Ctrl + Alt + 1");
    expect(button).toHaveAttribute("data-captures-keys");
    await userEvent.click(button);
    expect(button).toHaveTextContent("Press keys…");
    await press("Control", { code: "ControlLeft", ctrlKey: true });
    await press("Shift", { code: "ShiftLeft", ctrlKey: true, shiftKey: true });
    expect(onchange).not.toHaveBeenCalled(); // modifiers alone wait for a key
    await press("K", { code: "KeyK", ctrlKey: true, shiftKey: true });
    expect(onchange).toHaveBeenCalledExactlyOnceWith("Ctrl + Shift + K");
    expect(button).not.toHaveClass("capturing");
  });

  test("function keys and arrows", async () => {
    const { onchange, button } = setup();
    await userEvent.click(button);
    await press("F7", { code: "F7" });
    await userEvent.click(button);
    await press("ArrowLeft", { code: "ArrowLeft", altKey: true });
    expect(onchange.mock.calls).toEqual([["F7"], ["Alt + Left"]]);
  });

  test("a key Relay can't use asks for another", async () => {
    const { onchange, button } = setup();
    await userEvent.click(button);
    await press("?", { code: "Slash", shiftKey: true });
    expect(button).toHaveTextContent("Use a letter, digit, F-key or arrow");
    expect(onchange).not.toHaveBeenCalled();
    await press("1", { code: "Digit1", ctrlKey: true });
    expect(onchange).toHaveBeenCalledWith("Ctrl + 1");
  });

  test("Esc cancels and Backspace clears", async () => {
    const { onchange, button } = setup();
    await userEvent.click(button);
    await press("Escape", { code: "Escape" });
    expect(onchange).not.toHaveBeenCalled();
    expect(button).toHaveTextContent("Ctrl + Alt + 1");
    await userEvent.click(button);
    await press("Backspace", { code: "Backspace" });
    expect(onchange).toHaveBeenCalledExactlyOnceWith("");
  });

  test("leaving the window cancels", async () => {
    const { onchange, button } = setup();
    await userEvent.click(button);
    await fireEvent.blur(window);
    await press("K", { code: "KeyK", ctrlKey: true });
    expect(onchange).not.toHaveBeenCalled();
    expect(button).toHaveTextContent("Ctrl + Alt + 1");
  });

  test("keys pressed while capturing don't reach the rest of the page", async () => {
    const { button } = setup();
    const outer = vi.fn();
    window.addEventListener("keydown", outer);
    await userEvent.click(button);
    await press("K", { code: "KeyK", ctrlKey: true });
    expect(outer).not.toHaveBeenCalled();
    window.removeEventListener("keydown", outer);
  });

  test("disabled, it doesn't capture", async () => {
    const { onchange, button } = setup("F8", true);
    expect(button).toBeDisabled();
    await fireEvent.click(button);
    await press("K", { code: "KeyK", ctrlKey: true });
    expect(onchange).not.toHaveBeenCalled();
  });
});
