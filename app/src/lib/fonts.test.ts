// Tests for the letter-case rules in fonts.ts (how a translation is written
// to look like the game's text).
import { describe, expect, it } from "vitest";
import { applyCase, caseFor, styleCase } from "./fonts";

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

describe("caseFor", () => {
  it("follows the picture when left automatic (Hades' small-capital menus)", () => {
    expect(caseFor("", "smallCaps")).toBe("smallCaps");
    expect(caseFor("", "upper")).toBe("upper");
    expect(caseFor("", "")).toBe("");
    expect(caseFor("", undefined)).toBe("");
  });

  it("lets a chosen case win", () => {
    expect(caseFor("upper", "smallCaps")).toBe("upper");
    expect(caseFor("none", "smallCaps")).toBe("");
  });

  it("turns an all-capitals translation into small capitals like the game", () => {
    expect(applyCase("PRESENTE", caseFor("", "smallCaps"))).toBe("Presente");
  });
});

describe("applyCase with color tags", () => {
  it("brings small capitals read as capitals back to sentence case, tags untouched", () => {
    expect(applyCase("<c1>RAIZ</c1> DE MANDRÁGORA", "smallCaps")).toBe("<c1>Raiz</c1> de mandrágora");
  });
});

