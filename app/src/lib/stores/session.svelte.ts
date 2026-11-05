// session.svelte.ts — the translation session, kept outside the Play screen so
// it keeps running while the user visits Profiles, Fonts or Settings (and so
// those screens can change the look of a game while it is being translated).
import {
  applyAreas as applyAreasNow,
  onOcrText,
  onTranslation,
  profileKey,
  startSession,
  stopSession,
  type GameWindow,
  type OcrTextEvent,
  type TranslationEvent,
} from "../api";

const FEED_SIZE = 6; // recognized lines kept for the Play screen's feed

export const session = $state({
  running: false,
  game: null as GameWindow | null,
  lines: [] as OcrTextEvent[], // newest first
  translations: {} as Record<number, TranslationEvent>, // by frame id
});

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
  try {
    unlisteners.push(
      await onOcrText((e) => {
        if (e.text) session.lines = [e, ...session.lines].slice(0, FEED_SIZE);
      }),
    );
    unlisteners.push(await onTranslation((e) => (session.translations[e.frameId] = e)));
    await startSession(game.handle, profileKey(game));
    session.game = game;
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
