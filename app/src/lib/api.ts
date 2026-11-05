// api.ts — calls into the Rust side (Tauri commands).
//
// Outside Tauri (running just `npm run dev` in the browser) the functions return
// sample data, so the UI can be worked on without building Rust or C++.
import { invoke, Channel } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface CoreStatus {
  loaded: boolean;
  apiVersion: number | null;
  error: string | null;
}

export interface GameWindow {
  handle: number;
  title: string;
  process: string;
}

// [x, y, width, height] in capture pixels (the same space the core crops in).
export type Region = [number, number, number, number];

export type AreaMode = "dialogue" | "screen";

export type AreaStyle = "inPlace" | "classic";

// One area of the game screen (mirrors GameArea in settings.rs). Games use
// different fonts and colors for dialogue, songs, names..., so the look lives here.
export interface GameArea {
  name: string;
  mode: AreaMode;
  region: Region | null;
  font: string; // family name, "imported:<file>", or "" for the default
  style: AreaStyle;
}

// What Subquest remembers about each game (mirrors GameProfile in settings.rs).
export interface GameProfile {
  title: string;
  areas: GameArea[]; // the UI uses the first one for now
}

// A new area with the most common setup: dialogue box, game colors, default font.
export function newArea(): GameArea {
  return { name: "", mode: "dialogue", region: null, font: "", style: "inPlace" };
}

export interface Settings {
  uiLanguage: string;
  sourceLanguage: string;
  targetLanguage: string;
  ocrEngine: string;
  translator: { kind: string; baseUrl: string; model: string; apiKey: string };
  overlay: { fontSize: number; backgroundOpacity: number; showOriginal: boolean };
  tutorialDone: boolean;
  profiles: Record<string, GameProfile>; // key: lowercase executable name
}

// Error coming from Rust: `kind` is the key in `errors.<kind>` in the locale files.
export interface AppError {
  kind: string;
  detail: string;
}

// Preview frame: downscaled image + real capture size (to convert coordinates).
export interface PreviewFrame {
  image: ImageData;
  sourceWidth: number;
  sourceHeight: number;
}

export interface OcrTextEvent {
  text: string;
  frameId: number;
  ocrMs: number; // recognition only
  totalMs: number; // from frame capture until the text was ready
  boxColor: string; // dialogue box color sampled from the game ("#rrggbb")
  textColor: string; // text color sampled from the game ("#rrggbb")
  frameWidth: number; // captured area size: line rects are in these pixels
  frameHeight: number;
  lines: OcrLine[]; // empty (with empty text) when the text left the screen
  sameAs?: number | null; // this is that earlier line read again: keep its translation
}

// One recognized line, with its place and its own colors.
export interface OcrLine {
  text: string;
  rect: Region; // [x, y, width, height] inside the captured area
  boxColor: string; // color right around the line
  textColor: string; // color of the line's letters
}

// True when the page runs inside the Tauri app.
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// Defaults mirrored from settings.rs, used in browser mode.
export const defaultSettings: Settings = {
  uiLanguage: "pt-BR",
  sourceLanguage: "en",
  targetLanguage: "pt-BR",
  ocrEngine: "windows",
  translator: { kind: "openai-compatible", baseUrl: "https://generativelanguage.googleapis.com/v1beta/openai/", model: "", apiKey: "" },
  overlay: { fontSize: 22, backgroundOpacity: 0.78, showOriginal: false },
  tutorialDone: false,
  profiles: {},
};

// Profile key for a window: the executable name survives restarts, the HWND doesn't.
export function profileKey(w: GameWindow): string {
  return (w.process || w.title).toLowerCase();
}

const mockWindows: GameWindow[] = [
  { handle: 1, title: "Hollow Knight", process: "hollow_knight.exe" },
  { handle: 2, title: "OMORI", process: "OMORI.exe" },
  { handle: 3, title: "Discord", process: "Discord.exe" },
];

