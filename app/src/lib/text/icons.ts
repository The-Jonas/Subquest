// icons.ts — placeholders for pictures inside the game's text.
//
// The core replaces a picture drawn inside a line (a gamepad button) with
// "{1}", "{2}"... (core/src/vision/icons.h); the translator keeps them where
// they belong ("Aperte {1} para pular") and the app draws the picture there.

import type { IconPicture, Region } from "../api";

const TOKEN = /\{(\d+)\}/g;
// Words the game writes in another color come tagged, "<c1>Grasp</c1>"
// (src-tauri/src/highlights.rs). Case-insensitive: a text drawn in capitals
// is upper-cased with its tags.
const TAG = /<(\/?)c(\d+)>/gi;
const ANY = /\{(\d+)\}|<(\/?)c(\d+)>/gi;

// A piece of text: plain text, or the picture of placeholder `icon` (0 = {1});
// `mark`: inside color tag `mark` (0 = <c1>).
export type Piece = ({ text: string } | { icon: number }) & { mark?: number };

// Splits a text into plain text and pictures, in order, with the color tag each sits in.
export function pieces(text: string): Piece[] {
  const out: Piece[] = [];
  let last = 0;
  let mark: number | undefined;
  const push = (piece: Piece) => out.push(mark === undefined ? piece : { ...piece, mark });
  for (const match of text.matchAll(ANY)) {
    if (match.index > last) push({ text: text.slice(last, match.index) });
    last = match.index + match[0].length;
    if (match[1] !== undefined) push({ icon: Number(match[1]) - 1 });
    else if (match[2] === "/") mark = undefined;
    else mark = Number(match[3]) - 1;
  }
  if (last < text.length) push({ text: text.slice(last) });
  return out;
}

// The text without its color tags.
export function untagged(text: string): string {
  return text.replace(TAG, "");
}

// Lines cut from one tagged text, each closing the tag it leaves open and the
// next opening it again ("<c1>Arcana" / "Cards</c1>" would lose the color).
export function balanceTags(lines: string[]): string[] {
  let open: string | null = null;
  return lines.map((line) => {
    let out = open ? `<c${open}>${line}` : line;
    for (const m of line.matchAll(TAG)) open = m[1] === "/" ? null : m[2];
    if (open) out += `</c${open}>`;
    return out;
  });
}

// Placeholders of the original that the translation lost (the translator
// dropped them), so the pictures can still be shown.
export function missingIcons(original: string, translation: string): string[] {
  const kept = new Set(translation.match(TOKEN) ?? []);
  return [...new Set(original.match(TOKEN) ?? [])].filter((token) => !kept.has(token));
}

// The translation with the lost placeholders put back at the start of its
// first line (buttons are usually at the start of prompts).
export function withAllIcons(original: string, translation: string): string {
  const missing = missingIcons(original, translation);
  return missing.length ? `${missing.join(" ")} ${translation}` : translation;
}

// The text as it is measured for layout: each picture takes about the room of two wide letters.
export function forMeasure(text: string): string {
  return untagged(text).replace(TOKEN, "MM");
}

// Where a line sits sideways with its pictures: its letters' box (`rect`)
// widened to the icons its text names ("{1} GIFT": the key and the word). The
// translation is placed by this (a centered "[G] Gift" stays centered as a
// whole), while its size still comes from the letters alone.
export function extentOf(text: string, rect: Region, icons: IconPicture[]): Region {
  let x0 = rect[0];
  let x1 = rect[0] + rect[2];
  for (const match of text.matchAll(TOKEN)) {
    const icon = icons[Number(match[1]) - 1];
    if (!icon) continue;
    x0 = Math.min(x0, icon.rect[0]);
    x1 = Math.max(x1, icon.rect[0] + icon.rect[2]);
  }
  return [x0, rect[1], x1 - x0, rect[3]];
}
