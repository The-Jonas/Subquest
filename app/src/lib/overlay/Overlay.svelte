<!--
  Overlay — the subtitle drawn over the game. Shows the translation of the
  newest line; while a new line is still waiting for its first words, the
  previous translation stays up, so the text never flickers to empty.
  The text shrinks on its own when a translation is longer than the area.
-->
<script lang="ts">
  import { onDestroy, tick } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getSettings, inTauri, defaultSettings, type Settings, type TranslationEvent, type OcrTextEvent } from "../api";

  const MIN_FONT = 12; // never shrink below this; past it, long text is cut rather than unreadable

  let settings = $state<Settings>(structuredClone(defaultSettings));
  let originals: Record<number, string> = {};
  let shownFrame = 0;
  let text = $state("");
  let original = $state("");
  let fontSize = $state(defaultSettings.overlay.fontSize);
  let box = $state<HTMLDivElement>();
  const unlisteners: (() => void)[] = [];

  // Shrinks the font one step at a time until the text fits the window.
  async function fit() {
    fontSize = settings.overlay.fontSize;
    await tick();
    while (box && fontSize > MIN_FONT && (box.scrollHeight > box.clientHeight || box.scrollWidth > box.clientWidth)) {
      fontSize -= 1;
      await tick();
    }
  }

  // Shows a translation if it belongs to the newest line seen so far.
  function onTranslation(e: TranslationEvent) {
    if (e.frameId < shownFrame || !e.text || e.errorKind) return;
    shownFrame = e.frameId;
    text = e.text;
    original = originals[e.frameId] ?? "";
    fit();
  }

  // Remembers the original of each line (for the "show original" option).
  function onOcr(e: OcrTextEvent) {
    originals[e.frameId] = e.text;
  }

  // Loads settings and subscribes to lines, translations and settings changes.
  async function setup() {
    if (!inTauri) {
      text = "Você voltou. É claro que voltou.";
      original = "You came back. Of course you did.";
      return;
    }
    settings = await getSettings();
    fontSize = settings.overlay.fontSize;
    unlisteners.push(await listen<OcrTextEvent>("ocr-text", (e) => onOcr(e.payload)));
    unlisteners.push(await listen<TranslationEvent>("translation", (e) => onTranslation(e.payload)));
    unlisteners.push(
      await listen<Settings>("settings-changed", (e) => {
        settings = e.payload;
        fit();
      }),
    );
  }

  // Refits when the window is resized (the game window moved or changed size).
  function onResize() {
    if (text) fit();
  }

  setup();
  onDestroy(() => unlisteners.forEach((fn) => fn()));
</script>

<svelte:window onresize={onResize} />

<div
  class="subtitle"
  class:empty={!text}
  bind:this={box}
  style:font-size="{fontSize}px"
  style:background="rgb(10 12 24 / {settings.overlay.backgroundOpacity})"
>
  {#if settings.overlay.showOriginal && original}
    <small>{original}</small>
  {/if}
  <p>{text}</p>
</div>

<style>
  /* The page itself must be fully transparent: only the subtitle box is drawn */
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
    font-family: "Atkinson Hyperlegible Next Variable", system-ui, sans-serif;
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
