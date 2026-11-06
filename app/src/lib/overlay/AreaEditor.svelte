<!--
  AreaEditor — edits the game's areas right over the game (toolbar or shortcut).
  Where the mouse goes down decides what happens, so there are no tools to
  switch between:
    - inside a box: move it
    - on its edges or corners (the small squares): resize it
    - on empty space: draw a new area
  The tab above each box holds its number and name (click the name to rename)
  and a button to remove it. Nothing is saved until "Done"; Esc cancels.
  Areas that read the whole screen have no box and are kept as they are.
-->
<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "../i18n/index.svelte";
  import { newArea, type GameArea, type Region } from "../api";

  let {
    areas,
    scale,
    ondone,
  }: {
    areas: GameArea[]; // the game's areas (regions in window pixels)
    scale: number; // window pixels per CSS pixel (screen DPI)
    ondone: (areas: GameArea[] | null) => void; // the new list, or null when cancelled
  } = $props();

  const MIN = 16; // smallest box side, in CSS px
  const HANDLES = ["nw", "n", "ne", "e", "se", "s", "sw", "w"] as const;
  type Handle = (typeof HANDLES)[number];

  // One editable box: its area and its rectangle in CSS px (screen areas have none).
  interface Box {
    area: GameArea;
    rect: Region | null;
  }

  // A copy taken when the editor opens: edits stay here until "Done".
  // svelte-ignore state_referenced_locally
  let boxes = $state<Box[]>(
    areas.map((a) => ({
      area: { ...a },
      rect: a.mode !== "screen" && a.region ? (a.region.map((v) => v / scale) as Region) : null,
    })),
  );
  let renaming = $state<number | null>(null);
  let selected = $state<number | null>(null);

  // The drag in progress: what it does, where it started, the box as it was.
  let drag: { kind: "move" | "resize" | "new"; box: number; handle?: Handle; x: number; y: number; start: Region } | null =
    null;

  const screenAreas = $derived(boxes.filter((b) => !b.rect).length);

  // Name shown on a box's tab.
  function label(b: Box, i: number): string {
    return b.area.name.trim() || t("play.area.defaultName", { n: String(i + 1) });
  }

  // Mouse down on empty space: a new box grows from there.
  function onLayerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    renaming = null;
    boxes.push({ area: { ...newArea(), mode: "dialogue" }, rect: [e.clientX, e.clientY, 0, 0] });
    const i = boxes.length - 1;
    selected = i;
    start(e, "new", i);
  }

  // Mouse down inside a box (move) or on one of its squares (resize).
  function onBoxDown(e: PointerEvent, i: number, handle?: Handle) {
    if (e.button !== 0) return;
    e.stopPropagation();
    renaming = null;
    selected = i;
    start(e, handle ? "resize" : "move", i, handle);
  }

  // Captures the pointer so the drag keeps going outside the box.
  function start(e: PointerEvent, kind: "move" | "resize" | "new", box: number, handle?: Handle) {
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    drag = { kind, box, handle, x: e.clientX, y: e.clientY, start: [...boxes[box].rect!] as Region };
  }

  // Follows the drag: moves, resizes or draws the box (kept inside the screen).
  function onMove(e: PointerEvent) {
    if (!drag) return;
    const dx = e.clientX - drag.x;
    const dy = e.clientY - drag.y;
    let [x, y, w, h] = drag.start;
    if (drag.kind === "move") {
      x = Math.min(Math.max(0, x + dx), innerWidth - w);
      y = Math.min(Math.max(0, y + dy), innerHeight - h);
    } else if (drag.kind === "new") {
      [x, y, w, h] = [Math.min(drag.x, e.clientX), Math.min(drag.y, e.clientY), Math.abs(dx), Math.abs(dy)];
    } else {
      const handle = drag.handle!;
      if (handle.includes("w")) [x, w] = [Math.min(x + dx, x + w - MIN), Math.max(MIN, w - dx)];
      if (handle.includes("e")) w = Math.max(MIN, w + dx);
      if (handle.includes("n")) [y, h] = [Math.min(y + dy, y + h - MIN), Math.max(MIN, h - dy)];
      if (handle.includes("s")) h = Math.max(MIN, h + dy);
    }
    boxes[drag.box].rect = [x, y, w, h];
  }

  // Ends the drag; a new box too small to be meant is dropped.
  function onUp() {
    if (!drag) return;
    const [, , w, h] = boxes[drag.box].rect!;
    if (drag.kind === "new" && (w < MIN || h < MIN)) {
      boxes.splice(drag.box, 1);
      selected = null;
    }
    drag = null;
  }

  // Removes an area.
  function remove(i: number) {
    boxes.splice(i, 1);
    selected = null;
    renaming = null;
  }

  // Saves: boxes back to window pixels, screen areas untouched.
  function done() {
    ondone(
      boxes.map((b) =>
        b.rect ? { ...b.area, region: b.rect.map((v) => Math.round(v * scale)) as Region } : { ...b.area },
      ),
    );
  }

  // Keyboard: Esc cancels (or leaves a name being typed), Enter saves.
  function onKey(e: KeyboardEvent) {
    if (renaming !== null) {
      if (e.key === "Enter" || e.key === "Escape") renaming = null;
      return;
    }
    if (e.key === "Escape") ondone(null);
    if (e.key === "Enter") done();
    if ((e.key === "Delete" || e.key === "Backspace") && selected !== null) remove(selected);
  }

  // Focuses an input as soon as it appears (renaming).
  function focusOnMount(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  onMount(() => window.focus());
</script>

<svelte:window onkeydown={onKey} />

<div
  class="editor"
  onpointerdown={onLayerDown}
  onpointermove={onMove}
  onpointerup={onUp}
  oncontextmenu={(e) => e.preventDefault()}
  role="presentation"
>
  <div class="hint" onpointerdown={(e) => e.stopPropagation()} role="toolbar" tabindex="-1">
    <div>
      <strong>{t("areaEditor.title")}</strong>
      <span>{t("areaEditor.help")}</span>
      {#if screenAreas}<span class="note">{t("areaEditor.screenAreas", { count: String(screenAreas) })}</span>{/if}
    </div>
    <button onclick={() => ondone(null)}>{t("areaEditor.cancel")}</button>
    <button class="primary" onclick={done}>{t("areaEditor.done")}</button>
  </div>

  {#each boxes as b, i (b)}
    {#if b.rect}
      {@const [x, y, w, h] = b.rect}
      <div
        class="box"
        class:selected={selected === i}
        style:left="{x}px"
        style:top="{y}px"
        style:width="{w}px"
        style:height="{h}px"
        onpointerdown={(e) => onBoxDown(e, i)}
        role="presentation"
      >
        <!-- Tab: number, name (click to rename) and remove -->
        <div class="tab" onpointerdown={(e) => e.stopPropagation()} role="presentation">
          <span class="num">{i + 1}</span>
          {#if renaming === i}
            <input
              use:focusOnMount
              value={b.area.name}
              maxlength="40"
              placeholder={t("play.area.namePlaceholder")}
              oninput={(e) => (b.area.name = e.currentTarget.value)}
              onblur={() => (renaming = null)}
            />
          {:else}
            <button class="name" onclick={() => (renaming = i)} title={t("areaEditor.rename")}>{label(b, i)}</button>
          {/if}
          <button class="remove" onclick={() => remove(i)} title={t("areaEditor.remove")}>✕</button>
        </div>
        {#each HANDLES as handle (handle)}
          <span class="handle {handle}" onpointerdown={(e) => onBoxDown(e, i, handle)} role="presentation"></span>
        {/each}
      </div>
    {/if}
  {/each}
</div>

<style>
  .editor {
    position: absolute;
    inset: 0;
    background: rgb(5 6 14 / 0.35);
    cursor: crosshair;
    font: 14px "Atkinson Hyperlegible Next Variable", system-ui, sans-serif;
    color: #eae4d3;
  }

  .hint {
    position: absolute;
    top: 24px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px 10px 18px;
    border: 1.5px solid #f2c14e;
    border-radius: 12px;
    background: rgb(15 18 34 / 0.95);
    white-space: nowrap;
    cursor: default;
  }
  .hint div {
    display: grid;
    gap: 2px;
  }
  .hint span {
    color: #b9b3a3;
    font-size: 12.5px;
  }
  .hint .note {
    color: #8f8a7d;
  }
  .hint button {
    padding: 7px 14px;
    border: 1px solid #3a4160;
    border-radius: 8px;
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .hint button.primary {
    border-color: #f2c14e;
    background: #f2c14e;
    color: #1b1608;
    font-weight: 700;
  }

  /* An area: the inside moves it */
  .box {
    position: absolute;
    box-sizing: border-box;
    border: 2px solid rgb(242 193 78 / 0.8);
    border-radius: 3px;
    background: rgb(242 193 78 / 0.06);
    cursor: move;
  }
  .box.selected {
    border-color: #f2c14e;
    background: rgb(242 193 78 / 0.12);
  }

  /* Tab above the box, like the area tabs in Subquest */
  .tab {
    position: absolute;
    left: -2px;
    bottom: 100%;
    display: flex;
    align-items: center;
    gap: 4px;
    margin-bottom: 4px;
    padding: 3px 4px 3px 3px;
    border-radius: 7px;
    background: rgb(15 18 34 / 0.95);
    cursor: default;
    white-space: nowrap;
  }
  .num {
    display: grid;
    place-items: center;
    min-width: 20px;
    height: 20px;
    border-radius: 5px;
    background: #f2c14e;
    color: #1b1608;
    font-size: 12px;
    font-weight: 700;
  }
  .tab button {
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .name {
    padding: 2px 6px;
  }
  .name:hover {
    background: rgb(234 228 211 / 0.12);
  }
  .remove {
    padding: 2px 6px;
    color: #b9b3a3 !important;
  }
  .remove:hover {
    background: rgb(224 110 110 / 0.25);
    color: #fff !important;
  }
  .tab input {
    width: 180px;
    padding: 2px 6px;
    border: 1px solid #f2c14e;
    border-radius: 5px;
    background: #0d1018;
    color: inherit;
    font: inherit;
    font-size: 13px;
    outline: none;
  }

  /* Resize squares on the corners and edges */
  .handle {
    position: absolute;
    width: 10px;
    height: 10px;
    margin: -6px;
    border: 1.5px solid #1b1608;
    border-radius: 2px;
    background: #f2c14e;
  }
  .nw { left: 0; top: 0; cursor: nwse-resize; }
  .n { left: 50%; top: 0; cursor: ns-resize; }
  .ne { right: 0; top: 0; cursor: nesw-resize; }
  .e { right: 0; top: 50%; cursor: ew-resize; }
  .se { right: 0; bottom: 0; cursor: nwse-resize; }
  .s { left: 50%; bottom: 0; cursor: ns-resize; }
  .sw { left: 0; bottom: 0; cursor: nesw-resize; }
  .w { left: 0; top: 50%; cursor: ew-resize; }
</style>
