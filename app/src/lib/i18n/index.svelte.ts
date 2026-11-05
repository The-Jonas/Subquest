// i18n — traduções da interface.
//
// Para adicionar um idioma: criar `locales/<tag>.json` com as mesmas chaves
// do pt-BR.json e registrar abaixo em `locales` e `localeNames`.
import ptBR from "./locales/pt-BR.json";
import en from "./locales/en.json";
import es from "./locales/es.json";

export type Locale = "pt-BR" | "en" | "es";

const locales: Record<Locale, unknown> = { "pt-BR": ptBR, en, es };

// Nome de cada idioma escrito nele mesmo (é assim que a pessoa o procura).
export const localeNames: Record<Locale, string> = {
  "pt-BR": "Português",
  en: "English",
  es: "Español",
};

let current = $state<Locale>("pt-BR");

// Idioma atual (reativo: componentes que leem isto atualizam sozinhos).
export function locale(): Locale {
  return current;
}

// Troca o idioma da interface.
export function setLocale(next: Locale) {
  current = next;
  document.documentElement.lang = next;
}

// Navega por um caminho "a.b.c" dentro do JSON do idioma.
function lookup(tree: unknown, path: string): unknown {
  return path.split(".").reduce<unknown>(
    (node, key) => (node && typeof node === "object" ? (node as Record<string, unknown>)[key] : undefined),
    tree,
  );
}

// Texto traduzido; troca {nome} pelos valores em `params`.
// Se faltar a chave no idioma atual, cai para o pt-BR e depois para a própria chave.
export function t(key: string, params: Record<string, string> = {}): string {
  const value = lookup(locales[current], key) ?? lookup(locales["pt-BR"], key);
  if (typeof value !== "string") return key;
  return value.replace(/\{(\w+)\}/g, (_, name: string) => params[name] ?? `{${name}}`);
}

// Valor bruto (listas e objetos), ex.: as páginas do tutorial.
export function tRaw<T>(key: string): T {
  return (lookup(locales[current], key) ?? lookup(locales["pt-BR"], key)) as T;
}
