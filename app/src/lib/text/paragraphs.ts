// paragraphs.ts — turns OCR lines back into the paragraphs they came from, for
// reading in the app (the Play feed and History).
//
// The OCR gives one line per line on screen, and the translation follows the
// same lines. In a dialogue box a sentence wraps wherever the box ends, so
// keeping those breaks in the app reads like a staircase ("finalizar" /
// "inimigos"). A break is kept only when it looks meant: menu items, a new
// sentence starting in capitals, a list.

// Line ends that mean the sentence goes on in the next line.
const GOES_ON = /[\p{L}\p{N},;:\-–—(]$/u;
// Line ends that close a sentence.
const ENDS = /[.!?…"”»)\]]$/u;

// Whether line `a` continues into line `b` (the break is just the box's edge).
export function continues(a: string, b: string): boolean {
  const end = a.trimEnd();
  const next = b.trimStart();
  if (!end || !next) return false;
  if (/[-–—]$/u.test(end)) return true; // "— or a strong or" / "dextrous unit"
  if (ENDS.test(end)) return false;
  // Next line starts in lowercase (or a digit / opening punctuation within a sentence).
  const first = next[0];
  const lower = first.toLowerCase() === first && first.toUpperCase() !== first;
  return GOES_ON.test(end) && lower;
}

// For each break between lines, whether it is joined (true) or kept (false).
export function joinsOf(lines: string[]): boolean[] {
  return lines.slice(0, -1).map((line, i) => continues(line, lines[i + 1]));
}

// Joins the lines whose break is in `joins` with a space.
export function applyJoins(lines: string[], joins: boolean[]): string {
  let out = lines[0] ?? "";
  for (let i = 1; i < lines.length; i++) out += (joins[i - 1] ? " " : "\n") + lines[i];
  return out;
}

// The original and its translation as paragraphs. The original decides where
// the paragraphs are (its lines are what the game shows); the translation
// follows it line by line when it has the same number of lines, and is joined
// whole when the original was one paragraph.
export function asParagraphs(original: string, translation: string): { original: string; translation: string } {
  const lines = original.split("\n");
  const joins = joinsOf(lines);
  const translated = translation.split("\n");
  let tr = translation;
  if (translated.length === lines.length) tr = applyJoins(translated, joins);
  else if (joins.length > 0 && joins.every(Boolean)) tr = translated.join(" ");
  return { original: applyJoins(lines, joins), translation: tr };
}
