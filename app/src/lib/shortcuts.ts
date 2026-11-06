// shortcuts.ts — the in-game shortcuts: which actions exist, their default
// keys, and turning a key press into text like "Ctrl+Shift+A" (the format
// Rust parses with the global-shortcut plugin).

export type ShortcutAction = "area" | "edit" | "style" | "pause" | "hide" | "toolbar";
export type Shortcuts = Record<ShortcutAction, string>;

// Order shown on the Shortcuts screen.
export const shortcutActions: ShortcutAction[] = ["toolbar", "area", "edit", "style", "pause", "hide"];

export const defaultShortcuts: Shortcuts = {
  area: "Ctrl+Shift+A",
  edit: "Ctrl+Shift+E",
  style: "Ctrl+Shift+S",
  pause: "Ctrl+Shift+T",
  hide: "Ctrl+Shift+H",
  toolbar: "Ctrl+Shift+B",
};

// Keys that can end a shortcut, by KeyboardEvent.code, with their name.
// Kept to keys every layout has and the plugin understands.
function keyName(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^F([1-9]|1[0-2])$/.test(code)) return code;
  const named = ["Space", "Insert", "Delete", "Home", "End", "PageUp", "PageDown", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"];
  return named.includes(code) ? code : null;
}

// Result of pressing keys while a shortcut is being recorded.
export type Recorded = { keys: string } | { waiting: true } | { error: "needsModifier" | "unsupportedKey" };

// Turns a key press into a shortcut. Only modifiers so far: keep waiting.
// Letters, digits and the like need Ctrl, Alt or Win, or the shortcut would
// fire while typing; F1-F12 may go alone.
export function recordKey(e: KeyboardEvent): Recorded {
  if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return { waiting: true };
  const key = keyName(e.code);
  if (!key) return { error: "unsupportedKey" };
  const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean);
  const isFunctionKey = /^F\d+$/.test(key);
  if (!isFunctionKey && !mods.some((m) => m !== "Shift")) return { error: "needsModifier" };
  return { keys: [...mods, key].join("+") };
}

// The action already using these keys, if any (other than `except`).
export function takenBy(shortcuts: Shortcuts, keys: string, except: ShortcutAction): ShortcutAction | null {
  const same = (a: string) => a.toLowerCase() === keys.toLowerCase();
  return shortcutActions.find((a) => a !== except && shortcuts[a] && same(shortcuts[a])) ?? null;
}

// Keys shown in a tooltip: " (Ctrl+Shift+A)", or nothing when unset.
export function keyHint(keys: string | undefined): string {
  return keys ? ` (${keys})` : "";
}
