// Global UI state: settings and which screen is open.
import { listen } from "@tauri-apps/api/event";
import { getSettings, saveSettings, defaultSettings, coreStatus, inTauri, type Settings, type CoreStatus } from "../api";
import { setLocale, type Locale } from "../i18n/index.svelte";
import { watchStyles } from "./styles.svelte";
import { startTour } from "./tour.svelte";

export type Screen = "play" | "games" | "history" | "settings";
export type SettingsTab = "translation" | "look" | "shortcuts" | "fonts" | "storage";

export const app = $state({
  screen: "play" as Screen,
  settingsTab: "translation" as SettingsTab, // open part of the Settings screen
  settings: structuredClone(defaultSettings) as Settings,
  core: { loaded: false, apiVersion: null, error: null } as CoreStatus,
  ready: false,
});

// Loads settings and core status; starts the guided tour the first time.
export async function boot() {
  const [settings, core] = await Promise.all([getSettings(), coreStatus()]);
  app.settings = settings;
  app.core = core;
  setLocale(settings.uiLanguage as Locale);
  app.ready = true;
  if (!settings.tutorialDone) startTour();
  void watchStyles();
  // Areas marked from the in-game toolbar change the settings from outside this window.
  if (inTauri) void listen<Settings>("settings-changed", (e) => (app.settings = e.payload));
}

// Saves the current settings and applies the UI language.
export async function persistSettings() {
  setLocale(app.settings.uiLanguage as Locale);
  await saveSettings($state.snapshot(app.settings));
}
