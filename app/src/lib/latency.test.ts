// Tests for latency.ts: summing up where the wait for a translation goes.
import { describe, expect, it } from "vitest";
import { formatMs, latencyReport, lineWait, quantile } from "./latency";

describe("quantile", () => {
  it("gives the middle value and interpolates", () => {
    expect(quantile([3, 1, 2], 0.5)).toBe(2);
    expect(quantile([0, 10], 0.9)).toBe(9);
    expect(quantile([], 0.5)).toBe(0);
  });
});

describe("latencyReport", () => {
  it("is null before any translation reached the screen", () => {
    expect(latencyReport([{ settle: 100, read: 40 }])).toBeNull();
  });

  it("sums up each stage and the whole wait", () => {
    const report = latencyReport([
      { settle: 100, read: 40, first: 500, done: 1500 },
      { settle: 300, read: 60, first: 700, done: 900 },
      { settle: 200, read: 50, first: 600 }, // still streaming: counts, except for "rest"
      { settle: 999, read: 999 }, // never translated: left out
    ])!;
    expect(report.lines).toBe(3);
    expect(report.stages.settle.median).toBe(200);
    expect(report.stages.read.median).toBe(50);
    expect(report.stages.first.median).toBe(600);
    expect(report.stages.rest.median).toBe(600); // (1000 + 200) / 2
    expect(report.toFirst.median).toBe(850); // 640, 850, 1060
  });
});

describe("lineWait", () => {
  it("adds the stages up to the first translated words", () => {
    expect(lineWait({ settle: 100, read: 40, first: 500 })).toBe(640);
    expect(lineWait({ settle: 100, read: 40 })).toBeNull();
  });
});

describe("formatMs", () => {
  it("writes short waits in ms and long ones in seconds", () => {
    expect(formatMs(340.4, "pt-BR")).toBe("340 ms");
    expect(formatMs(1234, "pt-BR")).toBe("1,2 s");
    expect(formatMs(1234, "en")).toBe("1.2 s");
  });
});
