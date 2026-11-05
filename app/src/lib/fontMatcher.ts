// fontMatcher.ts — finds, among the fonts already installed, the one that looks
// most like a style's print. No network and no model: it draws the line the
// game showed in each font and compares the shapes with the print.
//
// How: the print's letters are separated from the background by their color
// (the style's text color), cropped to the ink and shrunk to a small height.
// Each candidate font draws the same text, gets the same treatment, and is
// scored by how much the two ink masks overlap (after matching the widths),
// minus a penalty for being wider or narrower overall. Highest score wins.
import type { FontOption } from "./fonts";

const HEIGHT = 28; // masks are compared at this height, in pixels
const INK_DISTANCE = 90; // color distance from the text color that still counts as ink
const WIDTH_PENALTY = 0.6; // weight of the overall width difference in the score

// A black-and-white picture: 1 = ink.
interface Mask {
  width: number;
  height: number;
  ink: Uint8Array;
}

// Crops a mask to its ink and scales it to HEIGHT (keeping the proportions).
function normalize(width: number, height: number, isInk: (i: number) => boolean): Mask | null {
  let x0 = width, y0 = height, x1 = -1, y1 = -1;
  for (let y = 0; y < height; y++)
    for (let x = 0; x < width; x++)
      if (isInk(y * width + x)) {
        x0 = Math.min(x0, x);
        x1 = Math.max(x1, x);
        y0 = Math.min(y0, y);
        y1 = Math.max(y1, y);
      }
  if (x1 < 0 || y1 - y0 < 4) return null; // no ink, or too little to judge
  const w = x1 - x0 + 1;
  const h = y1 - y0 + 1;
  const outW = Math.max(1, Math.round((w * HEIGHT) / h));
  const ink = new Uint8Array(outW * HEIGHT);
  for (let y = 0; y < HEIGHT; y++)
    for (let x = 0; x < outW; x++) {
      const sx = x0 + Math.floor((x * w) / outW);
      const sy = y0 + Math.floor((y * h) / HEIGHT);
      ink[y * outW + x] = isInk(sy * width + sx) ? 1 : 0;
    }
  return { width: outW, height: HEIGHT, ink };
}

// Ink mask of the print: pixels close to the style's text color.
async function printMask(print: Blob, textRgb: number): Promise<Mask | null> {
  const bitmap = await createImageBitmap(print);
  const canvas = new OffscreenCanvas(bitmap.width, bitmap.height);
  const ctx = canvas.getContext("2d")!;
  ctx.drawImage(bitmap, 0, 0);
  const { data } = ctx.getImageData(0, 0, bitmap.width, bitmap.height);
  const [r, g, b] = [(textRgb >> 16) & 255, (textRgb >> 8) & 255, textRgb & 255];
  const limit = INK_DISTANCE * INK_DISTANCE;
  return normalize(bitmap.width, bitmap.height, (i) => {
    const dr = data[i * 4] - r, dg = data[i * 4 + 1] - g, db = data[i * 4 + 2] - b;
    return dr * dr + dg * dg + db * db < limit;
  });
}

// Ink mask of a text drawn in a font (black on transparent, at a large size).
function fontMask(text: string, family: string): Mask | null {
  const size = 64;
  const canvas = new OffscreenCanvas(8, 8);
  let ctx = canvas.getContext("2d")!;
  const font = `${size}px ${family}`;
  ctx.font = font;
  const width = Math.ceil(ctx.measureText(text).width) + size;
  canvas.width = width;
  canvas.height = size * 2;
  ctx = canvas.getContext("2d")!; // resizing resets the context
  ctx.font = font;
  ctx.textBaseline = "middle";
  ctx.fillText(text, size / 2, size);
  const { data } = ctx.getImageData(0, 0, width, size * 2);
  return normalize(width, size * 2, (i) => data[i * 4 + 3] > 128);
}

// How alike two masks are: overlap of the ink after stretching `b` to `a`'s
// width (intersection over union), minus the overall width difference.
function similarity(a: Mask, b: Mask): number {
  let both = 0, either = 0;
  for (let y = 0; y < HEIGHT; y++)
    for (let x = 0; x < a.width; x++) {
      const pa = a.ink[y * a.width + x];
      const pb = b.ink[y * b.width + Math.floor((x * b.width) / a.width)];
      both += pa & pb;
      either += pa | pb;
    }
  const overlap = either ? both / either : 0;
  return overlap - WIDTH_PENALTY * Math.abs(Math.log(b.width / a.width));
}

// The option that draws `sample` most like the print, or null when the print
// has too little ink to judge. Fonts must already be loaded in the page.
export async function closestFont(
  print: Blob,
  textRgb: number,
  sample: string,
  options: FontOption[],
): Promise<FontOption | null> {
  const target = await printMask(print, textRgb);
  const text = sample.split("\n")[0].trim();
  if (!target || !text) return null;
  let best: FontOption | null = null;
  let bestScore = -Infinity;
  for (const option of options) {
    const mask = fontMask(text, option.family);
    if (!mask) continue;
    const score = similarity(target, mask);
    if (score > bestScore) {
      bestScore = score;
      best = option;
    }
  }
  return best;
}
