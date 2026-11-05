// api.ts — chamadas ao lado Rust (comandos Tauri).
//
// Fora do Tauri (rodando só `npm run dev` no navegador) as funções devolvem
// dados de exemplo. Assim dá para mexer no visual sem compilar Rust nem C++.
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

export interface Settings {
  uiLanguage: string;
  sourceLanguage: string;
  targetLanguage: string;
  ocrEngine: string;
  translator: { kind: string; baseUrl: string; model: string; apiKey: string };
  overlay: { fontSize: number; backgroundOpacity: number; showOriginal: boolean };
  tutorialDone: boolean;
}

// Erro vindo do Rust: `kind` é a chave em `errors.<kind>` nos arquivos de idioma.
export interface AppError {
  kind: string;
  detail: string;
}

// Quadro da miniatura: imagem reduzida + tamanho real da captura (para converter coordenadas).
export interface PreviewFrame {
  image: ImageData;
  sourceWidth: number;
  sourceHeight: number;
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

// Extrai o `kind` de qualquer erro (do Rust ou não) e registra o detalhe no console.
export function errorKind(e: unknown): string {
  if (e && typeof e === "object" && "kind" in e) {
    const err = e as AppError;
    console.warn(`[subquest] ${err.kind}: ${err.detail}`);
    return err.kind;
  }
  console.warn("[subquest]", e);
  return "unknown";
}

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

// Lê o cabeçalho de 16 bytes (4 x u32: largura, altura, largura real, altura real)
// seguido dos pixels RGBA, no formato montado por `on_preview_frame` no Rust.
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

// Quadro falso para o modo navegador: céu, chão e uma "caixa de diálogo".
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

// Liga a miniatura ao vivo; `onFrame` recebe cada quadro. Devolve a função que desliga.
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

// Escuta as falas reconhecidas pelo OCR; devolve a função que cancela a escuta.
export async function onOcrText(handler: (e: OcrTextEvent) => void): Promise<UnlistenFn> {
  if (!inTauri) return () => {};
  return listen<OcrTextEvent>("ocr-text", (event) => handler(event.payload));
}
