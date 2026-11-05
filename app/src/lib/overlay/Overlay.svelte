<!--
  Overlay — draws the translations over the whole game window, one set per
  area of the game's profile (dialogue, song lyrics, names...). Each line's
  font comes from its text style (chosen by the user, or the closest installed
  one found by the app), falling back to its area's font. Each area has its own
  style of drawing:

  "inPlace" (default): every translated line is drawn exactly over its original
  line, on a patch of the color around that line, in that line's text color,
  at the size, baseline and alignment of the original. It looks like the game.

  "classic": a subtitle box with the whole translation, over the area (or as a
  band near the bottom when the area is the whole window).

  Per area: when a new line appears, the old translation disappears at once
  (in place) so it never covers new text; the new one appears as it streams in.
  The same line read again by OCR (small differences) keeps its translation.
  When the text leaves the screen, the translation goes away HIDE_AFTER_MS later.
-->
<script lang="ts">
  import { onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import {
    activeProfile,
    getSettings,
    listStyles,
    type OcrLine,
    type TextStyle,
    type LetterCase,
    inTauri,
    defaultSettings,
    newArea,
    type GameArea,
    type Settings,
    type TranslationEvent,
    type OcrTextEvent,
  } from "../api";
  import { cssFamily, ensureFont } from "../fonts";
  import { layout, pairLines, type Metrics, type Patch } from "./layout";

  const MIN_FONT = 12; // classic box: never shrink below this
  const HIDE_AFTER_MS = 1500; // how long a translation outlives its text on screen

  // What one area shows right now (reactive).
  interface AreaView {
    current: OcrTextEvent | null; // line the in-place view follows
    translation: string; // its translation so far
    classicText: string;
    classicOriginal: string;
  }

  // Bookkeeping of one area (not shown, so not reactive).
  interface AreaMeta {
    aliases: Set<number>; // frame ids that are re-readings of `current`
    hideTimer?: ReturnType<typeof setTimeout>;
    shownFrame: number; // classic: translations of older frames are ignored
    lastFrame: number; // newest frame id seen from OCR
  }

  let settings = $state<Settings>(structuredClone(defaultSettings));
  let profileKey = $state<string | null>(null);
  let views = $state<Record<number, AreaView>>({});
  let viewWidth = $state(0);
  let viewHeight = $state(0);
  let fontReady = $state(0); // bumps when a font finishes loading, so text is measured again
  const metas = new Map<number, AreaMeta>();
  let gameStyles = $state<Record<string, TextStyle>>({}); // the game's text styles, by id
  const lines: Record<number, OcrTextEvent> = {}; // OCR events by frame id
  const unlisteners: (() => void)[] = [];

  // Areas of the game being translated (at least one, for old profiles).
  const areas = $derived<GameArea[]>((profileKey && settings.profiles[profileKey]?.areas) || [newArea()]);

  // Look of an area, with the default look for an unknown index.
  function areaAt(i: number): GameArea {
    return areas[i] ?? areas[0] ?? newArea();
  }

  // Creates the state of an area the first time it shows text.
  function view(i: number): AreaView {
    views[i] ??= {
      current: null,
      translation: "",
      classicText: "",
      classicOriginal: "",
    };
    return views[i];
  }
  function meta(i: number): AreaMeta {
    if (!metas.has(i)) metas.set(i, { aliases: new Set(), shownFrame: 0, lastFrame: 0 });
    return metas.get(i)!;
  }

  // Canvas used only to measure texts for the in-place layout.
  const measureCtx = document.createElement("canvas").getContext("2d")!;

  // Font and letter case of a line: its style's chosen font, else the area's
  // font when one was set, else the style's automatic match, else the default.
  function lookFor(a: GameArea, styleId: string | null | undefined): { font: string; caps: LetterCase } {
    const style = styleId ? gameStyles[styleId] : undefined;
    if (style?.font) return { font: style.font, caps: style.caps };
    if (a.font) return { font: a.font, caps: a.caps };
    return { font: style?.autoFont ?? "", caps: a.caps };
  }

  // Measures texts in a font and letter case.
  function measurer(look: { font: string; caps: LetterCase }) {
    const family = cssFamily(look.font);
    return (text: string, px: number): Metrics => {
      measureCtx.font = `${look.caps === "smallCaps" ? "small-caps " : ""}${px}px ${family}`;
      const m = measureCtx.measureText(look.caps === "upper" ? text.toUpperCase() : text);
      return {
        width: m.width,
        inkAscent: m.actualBoundingBoxAscent,
        inkDescent: m.actualBoundingBoxDescent,
        fontAscent: m.fontBoundingBoxAscent,
        fontDescent: m.fontBoundingBoxDescent,
      };
    };
  }

  // In-place patches of every area (recomputed on resize or when a font loads).
  // A lone line keeps its left edge in a dialogue box; loose labels on the
  // whole screen (menus, signs) stay centered on the original.
  const patches = $derived.by(() => {
    if (viewHeight <= 0 || !fontReady) return [];
    const out: { look: { font: string; caps: LetterCase }; patch: Patch; frame: number }[] = [];
    for (const [key, v] of Object.entries(views)) {
      const a = areaAt(Number(key));
      if (a.style === "classic" || !v.current || !v.translation) continue;
      const placed = layout(
        pairLines(v.current.lines, v.translation),
        { width: v.current.frameWidth, height: v.current.frameHeight },
        { width: viewWidth, height: viewHeight },
        (line: OcrLine) => measurer(lookFor(a, line.style)),
        a.mode === "screen" ? "center" : "left",
      );
      for (const patch of placed) out.push({ look: lookFor(a, patch.style), patch, frame: v.current.frameId });
    }
    return withoutOverlaps(out);
  });

  // Two areas can read the same text (a marked box inside a whole-screen area),
  // and then two translations would be drawn on top of each other. Where
  // patches overlap, only the newest reading stays.
  function withoutOverlaps<T extends { patch: Patch; frame: number }>(items: T[]): T[] {
    const kept: T[] = [];
    for (const item of [...items].sort((a, b) => b.frame - a.frame)) {
      const c = item.patch.cover;
      const clash = kept.some(({ patch: { cover: k } }) => {
        const w = Math.min(c.left + c.width, k.left + k.width) - Math.max(c.left, k.left);
        const h = Math.min(c.top + c.height, k.top + k.height) - Math.max(c.top, k.top);
        const smaller = Math.min(c.width * c.height, k.width * k.height);
        return w > 0 && h > 0 && smaller > 0 && (w * h) / smaller > 0.3;
      });
      if (!clash) kept.push(item);
    }
    return kept;
  }

  // Classic boxes: over their area, or a band near the bottom for the whole window.
  const boxes = $derived.by(() => {
    const out: { area: GameArea; v: AreaView; style: string }[] = [];
    for (const [key, v] of Object.entries(views)) {
      const a = areaAt(Number(key));
      if (a.style !== "classic" || !v.classicText) continue;
      const frame = v.current; // window size, to place the area in the overlay
      let style = "left:15%;width:70%;bottom:4%;height:22%";
      if (a.mode !== "screen" && a.region && frame) {
        const [x, y, w, h] = a.region;
        style = `left:${(x / frame.frameWidth) * 100}%;top:${(y / frame.frameHeight) * 100}%;width:${(w / frame.frameWidth) * 100}%;height:${(h / frame.frameHeight) * 100}%`;
      }
      out.push({ area: a, v, style });
    }
    return out;
  });

  // Svelte action: shrinks a classic box's font one step at a time until the text fits.
  function fitText(node: HTMLElement, _deps: unknown) {
    const fit = () => {
      let size = settings.overlay.fontSize;
      node.style.fontSize = `${size}px`;
      while (size > MIN_FONT && (node.scrollHeight > node.clientHeight || node.scrollWidth > node.clientWidth)) {
        size -= 1;
        node.style.fontSize = `${size}px`;
      }
    };
    fit();
    return { update: fit };
  }

  // True when a frame id is the area's line on screen or a re-reading of it.
  function isCurrent(i: number, frameId: number): boolean {
    const v = views[i];
    return !!v?.current && (frameId === v.current.frameId || meta(i).aliases.has(frameId));
  }

  // Removes an area's translation from the screen (its text is gone from the game).
  function hide(i: number) {
    const v = view(i);
    const m = meta(i);
    v.current = null;
    v.translation = "";
    v.classicText = "";
    v.classicOriginal = "";
    m.aliases = new Set();
    m.shownFrame = m.lastFrame + 1; // a late translation must not bring it back
  }

  // New text in an area. Empty text: it left the screen, hide a moment later.
  // The same line read again: keep what is shown. Otherwise, in place, clear
  // the old translation right away so it never covers the new text.
  function onOcr(e: OcrTextEvent) {
    const i = e.area ?? 0;
    const v = view(i);
    const m = meta(i);
    clearTimeout(m.hideTimer);
    m.lastFrame = Math.max(m.lastFrame, e.frameId);
    if (!e.text) {
      m.hideTimer = setTimeout(() => hide(i), HIDE_AFTER_MS);
      return;
    }
    lines[e.frameId] = e;
    if (e.sameAs != null && isCurrent(i, e.sameAs)) {
      m.aliases.add(e.frameId);
      return;
    }
    if (!v.current || e.frameId > v.current.frameId) {
      v.current = e;
      v.translation = "";
      m.aliases = new Set();
    }
  }

  // Translation progress: in place follows the area's newest line; classic
  // keeps the previous text until the new one has its first words.
  function onTranslation(e: TranslationEvent) {
    if (!e.text || e.errorKind) return;
    const line = lines[e.frameId];
    if (!line) return;
    const i = line.area ?? 0;
    const v = view(i);
    const m = meta(i);
    if (isCurrent(i, e.frameId)) v.translation = e.text;
    else if (v.current && e.frameId < v.current.frameId) return; // an older line finished late
    if (e.frameId < m.shownFrame) return;
    m.shownFrame = e.frameId;
    v.classicText = e.text;
    v.classicOriginal = line.text;
  }

  // Loads every font the game's areas and styles use, then lets the layout measure with them.
  async function loadFonts() {
    const fonts = [
      ...areas.map((a) => a.font),
      ...Object.values(gameStyles).flatMap((s) => [s.font, s.autoFont]),
    ];
    await Promise.all(fonts.map((f) => ensureFont(f).catch(() => {})));
    fontReady += 1;
  }

  // Reloads the game's text styles (a new one was found, or a font changed).
  async function loadStyles() {
    if (!profileKey) return;
    const list = await listStyles(profileKey).catch(() => [] as TextStyle[]);
    gameStyles = Object.fromEntries(list.map((s) => [s.id, s]));
    await loadFonts();
  }

  // Switches to another game's look (a new session started).
  async function useProfile(key: string | null) {
    profileKey = key;
    views = {};
    metas.clear();
    await loadStyles();
  }

  // Loads settings and subscribes to lines, translations, sessions and settings changes.
  async function setup() {
    if (!inTauri) return demo();
    settings = await getSettings();
    await useProfile(await activeProfile());
    unlisteners.push(await listen<string>("session-started", (e) => useProfile(e.payload)));
    unlisteners.push(await listen<OcrTextEvent>("ocr-text", (e) => onOcr(e.payload)));
    unlisteners.push(await listen<TranslationEvent>("translation", (e) => onTranslation(e.payload)));
    unlisteners.push(await listen("styles-changed", () => loadStyles()));
    unlisteners.push(
      await listen<Settings>("settings-changed", (e) => {
        settings = e.payload;
        loadFonts();
      }),
    );
  }

  // Browser mode: a two-line duet, to see the in-place layout without a game.
  function demo() {
    const e: OcrTextEvent = {
      text: "Seas reflect her gleaming beauty\nSeas reflect her gleaming beauty",
      frameId: 1,
      ocrMs: 8,
      totalMs: 10,
      boxColor: "#16201c",
      textColor: "#5fb56b",
      frameWidth: 1400,
      frameHeight: 300,
      area: 0,
      lines: [
        { text: "Seas reflect her gleaming beauty", rect: [330, 70, 700, 48], boxColor: "#1a2420", textColor: "#cfd8d2" },
        { text: "Seas reflect her gleaming beauty", rect: [330, 170, 700, 48], boxColor: "#16201c", textColor: "#5fb56b" },
      ],
    };
    lines[1] = e;
    view(0).current = e;
    view(0).translation = "Mares refletem sua beleza radiante\nMares refletem sua beleza radiante";
    document.fonts.ready.then(() => (fontReady += 1));
  }

  setup();
  onDestroy(() => {
    metas.forEach((m) => clearTimeout(m.hideTimer));
    unlisteners.forEach((fn) => fn());
  });
</script>

<div class="stage" bind:clientWidth={viewWidth} bind:clientHeight={viewHeight}>
  {#each patches as { look, patch: p }, i (i)}
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
      class:upper={look.caps === "upper"}
      class:small-caps={look.caps === "smallCaps"}
      style:left="{p.textLeft}px"
      style:top="{p.textTop}px"
      style:height="{p.lineHeight}px"
      style:line-height="{p.lineHeight}px"
      style:font-size="{p.fontPx}px"
      style:font-family={cssFamily(look.font)}
      style:color={p.textColor}
    >
      {p.text}
    </div>
  {/each}

  {#each boxes as { area, v, style }, i (i)}
    <div
      class="subtitle"
      {style}
      use:fitText={[v.classicText, settings.overlay.fontSize, viewWidth, viewHeight]}
      class:upper={area.caps === "upper"}
      class:small-caps={area.caps === "smallCaps"}
      style:font-family={cssFamily(area.font)}
      style:background="rgb(10 12 24 / {settings.overlay.backgroundOpacity})"
    >
      {#if settings.overlay.showOriginal && v.classicOriginal}
        <small>{v.classicOriginal}</small>
      {/if}
      <p>{v.classicText}</p>
    </div>
  {/each}
</div>

<style>
  /* The page itself must be fully transparent: only patches / boxes are drawn */
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
  .upper {
    text-transform: uppercase;
  }
  .small-caps {
    font-variant-caps: small-caps;
  }

  .subtitle {
    position: absolute;
    box-sizing: border-box;
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
