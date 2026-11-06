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
import type { OcrLine, Region, TextEffect } from "../api";
import { continues } from "../text/paragraphs";

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
  effect?: TextEffect | null; // the game's outline or shadow, in window pixels
  letterCase?: string | null; // case seen on the original line ("smallCaps", "upper")
  weight?: number | null; // weight of the original line's letters (CSS font weight)
}

const MIN_SHRINK = 0.55; // a long translation may shrink to 55% of the original size, not less
const TITLE_SHRINK = 0.75; // a lone line shrinks this far to stay on one line before it breaks
const FALLBACK_INK = 0.72; // ink height / font size when the original can't be measured
const EDGE = 0.98; // keep 2% of the room free at the screen edges

// Where a line is placed: sideways with its icons (`extent`), top and height
// of its letters (`rect`, which gives the size).
function placement(line: OcrLine): Region {
  const e = line.extent ?? line.rect;
  return [e[0], line.rect[1], e[2], line.rect[3]];
}

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
  bounds?: { left: number; right: number }, // px the text must stay within (default: the whole view)
): Patch[] {
  const scale = view.height / frame.height; // CSS px per capture px
  const minX = bounds?.left ?? 0;
  const maxX = bounds?.right ?? view.width;
  const rects = pairs.map((p) => placement(p.line));
  const align: Align[] = [];
  for (const group of blocks(rects)) {
    const a = detectAlign(group.map((i) => rects[i]), single);
    group.forEach((i) => (align[i] = a));
  }

  const patches: Patch[] = [];
  pairs.forEach(({ line, text }, i) => {
    if (!text) return;
    const measure = measureFor(line);
    let [x, y, w, h] = placement(line).map((v) => v * scale);

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
        ? maxX - x
        : align[i] === "right"
          ? x + w - minX
          : 2 * Math.min(center - minX, maxX - center);
    const width = measure(text, base).width;
    const fontPx = width > room * EDGE ? Math.max(base * MIN_SHRINK, (base * room * EDGE) / width) : base;
    const m = measure(text, fontPx);
    const o = measure(line.text || text, fontPx);

    // Ink centered where the original's was: same baseline when not shrunk.
    const baseline = y + h / 2 + (o.inkAscent - o.inkDescent) / 2;
    let left = align[i] === "left" ? x : align[i] === "right" ? x + w - m.width : center - m.width / 2;
    left = Math.min(Math.max(minX, left), Math.max(minX, maxX - m.width));

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
      effect: line.effect ?? null,
      letterCase: line.letterCase ?? null,
      weight: line.weight ?? null,
    });
  });
  return patches;
}

// Splits a text into lines no wider than `width` at `px` (whole words; a
// single word wider than the room gets a line of its own).
export function wrap(text: string, width: number, px: number, measure: Measure): string[] {
  const words = text.split(/\s+/).filter(Boolean);
  const lines: string[] = [];
  let current = "";
  for (const word of words) {
    const tried = current ? `${current} ${word}` : word;
    if (current && measure(tried, px).width > width) {
      lines.push(current);
      current = word;
    } else {
      current = tried;
    }
  }
  if (current) lines.push(current);
  return lines;
}

// Middle value of a list of numbers (0 for an empty list).
function median(values: number[]): number {
  if (!values.length) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)];
}

const CENTERED = 0.08; // a lone line whose middle is this close to the area's (share of its width) is centered

// Alignment of a line that stands alone in a marked area: centered when its
// middle is the area's (a menu button, a title), otherwise left (dialogue).
export function loneAlign(rect: Region, area: Region): Align {
  const middle = rect[0] + rect[2] / 2;
  const areaMiddle = area[0] + area[2] / 2;
  return Math.abs(middle - areaMiddle) <= area[2] * CENTERED ? "center" : "left";
}

// Alignment of a line that is a paragraph by itself, inside a marked area:
// the other lines tell first. Sharing its left edge with them (an item's
// title, its description and its bullet all start at the same x) it is left
// aligned, even when it happens to sit in the middle; sharing its middle (a
// centered menu) it is centered. Only alone does its place decide (loneAlign).
function aloneAlign(index: number, rects: Region[], area: Region): Align {
  const r = rects[index];
  const near = Math.max(4, r[3] * 0.5);
  const middle = r[0] + r[2] / 2;
  let lefts = 0;
  let middles = 0;
  rects.forEach((o, k) => {
    if (k === index) return;
    if (Math.abs(o[0] - r[0]) <= near) lefts++;
    if (Math.abs(o[0] + o[2] / 2 - middle) <= near) middles++;
  });
  if (lefts > middles) return "left";
  if (middles > lefts) return "center";
  return loneAlign(r, area);
}

