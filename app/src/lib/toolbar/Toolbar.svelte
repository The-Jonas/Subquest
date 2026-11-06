<!--
  Toolbar — a small floating bar over the game, so areas and styles can be set
  without leaving it. Drag it by the handle on the left. Every button also has
  a global shortcut (shown in its tooltip).
-->
<script lang="ts">
  import { onDestroy } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow, PhysicalPosition } from "@tauri-apps/api/window";
  import { t, setLocale, type Locale } from "../i18n/index.svelte";
  import { keyHint, defaultShortcuts, type Shortcuts } from "../shortcuts";
  import { beginSelection, getSettings, inTauri, type Settings, sessionFlags, showMain, toggleHidden, togglePause, toggleToolbar } from "../api";

  let paused = $state(false);
  let tip = $state(""); // label of the button under the mouse, drawn below the bar
  let keys = $state<Shortcuts>({ ...defaultShortcuts }); // shown in the tooltips
  let hidden = $state(false);
  const unlisteners: (() => void)[] = [];

  // Button definitions: icon path on a 20px grid, label key and shortcut letter.
  const ICONS = {
    area: "M3.5 6V3.5H6M14 3.5h2.5V6M16.5 14v2.5H14M6 16.5H3.5V14M10 7v6M7 10h6",
    style: "M3 15.5l3.5-11h1l3.5 11M4.6 11.5h4.8M15 10a2.5 2.5 0 1 0 0 5a2.5 2.5 0 0 0 0-5zM17.5 9.5v6",
    pause: "M7 4.5v11M13 4.5v11",
    play: "M6.5 4.5v11l9-5.5z",
    hide: "M2.5 10s3-5.5 7.5-5.5S17.5 10 17.5 10s-3 5.5-7.5 5.5S2.5 10 2.5 10zM10 8a2 2 0 1 0 0 4a2 2 0 0 0 0-4zM3.5 16.5l13-13",
    show: "M2.5 10s3-5.5 7.5-5.5S17.5 10 17.5 10s-3 5.5-7.5 5.5S2.5 10 2.5 10zM10 8a2 2 0 1 0 0 4a2 2 0 0 0 0-4z",
    open: "M11 3.5h5.5V9M16.5 3.5L9 11M14 12.5v4H3.5V6h4",
    edit: "M3.5 3.5h7v7h-7zM10.5 10.5l2 6 1.5-2.5 2.5-1.5z",
    tuck: "M5 8l5 5 5-5M4 16.5h12", // arrow down into a line: put the bar away
  };

  // Moving the bar by its handle. Done by hand instead of the system drag
  // (data-tauri-drag-region): this window never takes the focus, and the
  // system move loop doesn't start for such windows.
  let moving = $state.raw<{ x: number; y: number; left: number; top: number } | null>(null);
  let frame = 0; // pending animation frame: one move per screen refresh at most

  // Grabs the handle: remembers where the pointer and the window were.
  async function grab(e: PointerEvent) {
    if (!inTauri || e.button !== 0) return;
    const handle = e.currentTarget as HTMLElement;
    handle.setPointerCapture(e.pointerId); // keeps the events coming outside the bar
    const start = { x: e.screenX, y: e.screenY };
    const at = await getCurrentWindow().outerPosition();
    moving = { ...start, left: at.x, top: at.y };
  }

  // Follows the pointer; screen coordinates are CSS pixels, the window wants physical ones.
  function drag(e: PointerEvent) {
    if (!moving) return;
    const m = moving;
    const scale = window.devicePixelRatio || 1;
    const x = Math.round(m.left + (e.screenX - m.x) * scale);
    const y = Math.round(m.top + (e.screenY - m.y) * scale);
    cancelAnimationFrame(frame);
    frame = requestAnimationFrame(() => getCurrentWindow().setPosition(new PhysicalPosition(x, y)));
  }

  // Lets go of the handle.
  function drop() {
    moving = null;
  }

  // Pause or resume reading the game.
  async function pause() {
    paused = await togglePause();
  }

  // Hide or show the translations.
  async function hide() {
    hidden = await toggleHidden();
  }

  // Follows changes made by the shortcuts, and starts in the app's language.
  async function setup() {
    if (!inTauri) return;
    const settings = await getSettings();
    setLocale(settings.uiLanguage as Locale);
    keys = settings.shortcuts;
    unlisteners.push(await listen<Settings>("settings-changed", (e) => (keys = e.payload.shortcuts)));
    [paused, hidden] = await sessionFlags();
    unlisteners.push(await listen<boolean>("session-paused", (e) => (paused = e.payload)));
    unlisteners.push(await listen<boolean>("overlay-hidden", (e) => (hidden = e.payload)));
  }

  setup();
  onDestroy(() => unlisteners.forEach((fn) => fn()));
