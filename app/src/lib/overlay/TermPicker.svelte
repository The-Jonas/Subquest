<!--
  TermPicker — picks a word right over the game for the glossary.

  The words come from the last OCR reading of each area (with their boxes), so
  the user aims at the game's own text: hover shows a word, a click picks it,
  a drag picks several in a row. A small box then asks whether the term is
  translated (and as what) or kept as it is. Saving reads the screen again,
  so the line on screen comes back translated with the new term.
-->
<script lang="ts">
  import { onMount, tick } from "svelte";
  import { t } from "../i18n/index.svelte";
  import { errorKind, getGlossary, saveGlossary, type GlossaryEntry, type OcrTextEvent } from "../api";
  import { findEntry, joinWords, upsertEntry } from "../glossary";

  interface Props {
    events: OcrTextEvent[]; // the reading on screen now, one per area
    viewWidth: number; // overlay size in CSS px (the game window)
    gameKey: string;
    ondone: () => void; // saved or cancelled: the game gets the mouse back
  }
  let { events, viewWidth, gameKey, ondone }: Props = $props();

  // Every word on screen, in CSS px, with the line it belongs to.
  interface Spot {
    line: number; // index among all lines on screen
    index: number; // word index within its line
    text: string;
    left: number;
    top: number;
    width: number;
    height: number;
  }
  const spots = $derived.by(() => {
    const out: Spot[] = [];
    let line = 0;
    for (const e of events) {
      const scale = e.frameWidth > 0 ? viewWidth / e.frameWidth : 1;
      for (const l of e.lines) {
        (l.words ?? []).forEach((w, index) => {
          const [x, y, width, height] = w.rect;
          out.push({ line, index, text: w.text, left: x * scale, top: y * scale, width: width * scale, height: height * scale });
        });
        line += 1;
      }
    }
    return out;
  });

  // The words picked: one line, from one word to another (in any direction).
  let pick = $state<{ line: number; from: number; to: number } | null>(null);
  let dragging = $state(false);
  const picked = $derived(
    pick
      ? spots.filter((s) => s.line === pick!.line && s.index >= Math.min(pick!.from, pick!.to) && s.index <= Math.max(pick!.from, pick!.to))
      : [],
  );

  // The question box, once the pick is done.
  let asking = $state(false);
  let source = $state("");
  let target = $state("");
  let keep = $state(false);
  let existing = $state<GlossaryEntry | undefined>();
  let entries = $state<GlossaryEntry[]>([]); // the game's glossary (words already in it are outlined)
  let saving = $state(false);
  let sourceInput: HTMLInputElement | undefined = $state();
  let targetInput: HTMLInputElement | undefined = $state();

  // Pressing on a word starts the pick; pressing elsewhere cancels a pick in progress.
  function down(e: PointerEvent, spot: Spot) {
    if (e.button !== 0) return;
    e.stopPropagation();
    asking = false;
    pick = { line: spot.line, from: spot.index, to: spot.index };
    dragging = true;
  }

  // Dragging over another word of the same line stretches the pick.
  function over(spot: Spot) {
    if (dragging && pick && spot.line === pick.line) pick.to = spot.index;
  }

  // Letting go: the picked words become the term, and the question box opens.
  async function up() {
    if (!dragging || !pick) return;
    dragging = false;
    source = joinWords(picked.map((s) => s.text));
    existing = findEntry(entries, source);
    keep = existing?.keep ?? false;
    target = existing && !existing.keep ? existing.target : "";
    asking = true;
    await tick();
    if (!keep) targetInput?.focus(); // a kept term has nothing to type: Enter saves it as it is
  }

  // Saves the term (adding it or changing the one already there) and ends picking.
  async function save() {
    if (!source.trim() || saving) return;
    saving = true;
    try {
      entries = await getGlossary(gameKey);
      await saveGlossary(gameKey, upsertEntry(entries, { source, target, keep }));
      ondone();
    } catch (e) {
      errorKind(e);
      saving = false;
    }
  }

  // Esc closes the box first, then leaves picking; Enter saves.
  function key(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      if (asking) {
        asking = false;
        pick = null;
      } else ondone();
    } else if (e.key === "Enter" && asking) {
      e.preventDefault();
      void save();
    }
  }

  // Where the question box goes: under the picked words (above them near the
  // bottom of the screen), with its little arrow pointing at their middle.
  const BOX_WIDTH = 380;
  const boxAt = $derived.by(() => {
    if (!picked.length) return { left: 0, top: 0, above: false, caret: 0 };
    const x0 = Math.min(...picked.map((s) => s.left));
    const x1 = Math.max(...picked.map((s) => s.left + s.width));
    const top = Math.min(...picked.map((s) => s.top));
    const bottom = Math.max(...picked.map((s) => s.top + s.height));
    const above = bottom > window.innerHeight - 300;
    const middle = (x0 + x1) / 2;
    const left = Math.max(12, Math.min(middle - 60, window.innerWidth - BOX_WIDTH - 12));
    const caret = Math.max(22, Math.min(middle - left, BOX_WIDTH - 22));
    return { left, top: above ? top - 14 : bottom + 14, above, caret };
  });

  // The line the term came from, with the picked words marked, so it is clear
  // which sentence it belongs to.
  const context = $derived.by(() => {
    if (!pick) return { words: [] as { text: string; on: boolean }[], spaced: true };
    const words = spots.filter((s) => s.line === pick!.line).map((s) => ({ text: s.text, on: picked.includes(s) }));
    return { words, spaced: !/[\u3040-\u30ff\u3400-\u9fff]/u.test(words.map((w) => w.text).join("")) };
  });

  // Closes the question box and lets another word be picked.
  function closeBox() {
    asking = false;
    pick = null;
  }

  // Switches between translating the term and keeping it as it is.
  async function choose(keepIt: boolean) {
    keep = keepIt;
    await tick();
    if (!keepIt) targetInput?.focus();
  }

  onMount(() => {
    getGlossary(gameKey)
      .then((loaded) => (entries = loaded))
      .catch(errorKind);
  });
