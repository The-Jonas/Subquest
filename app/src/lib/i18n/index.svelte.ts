// i18n — UI translations.
//
// To add a language: create `locales/<tag>.json` with the same keys as
// pt-BR.json and register it below in `locales` and `localeNames`.
import ptBR from "./locales/pt-BR.json";
import en from "./locales/en.json";
import es from "./locales/es.json";

export type Locale = "pt-BR" | "en" | "es";

const locales: Record<Locale, unknown> = { "pt-BR": ptBR, en, es };

// Each language's name written in itself (that's how people look for it).
export const localeNames: Record<Locale, string> = {
  "pt-BR": "Português",
  en: "English",
  es: "Español",
};

let current = $state<Locale>("pt-BR");

// Current language (reactive: components reading it update on their own).
export function locale(): Locale {
  return current;
}

// Switches the UI language.
export function setLocale(next: Locale) {
  current = next;
  document.documentElement.lang = next;
}

// Walks an "a.b.c" path inside the locale JSON.
function lookup(tree: unknown, path: string): unknown {
  return path.split(".").reduce<unknown>(
    (node, key) => (node && typeof node === "object" ? (node as Record<string, unknown>)[key] : undefined),
    tree,
  );
}

// Translated text; replaces {name} with the values in `params`.
// If the key is missing in the current language, falls back to pt-BR, then to the key itself.
export function t(key: string, params: Record<string, string> = {}): string {
  const value = lookup(locales[current], key) ?? lookup(locales["pt-BR"], key);
  if (typeof value !== "string") return key;
  return value.replace(/\{(\w+)\}/g, (_, name: string) => params[name] ?? `{${name}}`);
}

// Raw value (lists and objects), e.g. the tutorial pages.
export function tRaw<T>(key: string): T {
  return (lookup(locales[current], key) ?? lookup(locales["pt-BR"], key)) as T;
}
