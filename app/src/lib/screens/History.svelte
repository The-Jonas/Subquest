<!--
  History — every line of the session with its translation. Games like Hades
  drop a line out of nowhere and move on in a second; the ones that passed
  before their translation was ready are marked, so they can be read here.
-->
<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { app } from "../stores/app.svelte";
  import { session, type HistoryEntry } from "../stores/session.svelte";

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

  {#if session.history.length}
    <div class="tools">
      <input class="input search" type="search" bind:value={query} placeholder={t("history.search")} />
      <label class="toggle">
        <input type="checkbox" bind:checked={onlyMissed} />
        {t("history.onlyMissed")}
      </label>
    </div>

    <ol class="lines">
      {#each shown as h (h.frameId)}
        <li class:missed={h.missed}>
          <div class="meta">
            <span class="time">{time(h.at)}</span>
            <span class="area">{areaName(h)}</span>
            {#if h.missed}<span class="tag missed-tag">{t("history.missed")}</span>{/if}
            {#if h.quick && h.done}<span class="tag">{t("history.quick")}</span>{/if}
          </div>
          {#if h.failed}
            <p class="text failed">{t("history.failed")}</p>
          {:else if h.translation}
            <p class="text" class:streaming={!h.done}>{h.translation}</p>
          {:else}
            <p class="text pending">…</p>
          {/if}
          <p class="original">{h.original}</p>
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
    gap: 8px;
  }
  .lines li {
    display: grid;
    gap: 4px;
    padding: 12px 16px;
    border: 1px solid var(--line);
    border-radius: var(--radius-control);
    background: var(--night-deep);
  }
  /* Missed lines: the reason this screen exists, so they get the accent */
  .lines li.missed {
    border-left: 3px solid var(--gold);
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
  .text {
    margin: 0;
    font-size: var(--text-md, 1.05rem);
    white-space: pre-line;
  }
  .streaming::after {
    content: "▍";
    margin-left: 2px;
    color: var(--gold);
  }
  .pending {
    color: var(--ink-faint);
  }
  .failed {
    color: var(--danger);
    font-size: var(--text-sm);
  }
  .original {
    margin: 0;
    color: var(--ink-dim);
    font-size: var(--text-sm);
    white-space: pre-line;
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