// Technical detail of an error (the API's own message), for screens where the user is configuring things.
export function errorDetail(e: unknown): string {
  if (e && typeof e === "object" && "detail" in e) return String((e as AppError).detail);
  return String(e);
}

// Extracts the `kind` from any error (from Rust or not) and logs the detail to the console.
export function errorKind(e: unknown): string {
  if (e && typeof e === "object" && "kind" in e) {
    const err = e as AppError;
    console.warn(`[subquest] ${err.kind}: ${err.detail}`);
    return err.kind;
  }
  console.warn("[subquest]", e);
  return "unknown";
}

// Status of the C++ core (DLL loaded or not).
export async function coreStatus(): Promise<CoreStatus> {
  if (!inTauri) return { loaded: false, apiVersion: null, error: "modo navegador" };
  return invoke("core_status");
}

// Open windows that can be captured.
export async function listWindows(): Promise<GameWindow[]> {
  if (!inTauri) return mockWindows;
  return invoke("list_windows");
}

// Starts translating; `region` = [x, y, width, height] or null for the whole window.
export async function startSession(windowHandle: number, region: Region | null, profileKey: string) {
  if (!inTauri) return;
  await invoke("start_session", { windowHandle, region, profileKey });
}

// Profile key of the game being translated (the overlay styles itself from it).
export async function activeProfile(): Promise<string | null> {
  if (!inTauri) return null;
  return invoke("active_profile");
}

// A font imported by the user into the app's folder.
export interface ImportedFont {
  file: string; // id
  name: string; // display name
}

let browserFonts: ImportedFont[] = []; // browser mode keeps imports in memory only
const browserFontBytes = new Map<string, ArrayBuffer>();

// Lists the imported fonts.
export async function listFonts(): Promise<ImportedFont[]> {
  if (!inTauri) return [...browserFonts];
  return invoke("list_fonts");
}

// Imports a font file chosen by the user; the bytes travel raw, the name in a header.
export async function importFont(file: File): Promise<ImportedFont> {
  const bytes = await file.arrayBuffer();
  if (!inTauri) {
    const entry = { file: file.name, name: file.name.replace(/\.[^.]+$/, "") };
    browserFonts = [...browserFonts.filter((f) => f.file !== entry.file), entry];
    browserFontBytes.set(entry.file, bytes);
    return entry;
  }
  return invoke("import_font", new Uint8Array(bytes), {
    headers: { "x-file-name": encodeURIComponent(file.name) },
  });
}

// Bytes of an imported font, to register it with the FontFace API.
export async function readFont(file: string): Promise<ArrayBuffer> {
  if (!inTauri) return browserFontBytes.get(file) ?? new ArrayBuffer(0);
  return invoke("read_font", { file });
}

// Deletes an imported font.
export async function removeFont(file: string): Promise<void> {
  if (!inTauri) {
    browserFonts = browserFonts.filter((f) => f.file !== file);
    return;
  }
  await invoke("remove_font", { file });
}

// A Google Font suggested for a game, downloaded and waiting for the user's choice.
export interface FontCandidate {
  family: string;
  weight: number;
  reason: string; // why it matches, in the UI language
  file: string; // id; also its file name once kept
}

// What the model read in the crop and the fonts it suggests, best match first.
export interface FontDetection {
  text: string;
  candidates: FontCandidate[];
}

// Asks the translator model which Google Fonts look like the game's text (PNG crop).
export async function detectFont(png: Blob): Promise<FontDetection> {
  if (!inTauri) {
    // Browser mode: built-in fonts stand in for the downloads.
    await new Promise((r) => setTimeout(r, 900));
    return {
      text: "Seas reflect her gleaming beauty",
      candidates: [
        { family: "Cinzel", weight: 600, reason: "serifas clássicas, maiúsculas largas", file: "Cinzel-600.ttf" },
        { family: "Crimson Pro", weight: 400, reason: "serifa de livro, traço fino", file: "CrimsonPro-400.ttf" },
      ],
    };
  }
  return invoke("detect_font", new Uint8Array(await png.arrayBuffer()));
}

