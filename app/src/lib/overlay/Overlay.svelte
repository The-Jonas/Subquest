<!--
  Overlay — draws the translation over the game.

  "inPlace" (default): every translated line is drawn exactly over its original
  line, on a patch of the color around that line, in that line's text color,
  at a size taken from the line's height. It looks like the game itself.

  "classic": one subtitle box with the whole translation, like a movie.

  When a new line appears in the game, the old translation disappears at once
  (in-place) so it never covers new text; the new one appears as it streams in.
  The same line read again by OCR (small differences) keeps its translation.
  When the text leaves the screen, the translation goes away HIDE_AFTER_MS later.
-->
<script lang="ts">
  import { onDestroy, tick } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import {
    activeProfile,
    getSettings,
    inTauri,
    defaultSettings,
    newArea,
    type GameArea,
    type Settings,
    type TranslationEvent,
    type OcrTextEvent,
  } from "../api";
  import { cssFamily, ensureFont } from "../fonts";
  import { layout, pairLines, type Metrics } from "./layout";

  const MIN_FONT = 12; // classic box: never shrink below this
  const HIDE_AFTER_MS = 1500; // how long a translation outlives its text on screen

  let settings = $state<Settings>(structuredClone(defaultSettings));
  let profileKey = $state<string | null>(null);
  let lines: Record<number, OcrTextEvent> = {}; // OCR events by frame id
  let current = $state<OcrTextEvent | null>(null); // line the in-place view follows
  let translation = $state(""); // its translation so far
  let viewWidth = $state(0);
  let viewHeight = $state(0);
  let fontReady = $state(0); // bumps when a font finishes loading, so text is measured again

  // Classic box state
  let classicText = $state("");
  let classicOriginal = $state("");
  let classicColors = $state({ box: "#0a0c18", text: "#ffffff" });
  let fontSize = $state(defaultSettings.overlay.fontSize);
  let box = $state<HTMLDivElement>();
  let shownFrame = 0; // classic: translations of older frames are ignored
  let lastFrame = 0; // newest frame id seen from OCR
  let aliases = new Set<number>(); // frame ids that are re-readings of `current`
  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  const unlisteners: (() => void)[] = [];

  // The area being translated (first area of the active game's profile).
  const area = $derived<GameArea>((profileKey && settings.profiles[profileKey]?.areas[0]) || newArea());
  const inPlace = $derived(area.style !== "classic");
  const family = $derived(cssFamily(area.font));

  // Canvas used only to measure texts for the in-place layout.
  const measureCtx = document.createElement("canvas").getContext("2d")!;

  // Metrics of a text at a size, in the area's font and letter case.
  function measure(text: string, px: number): Metrics {
    measureCtx.font = `${area.caps === "smallCaps" ? "small-caps " : ""}${px}px ${family}`;
    const m = measureCtx.measureText(area.caps === "upper" ? text.toUpperCase() : text);
    return {
      width: m.width,
      inkAscent: m.actualBoundingBoxAscent,
      inkDescent: m.actualBoundingBoxDescent,
      fontAscent: m.fontBoundingBoxAscent,
      fontDescent: m.fontBoundingBoxDescent,
    };
  }

  // In-place patches for the current line and its translation (recomputed on resize).
  // A lone line keeps its left edge in a dialogue box; loose labels on the
  // whole screen (menus, signs) stay centered on the original.
  const patches = $derived(
    inPlace && current && translation && viewHeight > 0 && fontReady
      ? layout(
          pairLines(current.lines, translation),
          { width: current.frameWidth, height: current.frameHeight },
          { width: viewWidth, height: viewHeight },
          measure,
          area.mode === "screen" ? "center" : "left",
        )
      : [],
  );

  // Classic box: shrinks the font one step at a time until the text fits.
  async function fit() {
    fontSize = settings.overlay.fontSize;
    await tick();
    while (box && fontSize > MIN_FONT && (box.scrollHeight > box.clientHeight || box.scrollWidth > box.clientWidth)) {
      fontSize -= 1;
      await tick();
    }
  }

  // True when a frame id is the line on screen or a re-reading of it.
  function isCurrent(frameId: number): boolean {
    return !!current && (frameId === current.frameId || aliases.has(frameId));
  }

  // Removes every translation from the screen (the text is gone from the game).
  function hide() {
    current = null;
    translation = "";
    aliases = new Set();
    classicText = "";
    classicOriginal = "";
    shownFrame = lastFrame + 1; // a late translation must not bring them back
  }

  // New text in the game. Empty text: it left the screen, hide a moment later.
  // The same line read again: keep what is shown. Otherwise, in place, clear
  // the old translation right away so it never covers the new text.
  function onOcr(e: OcrTextEvent) {
    clearTimeout(hideTimer);
    lastFrame = Math.max(lastFrame, e.frameId);
    if (!e.text) {
      hideTimer = setTimeout(hide, HIDE_AFTER_MS);
      return;
    }
    lines[e.frameId] = e;
    if (e.sameAs != null && isCurrent(e.sameAs)) {
      aliases.add(e.frameId);
      return;
    }
    if (!current || e.frameId > current.frameId) {
      current = e;
      translation = "";
      aliases = new Set();
    }
  }

  // Translation progress: in place follows the newest line; classic keeps the
  // previous text until the new one has its first words.
  function onTranslation(e: TranslationEvent) {
    if (!e.text || e.errorKind) return;
    if (isCurrent(e.frameId)) translation = e.text;
    else if (current && e.frameId < current.frameId) return; // an older line finished late
    if (e.frameId < shownFrame) return;
    shownFrame = e.frameId;
    classicText = e.text;
    const line = lines[e.frameId];
    classicOriginal = line?.text ?? "";
    if (line) classicColors = { box: line.boxColor, text: line.textColor };
    fit();
  }

  // Switches to another game's look (a new session started).
  async function useProfile(key: string | null) {
    profileKey = key;
    current = null;
    translation = "";
    await ensureFont(area.font).catch(() => {});
    fontReady += 1;
    fit();
  }

  // Loads settings and subscribes to lines, translations, sessions and settings changes.
  async function setup() {
    if (!inTauri) return demo();
    settings = await getSettings();
    fontSize = settings.overlay.fontSize;
    await useProfile(await activeProfile());
    unlisteners.push(await listen<string>("session-started", (e) => useProfile(e.payload)));
    unlisteners.push(await listen<OcrTextEvent>("ocr-text", (e) => onOcr(e.payload)));
    unlisteners.push(await listen<TranslationEvent>("translation", (e) => onTranslation(e.payload)));
    unlisteners.push(
      await listen<Settings>("settings-changed", (e) => {
        settings = e.payload;
        ensureFont(area.font)
          .catch(() => {})
          .then(() => (fontReady += 1));
        fit();
      }),
    );
  }

  // Browser mode: a two-line duet, to see the in-place layout without a game.
  function demo() {
    current = {
      text: "Seas reflect her gleaming beauty\nSeas reflect her gleaming beauty",
      frameId: 1,
      ocrMs: 8,
      totalMs: 10,
      boxColor: "#16201c",
      textColor: "#5fb56b",
      frameWidth: 1400,
      frameHeight: 300,
      lines: [
        { text: "Seas reflect her gleaming beauty", rect: [330, 70, 700, 48], boxColor: "#1a2420", textColor: "#cfd8d2" },
        { text: "Seas reflect her gleaming beauty", rect: [330, 170, 700, 48], boxColor: "#16201c", textColor: "#5fb56b" },
      ],
    };
    translation = "Mares refletem sua beleza radiante\nMares refletem sua beleza radiante";
    document.fonts.ready.then(() => (fontReady += 1));
  }

  setup();
  onDestroy(() => {
    clearTimeout(hideTimer);
    unlisteners.forEach((fn) => fn());
  });
