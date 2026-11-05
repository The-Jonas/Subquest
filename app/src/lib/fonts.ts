// fonts.ts — the fonts the overlay can use, and how to load the imported ones.
//
// Three sources:
//  - built-in: bundled with the app (work offline, same on every PC)
//  - system:   fonts that ship with Windows
//  - imported: files the user added in the Fonts screen ("imported:<file>")
import { listFonts, readFont, type ImportedFont } from "./api";

export interface FontOption {
  value: string; // what is stored in the area ("" = default)
  label: string; // what the list shows
  family: string; // CSS font-family to render it
}

// Bundled fonts, chosen to cover the most common game looks.
export const builtInFonts: FontOption[] = [
  { value: "", label: "Atkinson Hyperlegible", family: "'Atkinson Hyperlegible Next Variable'" },
  { value: "Cinzel Variable", label: "Cinzel", family: "'Cinzel Variable'" },
  { value: "Crimson Pro Variable", label: "Crimson Pro", family: "'Crimson Pro Variable'" },
  { value: "Pixelify Sans Variable", label: "Pixelify Sans", family: "'Pixelify Sans Variable'" },
];

// Fonts that come with Windows 10/11.
export const systemFonts: FontOption[] = [
  "Georgia",
  "Palatino Linotype",
  "Book Antiqua",
  "Cambria",
  "Constantia",
  "Segoe UI",
  "Trebuchet MS",
  "Gabriola",
  "Consolas",
].map((name) => ({ value: name, label: name, family: `'${name}'` }));

const IMPORTED = "imported:";

// The CSS family registered for an imported file (unique and safe to quote).
function importedFamily(file: string): string {
  return `sq-imported-${file.replace(/[^a-zA-Z0-9_-]/g, "_")}`;
}

// Option for an imported font.
export function importedOption(font: ImportedFont): FontOption {
  return { value: IMPORTED + font.file, label: font.name, family: `'${importedFamily(font.file)}'` };
}

// CSS font-family for a stored value, with the app font as fallback.
export function cssFamily(value: string): string {
  const fallback = "'Atkinson Hyperlegible Next Variable', system-ui, sans-serif";
  if (!value) return fallback;
  if (value.startsWith(IMPORTED)) return `'${importedFamily(value.slice(IMPORTED.length))}', ${fallback}`;
  return `'${value}', ${fallback}`;
}

const loaded = new Set<string>();

// Registers an imported font with the page (FontFace API), once per file.
export async function loadImportedFont(file: string): Promise<void> {
  if (loaded.has(file)) return;
  const bytes = await readFont(file);
  if (!bytes.byteLength) return;
  const face = new FontFace(importedFamily(file), bytes);
  await face.load();
  document.fonts.add(face);
  loaded.add(file);
}

// Makes sure a stored value can render (only imported fonts need loading).
export async function ensureFont(value: string): Promise<void> {
  if (value.startsWith(IMPORTED)) await loadImportedFont(value.slice(IMPORTED.length));
}

// Lists imported fonts and registers them all (used by screens that preview fonts).
export async function loadAllImported(): Promise<ImportedFont[]> {
  const fonts = await listFonts();
  await Promise.all(fonts.map((f) => loadImportedFont(f.file).catch(() => {})));
  return fonts;
}
