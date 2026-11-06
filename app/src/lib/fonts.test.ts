// Tests for the letter-case rules in fonts.ts (how a translation is written
// to look like the game's text).
import { describe, expect, it } from "vitest";
import { applyCase, styleCase } from "./fonts";

describe("styleCase", () => {
  it("'as in the game' takes the case seen on the print", () => {
    expect(styleCase({ caps: "", seenCase: "smallCaps" })).toBe("smallCaps");
    expect(styleCase({ caps: "", seenCase: null })).toBe("");
  });

  it("'as translated' keeps the translation as written", () => {
    expect(styleCase({ caps: "none", seenCase: "upper" })).toBe("");
  });

  it("an explicit case wins over the print", () => {
    expect(styleCase({ caps: "upper", seenCase: "smallCaps" })).toBe("upper");
  });
});

describe("applyCase", () => {
  it("writes everything in capitals for 'upper'", () => {
    expect(applyCase("Jogar agora", "upper")).toBe("JOGAR AGORA");
  });

  it("turns an all-capitals translation back to sentence case for small capitals", () => {
    // OCR reads small capitals as big ones: "PLAY" -> "JOGAR" must look like "Play".
    expect(applyCase("JOGAR", "smallCaps")).toBe("Jogar");
    expect(applyCase("OPÇÕES DE JOGO", "smallCaps")).toBe("Opções de jogo");
  });

  it("keeps a mixed-case translation as written for small capitals", () => {
    expect(applyCase("Jogar Agora", "smallCaps")).toBe("Jogar Agora");
  });

  it("leaves numbers and symbols alone", () => {
    expect(applyCase("123!", "smallCaps")).toBe("123!");
  });
});
