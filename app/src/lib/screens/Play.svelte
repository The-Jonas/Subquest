<!--
  Play — the main three-step flow: game -> area -> start.
  Each step is a "line" of the dialogue box.
-->
<script lang="ts">
  import { onDestroy } from "svelte";
  import DialogueBox from "../components/DialogueBox.svelte";
  import GamePreview from "../components/GamePreview.svelte";
  import { t } from "../i18n/index.svelte";
  import { app } from "../stores/app.svelte";
  import {
    inTauri,
    listWindows,
    startSession,
    stopSession,
    onOcrText,
    errorKind,
    type GameWindow,
    type OcrTextEvent,
  } from "../api";

  type Step = 0 | 1 | 2;
  type AreaMode = "dialogue" | "screen";

  let step = $state<Step>(0);
  let windows = $state<GameWindow[]>([]);
  let loading = $state(false);
  let game = $state<GameWindow | null>(null);
  let areaMode = $state<AreaMode>("dialogue");
  let running = $state(false);
  let error = $state<string | null>(null);
  let lines = $state<OcrTextEvent[]>([]);
  let unlisten: (() => void) | null = null;

  const stepKeys = ["game", "area", "start"] as const;
  const areaModes: AreaMode[] = ["dialogue", "screen"];

  // In the real app, without the core DLL there is nothing to list or translate.
  const coreMissing = $derived(inTauri && !app.core.loaded);

  const question = $derived(
    running ? t("play.start.running", { game: game?.title ?? "" }) : t(`play.${stepKeys[step]}.question`),
  );

  // Fetches open windows; keeps the previous list on error.
  async function refresh() {
    if (coreMissing) return;
    loading = true;
    error = null;
    try {
      windows = await listWindows();
    } catch (e) {
      error = t(`errors.${errorKind(e)}`);
    } finally {
      loading = false;
    }
  }

  // Picks the game and moves on to the area step.
  function pick(w: GameWindow) {
    game = w;
    error = null;
    step = 1;
  }

  // Starts the core session and begins listening for recognized lines.
  async function start() {
    if (!game) return;
    error = null;
    try {
      unlisten = await onOcrText((e) => (lines = [e, ...lines].slice(0, 6)));
      await startSession(game.handle, null);
      running = true;
    } catch (e) {
      unlisten?.();
      error = t(`errors.${errorKind(e)}`);
    }
  }

  // Stops the session and goes back to the summary.
  async function stop() {
    try {
      await stopSession();
    } finally {
      unlisten?.();
      unlisten = null;
      running = false;
    }
  }

  onDestroy(() => unlisten?.());
  refresh();
</script>