</script>

<svelte:window onkeydown={key} onpointerup={up} />

<div
  class="layer"
  onpointerdown={(e) => {
    if (e.button === 2) ondone();
    else if (!asking) pick = null;
  }}
  oncontextmenu={(e) => e.preventDefault()}
  role="presentation"
>
  <div class="hint">
    <strong>{spots.length ? t("glossary.pick.title") : t("glossary.pick.nothing")}</strong>
    {#if spots.length}<span>{t("glossary.pick.howTo")}</span>{/if}
    <button onpointerdown={(e) => e.stopPropagation()} onclick={ondone}>{t("glossary.pick.cancel")}</button>
  </div>

  {#each spots as s (`${s.line}-${s.index}`)}
    <div
      class="word"
      class:picked={picked.includes(s)}
      class:known={!!findEntry(entries, s.text)}
      style:left="{s.left}px"
      style:top="{s.top}px"
      style:width="{s.width}px"
      style:height="{s.height}px"
      onpointerdown={(e) => down(e, s)}
      onpointerenter={() => over(s)}
      role="presentation"
    ></div>
  {/each}

  {#if asking}
    <form
      class="ask"
      class:above={boxAt.above}
      style:left="{boxAt.left}px"
      style:top="{boxAt.top}px"
      style:width="{BOX_WIDTH}px"
      style:--caret="{boxAt.caret}px"
      onpointerdown={(e) => e.stopPropagation()}
      onsubmit={(e) => {
        e.preventDefault();
        void save();
      }}
    >
      <header>
        <svg class="book" viewBox="0 0 20 20" aria-hidden="true">
          <path d="M3 4.5h5a2 2 0 0 1 2 1.5a2 2 0 0 1 2-1.5h5v11h-5a2 2 0 0 0-2 1.5a2 2 0 0 0-2-1.5H3zM10 6v10.5" />
        </svg>
        <span class="eyebrow">{t("glossary.title")}</span>
        {#if existing}<span class="badge">{t("glossary.pick.inGlossary")}</span>{/if}
        <button type="button" class="close" onclick={closeBox} aria-label={t("glossary.pick.cancel")}>×</button>
      </header>

      <!-- The game's line, with the picked words marked -->
      <p class="context">
        {#each context.words as w, i (i)}{#if i > 0 && context.spaced}{" "}{/if}<span class:on={w.on}>{w.text}</span>{/each}
      </p>

      <input class="term" bind:this={sourceInput} bind:value={source} aria-label={t("glossary.pick.term")} spellcheck="false" />

      <div class="switch" class:right={keep} role="radiogroup" aria-label={t("glossary.pick.term")}>
        <span class="thumb" aria-hidden="true"></span>
        <button type="button" role="radio" aria-checked={!keep} onclick={() => choose(false)}>{t("glossary.pick.translate")}</button>
        <button type="button" role="radio" aria-checked={keep} onclick={() => choose(true)} title={t("glossary.pick.keepHelp")}>
          {t("glossary.pick.keep")}
        </button>
      </div>

      <div class="result">
        <span class="arrow" aria-hidden="true">→</span>
        {#if keep}
          <span class="kept"><strong>{source.trim() || "…"}</strong> · {t("glossary.pick.keptShort")}</span>
        {:else}
          <input
            class="target"
            bind:this={targetInput}
            bind:value={target}
            placeholder={t("glossary.pick.targetPlaceholder")}
            aria-label={t("glossary.pick.translate")}
          />
        {/if}
      </div>

      <footer>
        <span class="keys"><kbd>Enter</kbd> {t("glossary.pick.keysSave")} · <kbd>Esc</kbd> {t("glossary.pick.keysCancel")}</span>
        <button type="submit" class="primary" disabled={saving || !source.trim() || (!keep && !target.trim())}>
          {saving ? t("glossary.pick.saving") : t("glossary.pick.save")}
        </button>
      </footer>
    </form>
  {/if}
</div>

<style>
  /* Lighter than the area selection: the words must stay readable */
  .layer {
    position: absolute;
    inset: 0;
    z-index: 10;
    background: rgb(8 10 22 / 0.2);
    cursor: default;
    font-family: "Atkinson Hyperlegible Next Variable", system-ui, sans-serif;
  }
  .hint {
    position: absolute;
    top: 24px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 12px 10px 18px;
    border: 1.5px solid color-mix(in srgb, var(--gold) 55%, transparent);
    border-radius: 12px;
    background: color-mix(in srgb, var(--night-deep) 94%, transparent);
    color: var(--ink);
    font-size: 13px;
    white-space: nowrap;
  }
  .hint span {
    color: var(--ink-dim);
  }
  .hint button {
    padding: 6px 12px;
    border: 1px solid var(--line);
    border-radius: 6px;
    background: transparent;
    color: var(--ink);
    cursor: pointer;
  }
  /* Each word: a faint outline to aim at, gold when hovered or picked */
  .word {
    position: absolute;
    margin: -3px;
    padding: 3px;
    box-sizing: content-box;
    border: 1.5px solid rgb(255 255 255 / 0.18);
    border-radius: 4px;
    cursor: pointer;
  }
  .word.known {
    border-color: color-mix(in srgb, var(--live) 70%, transparent);
  }
  .word:hover {
    border-color: var(--gold);
    background: color-mix(in srgb, var(--gold) 12%, transparent);
  }
  .word.picked {
    border-color: var(--gold);
    background: color-mix(in srgb, var(--gold) 28%, transparent);
  }
  /* The question box: a little card from the same family as the toolbar,
     with an arrow pointing at the picked words */
  .ask {
    position: absolute;
    box-sizing: border-box;
    display: grid;
    gap: 12px;
    padding: 14px 16px 14px;
    border: 1.5px solid color-mix(in srgb, var(--gold) 50%, transparent);
    border-radius: 14px;
    background:
      linear-gradient(180deg, color-mix(in srgb, var(--gold) 7%, transparent), transparent 70px),
      var(--night-deep);
    color: var(--ink);
    box-shadow:
      0 18px 40px rgb(0 0 0 / 0.5),
      0 0 0 1px rgb(0 0 0 / 0.35);
    animation: rise 140ms ease-out;
  }
  .ask.above {
    transform: translateY(-100%);
    animation-name: drop;
  }
  /* The arrow: a small rotated square on the edge facing the words */
  .ask::before {
    content: "";
    position: absolute;
    left: calc(var(--caret) - 7px);
    top: -8px;
    width: 12px;
    height: 12px;
    background: color-mix(in srgb, var(--gold) 7%, var(--night-deep));
    border-left: 1.5px solid color-mix(in srgb, var(--gold) 50%, transparent);
    border-top: 1.5px solid color-mix(in srgb, var(--gold) 50%, transparent);
    transform: rotate(45deg);
  }
  .ask.above::before {
    top: auto;
    bottom: -8px;
    background: var(--night-deep);
    transform: rotate(225deg);
  }
  @keyframes rise {
    from {
      opacity: 0;
      translate: 0 6px;
    }
  }
  @keyframes drop {
    from {
      opacity: 0;
      translate: 0 -6px;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .ask {
      animation: none;
    }
  }

  header {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .book {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: var(--gold);
    stroke-width: 1.6;
    stroke-linejoin: round;
  }
  .eyebrow {
    color: var(--gold);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }
  .badge {
    padding: 2px 8px;
    border: 1px solid color-mix(in srgb, var(--live) 60%, transparent);
    border-radius: 999px;
    color: var(--live);
    font-size: 11px;
  }
  .close {
    margin-left: auto;
    width: 26px;
    height: 26px;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--ink-faint);
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
  }
  .close:hover {
    background: rgb(255 255 255 / 0.06);
    color: var(--ink);
  }

  /* The game's line, quiet, with the picked words underlined in the accent */
  .context {
    margin: 0;
    color: var(--ink-faint);
    font-size: 13px;
    line-height: 1.45;
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .context .on {
    color: var(--ink);
    text-decoration: underline;
    text-decoration-color: var(--gold);
    text-decoration-thickness: 2px;
    text-underline-offset: 3px;
  }

  /* The term itself, big: it reads like a title, but can still be fixed */
  .term {
    box-sizing: border-box;
    width: 100%;
    margin: -2px 0 0 -6px;
    padding: 2px 6px;
    border: 1px solid transparent;
    border-radius: 8px;
    background: transparent;
    color: var(--ink);
    font: 700 26px/1.2 "Archivo Variable", "Atkinson Hyperlegible Next Variable", system-ui, sans-serif;
    font-stretch: 85%;
  }
  .term:hover {
    border-color: var(--line);
  }
  .term:focus {
    outline: none;
    border-color: var(--gold);
    background: var(--night);
  }

  /* Translate / keep: one switch with a sliding thumb */
  .switch {
    position: relative;
    display: grid;
    grid-template-columns: 1fr 1fr;
    padding: 3px;
    border-radius: 10px;
    background: var(--night);
    border: 1px solid var(--line);
  }
  .thumb {
    position: absolute;
    top: 3px;
    bottom: 3px;
    left: 3px;
    width: calc(50% - 3px);
    border-radius: 7px;
    background: var(--panel-raised);
    box-shadow: inset 0 0 0 1px color-mix(in srgb, var(--gold) 55%, transparent);
    transition: translate 160ms ease;
  }
  .switch.right .thumb {
    translate: 100% 0;
  }
  .switch button {
    position: relative; /* above the thumb */
    padding: 7px 10px;
    border: 0;
    background: transparent;
    color: var(--ink-dim);
    font: inherit;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
  }
  .switch button[aria-checked="true"] {
    color: var(--ink);
  }

  /* What the term becomes */
  .result {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 42px;
  }
  .arrow {
    color: var(--gold);
    font-size: 18px;
  }
  .target {
    flex: 1;
    box-sizing: border-box;
    padding: 9px 11px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--night);
    color: var(--ink);
    font: inherit;
    font-size: 16px;
  }
  .target:focus {
    outline: none;
    border-color: var(--gold);
  }
  .kept {
    color: var(--ink-dim);
    font-size: 14px;
  }
  .kept strong {
    color: var(--ink);
    font-size: 16px;
  }

  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding-top: 10px;
    border-top: 1px solid var(--line);
  }
  .keys {
    color: var(--ink-faint);
    font-size: 12px;
  }
  kbd {
    padding: 1px 6px;
    border: 1px solid var(--line);
    border-bottom-width: 2px;
    border-radius: 5px;
    color: var(--ink-dim);
    font: inherit;
    font-size: 11px;
  }
  .primary {
    padding: 9px 16px;
    border: 0;
    border-radius: 9px;
    background: var(--gold);
    color: var(--gold-ink);
    font: inherit;
    font-size: 14px;
    font-weight: 700;
    cursor: pointer;
  }
  .primary:hover:not(:disabled) {
    filter: brightness(1.08);
  }
  .primary:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