</script>

<svelte:window onresize={() => !inPlace && classicText && fit()} />

<div class="stage" bind:clientWidth={viewWidth} bind:clientHeight={viewHeight}>
  {#if inPlace}
    {#each patches as p, i (i)}
      <!-- The cover hides the original line with the color around it... -->
      <div
        class="cover"
        style:left="{p.cover.left}px"
        style:top="{p.cover.top}px"
        style:width="{p.cover.width}px"
        style:height="{p.cover.height}px"
        style:--patch-bg={p.boxColor}
        style:--pad="{Math.max(2, p.fontPx * 0.18)}px"
      ></div>
      <!-- ...and the translation takes its place: same size, baseline and alignment -->
      <div
        class="line"
        class:upper={area.caps === "upper"}
        class:small-caps={area.caps === "smallCaps"}
        style:left="{p.textLeft}px"
        style:top="{p.textTop}px"
        style:height="{p.lineHeight}px"
        style:line-height="{p.lineHeight}px"
        style:font-size="{p.fontPx}px"
        style:font-family={family}
        style:color={p.textColor}
      >
        {p.text}
      </div>
    {/each}
  {:else}
    <div
      class="subtitle"
      class:empty={!classicText}
      bind:this={box}
      style:font-size="{fontSize}px"
      style:font-family={family}
      style:background="rgb(10 12 24 / {settings.overlay.backgroundOpacity})"
    >
      {#if settings.overlay.showOriginal && classicOriginal}
        <small>{classicOriginal}</small>
      {/if}
      <p>{classicText}</p>
    </div>
  {/if}
</div>

<style>
  /* The page itself must be fully transparent: only patches / the box are drawn */
  :global(html.overlay),
  :global(html.overlay body) {
    margin: 0;
    height: 100%;
    background: transparent;
    overflow: hidden;
  }
  :global(#overlay) {
    height: 100%;
  }

  .stage {
    position: relative;
    width: 100%;
    height: 100%;
  }

  /* In place: the cover "erases" the original line with the color around it.
     It reaches a bit past the letters (anti-aliased edges) and is softly
     blurred, so its edges melt into textured or gradient backgrounds instead
     of showing a hard rectangle. */
  .cover {
    position: absolute;
    box-sizing: content-box;
    padding: var(--pad);
    margin: calc(var(--pad) * -1);
    background: var(--patch-bg, transparent);
    border-radius: var(--pad);
    filter: blur(calc(var(--pad) * 0.7));
  }
  .line {
    position: absolute;
    white-space: nowrap;
  }
  .line.upper {
    text-transform: uppercase;
  }
  .line.small-caps {
    font-variant-caps: small-caps;
  }

  .subtitle {
    box-sizing: border-box;
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    padding: 0.4em 0.8em;
    border-radius: 8px;
    overflow: hidden;
    color: #fff;
    line-height: 1.3;
    text-shadow:
      0 1px 2px rgb(0 0 0 / 0.9),
      0 0 6px rgb(0 0 0 / 0.6);
    transition: opacity 120ms;
  }
  .subtitle.empty {
    opacity: 0;
  }
  p {
    margin: 0;
    white-space: pre-line;
  }
  small {
    display: block;
    font-size: 0.62em;
    opacity: 0.75;
    margin-bottom: 0.2em;
    white-space: pre-line;
  }
</style>