// Bytes of a suggested font, to show a sample before keeping it.
export async function candidateFont(file: string): Promise<ArrayBuffer> {
  if (!inTauri) return new ArrayBuffer(0);
  return invoke("candidate_font", { file });
}

// Keeps a suggested font in the Fonts folder; returns it as an imported font.
export async function keepCandidateFont(file: string): Promise<ImportedFont> {
  if (!inTauri) {
    const entry = { file, name: file.replace(/\.[^.]+$/, "") };
    browserFonts = [...browserFonts.filter((f) => f.file !== file), entry];
    return entry;
  }
  return invoke("keep_candidate_font", { file });
}

// Stops translating.
export async function stopSession() {
  if (!inTauri) return;
  await invoke("stop_session");
}

// Reads the saved settings.
export async function getSettings(): Promise<Settings> {
  if (!inTauri) return structuredClone(defaultSettings);
  return invoke("get_settings");
}

// Saves the settings.
export async function saveSettings(settings: Settings) {
  if (!inTauri) return;
  await invoke("save_settings", { settings });
}

// Reads the 16-byte header (4 x u32: width, height, source width, source height)
// followed by the RGBA pixels, in the layout built by `on_preview_frame` in Rust.
function decodePreview(buffer: ArrayBuffer): PreviewFrame {
  const header = new DataView(buffer, 0, 16);
  const width = header.getUint32(0, true);
  const height = header.getUint32(4, true);
  const pixels = new Uint8ClampedArray(buffer, 16, width * height * 4);
  return {
    image: new ImageData(pixels, width, height),
    sourceWidth: header.getUint32(8, true),
    sourceHeight: header.getUint32(12, true),
  };
}

// Fake frame for browser mode: a sky and a "dialogue box".
function fakePreview(width: number): PreviewFrame {
  const height = Math.round((width * 9) / 16);
  const canvas = new OffscreenCanvas(width, height);
  const ctx = canvas.getContext("2d")!;
  const sky = ctx.createLinearGradient(0, 0, 0, height);
  sky.addColorStop(0, "#2b1d3f");
  sky.addColorStop(1, "#6b2f3a");
  ctx.fillStyle = sky;
  ctx.fillRect(0, 0, width, height);
  ctx.fillStyle = "rgb(8 8 14 / 0.85)";
  ctx.fillRect(width * 0.12, height * 0.68, width * 0.76, height * 0.24);
  ctx.fillStyle = "#e8dcc0";
  ctx.font = `${Math.round(height * 0.05)}px serif`;
  ctx.fillText("You came back. Of course you did.", width * 0.16, height * 0.79);
  return { image: ctx.getImageData(0, 0, width, height), sourceWidth: 1920, sourceHeight: 1080 };
}

// Starts the live preview; `onFrame` gets every frame. Returns the function that stops it.
export async function startPreview(
  windowHandle: number,
  maxWidth: number,
  onFrame: (frame: PreviewFrame) => void,
): Promise<() => Promise<void>> {
  if (!inTauri) {
    onFrame(fakePreview(maxWidth));
    return async () => {};
  }
  const channel = new Channel<ArrayBuffer>();
  channel.onmessage = (data) => onFrame(decodePreview(data));
  await invoke("start_preview", { windowHandle, maxWidth, channel });
  return async () => {
    await invoke("stop_preview");
  };
}

// Translation progress for one OCR line (streamed: several events, the last with done = true).
export interface TranslationEvent {
  frameId: number;
  text: string;
  done: boolean;
  cached: boolean;
  elapsedMs: number; // since the OCR text arrived
  errorKind: string | null;
}

