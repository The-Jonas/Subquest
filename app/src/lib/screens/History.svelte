<!--
  History — every line of the session with its translation. Games like Hades
  drop a line out of nowhere and move on in a second; the ones that passed
  before their translation was ready are marked, so they can be read here.
-->
<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { app } from "../stores/app.svelte";
  import { session, type HistoryEntry } from "../stores/session.svelte";
  import LinePair from "../components/LinePair.svelte";
  import { formatMs, latencyReport, lineWait, type LineTiming } from "../latency";

  let onlyMissed = $state(false);
  let query = $state("");

  // Opening the screen counts as seeing the missed lines.
  session.unseenMissed = 0;
  $effect(() => {
    if (session.unseenMissed > 0) session.unseenMissed = 0; // new ones while the screen is open
  });

  // Lines matching the filter and the search (in the original or the translation).
  const shown = $derived(
    session.history.filter((h) => {
      if (onlyMissed && !h.missed) return false;
      const q = query.trim().toLowerCase();
      return !q || h.original.toLowerCase().includes(q) || h.translation.toLowerCase().includes(q);
    }),
  );
  const missedCount = $derived(session.history.filter((h) => h.missed).length);

  // Where the wait for translations went this session (lines that got one).
  const report = $derived(latencyReport(session.history.map((h) => h.timing)));
  const stageKeys = ["settle", "read", "first"] as const; // the wait up to the first words, in order
  const biggest = $derived(
    report ? stageKeys.reduce((a, b) => (report.stages[b].median > report.stages[a].median ? b : a)) : null,
  );

  // A time, written for the interface language.
  function ms(value: number): string {
    return formatMs(value, app.settings.uiLanguage);
  }

  // Tooltip of a line's time: each stage of its wait.
  function stagesOf(timing: LineTiming): string {
    return stageKeys
      .filter((k) => timing[k] != null)
      .map((k) => `${t(`history.latency.${k}`)}: ${ms(timing[k]!)}`)
      .join(" · ");
  }

  // Clock time of a line, to find "that line from a minute ago".
  function time(at: number): string {
    return new Date(at).toLocaleTimeString(app.settings.uiLanguage, { hour: "2-digit", minute: "2-digit", second: "2-digit" });
  }

  // Name of the area a line came from: the user's name, or "Area N".
  function areaName(h: HistoryEntry): string {
    return h.areaName.trim() || t("play.area.defaultName", { n: String(h.area + 1) });
  }
</script>

