// The small controls every tab is built from.
import { describe, expect, test, vi } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/svelte";
import userEvent from "@testing-library/user-event";
import HotkeyCapture from "./HotkeyCapture.svelte";
import Icon from "./Icon.svelte";

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
    expect(within(button).getByRole("img", { name: "Ctrl + Alt + 1" })).toBeInTheDocument();
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
    expect(within(button).getByRole("img", { name: "Ctrl + Alt + 1" })).toBeInTheDocument();
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
    expect(within(button).getByRole("img", { name: "Ctrl + Alt + 1" })).toBeInTheDocument();
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