</script>

<!-- Labels are drawn by the page, under the bar: the browser's own tooltips
     open behind the bar and the overlay, which both stay on top of everything -->
{#snippet tool(label: string, icon: string, run: () => unknown, on = false)}
  <button
    class:on
    onclick={run}
    aria-label={label}
    onmouseenter={() => (tip = label)}
    onmouseleave={() => (tip = "")}
  >
    <svg viewBox="0 0 20 20"><path d={icon} /></svg>
  </button>
{/snippet}

<div class="wrap">
  <div class="bar">
    <!-- Handle: the only part that moves the window -->
    <div
      class="grip"
      class:moving={!!moving}
      onpointerdown={grab}
      onpointermove={drag}
      onpointerup={drop}
      onpointercancel={drop}
      role="presentation"
      onmouseenter={() => (tip = t("toolbar.move"))}
      onmouseleave={() => (tip = "")}
    >
      <svg viewBox="0 0 32 32" width="18" height="18" aria-hidden="true">
        <rect x="3" y="9" width="26" height="18" rx="4" fill="none" stroke="currentColor" stroke-width="2.4" />
        <rect x="6" y="5.5" width="11" height="6" rx="2" fill="var(--gold)" />
      </svg>
    </div>

    {@render tool(t("toolbar.area") + keyHint(keys.area), ICONS.area, () => beginSelection("area"))}
    {@render tool(t("toolbar.edit") + keyHint(keys.edit), ICONS.edit, () => beginSelection("edit"))}
    {@render tool(t("toolbar.style") + keyHint(keys.style), ICONS.style, () => beginSelection("style"))}
    {@render tool(
      (paused ? t("toolbar.resume") : t("toolbar.pause")) + keyHint(keys.pause),
      paused ? ICONS.play : ICONS.pause,
      pause,
      paused,
    )}
    {@render tool(
      (hidden ? t("toolbar.show") : t("toolbar.hide")) + keyHint(keys.hide),
      hidden ? ICONS.hide : ICONS.show,
      hide,
      hidden,
    )}
    <span class="sep"></span>
    {@render tool(t("toolbar.open"), ICONS.open, () => showMain())}
    <span class="sep"></span>
    {@render tool(
      keys.toolbar ? t("toolbar.close", { keys: keys.toolbar }) : t("toolbar.closeNoKey"),
      ICONS.tuck,
      () => toggleToolbar(),
    )}
  </div>
  {#if tip}<div class="tip">{tip}</div>{/if}
</div>

<style>
  :global(html.toolbar),
  :global(html.toolbar body) {
    margin: 0;
    height: 100%;
    background: transparent;
    overflow: hidden;
  }
  :global(#toolbar) {
    height: 100%;
  }

  .wrap {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
  }
  /* A small dialogue-box-style pill: dark, gold edge, out of the way */
  .bar {
    box-sizing: border-box;
    height: 48px;
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 4px 6px 4px 4px;
    border: 1.5px solid rgb(242 193 78 / 0.55);
    border-radius: 12px;
    background: rgb(15 18 34 / 0.92);
    color: var(--ink);
  }
  .grip {
    display: grid;
    place-items: center;
    width: 30px;
    height: 100%;
    cursor: grab;
    color: var(--ink-dim);
    touch-action: none;
  }
  .grip.moving {
    cursor: grabbing;
  }
  .grip svg {
    pointer-events: none; /* the handle itself gets the pointer, not the drawing */
  }
  button {
    display: grid;
    place-items: center;
    width: 38px;
    height: 36px;
    padding: 0;
    border: 0;
    border-radius: 8px;
    background: transparent;
    color: var(--ink);
    cursor: pointer;
  }
  button:hover {
    background: rgb(234 228 211 / 0.1);
  }
  /* Paused / hidden: the button that undoes it stays lit */
  button.on {
    background: var(--gold);
    color: var(--gold-ink);
  }
  svg {
    width: 20px;
    height: 20px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.7;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .grip svg {
    stroke-width: 0;
  }
  .grip svg rect:first-child {
    stroke-width: 2.4;
  }
  /* The label of the hovered button, under the bar on its right */
  .tip {
    max-width: 100%;
    padding: 4px 10px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: rgb(15 18 34 / 0.95);
    color: var(--ink);
    font-size: 12.5px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sep {
    width: 1px;
    height: 22px;
    margin: 0 4px;
    background: var(--line);
  }
</style>
