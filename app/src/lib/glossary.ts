// glossary.ts — small helpers for a game's glossary, shared by the editor in
// Games and the word picker over the game.
import type { GlossaryEntry } from "./api";

// How two terms are compared: case and spacing don't matter.
export function termKey(source: string): string {
  return source.trim().replace(/\s+/g, " ").toLowerCase();
}

// The entry for a term, if the glossary has it.
export function findEntry(entries: GlossaryEntry[], source: string): GlossaryEntry | undefined {
  const key = termKey(source);
  return entries.find((e) => termKey(e.source) === key);
}

// The glossary with `entry` added, or replacing the entry for the same term.
export function upsertEntry(entries: GlossaryEntry[], entry: GlossaryEntry): GlossaryEntry[] {
  const key = termKey(entry.source);
  const clean = { source: entry.source.trim(), target: entry.keep ? "" : entry.target.trim(), keep: entry.keep };
  const at = entries.findIndex((e) => termKey(e.source) === key);
  if (at < 0) return [...entries, clean];
  return entries.map((e, i) => (i === at ? clean : e));
}

// Scripts written without spaces between words (Japanese, Chinese).
const NO_SPACES = /[぀-ヿ㐀-鿿豈-﫿]/u;

// Punctuation at a term's ends: picked along with a word ("enemy.", "\"Konrad"),
// it would keep the term from matching the same word elsewhere.
const EDGE_PUNCTUATION = /^[\s"'“”‘’«»¡¿(\[]+|[\s"'“”‘’«».,;:!?…)\]。、！？]+$/gu;

// Picked words as one term: with spaces, except in scripts that use none, and
// without punctuation at the ends.
export function joinWords(words: string[]): string {
  const joined = words.join(" ");
  return (NO_SPACES.test(joined) ? words.join("") : joined).replace(EDGE_PUNCTUATION, "");
}
