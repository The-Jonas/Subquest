// Global UI state: settings and which screen is open.
import { listen } from "@tauri-apps/api/event";
import { getSettings, saveSettings, defaultSettings, coreStatus, inTauri, type Settings, type CoreStatus } from "../api";
import { setLocale, type Locale } from "../i18n/index.svelte";
import { watchStyles } from "./styles.svelte";

export type Screen = "play" | "history" | "styles" | "profiles" | "fonts" | "settings" | "shortcuts" | "tutorial";

export const app = $state({
  screen: "play" as Screen,
  settings: structuredClone(defaultSettings) as Settings,
  core: { loaded: false, apiVersion: null, error: null } as CoreStatus,
  ready: false,
});

// Loads settings and core status; opens the tutorial the first time.
export async function boot() {
  const [settings, core] = await Promise.all([getSettings(), coreStatus()]);
  app.settings = settings;
  app.core = core;
  setLocale(settings.uiLanguage as Locale);
  if (!settings.tutorialDone) app.screen = "tutorial";
  app.ready = true;
  void watchStyles();
  // Areas marked from the in-game toolbar change the settings from outside this window.
  if (inTauri) void listen<Settings>("settings-changed", (e) => (app.settings = e.payload));
}

// Saves the current settings and applies the UI language.
export async function persistSettings() {
  setLocale(app.settings.uiLanguage as Locale);
  await saveSettings($state.snapshot(app.settings));
}
