// icons.ts — placeholders for pictures inside the game's text.
//
// The core replaces a picture drawn inside a line (a gamepad button) with
// "{1}", "{2}"... (core/src/vision/icons.h); the translator keeps them where
// they belong ("Aperte {1} para pular") and the app draws the picture there.

import type { IconPicture, Region } from "../api";

const TOKEN = /\{(\d+)\}/g;

// A piece of text: plain text, or the picture of placeholder `icon` (0 = {1}).
export type Piece = { text: string } | { icon: number };

// Splits a text into plain text and pictures, in order.
export function pieces(text: string): Piece[] {
  const out: Piece[] = [];
  let last = 0;
  for (const match of text.matchAll(TOKEN)) {
    if (match.index > last) out.push({ text: text.slice(last, match.index) });
    out.push({ icon: Number(match[1]) - 1 });
    last = match.index + match[0].length;
  }
  if (last < text.length) out.push({ text: text.slice(last) });
  return out;
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
  return text.replace(TOKEN, "MM");
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
