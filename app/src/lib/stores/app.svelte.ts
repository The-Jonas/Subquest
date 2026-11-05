// Global UI state: settings and which screen is open.
import { getSettings, saveSettings, defaultSettings, coreStatus, type Settings, type CoreStatus } from "../api";
import { setLocale, type Locale } from "../i18n/index.svelte";

export type Screen = "play" | "profiles" | "fonts" | "settings" | "tutorial";

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
}

// Saves the current settings and applies the UI language.
export async function persistSettings() {
  setLocale(app.settings.uiLanguage as Locale);
  await saveSettings($state.snapshot(app.settings));
}
