// Tests for layout.ts: where translated lines go over the game.
// The font is replaced by a fake one with simple, predictable metrics (each
// character half the font size wide), so every number here can be checked by hand.
import { describe, expect, it } from "vitest";
import type { OcrLine, Region } from "../api";
import { blocks, detectAlign, layout, layoutArea, pairLines, splitKeycap, splitParagraphs, loneAlign, union, wrap, type Measure } from "./layout";

// Fake font: width = 0.5 px per character per px of size; ink 0.7 up, 0.2 down.
const measure: Measure = (text, px) => ({
  width: text.length * px * 0.5,
  inkAscent: px * 0.7,
  inkDescent: px * 0.2,
  fontAscent: px * 0.9,
  fontDescent: px * 0.25,
});

// An OCR line at a position (window pixels, same as the overlay here).
function line(text: string, rect: Region): OcrLine {
  return { text, rect, boxColor: "#101010", textColor: "#f0f0f0" };
}

const screen = { width: 1920, height: 1080 };

describe("pairLines", () => {
  it("puts each translated line over its own line when the count matches", () => {
    const lines = [line("Hello there", [0, 0, 100, 20]), line("General Kenobi", [0, 30, 120, 20])];
    const pairs = pairLines(lines, "Olá\nGeneral Kenobi");
    expect(pairs.map((p) => p.text)).toEqual(["Olá", "General Kenobi"]);
  });

  it("joins everything into one block over all lines when the count differs", () => {
    const lines = [line("a", [10, 0, 50, 20]), line("b", [0, 30, 100, 20])];
    const pairs = pairLines(lines, "uma linha só");
    expect(pairs).toHaveLength(1);
    expect(pairs[0].line.rect).toEqual([0, 0, 100, 50]);
  });

  it("ignores blank lines the model added at the end", () => {
    const pairs = pairLines([line("a", [0, 0, 10, 10])], "um\n\n");
    expect(pairs.map((p) => p.text)).toEqual(["um"]);
  });
});

describe("union", () => {
  it("is the smallest rectangle around all of them", () => {
    expect(union([[10, 10, 10, 10], [0, 30, 5, 5]])).toEqual([0, 10, 20, 25]);
  });
});

describe("blocks", () => {
  it("groups lines that sit together and keeps far ones apart", () => {
    const rects: Region[] = [
      [100, 100, 300, 20],
      [100, 125, 280, 20], // right under the first: same paragraph
      [900, 600, 100, 20], // far away: a block of its own
    ];
    const groups = blocks(rects).map((g) => [...g].sort());
    expect(groups).toHaveLength(2);
    expect(groups).toContainEqual([0, 1]);
    expect(groups).toContainEqual([2]);
  });
});

describe("detectAlign", () => {
  it("finds left-aligned dialogue", () => {
    expect(detectAlign([[100, 0, 300, 20], [100, 30, 120, 20]], "center")).toBe("left");
  });

  it("finds a centered menu", () => {
    expect(detectAlign([[430, 0, 120, 40], [385, 90, 210, 40], [440, 180, 100, 40]], "left")).toBe("center");
  });

  it("uses the given alignment for a lone line", () => {
    expect(detectAlign([[0, 0, 10, 10]], "center")).toBe("center");
  });
});

describe("splitKeycap", () => {
  it("finds a key drawn at the start of a prompt", () => {
    expect(splitKeycap("E CLEAN?")).toEqual({ key: "E", rest: "CLEAN?" });
    expect(splitKeycap("[F] Open")).toEqual({ key: "[F]", rest: "Open" });
    expect(splitKeycap("LB Back")).toEqual({ key: "LB", rest: "Back" });
  });

  it("leaves ordinary sentences alone", () => {
    expect(splitKeycap("A storm is coming")).toBeNull();
    expect(splitKeycap("I know")).toBeNull();
    expect(splitKeycap("Hello there")).toBeNull();
  });
});

describe("wrap", () => {
  it("breaks between words at the width", () => {
    // 10 px font: 5 px per character, so 40 px fits 8 characters.
    expect(wrap("um dois tres", 40, 10, measure)).toEqual(["um dois", "tres"]);
  });

  it("gives a word wider than the room a line of its own", () => {
    expect(wrap("a enormepalavra b", 40, 10, measure)).toEqual(["a", "enormepalavra", "b"]);
  });
});

