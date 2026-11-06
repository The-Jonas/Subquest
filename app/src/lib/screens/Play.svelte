<!--
  Play — the main three-step flow: game -> area -> start.
  Each step is a "line" of the dialogue box. While translating, the areas can
  be adjusted ("Adjust areas") without stopping: the session keeps running and
  picks up the new areas when they are applied.
-->
<script lang="ts">
  import DialogueBox from "../components/DialogueBox.svelte";
  import GamePreview from "../components/GamePreview.svelte";
  import { t } from "../i18n/index.svelte";
  import { app, persistSettings } from "../stores/app.svelte";
  import { session, startTranslating, stopTranslating, applyAreas } from "../stores/session.svelte";
  import {
    inTauri,
    listWindows,
    toggleToolbar,
    windowThumbnail,
    pictureUrl,
    errorKind,
    profileKey,
    newArea,
    type GameArea,
    type GameWindow,
    type OcrTextEvent,
    type TranslationEvent,
  } from "../api";

  type Step = 0 | 1 | 2;

  // Coming back to Play during a session shows the running game again.
  let step = $state<Step>(session.running ? 2 : 0);
  let windows = $state<GameWindow[]>([]);
  let thumbs = $state<Record<number, string>>({}); // window handle -> object URL of its picture
  let loading = $state(false);
  let game = $state<GameWindow | null>(session.game);
  let areas = $state<GameArea[]>([newArea()]); // the game's screen areas, edited in the area step
  let selected = $state(0); // area being edited
  let editing = $state(false); // adjusting the areas of the running session
  let applying = $state(false);
  let error = $state<string | null>(null);

  const running = $derived(session.running);
  const showFeed = $derived(running && !editing);

  const stepKeys = ["game", "area", "start"] as const;
  const readModes = ["dialogue", "screen"] as const; // marked areas, or the whole window (never both)
  const MAX_AREAS = 16; // same limit as the core (SQ_MAX_AREAS)
  const APPLY_TIMEOUT_MS = 8000; // applying new areas never takes this long when all is well

  const area = $derived(areas[selected] ?? areas[0]);
  // The game is read either through its marked areas or as one whole window.
  const wholeScreen = $derived(areas.length === 1 && areas[0].mode === "screen");

  // In the real app, without the core DLL there is nothing to list or translate.
  const coreMissing = $derived(inTauri && !app.core.loaded);

  const question = $derived(
    showFeed
      ? t("play.start.running", { game: game?.title ?? "" })
      : t(`play.${stepKeys[editing ? 1 : step]}.question`),
  );

  // Fetches open windows; keeps the previous list on error.
  async function refresh() {
    if (coreMissing) return;
    loading = true;
    error = null;
    try {
      windows = await listWindows();
      void loadThumbs(windows);
    } catch (e) {
      error = t(`errors.${errorKind(e)}`);
    } finally {
      loading = false;
    }
  }

  // Takes a small picture of each window, one after the other (a capture
  // each), so the games are recognized by sight. Old pictures are replaced.
  async function loadThumbs(list: GameWindow[]) {
    for (const w of list) {
      const blob = await windowThumbnail(w.handle);
      if (!blob || !windows.includes(w)) continue;
      thumbs[w.handle] = await pictureUrl(blob);
    }
  }

  // Every marked area needs its rectangle; the whole-window mode needs none.
  const canContinue = $derived(areas.every((a) => a.mode === "screen" || !!a.region));
  // First dialogue area still without a rectangle (it blocks Continue / Apply).
  const unmarked = $derived(areas.findIndex((a) => a.mode !== "screen" && !a.region));

  // Name shown for an area: the user's, or "Area N".
  function areaLabel(a: GameArea, i: number): string {
    return a.name.trim() || t("play.area.defaultName", { n: String(i + 1) });
  }

  // Loads a game's saved areas (or one new area) into the editor. Older profiles
  // could mix a whole-window area with marked ones: the marked ones win.
  function loadAreas(w: GameWindow) {
    const saved = $state.snapshot(app.settings.profiles[profileKey(w)]?.areas ?? []);
    const marked = saved.filter((a) => a.mode !== "screen");
    areas = marked.length ? marked : saved.length ? [saved[0]] : [newArea()];
    selected = 0;
  }

  // Switches how the game is read. Going back to areas restores the ones marked
  // before (still saved in the profile), so trying "whole window" loses nothing.
  function setReadMode(mode: (typeof readModes)[number]) {
    if (mode === "screen" ? wholeScreen : !wholeScreen) return;
    if (mode === "screen") {
      areas = [{ ...newArea(), mode: "screen" }];
    } else {
      const saved = game ? (app.settings.profiles[profileKey(game)]?.areas ?? []) : [];
      const marked = $state.snapshot(saved).filter((a) => a.mode !== "screen");
      areas = marked.length ? marked : [newArea()];
    }
    selected = 0;
  }

  // Picks the game, restores its saved areas (if any) and moves on to the area step.
  function pick(w: GameWindow) {
    game = w;
    error = null;
    loadAreas(w);
    step = 1;
  }

  // Opens the area editor over the running session.
  function adjust() {
    if (!game) return;
    error = null;
    loadAreas(game);
    editing = true;
  }

  // Saves the edited areas and hands them to the running session.
  async function apply() {
    if (!game || !canContinue) return;
    applying = true;
    error = null;
    try {
      await saveAreas();
      editing = false; // closes the preview first: the game is then captured by the session alone
      // Restarting reading takes a fraction of a second; if it ever hangs, say so instead of freezing.
      const timeout = new Promise<never>((_, reject) => setTimeout(() => reject(new Error("timeout")), APPLY_TIMEOUT_MS));
      await Promise.race([applyAreas(), timeout]);
    } catch (e) {
      error = e instanceof Error && e.message === "timeout" ? t("play.applyTimeout") : t(`errors.${errorKind(e)}`);
    } finally {
      applying = false;
    }
  }

  // Adds an area and selects it, so the next drag marks it.
  function addArea() {
    if (areas.length >= MAX_AREAS) return;
    areas.push(newArea());
    selected = areas.length - 1;
  }

  // Removes the selected area (a game always keeps at least one).
  function removeArea() {
    if (areas.length <= 1) return;
    areas.splice(selected, 1);
    selected = Math.min(selected, areas.length - 1);
  }

  // Writes the edited areas into the game's profile (the session reads them from there).
  async function saveAreas() {
    if (!game) return;
    app.settings.profiles[profileKey(game)] = { title: game.title, areas: $state.snapshot(areas) };
    await persistSettings();
  }

  // Saves the areas and moves on to the summary.
  async function confirmArea() {
    if (!game || !canContinue) return;
    try {
      await saveAreas();
    } catch (e) {
      errorKind(e); // not fatal: the session still works, it just won't be remembered
    }
    step = 2;
  }

  // Starts translating: the session lives in its own store, so it survives leaving this screen.
  async function start() {
    if (!game) return;
    error = null;
    try {
      await startTranslating(game);
    } catch (e) {
      error = t(`errors.${errorKind(e)}`);
    }
  }

  // Stops the session and goes back to the summary.
  async function stop() {
    editing = false;
    await stopTranslating().catch((e) => (error = t(`errors.${errorKind(e)}`)));
  }

  // Timing line for a line: OCR always, translation once it is done.
  function timing(line: OcrTextEvent, tr: TranslationEvent | undefined): string {
    const ocr = t("play.start.timing", { ocr: String(Math.round(line.ocrMs)), total: String(Math.round(line.totalMs)) });
    if (!tr?.done || tr.errorKind) return ocr;
    const key = tr.cached ? "play.start.cached" : tr.quick ? "play.start.quickDone" : "play.start.translated";
    return `${ocr}. ${t(key, { ms: String(Math.round(tr.elapsedMs)) })}`;
  }

  if (!session.running) refresh();