<div class="history">
  <header>
    <h1>{t("history.title")}</h1>
    <p class="sub">
      {session.game
        ? t("history.summary", { game: session.game.title, count: String(session.history.length), missed: String(missedCount) })
        : t("history.noSession")}
    </p>
  </header>

  {#if report}
    <!-- How long translations take to show up, and which stage takes the most -->
    <section class="latency" aria-label={t("history.latency.title")}>
      <div class="latency-head">
        <div>
          <span class="eyebrow">{t("history.latency.title")}</span>
          <p class="big">{ms(report.toFirst.median)}</p>
          <p class="sub">{t("history.latency.slow", { ms: ms(report.toFirst.p90) })}</p>
        </div>
        {#if biggest}<p class="advice">{t(`history.latency.biggest.${biggest}`)}</p>{/if}
      </div>
      <div class="bar" role="img" aria-label={stageKeys.map((k) => `${t(`history.latency.${k}`)} ${ms(report.stages[k].median)}`).join(", ")}>
        {#each stageKeys as k (k)}
          <span class="seg {k}" class:big={k === biggest} style:flex-grow={Math.max(report.stages[k].median, 1)}></span>
        {/each}
      </div>
      <ul class="legend">
        {#each stageKeys as k (k)}
          <li><span class="dot {k}"></span>{t(`history.latency.${k}`)} <strong>{ms(report.stages[k].median)}</strong></li>
        {/each}
        <li class="rest">{t("history.latency.rest", { ms: ms(report.stages.rest.median) })}</li>
      </ul>
    </section>
  {/if}

  {#if session.history.length}
    <div class="tools">
      <input class="input search" type="search" bind:value={query} placeholder={t("history.search")} />
      <label class="toggle">
        <input type="checkbox" bind:checked={onlyMissed} />
        {t("history.onlyMissed")}
      </label>
      <!-- Only in memory: closing Subquest clears it anyway -->
      <button class="btn btn-ghost" onclick={() => (session.history = [])}>{t("history.clear")}</button>
    </div>

    <ol class="lines">
      {#each shown as h (h.frameId)}
        <li>
          <div class="meta">
            <span class="time">{time(h.at)}</span>
            <span class="area">{areaName(h)}</span>
            {#if lineWait(h.timing) != null}<span class="wait" title={stagesOf(h.timing)}>{ms(lineWait(h.timing)!)}</span>{/if}
            {#if h.missed}<span class="tag missed-tag">{t("history.missed")}</span>{/if}
            {#if h.quick && h.done}<span class="tag">{t("history.quick")}</span>{/if}
            {#if h.cached && h.done}<span class="tag">{t("history.cached")}</span>{/if}
            {#if h.rereads > 0}<span class="tag" title={t("history.rereadHelp")}>{t("history.reread", { n: String(h.rereads) })}</span>{/if}
            {#if h.terms.length}<span class="tag glossary-tag">{t("history.glossary", { terms: h.terms.join(", ") })}</span>{/if}
          </div>
          <LinePair
            original={h.original}
            translation={h.translation}
            streaming={!h.done && !!h.translation}
            failed={h.failed ? t("history.failed") : ""}
          />
        </li>
      {:else}
        <li class="empty-filter">{t("history.nothingMatches")}</li>
      {/each}
    </ol>
  {:else}
    <div class="empty">
      <p>{t("history.empty")}</p>
      {#if !session.running}
        <button class="btn btn-primary" onclick={() => (app.screen = "play")}>{t("nav.play")}</button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .history {
    max-width: 760px;
    margin: 0 auto;
    display: grid;
    gap: 18px;
    padding-bottom: 40px;
  }
  h1 {
    margin: 0;
  }
  .sub {
    margin: 4px 0 0;
    color: var(--ink-dim);
    font-size: var(--text-sm);
  }

  .tools {
    display: flex;
    gap: 14px;
    align-items: center;
  }
  .search {
    flex: 1;
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--ink-dim);
    font-size: var(--text-sm);
    white-space: nowrap;
    cursor: pointer;
  }
  .toggle input {
    accent-color: var(--gold);
  }

  .lines {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 16px;
  }
  .lines li {
    display: grid;
    gap: 6px;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 10px;
    color: var(--ink-faint);
    font-size: 12px;
    font-variant-numeric: tabular-nums;
  }
  .area {
    color: var(--ink-dim);
  }
  .tag {
    padding: 1px 7px;
    border: 1px solid var(--line);
    border-radius: 999px;
  }
  .missed-tag {
    border-color: var(--gold);
    color: var(--gold);
  }
  .glossary-tag {
    border-color: var(--live);
    color: var(--live);
  }
  .wait {
    color: var(--ink-faint);
  }

  /* The latency panel: the usual wait in big type, its stages as one bar */
  .latency {
    display: grid;
    gap: 12px;
    padding: 16px 18px;
    border: 1px solid var(--line);
    border-radius: var(--radius-control);
    background: var(--night-deep);
  }
  .latency-head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 20px;
  }
  .eyebrow {
    color: var(--gold);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  .big {
    margin: 2px 0 0;
    font-family: var(--font-display);
    font-size: var(--text-xl);
    font-weight: 700;
    line-height: 1.1;
    font-variant-numeric: tabular-nums;
  }
  .advice {
    margin: 0;
    max-width: 42ch;
    color: var(--ink-dim);
    font-size: var(--text-sm);
    text-align: right;
  }
  .bar {
    display: flex;
    gap: 3px;
    height: 10px;
  }
  .seg {
    min-width: 6px;
    border-radius: 3px;
    opacity: 0.55;
  }
  .seg.big {
    opacity: 1;
  }
  .settle {
    background: var(--ink-dim);
  }
  .read {
    background: var(--live);
  }
  .first {
    background: var(--gold);
  }
  .legend {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px 18px;
    color: var(--ink-dim);
    font-size: var(--text-sm);
  }
  .legend li {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .legend strong {
    color: var(--ink);
    font-variant-numeric: tabular-nums;
  }
  .legend .rest {
    margin-left: auto;
    color: var(--ink-faint);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .empty,
  .empty-filter {
    color: var(--ink-dim);
  }
  .empty {
    display: grid;
    justify-items: start;
    gap: 12px;
  }
</style>
