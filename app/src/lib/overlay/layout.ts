// layout.ts — where each translated line goes on the overlay (pure functions, easy to test).
//
// The goal is that the translation takes the original's place exactly:
//  - size: the font size is chosen so the original text, drawn in the chosen
//    font, would have the same ink height as the OCR box (instead of a fixed
//    ratio, which is wrong for every font with a different cap height);
//  - height: the ink is centered where the original ink was (same baseline
//    when the size didn't have to shrink);
//  - alignment: lines stacked one above the other keep the alignment they
//    share (centered menus stay centered, dialogue stays left-aligned).
// Positions are CSS pixels of the overlay, which covers exactly the captured area.
import type { OcrLine, Region } from "../api";

export type Align = "left" | "center" | "right";

// Font metrics at a size, as canvas `measureText` gives them.
export interface Metrics {
  width: number;
  inkAscent: number; // from the baseline up to the top of the drawn letters
  inkDescent: number; // from the baseline down to the bottom of the drawn letters
  fontAscent: number; // the font's own ascent (line box), for placing the baseline
  fontDescent: number;
}

export type Measure = (text: string, px: number) => Metrics;

// One translated line: a cover that hides the original and the text on top.
export interface Patch {
  text: string;
  style: string | null; // the original line's text style (decides the font)
  cover: { left: number; top: number; width: number; height: number }; // px
  textLeft: number; // px
  textTop: number; // px: top of the line box (baseline = textTop + fontAscent)
  lineHeight: number; // px: exactly ascent + descent, so the baseline lands where computed
  fontPx: number;
  boxColor: string;
  textColor: string;
}

const MIN_SHRINK = 0.55; // a long translation may shrink to 55% of the original size, not less
const FALLBACK_INK = 0.72; // ink height / font size when the original can't be measured
const EDGE = 0.98; // keep 2% of the room free at the screen edges

// Pairs translated lines with original lines. When the model kept the line
// count (the normal case), each translation goes over its own line; otherwise
// the whole translation becomes one block over all the lines.
export function pairLines(lines: OcrLine[], translation: string): { line: OcrLine; text: string }[] {
  const parts = translation.split("\n").map((s) => s.trim());
  while (parts.length > 1 && parts[parts.length - 1] === "") parts.pop(); // trailing blank lines from the model
  if (lines.length > 0 && parts.length === lines.length) {
    return lines.map((line, i) => ({ line, text: parts[i] }));
  }
  if (lines.length === 0) return [];
  return [{ line: { ...lines[0], rect: union(lines.map((l) => l.rect)) }, text: parts.join(" ") }];
}

// Smallest rectangle containing all the given ones.
export function union(rects: Region[]): Region {
  const x0 = Math.min(...rects.map((r) => r[0]));
  const y0 = Math.min(...rects.map((r) => r[1]));
  const x1 = Math.max(...rects.map((r) => r[0] + r[2]));
  const y1 = Math.max(...rects.map((r) => r[1] + r[3]));
  return [x0, y0, x1 - x0, y1 - y0];
}

// True when two lines belong to the same block: they overlap sideways and the
// vertical gap between them is small compared with their height.
function sameBlock(a: Region, b: Region): boolean {
  const overlap = Math.min(a[0] + a[2], b[0] + b[2]) - Math.max(a[0], b[0]);
  const gap = Math.max(a[1], b[1]) - Math.min(a[1] + a[3], b[1] + b[3]);
  return overlap > 0 && gap < 2 * Math.max(a[3], b[3]);
}

// Key names games draw as button icons at the start of a line.
const KEY_NAMES = /^(LB|RB|LT|RT|L1|R1|L2|R2|ESC|TAB|SPACE|SHIFT|CTRL|ALT|ENTER)$/i;

// A button icon at the start of a line ("E CLEAN?", "[F] Open", "LB Back"),
// which must stay visible. A lone letter only counts when the next word is in
// capitals, so the "A" of "A storm is coming" (or "A NEW DAY") is left alone. Returns the key
// as read and the rest of the line, or null.
export function splitKeycap(text: string): { key: string; rest: string } | null {
  const match = /^(\S+)\s+(\S.*)$/s.exec(text.trim());
  if (!match) return null;
  const [, first, rest] = match;
  const bare = first.replace(/^[[(<]|[\])>]$/g, "");
  const bracketed = bare !== first && /^[A-Z0-9]{1,5}$/i.test(bare);
  // "A" and "I" are also words: as keys they come before a single word ("A CONFIRM").
  const word = /^[AI]$/.test(bare) && rest.trim().split(/\s+/).length > 1;
  const letter = /^[A-Z0-9]$/.test(bare) && /^[A-ZÀ-Ý]{2,}/.test(rest) && !word;
  return bracketed || letter || KEY_NAMES.test(bare) ? { key: first, rest } : null;
}