const SIZE_JUMP = 1.3; // letters this much taller (or shorter) than the line before: a title
const SHORT_LINE = 0.7; // a line this much narrower than the widest one can be a paragraph's end
const WIDE_STEP = 1.45; // a gap between lines this much wider than the group's tightest: a new paragraph
const BULLET = /^\s*[·•‣◦▪▸►\-–—*]\s/u; // a list item starts here

// Splits lines that sit together (sorted top to bottom) into paragraphs. A
// break goes where a title ends ("Turns" above its explanation) or a
// paragraph ends: the line is short and its text doesn't go on into the next
// one, the letters change size or text style, a list item starts ("• Grants
// +5 Health") or the gap to the next line is clearly wider than the others
// (games space paragraphs further apart than lines).
export function splitParagraphs(lines: OcrLine[], group: number[]): number[][] {
  if (!group.length) return [];
  const widest = Math.max(...group.map((i) => lines[i].rect[2]));
  const steps = group.slice(1).map((i, k) => lines[i].rect[1] - lines[group[k]].rect[1]);
  const tightest = Math.min(...steps.filter((v) => v > 0));
  const out: number[][] = [[group[0]]];
  for (let k = 1; k < group.length; k++) {
    const a = lines[group[k - 1]];
    const b = lines[group[k]];
    const [ha, hb] = [a.rect[3], b.rect[3]];
    const sizeJump = Math.max(ha, hb) / Math.max(1, Math.min(ha, hb)) > SIZE_JUMP;
    const otherStyle = !!a.style && !!b.style && a.style !== b.style;
    const ended = a.rect[2] < widest * SHORT_LINE && !continues(a.text, b.text);
    const bullet = BULLET.test(b.text);
    const wideGap = Number.isFinite(tightest) && steps[k - 1] > tightest * WIDE_STEP;
    if (sizeJump || otherStyle || ended || bullet || wideGap) out.push([group[k]]);
    else out[out.length - 1].push(group[k]);
  }
  return out;
}

// Hands the translated lines to the paragraphs in order when the model kept
// neither the original line count nor one line per paragraph: each paragraph
// takes the lines whose middle falls in its share of the original text
// (by characters), so a long description stays long and a title stays short.
// Null when there are fewer translated lines than paragraphs.
export function shareParts(paragraphs: string[], parts: string[]): string[][] | null {
  // List items first: the translated line that starts with a bullet starts the
  // paragraph that does ("· Found using..." -> "• Encontrado usando..."), and
  // the lines between bullets are shared out on their own. Without that, a
  // long item could take the line after it (the flavor text below).
  const bulletParagraphs = paragraphs.flatMap((p, i) => (BULLET.test(p) ? [i] : []));
  const bulletParts = parts.flatMap((p, i) => (BULLET.test(p) ? [i] : []));
  if (bulletParagraphs.length && bulletParagraphs.length === bulletParts.length) {
    const cutsP = [0, ...bulletParagraphs, paragraphs.length];
    const cutsQ = [0, ...bulletParts, parts.length];
    const out: string[][] = [];
    let ok = true;
    for (let k = 0; k + 1 < cutsP.length && ok; k++) {
      const ps = paragraphs.slice(cutsP[k], cutsP[k + 1]);
      const qs = parts.slice(cutsQ[k], cutsQ[k + 1]);
      if (!ps.length) {
        ok = !qs.length;
        continue;
      }
      const shared = shareRun(ps, qs);
      if (shared) out.push(...shared);
      else ok = false;
    }
    if (ok) return out;
  }
  return shareRun(paragraphs, parts);
}

// shareParts without list items: by length only.
function shareRun(paragraphs: string[], parts: string[]): string[][] | null {
  if (parts.length < paragraphs.length) return null;
  const total = paragraphs.reduce((n, p) => n + p.length, 0) || 1;
  const ends: number[] = [];
  let sum = 0;
  for (const p of paragraphs) ends.push((sum += p.length) / total);
  const allParts = parts.reduce((n, p) => n + p.length, 0) || 1;
  const out: string[][] = paragraphs.map(() => []);
  let at = 0;
  let g = 0;
  parts.forEach((part, k) => {
    const middle = (at + part.length / 2) / allParts;
    at += part.length;
    // Its share's paragraph, one step at a most (none is left empty), and on
    // to the next while there are just enough lines left for the rest.
    const own = ends.findIndex((end) => middle <= end);
    if (k > 0 && (own < 0 ? paragraphs.length - 1 : own) > g) g++;
    g = Math.max(g, paragraphs.length - (parts.length - k));
    out[g].push(part);
  });
  return out.every((o) => o.length) ? out : null;
}