describe("layout (line by line)", () => {
  it("draws a translation over its line, same size when it fits", () => {
    const [p] = layout([{ line: line("Hello", [100, 100, 100, 18]), text: "Olá" }], screen, screen, () => measure);
    // Ink 0.9 of the size must match the 18 px box: size 20.
    expect(p.fontPx).toBeCloseTo(20);
    expect(p.textLeft).toBe(100);
  });

  it("shrinks a translation too wide for the screen, never under 55%", () => {
    const long = "x".repeat(400);
    const [p] = layout([{ line: line("Hello", [100, 100, 100, 18]), text: long }], screen, screen, () => measure);
    expect(p.fontPx).toBeGreaterThanOrEqual(20 * 0.55 - 1e-9);
    expect(p.fontPx).toBeLessThan(20);
  });
});

describe("layoutArea (inside a marked box)", () => {
  // A Wesnoth tip: four English lines in a box; the Portuguese runs longer.
  const tip = [
    line("Did you know that villages are the key to winning a scenario? You can use this", [190, 520, 640, 18]),
    line("knowledge to lure your opponents into a trap, leaving some villages empty, but", [190, 548, 630, 18]),
    line("remember that it is much harder to drive an enemy unit out of a village! Plan", [190, 576, 620, 18]),
    line("your moves accordingly.", [190, 604, 190, 18]),
  ];
  const translation =
    "Para qualquer jogador, controlar vilas costuma ser a chave para vencer um cenário. Você pode usar este\n" +
    "conhecimento para atrair seus oponentes para uma armadilha, deixando algumas vilas desocupadas, mas\n" +
    "lembre-se de que se torna muito mais difícil expulsar uma unidade inimiga de uma vila! Planeje\n" +
    "seus movimentos de acordo.";
  const area: Region = [180, 510, 680, 130];
  const patches = layoutArea(tip, translation, area, screen, screen, () => measure);
  const right = area[0] + area[2];
  const bottom = area[1] + area[3];

  it("keeps every line inside the box's right edge, with the same margin as on the left", () => {
    const margin = tip[0].rect[0] - area[0];
    for (const p of patches) expect(p.textLeft + measure(p.text, p.fontPx).width).toBeLessThanOrEqual(right - margin);
  });

  it("keeps the paragraph above the box's bottom", () => {
    const last = patches[patches.length - 1];
    const baseline = last.textTop + last.fontPx * 0.9;
    expect(baseline + last.fontPx * 0.2).toBeLessThanOrEqual(bottom + 1e-6);
  });

  it("starts at the original's left edge and loses no word", () => {
    expect(patches.every((p) => p.textLeft === 190)).toBe(true);
    const words = (s: string) => s.split(/\s+/).filter(Boolean);
    expect(patches.flatMap((p) => words(p.text))).toEqual(words(translation));
  });

  it("covers the whole paragraph with one patch, carried by the first line", () => {
    expect(patches[0].cover.width).toBeGreaterThan(0);
    expect(patches.slice(1).every((p) => p.cover.width === 0)).toBe(true);
  });

  it("keeps a centered menu one item per line, where each item was", () => {
    const menu = [line("PLAY", [430, 600, 120, 40]), line("OPTIONS", [385, 690, 225, 40]), line("QUIT", [430, 780, 110, 40])];
    const items = layoutArea(menu, "JOGAR\nOPÇÕES\nSAIR", [360, 580, 280, 260], screen, screen, () => measure);
    expect(items.map((p) => p.text)).toEqual(["JOGAR", "OPÇÕES", "SAIR"]);
  });

  it("keeps the game's line breaks when each translated line already fits", () => {
    const short = [line("Yes", [200, 200, 60, 18]), line("No", [200, 230, 40, 18])];
    const items = layoutArea(short, "Sim\nNão", [180, 190, 300, 80], screen, screen, () => measure);
    expect(items.map((p) => p.text)).toEqual(["Sim", "Não"]);
  });
});

