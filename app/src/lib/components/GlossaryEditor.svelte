<!--
  GlossaryEditor — a game's glossary: terms that must always be translated the
  same way, or never translated (names). Each change is saved on its own; the
  next lines read from the game already use it.
-->
<script lang="ts">
  import { tick } from "svelte";
  import { t } from "../i18n/index.svelte";
  import { errorKind, getGlossary, saveGlossary, type GlossaryEntry } from "../api";

  interface Props {
    gameKey: string; // profile key of the game
  }
  let { gameKey }: Props = $props();

  let entries = $state<GlossaryEntry[]>([]);
  let savedAt = $state(0); // last save, for the short "Saved" note
  let list: HTMLUListElement | undefined = $state();

  // Loads the game's glossary whenever the game changes.
  $effect(() => {
    const key = gameKey;
    entries = [];
    getGlossary(key)
      .then((loaded) => {
        if (key === gameKey) entries = loaded;
      })
      .catch(errorKind);
  });

  // Saves every entry that has a term (rows still being typed are kept on screen).
  async function save() {
    const kept = $state.snapshot(entries).filter((e) => e.source.trim());
    try {
      await saveGlossary(gameKey, kept);
      savedAt = Date.now();
    } catch (e) {
      errorKind(e);
    }
  }

  // New empty row, with the cursor in its first field.
  async function add() {
    entries.push({ source: "", target: "", keep: false });
    await tick();
    list?.querySelector<HTMLInputElement>("li:last-child input")?.focus();
  }

  // Removes a row and saves.
  function remove(i: number) {
    entries.splice(i, 1);
    void save();
  }

  // "Saved" shows for two seconds after each save.
  let showSaved = $state(false);
  $effect(() => {
    if (!savedAt) return;
    showSaved = true;
    const timer = setTimeout(() => (showSaved = false), 2000);
    return () => clearTimeout(timer);
  });
</script>

<section class="block">
  <div class="head">
    <h2>{t("glossary.title")}</h2>
    {#if showSaved}<span class="saved" role="status">{t("glossary.saved")}</span>{/if}
    <button class="btn btn-ghost" onclick={add}>{t("glossary.add")}</button>
  </div>
  <p class="hint">{t("glossary.help")}</p>

  {#if entries.length === 0}
    <p class="none">{t("glossary.empty")}</p>
  {:else}
    <ul class="rows" bind:this={list}>
      <li class="labels" aria-hidden="true">
        <span>{t("glossary.source")}</span>
        <span class="arrow hidden">→</span>
        <span>{t("glossary.target")}</span>
        <span></span>
        <span></span>
      </li>
      {#each entries as entry, i (i)}
        <li>
          <input
            class="input"
            bind:value={entry.source}
            placeholder={t("glossary.sourcePlaceholder")}
            aria-label={t("glossary.source")}
            onchange={save}
          />
          <span class="arrow" aria-hidden="true">→</span>
          <input
            class="input"
            bind:value={entry.target}
            placeholder={entry.keep ? t("glossary.kept") : t("glossary.targetPlaceholder")}
            aria-label={t("glossary.target")}
            disabled={entry.keep}
            onchange={save}
          />
          <label class="keep" title={t("glossary.keepHelp")}>
            <input type="checkbox" bind:checked={entry.keep} onchange={save} />
            {t("glossary.keep")}
          </label>
          <button class="remove" onclick={() => remove(i)} aria-label={t("glossary.remove")} title={t("glossary.remove")}>×</button>
        </li>
      {/each}
    </ul>
  {/if}
  <p class="hint">{t("glossary.quickNote")}</p>
</section>

<style>
  .block {
    display: grid;
    gap: 10px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  h2 {
    margin: 0;
    font-size: var(--text-lg);
    flex: 1;
  }
  .saved {
    color: var(--live);
    font-size: var(--text-sm);
  }
  .hint,
  .none {
    margin: 0;
    color: var(--ink-faint);
    font-size: var(--text-sm);
    max-width: 75ch;
  }
  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 6px;
  }
  .rows li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 10px;
  }
  .labels {
    color: var(--ink-faint);
    font-size: 12px;
  }
  .arrow {
    color: var(--ink-faint);
  }
  .arrow.hidden {
    visibility: hidden; /* keeps the header columns lined up with the rows */
  }
  .input:disabled {
    opacity: 0.55;
  }
  .keep {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--ink-dim);
    font-size: var(--text-sm);
    white-space: nowrap;
    cursor: pointer;
  }
  .keep input {
    accent-color: var(--gold);
  }
  .remove {
    width: 30px;
    height: 30px;
    border: 1px solid transparent;
    border-radius: 50%;
    background: transparent;
    color: var(--ink-faint);
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
  }
  .remove:hover {
    color: var(--danger);
    border-color: var(--line);
  }
</style>
