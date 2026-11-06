// Tests for icons.ts: placeholders for pictures inside the game's text.
import { describe, expect, it } from "vitest";
import { forMeasure, missingIcons, pieces, withAllIcons } from "./icons";

describe("pieces", () => {
  it("splits text and pictures in order", () => {
    expect(pieces("Aperte {1} para pular")).toEqual([{ text: "Aperte " }, { icon: 0 }, { text: " para pular" }]);
    expect(pieces("{2}")).toEqual([{ icon: 1 }]);
    expect(pieces("sem ícones")).toEqual([{ text: "sem ícones" }]);
  });
});

describe("missingIcons / withAllIcons", () => {
  it("finds the pictures the translator dropped", () => {
    expect(missingIcons("Press {1} or {2}", "Aperte {2}")).toEqual(["{1}"]);
    expect(missingIcons("Press {1}", "Aperte {1}")).toEqual([]);
  });

  it("puts them back at the start", () => {
    expect(withAllIcons("Press {1} to skip", "Aperte para pular")).toBe("{1} Aperte para pular");
    expect(withAllIcons("Hello", "Olá")).toBe("Olá");
  });
});

describe("forMeasure", () => {
  it("gives each picture the room of two wide letters", () => {
    expect(forMeasure("Aperte {1} já")).toBe("Aperte MM já");
  });
});