describe("titles and paragraphs inside a box", () => {
  // The Wesnoth help page from the bug report: a title right above its explanation.
  const help = [
    line("Turns", [10, 10, 70, 24]),
    line("Every turn, each side in a scenario gets a chance to make their move. Once you've", [10, 42, 690, 20]),
    line("completed everything you wish to do this turn, click on the End Turn button in the", [10, 68, 690, 20]),
    line("bottom right of the screen.", [10, 94, 230, 20]),
  ];
  const area: Region = [0, 0, 720, 200];

  it("finds the title and the paragraph", () => {
    expect(splitParagraphs(help, [0, 1, 2, 3])).toEqual([[0], [1, 2, 3]]);
  });

  it("keeps a wrapped paragraph together, even across a sentence end", () => {
    const wrapped = [
      line("The other sides will make their move. Some scenarios", [10, 10, 690, 20]),
      line("must be completed in a certain number of turns.", [10, 36, 600, 20]),
    ];
    expect(splitParagraphs(wrapped, [0, 1])).toEqual([[0, 1]]);
  });

  it("splits where the letters change size", () => {
    const sized = [line("Chapter One", [10, 10, 600, 40]), line("It was a dark night and the", [10, 60, 640, 20])];
    expect(splitParagraphs(sized, [0, 1])).toHaveLength(2);
  });

  it("splits where the text style changes", () => {
    const styled = [
      { ...line("Konrad:", [10, 10, 680, 20]), style: "s1" },
      { ...line("we must go on to the north", [10, 36, 680, 20]), style: "s2" },
    ];
    expect(splitParagraphs(styled, [0, 1])).toHaveLength(2);
  });

  it("draws the title on its own line, not glued to the explanation", () => {
    const translation = "Turnos\nEm cada turno, cada lado em um cenário tem a chance de fazer seu movimento. Assim que você\nconcluir tudo o que deseja fazer neste turno, clique no botão Fim de Turno no canto\ninferior direito da tela.";
    const patches = layoutArea(help, translation, area, screen, screen, () => measure);
    expect(patches[0].text).toBe("Turnos");
    expect(patches.slice(1).some((p) => p.text.startsWith("Turnos"))).toBe(false);
    expect(patches[1].textTop).toBeGreaterThan(patches[0].textTop);
  });

  it("keeps them apart when the model answered one line per paragraph", () => {
    const translation = "Turnos\nEm cada turno, cada lado em um cenário tem a chance de fazer seu movimento. Assim que você concluir tudo, clique em Fim de Turno.";
    const patches = layoutArea(help, translation, area, screen, screen, () => measure);
    expect(patches[0].text).toBe("Turnos");
    expect(patches[1].text.startsWith("Em cada turno")).toBe(true);
  });
});

describe("lone lines in a marked area (menu buttons)", () => {
  // The Wesnoth main menu marked as one area: each button label stands alone.
  const area: Region = [1375, 200, 192, 470];
  const mapEditor = line("Map Editor", [1430, 405, 82, 14]);
  const addOns = line("Add-ons", [1442, 363, 58, 14]);

  it("tells a centered label from a dialogue line", () => {
    expect(loneAlign(mapEditor.rect, area)).toBe("center");
    expect(loneAlign([190, 520, 300, 18], [180, 510, 680, 130])).toBe("left");
  });

  it("keeps a longer translation on one line, centered on the button", () => {
    const patches = layoutArea([mapEditor], "Editor de Mapas", area, screen, screen, () => measure);
    expect(patches).toHaveLength(1);
    const p = patches[0];
    const middle = p.textLeft + measure(p.text, p.fontPx).width / 2;
    expect(Math.abs(middle - (1430 + 41))).toBeLessThan(1);
  });

  it("stays inside the area, shrinking a little if it must", () => {
    const patches = layoutArea([addOns], "Complementos e extensões", area, screen, screen, () => measure);
    const p = patches[0];
    expect(p.textLeft).toBeGreaterThanOrEqual(1375);
    expect(p.textLeft + measure(p.text, p.fontPx).width).toBeLessThanOrEqual(1375 + 192 + 1e-6);
  });

  it("still wraps a lone dialogue line that runs past the box", () => {
    const talk = line("Konrad, we must go now.", [190, 520, 230, 18]);
    const long = "Konrad, precisamos ir agora mesmo, antes que os orcs atravessem o rio e cheguem até a vila.";
    const patches = layoutArea([talk], long, [180, 510, 680, 130], screen, screen, () => measure);
    expect(patches.length).toBeGreaterThan(1);
  });
});
