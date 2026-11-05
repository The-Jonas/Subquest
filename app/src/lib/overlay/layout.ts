// layout.ts — where each translated line goes on the overlay (pure functions, easy to test).
//
// All positions are in percentages of the captured area, because the overlay
// window covers exactly that area: whatever the screen's DPI, 30% is 30%.
import type { OcrLine, Region } from "../api";

export interface Patch {
  text: string;
  left: number; // % of the area width
  top: number; // % of the area height
  minWidth: number; // % of the area width (never narrower than the original line)
  height: number; // % of the area height
  fontPx: number; // CSS pixels
  boxColor: string;
  textColor: string;
}

const CAP_TO_BOX = 0.82; // font size relative to the OCR line box height (boxes hug the letters)
const MIN_SHRINK = 0.55; // a long translation may shrink to 55% of the original size, not less

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

// Builds the patches: same place and colors as each original line, font sized
// from the line height, shrunk only as much as needed to fit before the
// area's right edge. `measure(text, px)` returns the text width in CSS px.
export function layout(
  pairs: { line: OcrLine; text: string }[],
  frame: { width: number; height: number },
  view: { width: number; height: number },
  measure: (text: string, px: number) => number,
): Patch[] {
  const scale = view.height / frame.height; // CSS px per capture px
  return pairs
    .filter((p) => p.text)
    .map(({ line, text }) => {
      const [x, y, w, h] = line.rect;
      const base = h * scale * CAP_TO_BOX;
      const available = (frame.width - x) * scale * 0.98; // room until the right edge
      const width = measure(text, base);
      const fontPx = width > available ? Math.max(base * MIN_SHRINK, (base * available) / width) : base;
      return {
        text,
        left: (x / frame.width) * 100,
        top: (y / frame.height) * 100,
        minWidth: (w / frame.width) * 100,
        height: (h / frame.height) * 100,
        fontPx,
        boxColor: line.boxColor,
        textColor: line.textColor,
      };
    });
}
