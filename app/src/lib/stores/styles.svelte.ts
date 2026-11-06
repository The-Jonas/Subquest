// styles.svelte.ts — the text styles of the game on screen (see styles.rs).
// Styles are taken by the user: a print of a phrase in the game, then a font
// for it. When a print is taken from the toolbar, Subquest comes to the front
// on the Styles screen with that style's font search open.
import { listen } from "@tauri-apps/api/event";
import { inTauri, listStyles, type TextStyle } from "../api";
import { app } from "./app.svelte";

export const styles = $state({
  key: null as string | null, // game shown (the one being translated, or picked in the Styles screen)
  list: [] as TextStyle[],
  focus: null as string | null, // style just taken: its font search opens by itself
});

let watching = false;

// Reloads the shown game's styles.
async function refresh() {
  const key = styles.key;
  if (!key) return;
  const list = await listStyles(key).catch(() => [] as TextStyle[]);
  if (styles.key === key) styles.list = list; // another game may have been picked meanwhile
}

// Shows a game's styles (and keeps following them while they change).
export async function showStyles(key: string) {
  styles.key = key;
  await refresh();
}

// Listens for styles changing and for prints taken in the game (once, at startup).
export async function watchStyles() {
  if (!inTauri || watching) return;
  watching = true;
  await listen("styles-changed", () => void refresh());
  await listen<string>("style-captured", async (e) => {
    await refresh();
    styles.focus = e.payload;
    app.screen = "games";
  });
  await listen<string>("navigate", (e) => (app.screen = e.payload as typeof app.screen));
}
