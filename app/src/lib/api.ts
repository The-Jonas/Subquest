// api.ts — calls into the Rust side (Tauri commands).
//
// Outside Tauri (running just `npm run dev` in the browser) the functions return
// sample data, so the UI can be worked on without building Rust or C++.
import { defaultShortcuts, type Shortcuts } from "./shortcuts";
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
  caps: LetterCase; // letter case the game uses, applied to the translation
}

// "" keeps the translation as written; the others copy the game's look.
// For areas "" keeps the translation as written. For text styles "" means "as
// in the game" (the case measured on the print) and "none" keeps it as written.
export type LetterCase = "" | "none" | "upper" | "smallCaps";

// What Subquest remembers about each game (mirrors GameProfile in settings.rs).
export interface GameProfile {
  title: string;
  areas: GameArea[]; // the UI uses the first one for now
}

// A new area with the most common setup: dialogue box, game colors, default font.
export function newArea(): GameArea {
  return { name: "", mode: "dialogue", region: null, font: "", style: "inPlace", caps: "" };
}

export type UiTheme = "graphite" | "night" | "moss" | "plum";

export interface Settings {
  uiLanguage: string;
  uiTheme: UiTheme; // colors of Subquest's own windows
  sourceLanguage: string;
  targetLanguage: string;
  ocrEngine: string;
  translator: { kind: string; baseUrl: string; model: string; visionModel: string; apiKey: string };
  quick: { engine: QuickEngine; deeplKey: string }; // second translation layer
  shortcuts: Shortcuts; // in-game keys, like "Ctrl+Shift+A" ("" = none)
  overlay: { fontSize: number; backgroundOpacity: number; showOriginal: boolean; look: AreaStyle };
  tutorialDone: boolean;
  profiles: Record<string, GameProfile>; // key: lowercase executable name
}

// Quick translator shown while the main one answers ("" = off).
export type QuickEngine = "" | "google" | "deepl";

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
  frameWidth: number; // whole game window size: line rects are in these pixels
  frameHeight: number;
  lines: OcrLine[]; // empty (with empty text) when the text left the screen; rects in window pixels
  area: number; // index of the profile area this text came from
  sameAs?: number | null; // this is that earlier line read again: keep its translation
}

// One recognized line, with its place and its own colors.
export interface OcrLine {
  text: string;
  rect: Region; // [x, y, width, height] in window pixels
  boxColor: string; // color right around the line
  textColor: string; // color of the line's letters
  style?: string | null; // id of the game's text style this line belongs to
}

// One way a game writes text, from a print the user took (mirrors TextStyle in styles.rs).
export interface TextStyle {
  id: string;
  name: string;
  textRgb: number; // letter color measured on the print, 0xRRGGBB
  font: string; // chosen font ("" = not chosen yet)
  caps: LetterCase;
  seenCase?: string | null; // case measured on the print: "", "upper" or "smallCaps"
  manual: boolean;
}

// Browser mode: one made-up style, with its print drawn on the fly.
const browserStyles: TextStyle[] = [
  { id: "s1", name: "", textRgb: 0x3de8a0, font: "", caps: "", seenCase: "smallCaps", manual: true },
];

// A game's text styles, in the order they were taken.
export async function listStyles(profileKey: string): Promise<TextStyle[]> {
  if (!inTauri) return structuredClone(browserStyles);
  return invoke("list_styles", { profileKey });
}

// PNG of a style's print (the phrase as the game drew it).
export async function stylePrint(profileKey: string, id: string): Promise<Blob> {
  if (!inTauri) {
    const s = browserStyles.find((x) => x.id === id)!;
    const canvas = document.createElement("canvas");
    canvas.width = 300;
    canvas.height = 70;
    const ctx = canvas.getContext("2d")!;
    ctx.fillStyle = "#0f2a22";
    ctx.fillRect(0, 0, canvas.width, canvas.height);
    ctx.fillStyle = "#" + s.textRgb.toString(16).padStart(6, "0");
    ctx.font = "bold 44px Georgia";
    ctx.fillText("PLAY", 24, 52);
    return new Promise((resolve) => canvas.toBlob((b) => resolve(b!), "image/png"));
  }
  const bytes = asBuffer(await invoke("style_print", { profileKey, id }));
  return new Blob([bytes], { type: "image/png" });
}

