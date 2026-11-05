// styles.svelte.ts — the text styles of the game on screen (see styles.rs).
// Keeps the list fresh while a session finds new styles, counts the unseen
// ones for the sidebar, and gives every new style a font right away: the
// installed font that looks most like its print, so nothing waits for the user.
import { listen } from "@tauri-apps/api/event";
import { inTauri, listStyles, stylePrint, updateStyle, type TextStyle } from "../api";
import { builtInFonts, importedOption, loadAllImported, systemFonts } from "../fonts";
import { closestFont } from "../fontMatcher";

export const styles = $state({
  key: null as string | null, // game shown (the one being translated, or picked in the Styles screen)
  list: [] as TextStyle[],
  unseen: 0, // styles not yet seen in the Styles screen
});

const tried = new Set<string>(); // styles the matcher already looked at (game + id)
let matching = false;
let watching = false;

// Reloads the shown game's styles and matches fonts for the new ones.
async function refresh() {
  const key = styles.key;
  if (!key) return;
  const list = await listStyles(key).catch(() => [] as TextStyle[]);
  if (styles.key !== key) return; // another game was picked meanwhile
  styles.list = list;
  styles.unseen = list.filter((s) => s.new).length;
  void autoMatch(key);
}

// Gives every style without a font the closest installed one.
async function autoMatch(key: string) {
  if (matching) return;
  const todo = styles.list.filter((s) => !s.font && !s.autoFont && !tried.has(`${key}/${s.id}`));
  if (!todo.length) return;
  matching = true;
  try {
    const imported = await loadAllImported();
    const options = [...builtInFonts, ...systemFonts, ...imported.map(importedOption)];
    await Promise.all(options.map((o) => document.fonts.load(`64px ${o.family}`).catch(() => [])));
    for (const s of todo) {
      tried.add(`${key}/${s.id}`);
      const print = await stylePrint(key, s.id).catch(() => null);
      if (!print) continue;
      const best = await closestFont(print, s.textRgb, s.sample, options);
      if (!best) continue;
      // The default font is stored by name: "" would read as "not matched yet".
      const value = best.value || "Atkinson Hyperlegible Next Variable";
      await updateStyle(key, s.id, { autoFont: value });
      s.autoFont = value;
    }
  } finally {
    matching = false;
  }
}

// Shows a game's styles (and keeps following them while they change).
export async function showStyles(key: string) {
  styles.key = key;
  await refresh();
}

// Listens for styles found by the running session (once, at startup).
export async function watchStyles() {
  if (!inTauri || watching) return;
  watching = true;
  await listen("styles-changed", () => void refresh());
}