// Patches for a marked area (a dialogue box): instead of one translated line
// over each original line (Portuguese runs longer, so lines spilled out of the
// box), each paragraph is written again inside the area: same left edge, same
// size and line spacing as the game's text, wrapped at the area's right edge
// (keeping the same margin as on the left), and shrunk only if it would
// otherwise run past the area's bottom. `area` is the marked rectangle in
// capture pixels.
export function layoutArea(
  lines: OcrLine[],
  translation: string,
  area: Region,
  frame: { width: number; height: number },
  view: { width: number; height: number },
  measureFor: (line: OcrLine) => Measure,
): Patch[] {
  if (!lines.length) return [];
  const scale = view.height / frame.height; // CSS px per capture px
  const parts = translation
    .split("\n")
    .map((s) => s.trim())
    .filter((s, i, all) => s !== "" || i < all.length - 1);

  // Paragraphs: lines that sit together, split where a title or a paragraph
  // ends (splitParagraphs). With the model's line count matching the original,
  // each paragraph gets its own lines' translation; otherwise the translated
  // lines are shared out in order by length (shareParts); only when there
  // are fewer of them than paragraphs is everything one paragraph.
  const rects = lines.map(placement);
  let groups = blocks(rects)
    .map((g) => [...g].sort((a, b) => rects[a][1] - rects[b][1]))
    .flatMap((g) => splitParagraphs(lines, g));
  groups.sort((a, b) => rects[a[0]][1] - rects[b[0]][1]);
  const matched = parts.length === lines.length;
  const shared =
    !matched && groups.length > 1
      ? shareParts(
          groups.map((g) => g.map((i) => lines[i].text).join(" ")),
          parts.filter((p) => p !== ""),
        )
      : null;
  if (!matched && !shared) groups = [lines.map((_, i) => i).sort((a, b) => rects[a][1] - rects[b][1])];

  const [ax, ay, aw, ah] = area.map((v) => v * scale);
  const patches: Patch[] = [];
  // Paragraphs already placed, to push the next ones down when one grew
  // (Portuguese runs longer): the gap the game left under it is kept.
  const placed: { left: number; right: number; bottom0: number; bottom: number }[] = [];
  for (const [g, group] of groups.entries()) {
    const own = matched ? group.map((i) => parts[i]) : shared ? shared[g] : parts;
    const text = own.join(" ").replace(/\s+/g, " ").trim();
    if (!text) continue;
    const first = lines[group[0]];
    const measure = measureFor(first);
    const boxes = group.map((i) => rects[i].map((v) => v * scale));
    const left = Math.min(...boxes.map((b) => b[0]));
    const top = Math.min(...boxes.map((b) => b[1]));
    const right0 = Math.max(...boxes.map((b) => b[0] + b[2]));
    const bottom0 = Math.max(...boxes.map((b) => b[1] + b[3]));
    // How far down this paragraph goes: only as far as a paragraph above it
    // (side by side with it) now reaches into it, keeping a little room
    // between them (half the game's gap, or a third of a line if that is less).
    // Growing into the free space below is fine: nothing else moves. Never
    // out of the marked area: a paragraph already at its bottom stays put.
    const lineH = median(boxes.map((b) => b[3]));
    const shift = Math.min(
      Math.max(0, ay + ah - bottom0),
      Math.max(
        0,
        ...placed
          .filter((p) => p.bottom0 <= top + 1 && p.left < right0 && left < p.right)
          .map((p) => p.bottom + Math.min((top - p.bottom0) / 2, lineH / 3) - top),
      ),
    );
    const from = patches.length;
    const settle = () => {
      const mine = patches.slice(from);
      let bottom = bottom0;
      for (const p of mine) {
        bottom = Math.max(bottom, p.cover.top + p.cover.height);
        if (!shift) continue;
        // Moved down: the cover still hides the original above, and reaches the new place.
        p.textTop += shift;
        if (p.cover.height > 0) p.cover.height += shift;
      }
      placed.push({ left, right: right0, bottom0, bottom: bottom + shift });
    };

    // Size: each original line, in this font, as tall as its OCR box; the
    // paragraph takes the usual one (a line of capitals would skew it).
    const base = median(
      group.map((i, k) => {
        const probe = measure(lines[i].text || text, 100);
        const ink = probe.inkAscent + probe.inkDescent;
        return ink > 0 ? (boxes[k][3] * 100) / ink : boxes[k][3] / FALLBACK_INK;
      }),
    );
    // Line spacing: the game's own (distance between line tops), or a usual one for a lone line.
    const steps = boxes.slice(1).map((b, k) => b[1] - boxes[k][1]);
    const pitch = steps.length ? median(steps) : median(boxes.map((b) => b[3])) * 1.45;

    // Room: up to the area's right edge, with the same margin as on the left.
    const margin = Math.max(0, left - ax);
    const roomRight = Math.max(right0, ax + aw - margin);
    const width = roomRight - left;
    const bottomLimit = Math.max(bottom0, ay + ah);

    // Lines kept as the game broke them when that already fits: a menu
    // (centered words), or translated lines that each fit inside the area.
    // Only a left-aligned paragraph that would spill out is written again.
    // A lone line in the middle of the area is a label (a menu button, a
    // title): it stays on one line, centered where it was, inside the area.
    const align = group.length === 1 ? aloneAlign(group[0], rects, area) : detectAlign(group.map((i) => rects[i]), "left");
    const eachFits =
      matched && group.every((i) => measure(parts[i], base).width <= roomRight - rects[i][0] * scale);
    if (align !== "left" || eachFits) {
      const pairs = matched
        ? group.map((i) => ({ line: lines[i], text: parts[i] }))
        : pairLines(group.map((i) => lines[i]), text);
      patches.push(...layout(pairs, frame, view, measureFor, align, { left: ax, right: ax + aw }));
      settle();
      continue;
    }

    // Shrinks a little at a time until the paragraph fits under the area's bottom.
    let fontPx = base;
    let wrapped = wrap(text, width, fontPx, measure);
    const firstInk = (px: number) => {
      const o = measure(first.text || text, px);
      return { o, baseline: boxes[0][1] + boxes[0][3] / 2 + (o.inkAscent - o.inkDescent) / 2 };
    };
    const fits = (px: number, count: number) => {
      const { baseline } = firstInk(px);
      const m = measure("Hg", px);
      return baseline + shift + (count - 1) * pitch * (px / base) + m.inkDescent <= bottomLimit;
    };
    // A lone line (a title) stays one line, a little smaller, rather than
    // breaking in two ("RAIZ DE / MANDRÁGORA").
    if (group.length === 1 && wrapped.length > 1) {
      const one = (base * width) / Math.max(1, measure(text, base).width);
      if (one >= base * TITLE_SHRINK) {
        fontPx = one;
        wrapped = [text];
      }
    }
    while (fontPx > base * MIN_SHRINK && !fits(fontPx, wrapped.length)) {
      fontPx *= 0.95;
      wrapped = wrap(text, width, fontPx, measure);
    }
    const step = pitch * (fontPx / base);
    const { baseline } = firstInk(fontPx);

    // One cover for the whole paragraph (the original lines and the new ones),
    // carried by the first patch; the others need none.
    const widest = Math.max(...wrapped.map((l) => measure(l, fontPx).width));
    const last = measure(wrapped[wrapped.length - 1], fontPx);
    const coverTop = Math.min(top, baseline - measure(wrapped[0], fontPx).inkAscent);
    const coverBottom = Math.max(bottom0, baseline + (wrapped.length - 1) * step + last.inkDescent);
    const coverRight = Math.max(right0, left + widest);
    wrapped.forEach((lineText, k) => {
      const m = measure(lineText, fontPx);
      patches.push({
        text: lineText,
        style: first.style ?? null,
        cover:
          k === 0
            ? { left, top: coverTop, width: coverRight - left, height: coverBottom - coverTop }
            : { left, top: 0, width: 0, height: 0 },
        textLeft: left,
        textTop: baseline + k * step - m.fontAscent,
        lineHeight: m.fontAscent + m.fontDescent,
        fontPx,
        boxColor: first.boxColor,
        textColor: first.textColor,
        effect: first.effect ?? null,
        letterCase: first.letterCase ?? null,
        weight: first.weight ?? null,
      });
    });
    settle();
  }
  return patches;
}

// The CSS text-shadow that draws a game's effect around translated letters,
// `scale` = CSS px per window pixel. An outline 1 px wide is crisp (eight
// copies around the letters); wider ones are the soft halos games use, drawn
// as stacked blurred shadows. A drop shadow is one copy, offset.
export function effectShadow(effect: TextEffect | null | undefined, scale: number): string | null {
  if (!effect) return null;
  const px = (v: number) => `${+(v * scale).toFixed(2)}px`;
  if (effect.kind === "shadow") return `${px(effect.dx)} ${px(effect.dy)} ${px(effect.size)} ${effect.color}`;
  if (effect.size <= 1) {
    const ring = [-1, 0, 1].flatMap((dx) => [-1, 0, 1].filter((dy) => dx || dy).map((dy) => [dx, dy]));
    return ring.map(([dx, dy]) => `${px(dx * effect.size)} ${px(dy * effect.size)} 0 ${effect.color}`).join(", ");
  }
  const halo = `0 0 ${px(effect.size)} ${effect.color}`;
  return [halo, halo, `0 0 ${px(effect.size / 2)} ${effect.color}`].join(", ");
}