// Changes a style: its name, or its font and letter case. Left-out fields stay.
export async function updateStyle(
  profileKey: string,
  id: string,
  change: { name?: string; font?: string; caps?: LetterCase },
) {
  if (!inTauri) {
    const s = browserStyles.find((x) => x.id === id);
    if (s) Object.assign(s, change);
    return;
  }
  await invoke("update_style", { profileKey, id, ...change });
}

// What a rectangle drawn over the game is for.
export type SelectionPurpose = "area" | "style" | "edit";

// Saved translations: [count, file size in bytes].
export async function cacheUsage(): Promise<[number, number]> {
  if (!inTauri) return [1234, 180_000];
  return invoke("cache_usage");
}

// Forgets every saved translation.
export async function clearTranslationCache() {
  if (!inTauri) return;
  await invoke("clear_translation_cache");
}

// Ends editing the areas over the game: the new list (window pixels), or null to cancel.
export async function finishAreaEdit(areas: GameArea[] | null) {
  if (!inTauri) return;
  await invoke("finish_area_edit", { areas });
}

// Starts drawing a rectangle over the game (the overlay takes the mouse).
export async function beginSelection(purpose: SelectionPurpose) {
  if (!inTauri) return;
  await invoke("begin_selection", { purpose });
}

// Sends the rectangle drawn over the game, in window pixels (null = cancelled).
export async function finishSelection(purpose: SelectionPurpose, rect: Region | null) {
  if (!inTauri) return;
  await invoke("finish_selection", { purpose, rect });
}

// Pauses or resumes reading the game; returns true when now paused.
export async function togglePause(): Promise<boolean> {
  if (!inTauri) return false;
  return invoke("toggle_pause");
}

// Hides or shows the translations; returns true when now hidden.
export async function toggleHidden(): Promise<boolean> {
  if (!inTauri) return false;
  return invoke("toggle_hidden");
}

// Shows or hides the in-game toolbar; returns true when it is now shown.
export async function toggleToolbar(): Promise<boolean> {
  if (!inTauri) return false;
  return invoke("toggle_toolbar");
}

// [paused, hidden] when the toolbar opens.
export async function sessionFlags(): Promise<[boolean, boolean]> {
  if (!inTauri) return [false, false];
  return invoke("session_flags");
}

// Writes a line to <app data>/ui.log (the installed app has no console).
export function uiLog(source: string, message: string) {
  if (!inTauri) return console.warn(`[${source}] ${message}`);
  void invoke("ui_log", { source, message }).catch(() => {});
}

// Sends every uncaught error and rejected promise of a page to ui.log.
export function watchErrors(source: string) {
  window.addEventListener("error", (e) => uiLog(source, `error: ${e.message} at ${e.filename}:${e.lineno}`));
  window.addEventListener("unhandledrejection", (e) => {
    const reason = e.reason instanceof Error ? `${e.reason.message}\n${e.reason.stack ?? ""}` : String(e.reason);
    uiLog(source, `unhandled rejection: ${reason}`);
  });
}

// Raw bytes from Rust as an ArrayBuffer, whatever shape they arrive in.
// Normally an ArrayBuffer; but when the webview can't use Tauri's fast IPC
// path, the fallback delivers them as a plain array of numbers, and code
// that assumed an ArrayBuffer read garbage (broken pictures, a preview that
// never showed in the installed app).
export function asBuffer(data: unknown): ArrayBuffer {
  if (data instanceof ArrayBuffer) return data;
  if (ArrayBuffer.isView(data)) {
    return data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength) as ArrayBuffer;
  }
  if (Array.isArray(data)) return Uint8Array.from(data as number[]).buffer;
  throw new Error("unexpected binary data from the app");
}

