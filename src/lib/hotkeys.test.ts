import { describe, expect, it } from "vitest";
import { comboOf } from "./hotkeys";

const key = (key: string, code: string, mods: Partial<KeyboardEvent> = {}) =>
  ({ key, code, ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, ...mods }) as KeyboardEvent;

describe("comboOf", () => {
  it("builds combos in Ctrl, Alt, Shift, Win order", () => {
    expect(comboOf(key("1", "Digit1", { ctrlKey: true, altKey: true }))).toBe("Ctrl + Alt + 1");
    expect(comboOf(key("F7", "F7", { shiftKey: true }))).toBe("Shift + F7");
    expect(comboOf(key("ArrowLeft", "ArrowLeft", { metaKey: true, ctrlKey: true }))).toBe("Ctrl + Win + Left");
  });

  it("takes letters from the layout and digits from the key", () => {
    // AZERTY: the key labeled A sits where Q is on QWERTY; its top-row "1" key types "&".
    expect(comboOf(key("a", "KeyQ", { ctrlKey: true }))).toBe("Ctrl + A");
    expect(comboOf(key("&", "Digit1", { ctrlKey: true }))).toBe("Ctrl + 1");
  });

  it("falls back to the physical key on non-Latin layouts", () => {
    // Russian: the key labeled F on QWERTY types "а".
    expect(comboOf(key("а", "KeyF", { ctrlKey: true }))).toBe("Ctrl + F");
  });

  it("rejects keys relay-core can't register", () => {
    expect(comboOf(key("Control", "ControlLeft", { ctrlKey: true }))).toBeNull();
    expect(comboOf(key("ù", "Quote", { ctrlKey: true }))).toBeNull();
  });
});