// The translation without the key the model may have copied at its start.
function dropKey(translation: string, key: string): string {
  return translation.startsWith(key + " ") ? translation.slice(key.length + 1).trimStart() : translation;
}

// Groups lines into blocks (a menu, a paragraph): indices of each group.
export function blocks(rects: Region[]): number[][] {
  const parent = rects.map((_, i) => i);
  const root = (i: number): number => (parent[i] === i ? i : (parent[i] = root(parent[i])));
  for (let i = 0; i < rects.length; i++)
    for (let j = i + 1; j < rects.length; j++) if (sameBlock(rects[i], rects[j])) parent[root(i)] = root(j);
  const groups = new Map<number, number[]>();
  rects.forEach((_, i) => groups.set(root(i), [...(groups.get(root(i)) ?? []), i]));
  return [...groups.values()];
}

// Alignment shared by stacked lines: the edge (or center) that varies least.
// A lone line uses `single` (left in a dialogue box, center for loose labels).
export function detectAlign(rects: Region[], single: Align): Align {
  if (rects.length < 2) return single;
  const spread = (values: number[]) => Math.max(...values) - Math.min(...values);
  const left = spread(rects.map((r) => r[0]));
  const center = spread(rects.map((r) => r[0] + r[2] / 2));
  const right = spread(rects.map((r) => r[0] + r[2]));
  const tolerance = Math.max(2, Math.min(...rects.map((r) => r[3])) * 0.15); // left wins near-ties
  if (left <= center + tolerance && left <= right + tolerance) return "left";
  return center <= right ? "center" : "right";
}

// Builds the patches. `measureFor(line)` gives the measuring function of the
// font that line will be drawn in (each text style can have its own);
// `single` is the alignment of a line that has no neighbours.
export function layout(
  pairs: { line: OcrLine; text: string }[],
  frame: { width: number; height: number },
  view: { width: number; height: number },
  measureFor: (line: OcrLine) => Measure,
  single: Align = "left",
): Patch[] {
  const scale = view.height / frame.height; // CSS px per capture px
  const rects = pairs.map((p) => p.line.rect);
  const align: Align[] = [];
  for (const group of blocks(rects)) {
    const a = detectAlign(group.map((i) => rects[i]), single);
    group.forEach((i) => (align[i] = a));
  }

  const patches: Patch[] = [];
  pairs.forEach(({ line, text }, i) => {
    if (!text) return;
    const measure = measureFor(line);
    let [x, y, w, h] = line.rect.map((v) => v * scale);

    // A button icon at the start stays visible: the patch starts after it.
    const keycap = splitKeycap(line.text);
    if (keycap) {
      const whole = measure(line.text, 100).width;
      const skip = whole > 0 ? (measure(keycap.key + " ", 100).width / whole) * w : 0;
      x += skip;
      w -= skip;
      text = dropKey(text, keycap.key);
      line = { ...line, text: keycap.rest };
    }

    // Size: the original line, in this font, must be as tall as its OCR box.
    const probe = measure(line.text || text, 100);
    const ink = probe.inkAscent + probe.inkDescent;
    const base = ink > 0 ? (h * 100) / ink : h / FALLBACK_INK;

    // Room on the side(s) the text grows to, given its alignment.
    const center = x + w / 2;
    const room =
      align[i] === "left"
        ? view.width - x
        : align[i] === "right"
          ? x + w
          : 2 * Math.min(center, view.width - center);
    const width = measure(text, base).width;
    const fontPx = width > room * EDGE ? Math.max(base * MIN_SHRINK, (base * room * EDGE) / width) : base;
    const m = measure(text, fontPx);
    const o = measure(line.text || text, fontPx);

    // Ink centered where the original's was: same baseline when not shrunk.
    const baseline = y + h / 2 + (o.inkAscent - o.inkDescent) / 2;
    let left = align[i] === "left" ? x : align[i] === "right" ? x + w - m.width : center - m.width / 2;
    left = Math.min(Math.max(0, left), Math.max(0, view.width - m.width));

    // The cover hides the original and sits behind the new text, whichever is bigger.
    const coverLeft = Math.min(x, left);
    const coverRight = Math.max(x + w, left + m.width);
    const coverTop = Math.min(y, baseline - m.inkAscent);
    const coverBottom = Math.max(y + h, baseline + m.inkDescent);

    patches.push({
      text,
      style: line.style ?? null,
      cover: { left: coverLeft, top: coverTop, width: coverRight - coverLeft, height: coverBottom - coverTop },
      textLeft: left,
      textTop: baseline - m.fontAscent,
      lineHeight: m.fontAscent + m.fontDescent,
      fontPx,
      boxColor: line.boxColor,
      textColor: line.textColor,
    });
  });
  return patches;
}
