// session.svelte.ts — the translation session, kept outside the Play screen so
// it keeps running while the user visits Profiles, Fonts or Settings (and so
// those screens can change the look of a game while it is being translated).
//
// It also keeps the session's history: every line with its translation, and
// whether it reached the screen in time. A line "missed" its moment when the
// game moved on (new line, or the text left) before any translation arrived:
// those are the ones the History screen is for.
import { app } from "./app.svelte";
import type { LineTiming } from "../latency";
import { showStyles } from "./styles.svelte";
import { listen } from "@tauri-apps/api/event";
import {
  inTauri,
  applyAreas as applyAreasNow,
  onOcrText,
  onTranslation,
  profileKey,
  startSession,
  stopSession,
  type GameWindow,
  type IconPicture,
  type OcrTextEvent,
  type TranslationEvent,
} from "../api";

const FEED_SIZE = 6; // recognized lines kept for the Play screen's feed
const HISTORY_SIZE = 500; // lines kept in the History screen

// One line of the session, as the History screen shows it.
export interface HistoryEntry {
  frameId: number;
  area: number;
  areaName: string; // the area's name when the line was read ("" = unnamed)
  original: string;
  translation: string;
  quick: boolean; // the translation on screen came from the quick layer
  cached: boolean; // came straight from the cache (seen before)
  terms: string[]; // glossary terms found in the line
  rereads: number; // times OCR read this line again (the same translation was used)
  timing: LineTiming; // where the wait for its translation went
  icons: IconPicture[]; // pictures inside the line ("{1}" in its texts)
  done: boolean;
  failed: boolean;
  missed: boolean; // the game moved on before any translation was ready
  at: number; // when the line was read (ms since epoch)
}

export const session = $state({
  running: false,
  game: null as GameWindow | null,
  lines: [] as OcrTextEvent[], // newest first
  translations: {} as Record<number, TranslationEvent>, // by frame id
  history: [] as HistoryEntry[], // newest first; survives stopping, cleared when a session starts
  unseenMissed: 0, // missed lines since the History screen was last opened
  toolbarShown: false, // the in-game bar is on screen (its button in Play says so)
});

// Marks as missed every line of an area that still had no translation when the
// game moved on (the area shows a newer line, or its text left the screen).
function moveOn(area: number, frameId: number) {
  for (const entry of session.history) {
    if (entry.area === area && entry.frameId < frameId && !entry.translation && !entry.missed && !entry.failed) {
      entry.missed = true;
      session.unseenMissed += 1;
    }
  }
}

// A line from OCR: feeds the Play screen and the history. Re-readings of the
// same line are not new lines: they only count on the line they repeat.
function onLine(e: OcrTextEvent) {
  moveOn(e.area, e.frameId);
  if (e.text && e.sameAs != null) {
    const original = session.history.find((h) => h.frameId === e.sameAs);
    if (original) original.rereads += 1;
    return;
  }
  if (!e.text) return;
  session.lines = [e, ...session.lines].slice(0, FEED_SIZE);
  // Translations are only kept for the lines still in the feed (History keeps
  // its own copy): a long session must not grow this map forever.
  const shown = new Set(session.lines.map((l) => l.frameId));
  for (const id of Object.keys(session.translations)) {
    if (!shown.has(Number(id))) delete session.translations[Number(id)];
  }
  const profile = session.game ? app.profiles[profileKey(session.game)] : undefined;
  session.history.unshift({
    frameId: e.frameId,
    area: e.area,
    areaName: profile?.areas[e.area]?.name ?? "",
    original: e.text,
    translation: "",
    quick: false,
    cached: false,
    terms: [],
    rereads: 0,
    timing: { settle: e.settleMs ?? 0, read: e.totalMs },
    icons: e.icons ?? [],
    done: false,
    failed: false,
    missed: false,
    at: Date.now(),
  });
  if (session.history.length > HISTORY_SIZE) session.history.length = HISTORY_SIZE;
}

// Translation progress: the Play feed and the line's history entry follow it.
function onProgress(e: TranslationEvent) {
  session.translations[e.frameId] = e;
  const entry = session.history.find((h) => h.frameId === e.frameId);
  if (!entry) return;
  if (e.errorKind) {
    entry.failed = true;
    entry.done = true;
    return;
  }
  if (e.text && entry.timing.first == null) entry.timing.first = e.elapsedMs; // first words on screen
  if (e.done) entry.timing.done = e.elapsedMs;
  entry.translation = e.text;
  entry.quick = e.quick;
  entry.cached = e.cached;
  entry.terms = e.terms ?? [];
  entry.done = e.done;
}

let unlisteners: (() => void)[] = [];

// Stops listening to OCR and translation events.
function unlistenAll() {
  unlisteners.forEach((fn) => fn());
  unlisteners = [];
}

// Starts translating a game with the areas saved in its profile.
export async function startTranslating(game: GameWindow) {
  session.lines = [];
  session.translations = {};
  session.history = [];
  session.unseenMissed = 0;
  session.game = game; // set before the first line arrives: the history needs its area names
  try {
    unlisteners.push(await onOcrText(onLine));
    unlisteners.push(await onTranslation(onProgress));
    await startSession(game.handle, profileKey(game));
    void showStyles(profileKey(game)); // the Styles screen follows the game being translated
    session.running = true;
  } catch (e) {
    unlistenAll();
    throw e;
  }
}

// Stops translating and hides the overlay.
export async function stopTranslating() {
  try {
    await stopSession();
  } finally {
    unlistenAll();
    session.running = false;
  }
}

// Makes the running session read the areas just saved in the game's profile.
export async function applyAreas() {
  if (session.running) await applyAreasNow();
}

// The game closed during a session: Rust already stopped everything; the UI follows.
export async function watchSession() {
  if (!inTauri) return;
  await listen("session-ended", () => {
    unlistenAll();
    session.running = false;
  });
  await listen<boolean>("toolbar-shown", (e) => (session.toolbarShown = e.payload));
}
