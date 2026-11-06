// Tests for shortcuts.ts: turning key presses into shortcut text.
import { describe, expect, it } from "vitest";
import { defaultShortcuts, keyHint, recordKey, takenBy } from "./shortcuts";

// A key press as the browser reports it (only the fields recordKey reads).
function press(code: string, mods: { ctrl?: boolean; shift?: boolean; alt?: boolean; meta?: boolean } = {}, key = code) {
  return { code, key, ctrlKey: !!mods.ctrl, shiftKey: !!mods.shift, altKey: !!mods.alt, metaKey: !!mods.meta } as KeyboardEvent;
}

describe("recordKey", () => {
  it("records a letter with modifiers", () => {
    expect(recordKey(press("KeyA", { ctrl: true, shift: true }))).toEqual({ keys: "Ctrl+Shift+A" });
  });

  it("refuses a bare letter (it would fire while typing)", () => {
    expect(recordKey(press("KeyA"))).toEqual({ error: "needsModifier" });
  });

  it("accepts a bare function key", () => {
    expect(recordKey(press("F9"))).toEqual({ keys: "F9" });
  });

  it("keeps waiting while only modifiers are down", () => {
    expect(recordKey(press("ShiftLeft", { shift: true }, "Shift"))).toEqual({ waiting: true });
  });
});

describe("takenBy", () => {
  it("names the action already using the keys", () => {
    expect(takenBy(defaultShortcuts, "Ctrl+Shift+A", "style")).toBe("area");
  });

  it("doesn't count the action being changed", () => {
    expect(takenBy(defaultShortcuts, "Ctrl+Shift+A", "area")).toBeNull();
  });
});

describe("keyHint", () => {
  it("is empty without a shortcut", () => {
    expect(keyHint("")).toBe("");
    expect(keyHint(undefined)).toBe("");
  });
});