// A picture (PNG bytes) as a data: URL for an <img>. Not a blob: URL: the
// installed app's security policy refused those, so every picture showed
// broken (in dev mode no policy applies, which hid the problem).
export function pictureUrl(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result as string);
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(blob);
  });
}

// Paints a window in the chosen theme's colors (app.css [data-theme]).
export function applyTheme(theme: string | undefined) {
  document.documentElement.dataset.theme = theme || "graphite";
}

// Shows the game above Subquest for a moment, so a game that stopped drawing
// behind Subquest's window draws again and the preview gets a picture.
export async function peekWindow(handle: number) {
  if (!inTauri) return;
  await invoke("peek_window", { windowHandle: handle });
}

// Small picture of a window for the game list (null when none came: minimized...).
export async function windowThumbnail(handle: number): Promise<Blob | null> {
  if (!inTauri) {
    const c = document.createElement("canvas");
    c.width = 320;
    c.height = 180;
    const ctx = c.getContext("2d")!;
    ctx.fillStyle = `hsl(${(handle * 67) % 360} 30% 22%)`;
    ctx.fillRect(0, 0, c.width, c.height);
    return new Promise((resolve) => c.toBlob(resolve, "image/png"));
  }
  try {
    const bytes = asBuffer(await invoke("window_thumbnail", { windowHandle: handle }));
    return new Blob([bytes], { type: "image/png" });
  } catch {
    return null;
  }
}

// Brings Subquest to the front, optionally on a screen ("styles"...).
export async function showMain(screen?: string) {
  if (!inTauri) return;
  await invoke("show_main", { screen: screen ?? null });
}

// Forgets all of a game's styles and prints (used when its profile is forgotten).
export async function forgetStyles(profileKey: string) {
  if (!inTauri) return;
  await invoke("forget_styles", { profileKey });
}

// Forgets a style and its print.
export async function removeStyle(profileKey: string, id: string) {
  if (!inTauri) return;
  await invoke("remove_style", { profileKey, id });
}

// True when the page runs inside the Tauri app.
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// Defaults mirrored from settings.rs, used in browser mode.
export const defaultSettings: Settings = {
  uiLanguage: "pt-BR",
  uiTheme: "graphite",
  sourceLanguage: "en",
  targetLanguage: "pt-BR",
  ocrEngine: "windows",
  translator: { kind: "openai-compatible", baseUrl: "https://generativelanguage.googleapis.com/v1beta/openai/", model: "", visionModel: "", apiKey: "" },
  quick: { engine: "google", deeplKey: "" },
  shortcuts: { ...defaultShortcuts },
  overlay: { fontSize: 0, backgroundOpacity: 0.78, showOriginal: false, look: "inPlace" },
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

// Starts translating every area saved in the game's profile.
export async function startSession(windowHandle: number, profileKey: string) {
  if (!inTauri) return;
  await invoke("start_session", { windowHandle, profileKey });
}

// Makes the running session read the areas saved in its profile (no restart of the overlay).
export async function applyAreas() {
  if (!inTauri) return;
  await invoke("apply_areas");
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
  return asBuffer(await invoke("read_font", { file }));
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
  caps: LetterCase; // letter case the model saw in the game
  candidates: FontCandidate[];
}

// Asks the translator model which Google Fonts look like the game's text (PNG crop).
export async function detectFont(png: Blob): Promise<FontDetection> {
  if (!inTauri) {
    // Browser mode: built-in fonts stand in for the downloads.
    await new Promise((r) => setTimeout(r, 900));
    return {
      text: "Seas reflect her gleaming beauty",
      caps: "smallCaps",
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
  return asBuffer(await invoke("candidate_font", { file }));
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
function decodePreview(data: unknown): PreviewFrame {
  const buffer = asBuffer(data);
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
  const channel = new Channel<unknown>();
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
  quick: boolean; // from the quick layer (the main translator is late or failed)
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
        quick: false,
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
      area: 0,
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
