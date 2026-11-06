// latency.ts — where the wait between a line appearing in the game and its
// translation on screen goes, so the slowest stage is the one worked on.
//
// Stages of one line, in order:
//   settle — the text stops changing (typewriter effects) and Subquest waits a
//            few still frames to be sure it is complete;
//   read   — from that frame to the text being ready (waiting for the OCR
//            thread, plus the OCR itself);
//   first  — from the text to the first words of a translation on screen
//            (the quick one or the main one, whichever comes first);
//   rest   — from those first words until the final translation is complete.

// Times of one line, in milliseconds (first/done missing until they happen).
export interface LineTiming {
  settle: number;
  read: number;
  first?: number; // since the text was ready
  done?: number; // since the text was ready
}

// The usual time of a stage and its slow cases.
export interface StageSummary {
  median: number;
  p90: number; // 9 lines in 10 are at most this slow
}

export interface LatencyReport {
  lines: number; // lines with a translation on screen (the others don't count)
  stages: { settle: StageSummary; read: StageSummary; first: StageSummary; rest: StageSummary };
  toFirst: StageSummary; // the whole wait: text still in the game -> first words of the translation
}

// Value at a share (0..1) of the sorted list, interpolating between neighbors.
export function quantile(values: number[], share: number): number {
  if (!values.length) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const at = (sorted.length - 1) * share;
  const low = Math.floor(at);
  const high = Math.ceil(at);
  return sorted[low] + (sorted[high] - sorted[low]) * (at - low);
}

// Median and slow cases of a list of times.
function summary(values: number[]): StageSummary {
  return { median: quantile(values, 0.5), p90: quantile(values, 0.9) };
}

// Report over the lines whose translation reached the screen; null without any.
export function latencyReport(timings: LineTiming[]): LatencyReport | null {
  const shown = timings.filter((t) => t.first != null);
  if (!shown.length) return null;
  const done = shown.filter((t) => t.done != null);
  return {
    lines: shown.length,
    stages: {
      settle: summary(shown.map((t) => t.settle)),
      read: summary(shown.map((t) => t.read)),
      first: summary(shown.map((t) => t.first!)),
      rest: summary(done.map((t) => Math.max(0, t.done! - t.first!))),
    },
    toFirst: summary(shown.map((t) => t.settle + t.read + t.first!)),
  };
}

// The whole wait of one line until its first translated words (null until then).
export function lineWait(t: LineTiming): number | null {
  return t.first == null ? null : t.settle + t.read + t.first;
}

// Milliseconds as people read them: "340 ms", "1,2 s".
export function formatMs(ms: number, locale: string): string {
  if (ms < 1000) return `${Math.round(ms)} ms`;
  return `${(ms / 1000).toLocaleString(locale, { maximumFractionDigits: 1, minimumFractionDigits: 1 })} s`;
}
