// api.ts — chamadas ao lado Rust (comandos Tauri).
//
// Fora do Tauri (rodando só `npm run dev` no navegador) as funções devolvem
// dados de exemplo. Assim dá para mexer no visual sem compilar Rust nem C++.
import { invoke } from "@tauri-apps/api/core";
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

export interface Settings {
  uiLanguage: string;
  sourceLanguage: string;
  targetLanguage: string;
  ocrEngine: string;
  translator: { kind: string; baseUrl: string; model: string; apiKey: string };
  overlay: { fontSize: number; backgroundOpacity: number; showOriginal: boolean };
  tutorialDone: boolean;
}

export interface OcrTextEvent {
  text: string;
  frameId: number;
  ocrMs: number;
}

// Verdadeiro quando a página roda dentro do app Tauri.
export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// Padrões espelhados de settings.rs, usados no modo navegador.
export const defaultSettings: Settings = {
  uiLanguage: "pt-BR",
  sourceLanguage: "en",
  targetLanguage: "pt-BR",
  ocrEngine: "windows",
  translator: { kind: "openai-compatible", baseUrl: "http://localhost:11434/v1", model: "", apiKey: "" },
  overlay: { fontSize: 22, backgroundOpacity: 0.78, showOriginal: false },
  tutorialDone: false,
};

const mockWindows: GameWindow[] = [
  { handle: 1, title: "Hollow Knight", process: "hollow_knight.exe" },
  { handle: 2, title: "OMORI", process: "OMORI.exe" },
  { handle: 3, title: "Discord", process: "Discord.exe" },
];

// Situação do núcleo C++ (DLL carregada ou não).
export async function coreStatus(): Promise<CoreStatus> {
  if (!inTauri) return { loaded: false, apiVersion: null, error: "modo navegador" };
  return invoke("core_status");
}

// Janelas abertas que podem ser capturadas.
export async function listWindows(): Promise<GameWindow[]> {
  if (!inTauri) return mockWindows;
  return invoke("list_windows");
}

// Inicia a tradução; `region` = [x, y, largura, altura] ou null para a janela inteira.
export async function startSession(windowHandle: number, region: [number, number, number, number] | null) {
  if (!inTauri) return;
  await invoke("start_session", { windowHandle, region });
}

// Para a tradução.
export async function stopSession() {
  if (!inTauri) return;
  await invoke("stop_session");
}

// Lê as configurações salvas.
export async function getSettings(): Promise<Settings> {
  if (!inTauri) return structuredClone(defaultSettings);
  return invoke("get_settings");
}

// Grava as configurações.
export async function saveSettings(settings: Settings) {
  if (!inTauri) return;
  await invoke("save_settings", { settings });
}

// Escuta as falas reconhecidas pelo OCR; devolve a função que cancela a escuta.
export async function onOcrText(handler: (e: OcrTextEvent) => void): Promise<UnlistenFn> {
  if (!inTauri) return () => {};
  return listen<OcrTextEvent>("ocr-text", (event) => handler(event.payload));
}
