// Estado global da interface: configurações e qual tela está aberta.
import { getSettings, saveSettings, defaultSettings, coreStatus, type Settings, type CoreStatus } from "../api";
import { setLocale, type Locale } from "../i18n/index.svelte";

export type Screen = "play" | "profiles" | "settings" | "tutorial";

export const app = $state({
  screen: "play" as Screen,
  settings: structuredClone(defaultSettings) as Settings,
  core: { loaded: false, apiVersion: null, error: null } as CoreStatus,
  ready: false,
});

// Carrega configurações e status do núcleo; abre o tutorial na primeira vez.
export async function boot() {
  const [settings, core] = await Promise.all([getSettings(), coreStatus()]);
  app.settings = settings;
  app.core = core;
  setLocale(settings.uiLanguage as Locale);
  if (!settings.tutorialDone) app.screen = "tutorial";
  app.ready = true;
}

// Grava as configurações atuais e aplica o idioma da interface.
export async function persistSettings() {
  setLocale(app.settings.uiLanguage as Locale);
  await saveSettings($state.snapshot(app.settings));
}
