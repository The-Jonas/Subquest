// Tests for paragraphs.ts: which line breaks are kept when showing a line in the app.
import { describe, expect, it } from "vitest";
import { asParagraphs, continues } from "./paragraphs";

describe("continues", () => {
  it("joins a sentence wrapped by the box", () => {
    expect(continues("are ideal for finishing off", "enemies with only a few")).toBe(true);
    expect(continues("— or a strong or", "dextrous unit")).toBe(true);
  });

  it("keeps a break before a new sentence or item", () => {
    expect(continues("hitpoints remaining.", "Be sure to use leadership")).toBe(false);
    expect(continues("Play", "Options")).toBe(false);
  });

  it("joins after a dash even before a capital", () => {
    expect(continues("especially magical attacks —", "Units")).toBe(true);
  });

  it("keeps breaks in lines without lowercase (all-caps games can't be told apart)", () => {
    expect(continues("I'M GOING", "TO THE STORE")).toBe(false);
  });
});

describe("asParagraphs", () => {
  // The Wesnoth tip from the Play screen.
  const original =
    "Units with three or more attacks — especially magical attacks — are ideal for finishing off\n" +
    "enemies with only a few hitpoints remaining. Be sure to use leadership — or a strong or\n" +
    "dextrous unit — whenever the extra point or two Of damage makes a one-hit kill possible.";
  const translation =
    "Unidades com três ou mais ataques — especialmente ataques mágicos — são ideais para finalizar\n" +
    "inimigos com apenas alguns pontos de vida restantes. Certifique-se de usar liderança - ou um forte ou\n" +
    "unidade hábil – sempre que um ou dois pontos extras de dano tornam possível uma morte com um golpe.";

  it("turns a wrapped tip into one paragraph, in both languages", () => {
    const p = asParagraphs(original, translation);
    expect(p.original).not.toContain("\n");
    expect(p.translation).not.toContain("\n");
    expect(p.translation).toContain("finalizar inimigos");
  });

  it("keeps a menu one item per line", () => {
    const p = asParagraphs("Play\nOptions\nQuit", "Jogar\nOpções\nSair");
    expect(p.translation).toBe("Jogar\nOpções\nSair");
  });

  it("keeps the break between two sentences, on both sides", () => {
    const p = asParagraphs("Hello there.\nWho are you?", "Olá.\nQuem é você?");
    expect(p).toEqual({ original: "Hello there.\nWho are you?", translation: "Olá.\nQuem é você?" });
  });

  it("joins a translation with fewer lines when the original was one paragraph", () => {
    const p = asParagraphs("the quick brown fox\njumps over the dog", "a raposa pula o cão");
    expect(p.translation).toBe("a raposa pula o cão");
  });

  it("leaves a one-line text alone", () => {
    expect(asParagraphs("Quit", "Sair")).toEqual({ original: "Quit", translation: "Sair" });
  });
});
