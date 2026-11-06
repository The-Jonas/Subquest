// Tests for glossary.ts: finding and saving terms picked in the game.
import { describe, expect, it } from "vitest";
import { findEntry, joinWords, upsertEntry } from "./glossary";

const entries = [
  { source: "Pale King", target: "Rei Pálido", keep: false },
  { source: "Wesnoth", target: "", keep: true },
];

describe("findEntry", () => {
  it("finds a term whatever its case and spacing", () => {
    expect(findEntry(entries, "  pale   KING ")?.target).toBe("Rei Pálido");
    expect(findEntry(entries, "Konrad")).toBeUndefined();
  });
});

describe("upsertEntry", () => {
  it("adds a new term at the end", () => {
    const out = upsertEntry(entries, { source: " Konrad ", target: "", keep: true });
    expect(out).toHaveLength(3);
    expect(out[2]).toEqual({ source: "Konrad", target: "", keep: true });
  });

  it("replaces the entry of the same term, keeping its place", () => {
    const out = upsertEntry(entries, { source: "pale king", target: "Rei Pálido ", keep: false });
    expect(out).toHaveLength(2);
    expect(out[0]).toEqual({ source: "pale king", target: "Rei Pálido", keep: false });
  });

  it("drops the translation of a term that is kept as is", () => {
    expect(upsertEntry([], { source: "Wesnoth", target: "x", keep: true })[0].target).toBe("");
  });

  it("never changes the list it was given", () => {
    upsertEntry(entries, { source: "Konrad", target: "", keep: true });
    expect(entries).toHaveLength(2);
  });
});

describe("joinWords", () => {
  it("joins words with spaces", () => {
    expect(joinWords(["Pale", "King"])).toBe("Pale King");
  });

  it("drops punctuation picked along at the ends, not inside", () => {
    expect(joinWords(["reach", "the", "enemy."])).toBe("reach the enemy");
    expect(joinWords(['"Konrad,'])).toBe("Konrad");
    expect(joinWords(["Mr.", "Smith"])).toBe("Mr. Smith");
    expect(joinWords(["勇者！"])).toBe("勇者");
  });

  it("joins Japanese without spaces", () => {
    expect(joinWords(["勇", "者"])).toBe("勇者");
  });
});
