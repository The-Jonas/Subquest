<!--
  Overlay — draws the translation over the game.

  "inPlace" (default): every translated line is drawn exactly over its original
  line, on a patch of the color around that line, in that line's text color,
  at a size taken from the line's height. It looks like the game itself.

  "classic": one subtitle box with the whole translation, like a movie.

  When a new line appears in the game, the old translation disappears at once
  (in-place) so it never covers new text; the new one appears as it streams in.
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
  import { layout, pairLines } from "./layout";

  const MIN_FONT = 12; // classic box: never shrink below this

  let settings = $state<Settings>(structuredClone(defaultSettings));
  let profileKey = $state<string | null>(null);
  let lines: Record<number, OcrTextEvent> = {}; // OCR events by frame id
  let current = $state<OcrTextEvent | null>(null); // line the in-place view follows
  let translation = $state(""); // its translation so far
  let viewWidth = $state(0);
  let viewHeight = $state(0);

  // Classic box state
  let classicText = $state("");
  let classicOriginal = $state("");
  let classicColors = $state({ box: "#0a0c18", text: "#ffffff" });
  let fontSize = $state(defaultSettings.overlay.fontSize);
  let box = $state<HTMLDivElement>();
  let shownFrame = 0;
  const unlisteners: (() => void)[] = [];

  // The area being translated (first area of the active game's profile).
  const area = $derived<GameArea>((profileKey && settings.profiles[profileKey]?.areas[0]) || newArea());
  const inPlace = $derived(area.style !== "classic");
  const family = $derived(cssFamily(area.font));

  // Canvas used only to measure text widths for the in-place layout.
  const measureCtx = document.createElement("canvas").getContext("2d")!;

  // Width of a text at a size, in the area's font.
  function measure(text: string, px: number): number {
    measureCtx.font = `${px}px ${family}`;
    return measureCtx.measureText(text).width;
  }

  // In-place patches for the current line and its translation (recomputed on resize).
  const patches = $derived(
    inPlace && current && translation && viewHeight > 0
      ? layout(
          pairLines(current.lines, translation),
          { width: current.frameWidth, height: current.frameHeight },
          { width: viewWidth, height: viewHeight },
          measure,
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

  // New text in the game: in place, clear the old translation right away.
  function onOcr(e: OcrTextEvent) {
    lines[e.frameId] = e;
    if (!current || e.frameId > current.frameId) {
      current = e;
      translation = "";
    }
  }

  // Translation progress: in place follows the newest line; classic keeps the
  // previous text until the new one has its first words.
  function onTranslation(e: TranslationEvent) {
    if (!e.text || e.errorKind) return;
    if (current && e.frameId === current.frameId) translation = e.text;
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
        ensureFont(area.font).catch(() => {});
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
  }

  setup();
  onDestroy(() => unlisteners.forEach((fn) => fn()));
</script>

<svelte:window onresize={() => !inPlace && classicText && fit()} />

<div class="stage" bind:clientWidth={viewWidth} bind:clientHeight={viewHeight}>
  {#if inPlace}
    {#each patches as p, i (i)}
      <!-- One translated line over its original: same place, colors and size -->
      <div
        class="patch"
        style:left="{p.left}%"
        style:top="{p.top}%"
        style:min-width="{p.minWidth}%"
        style:height="{p.height}%"
        style:font-size="{p.fontPx}px"
        style:font-family={family}
        style:--patch-bg={p.boxColor}
        style:color={p.textColor}
        style:--pad="{Math.max(2, p.fontPx * 0.18)}px"
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

  /* In place: the patch "erases" the original line with the color around it.
     The color lives on a layer behind the text that reaches a bit past the
     letters (anti-aliased edges) and is softly blurred, so its edges melt into
     textured or gradient backgrounds instead of showing a hard rectangle. */
  .patch {
    position: absolute;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    white-space: nowrap;
    line-height: 1;
    isolation: isolate;
  }
  .patch::before {
    content: "";
    position: absolute;
    inset: calc(var(--pad) * -1);
    z-index: -1;
    background: var(--patch-bg, transparent);
    border-radius: var(--pad);
    filter: blur(calc(var(--pad) * 0.7));
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