// Ready-made addresses for the services people actually use. All of them speak
// the OpenAI chat format, so only the address changes.
export const translatorPresets = [
  { id: "gemini", baseUrl: "https://generativelanguage.googleapis.com/v1beta/openai/", needsKey: true },
  { id: "openai", baseUrl: "https://api.openai.com/v1", needsKey: true },
  { id: "openrouter", baseUrl: "https://openrouter.ai/api/v1", needsKey: true },
  { id: "ollama", baseUrl: "http://localhost:11434/v1", needsKey: false },
] as const;

export type TranslatorSettings = Settings["translator"];

// Translates a sample line with the given settings; resolves with the text and the time it took.
export async function testTranslator(
  translator: TranslatorSettings,
  sourceLanguage: string,
  targetLanguage: string,
): Promise<{ text: string; elapsedMs: number }> {
  if (!inTauri) {
    await new Promise((r) => setTimeout(r, 600));
    return { text: "A ponte caiu. Você vai ter que achar outro jeito de atravessar.", elapsedMs: 612 };
  }
  return invoke("test_translator", { translator, sourceLanguage, targetLanguage });
}

// Model ids offered by the configured service.
export async function listModels(translator: TranslatorSettings): Promise<string[]> {
  if (!inTauri) return ["gemini-flash-latest", "gemini-flash-lite-latest", "gemini-pro-latest"];
  return invoke("list_models", { translator });
}

// Listens for translation progress; returns the function that stops listening.
export async function onTranslation(handler: (e: TranslationEvent) => void): Promise<UnlistenFn> {
  if (!inTauri) {
    fakeTranslationHandler = handler;
    return () => (fakeTranslationHandler = null);
  }
  return listen<TranslationEvent>("translation", (event) => handler(event.payload));
}

// Browser mode: the fake OCR feed below also "translates", word by word.
let fakeTranslationHandler: ((e: TranslationEvent) => void) | null = null;
const fakeTranslations: Record<string, string> = {
  "Mmmm, mmmmrrnnnn...": "Hmmm, hmmmrrnnnn...",
  "You came back. Of course you did.": "Você voltou. É claro que voltou.",
  "The Crossroads are quiet tonight, Princess.": "As Encruzilhadas estão quietas esta noite, Princesa.",
};

// Streams a fake translation word by word, like the real backend does.
function fakeTranslate(frameId: number, source: string) {
  const words = (fakeTranslations[source] ?? source).split(" ");
  words.forEach((_, i) =>
    setTimeout(() => {
      const done = i === words.length - 1;
      fakeTranslationHandler?.({
        frameId,
        text: words.slice(0, i + 1).join(" "),
        done,
        cached: false,
        elapsedMs: 180 + i * 90,
        errorKind: null,
      });
    }, 180 + i * 90),
  );
}

// Browser mode: emits a few sample lines so the running screen can be designed.
function fakeOcrFeed(handler: (e: OcrTextEvent) => void): UnlistenFn {
  const samples = [
    "Mmmm, mmmmrrnnnn...",
    "You came back. Of course you did.",
    "The Crossroads are quiet tonight, Princess.",
  ];
  let frameId = 0;
  const timer = setInterval(() => {
    const text = samples[frameId % samples.length];
    frameId += 1;
    handler({
      text,
      frameId,
      ocrMs: 9 + Math.random() * 6,
      totalMs: 11 + Math.random() * 8,
      boxColor: "#14181f",
      textColor: frameId % 3 === 2 ? "#5fb56b" : "#e9e4d6",
      frameWidth: 1460,
      frameHeight: 281,
      lines: [{ text, rect: [120, 110, 760, 44], boxColor: "#14181f", textColor: "#e9e4d6" }],
    });
    fakeTranslate(frameId, text);
  }, 1800);
  return () => clearInterval(timer);
}

// Listens for lines recognized by OCR; returns the function that stops listening.
export async function onOcrText(handler: (e: OcrTextEvent) => void): Promise<UnlistenFn> {
  if (!inTauri) return fakeOcrFeed(handler);
  return listen<OcrTextEvent>("ocr-text", (event) => handler(event.payload));
}