<div class="play">
  <DialogueBox speaker={t("play.speaker")} line={question}>
    {#if running}
      <div class="feed" aria-live="polite">
        {#each lines as l (l.frameId)}
          <p>{l.text}</p>
        {:else}
          <p class="muted live">{t("play.start.waiting")}</p>
        {/each}
      </div>
      <div class="actions">
        <button class="btn" onclick={stop}>{t("play.start.stop")}</button>
      </div>
    {:else if step === 0 && coreMissing}
      <div class="notice" role="status">
        <strong>{t("core.missing")}</strong>
        <p>{t("core.missingHelp")}</p>
      </div>
    {:else if step === 0}
      <p class="muted">{t("play.game.hint")}</p>
      <ul class="windows">
        {#each windows as w (w.handle)}
          <li>
            <button class="window" class:selected={game?.handle === w.handle} onclick={() => pick(w)}>
              <strong>{w.title}</strong>
              <span>{w.process}</span>
            </button>
          </li>
        {:else}
          <li class="muted">{loading ? "…" : t("play.game.empty")}</li>
        {/each}
      </ul>
      <div class="actions">
        <button class="btn btn-ghost" onclick={refresh} disabled={loading}>{t("play.game.refresh")}</button>
      </div>
    {:else if step === 1}
      {#if game}
        <GamePreview windowHandle={game.handle} />
      {/if}
      <div class="choices" role="radiogroup">
        {#each areaModes as mode (mode)}
          <label class="choice" class:selected={areaMode === mode}>
            <input type="radio" name="area" value={mode} bind:group={areaMode} />
            <strong>{t(`play.area.${mode}`)}</strong>
            <span>{t(`play.area.${mode}Help`)}</span>
          </label>
        {/each}
      </div>
      {#if areaMode === "dialogue"}
        <p class="note">{t("play.area.notYet")}</p>
      {/if}
      <div class="actions">
        <button class="btn btn-ghost" onclick={() => (step = 0)}>{t("play.back")}</button>
        <button class="btn btn-primary" onclick={() => (step = 2)}>{t("play.next")}</button>
      </div>
    {:else}
      <p class="summary">
        {t("play.start.summary", {
          game: game?.title ?? "",
          source: t(`languages.${app.settings.sourceLanguage}`),
          target: t(`languages.${app.settings.targetLanguage}`),
        })}
      </p>
      <div class="actions">
        <button class="btn btn-ghost" onclick={() => (step = 0)}>{t("play.start.change")}</button>
        <button class="btn btn-primary" onclick={start}>{t("play.start.button")}</button>
      </div>
    {/if}

    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}
  </DialogueBox>

  <!-- Step track: numbering makes sense here, it is a real sequence -->
  <ol class="track" aria-label={t("play.steps.label")}>
    {#each stepKeys as key, i (key)}
      <li class:done={i < step || running} class:current={i === step && !running}>
        <span class="num">{i + 1}</span>
        {t(`play.steps.${key}`)}
      </li>
    {/each}
  </ol>
</div>

<style>
  .play {
    max-width: 680px;
    margin: 0 auto;
    padding-top: 28px;
    display: grid;
    gap: 22px;
  }

  .muted {
    color: var(--ink-dim);
  }

  /* Missing-core notice: alert border, without dumping the technical error on screen */
  .notice {
    display: grid;
    gap: 6px;
    padding: 14px 16px;
    border: 1px solid var(--line);
    border-left: 3px solid var(--danger);
    border-radius: var(--radius-control);
    background: var(--night-deep);
    color: var(--ink-dim);
  }
  .notice strong {
    color: var(--ink);
  }

  .windows {
    list-style: none;
    margin: 16px 0 0;
    padding: 0;
    display: grid;
    gap: 6px;
    max-height: 260px;
    overflow-y: auto;
  }

  .window {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 16px;
    padding: 12px 14px;
    border: 1px solid var(--line);
    border-radius: var(--radius-control);
    background: var(--night-deep);
    cursor: pointer;
    text-align: left;
  }
  .window span {
    color: var(--ink-faint);
    font-size: var(--text-sm);
  }
  .window:hover,
  .window.selected {
    border-color: var(--gold);
  }

  .choices {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .choice {
    display: grid;
    gap: 6px;
    align-content: start;
    padding: 14px 16px;
    border: 1px solid var(--line);
    border-radius: var(--radius-control);
    background: var(--night-deep);
    cursor: pointer;
  }
  .choice span {
    color: var(--ink-dim);
    font-size: var(--text-sm);
  }
  .choice.selected {
    border-color: var(--gold);
    box-shadow: inset 0 0 0 1px var(--gold);
  }
  .choice input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .choice:has(input:focus-visible) {
    outline: 2px solid var(--gold);
    outline-offset: 2px;
  }

  .note {
    margin-top: 12px;
    color: var(--ink-faint);
    font-size: var(--text-sm);
  }

  .summary {
    font-size: var(--text-md);
  }

  .feed {
    display: grid;
    gap: 8px;
  }
  .feed p:first-child:not(.muted) {
    font-size: var(--text-md);
  }
  .live {
    color: var(--live);
  }

  .actions {
    margin-top: 22px;
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .error {
    margin-top: 14px;
    color: var(--danger);
    font-size: var(--text-sm);
  }

  .track {
    list-style: none;
    margin: 0;
    padding: 0 8px;
    display: flex;
    gap: 22px;
    color: var(--ink-faint);
    font-size: var(--text-sm);
  }
  .track li {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .num {
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    border: 1px solid var(--line);
    font-weight: 700;
    font-size: 12px;
  }
  .track li.current {
    color: var(--ink);
  }
  .track li.current .num {
    background: var(--gold);
    border-color: var(--gold);
    color: var(--gold-ink);
  }
  .track li.done {
    color: var(--ink-dim);
  }
  .track li.done .num {
    border-color: var(--ink-dim);
  }
</style>