</script>

<!-- The area step gets a wider layout: a bigger preview makes marking precise -->
<div class="play" class:wide={(step === 1 && !running) || editing} data-tour="play-box">
  <DialogueBox line={question}>
    {#if showFeed}
      <ol class="feed" aria-live="polite">
        {#each session.lines as l (l.frameId)}
          {@const tr = session.translations[l.frameId]}
          <li>
            {#if tr?.errorKind}
              <p class="text">{l.text}</p>
              <p class="tr-error">{t(`errors.${tr.errorKind}`)}</p>
            {:else if tr?.text}
              <p class="text" class:streaming={!tr.done}>{tr.text}</p>
              <p class="original">{l.text}</p>
            {:else}
              <p class="text pending">{l.text}</p>
            {/if}
            <p class="timing">{timing(l, tr)}</p>
          </li>
        {:else}
          <li class="muted live">{t("play.start.waiting")}</li>
        {/each}
      </ol>
      <!-- The in-game bar is hidden until asked for: it shouldn't sit on the game -->
      <p class="toolbar-hint">{app.settings.shortcuts.toolbar
          ? t("play.start.toolbarHint", { keys: app.settings.shortcuts.toolbar })
          : t("play.start.toolbarHintNoKey")}</p>
      <div class="actions">
        <button class="btn btn-ghost" onclick={() => toggleToolbar()}>
          {session.toolbarShown ? t("play.start.toolbarHide") : t("play.start.toolbarShow")}
        </button>
        <button class="btn btn-ghost" onclick={adjust}>{t("play.start.adjust")}</button>
        <button class="btn" onclick={stop}>{t("play.start.stop")}</button>
      </div>
    {:else if editing || step === 1}
      <!-- How the game is read: its marked areas, or the whole window. Never both. -->
      <div class="read-mode" role="radiogroup" aria-label={t("play.area.question")}>
        {#each readModes as mode (mode)}
          {@const on = mode === "screen" ? wholeScreen : !wholeScreen}
          <label class="mode" class:selected={on}>
            <input type="radio" name="read-mode" checked={on} onchange={() => setReadMode(mode)} />
            <strong>{t(`play.area.${mode}`)}</strong>
            <span>{t(`play.area.${mode}Help`)}</span>
          </label>
        {/each}
      </div>
      {#if !wholeScreen}
      <!-- One tab per area of the game: dialogue, song lyrics, names... -->
      <div class="area-tabs" role="tablist">
        {#each areas as a, i (i)}
          <button
            class="area-tab"
            class:selected={i === selected}
            class:missing={!a.region}
            role="tab"
            aria-selected={i === selected}
            onclick={() => (selected = i)}
          >
            <span class="num">{i + 1}</span>{areaLabel(a, i)}
          </button>
        {/each}
        <button class="area-tab add" onclick={addArea} disabled={areas.length >= MAX_AREAS}>
          + {t("play.area.add")}
        </button>
      </div>
      {/if}
      {#if game}
        <GamePreview
          windowHandle={game.handle}
          selectable={!wholeScreen}
          bind:region={areas[selected].region}
          label={String(selected + 1)}
          onpick={(label) => (selected = Number(label) - 1)}
          others={areas
            .map((a, i) => ({ region: a.mode === "screen" ? null : a.region, label: String(i + 1) }))
            .filter((_, i) => i !== selected)}
        />
      {/if}
      <div class="area-hint">
        {#if wholeScreen}
          <p>{t("play.area.wholeWindow")}</p>
        {:else if area.region}
          <p>
            {t("play.area.marked", { width: String(area.region[2]), height: String(area.region[3]) })}
            <span class="edit-hint">{t("play.area.editHint")}</span>
          </p>
          <button class="btn btn-ghost" onclick={() => (areas[selected].region = null)}>{t("play.area.clear")}</button>
        {:else}
          <p>{t("play.area.drag")}</p>
        {/if}
      </div>
      {#if !wholeScreen}
      <div class="area-name">
        <label class="field">
          <span>{t("play.area.name")}</span>
          <input
            class="input"
            bind:value={areas[selected].name}
            placeholder={t("play.area.namePlaceholder")}
            maxlength="40"
          />
        </label>
        {#if areas.length > 1}
          <button class="btn btn-ghost" onclick={removeArea}>{t("play.area.remove")}</button>
        {/if}
      </div>
      {/if}
      {#if unmarked >= 0}
        <!-- Why Continue / Apply is off: one area has no rectangle yet -->
        <p class="unmarked">{t("play.area.unmarked", { name: areaLabel(areas[unmarked], unmarked) })}</p>
      {/if}
      <div class="actions">
        {#if editing}
          <!-- Live session: changes go to the game only when applied -->
          <button class="btn btn-ghost" onclick={() => (editing = false)} disabled={applying}>{t("play.cancel")}</button>
          <button class="btn btn-primary" onclick={apply} disabled={!canContinue || applying}>{t("play.apply")}</button>
        {:else}
          <button class="btn btn-ghost" onclick={() => (step = 0)}>{t("play.back")}</button>
          <button class="btn btn-primary" onclick={confirmArea} disabled={!canContinue}>{t("play.next")}</button>
        {/if}
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
              <!-- The window as it is now: recognized at a glance -->
              <span class="thumb">
                {#if thumbs[w.handle]}<img src={thumbs[w.handle]} alt="" />{/if}
              </span>
              <span class="name">
                <strong>{w.title}</strong>
                <small>{w.process}</small>
              </span>
            </button>
          </li>
        {:else}
          <li class="muted">{loading ? "…" : t("play.game.empty")}</li>
        {/each}
      </ul>
      <div class="actions">
        <button class="btn btn-ghost" onclick={refresh} disabled={loading}>{t("play.game.refresh")}</button>
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
        <!-- Back to the areas without picking the game again -->
        <button class="btn btn-ghost" onclick={() => game && (loadAreas(game), (step = 1))}>{t("play.start.adjust")}</button>
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

  /* Open windows as picture cards: the game is found by sight */
  .windows {
    list-style: none;
    margin: 16px 0 0;
    padding: 2px;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
    gap: 12px;
    max-height: 420px;
    overflow-y: auto;
  }
  .window {
    width: 100%;
    display: grid;
    padding: 0;
    border: 0;
    border-radius: var(--radius-control);
    background: var(--night-deep);
    outline: 1px solid var(--line);
    overflow: hidden;
    cursor: pointer;
    text-align: left;
    transition:
      outline-color 0.12s ease,
      transform 0.12s ease;
  }
  .window:hover,
  .window.selected {
    outline: 2px solid var(--gold);
  }
  .window:hover {
    transform: translateY(-2px);
  }
  @media (prefers-reduced-motion: reduce) {
    .window {
      transition: none;
    }
    .window:hover {
      transform: none;
    }
  }
  .thumb {
    aspect-ratio: 16 / 9;
    display: block;
    background: var(--picture);
  }
  .thumb img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .name {
    display: grid;
    padding: 8px 10px 9px;
    min-width: 0;
  }
  .name strong,
  .name small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name small {
    color: var(--ink-faint);
    font-size: var(--text-sm);
  }

  /* Read mode: two slim cards side by side, above everything it governs */
  .read-mode {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    margin-bottom: 14px;
  }
  .mode {
    position: relative; /* keeps the hidden radio inside the card */
    display: grid;
    gap: 2px;
    align-content: start;
    padding: 10px 14px;
    border: 1px solid var(--line);
    border-radius: var(--radius-control);
    background: var(--night-deep);
    cursor: pointer;
  }
  .mode span {
    color: var(--ink-dim);
    font-size: var(--text-sm);
  }
  .mode.selected {
    border-color: var(--gold);
    box-shadow: inset 0 0 0 1px var(--gold);
  }
  .mode input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .mode:has(input:focus-visible) {
    outline: 2px solid var(--gold);
    outline-offset: 2px;
  }

  .area-hint {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-height: 42px;
    margin: 6px 0 14px;
    color: var(--ink-dim);
    font-size: var(--text-sm);
  }
  .edit-hint {
    color: var(--ink-faint);
  }
  .edit-hint::before {
    content: " · ";
  }
  .area-hint .btn {
    padding: 6px 10px;
    font-size: var(--text-sm);
  }

  /* Area tabs: numbered like the outlines on the preview */
  .area-tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-bottom: 12px;
  }
  .area-tab {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px 6px 6px;
    border: 1px solid var(--line);
    border-radius: 999px;
    background: var(--night-deep);
    color: var(--ink-dim);
    cursor: pointer;
    font-size: var(--text-sm);
  }
  .area-tab.selected {
    border-color: var(--gold);
    color: var(--ink);
  }
  .area-tab.selected .num {
    background: var(--gold);
    border-color: var(--gold);
    color: var(--gold-ink);
  }
  /* A dialogue area that still has no rectangle: it blocks "Continue" */
  .area-tab.missing:not(.selected) {
    border-style: dashed;
  }
  .area-tab.add {
    padding-left: 12px;
    border-style: dashed;
  }
  .area-tab:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .area-name {
    display: flex;
    align-items: flex-end;
    gap: 10px;
    margin-bottom: 14px;
  }
  .area-name .field {
    flex: 1;
    display: grid;
    gap: 4px;
    color: var(--ink-dim);
    font-size: var(--text-sm);
  }

  .unmarked {
    margin: 14px 0 0;
    color: var(--gold);
    font-size: var(--text-sm);
  }
  .toolbar-hint {
    margin: 14px 0 0;
    color: var(--ink-dim);
    font-size: var(--text-sm);
  }

  .summary {
    font-size: var(--text-md);
  }

  .play.wide {
    max-width: 1320px;
  }

  /* Newest line on top, at full size; older ones fade back */
  .feed {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 14px;
  }
  .feed li {
    display: grid;
    gap: 2px;
  }
  .feed li:not(:first-child) {
    opacity: 0.55;
  }
  .feed .text {
    white-space: pre-line; /* keeps the line breaks the OCR found */
  }
  .feed li:first-child .text {
    font-size: var(--text-lg);
  }
  /* Original under the translation, quieter: it's there to check the OCR */
  .original {
    color: var(--ink-dim);
    font-size: var(--text-sm);
    white-space: pre-line;
  }
  /* Untranslated line waiting for the service */
  .pending {
    color: var(--ink-dim);
  }
  /* While the translation is still arriving, a gold caret follows the text */
  .streaming::after {
    content: "▍";
    margin-left: 2px;
    color: var(--gold);
  }
  .tr-error {
    color: var(--danger);
    font-size: var(--text-sm);
  }
  .timing {
    color: var(--ink-faint);
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
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
