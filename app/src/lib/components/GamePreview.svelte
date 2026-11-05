<!--
  GamePreview — live thumbnail of the game window.
  Starts capture on mount and stops it on unmount (or when the game changes).
  With `selectable`, dragging over the image marks the dialogue area; the
  result goes into `region`, in real capture pixels (not preview pixels).
-->
<script lang="ts">
  import { startPreview, errorKind, type PreviewFrame, type Region } from "../api";
  import { t } from "../i18n/index.svelte";

  let {
    windowHandle,
    selectable = false,
    region = $bindable(null),
  }: { windowHandle: number; selectable?: boolean; region?: Region | null } = $props();

  const MAX_WIDTH = 640; // preview width: enough to make out the dialogue box
  const MIN_SIZE = 12; // smallest area accepted, in capture pixels (filters out accidental clicks)

  let canvas = $state<HTMLCanvasElement>();
  let stage = $state<HTMLDivElement>();
  let live = $state(false);
  let error = $state<string | null>(null);
  let sourceWidth = $state(0);
  let sourceHeight = $state(0);
  let dragStart: [number, number] | null = null;
  let draft = $state<Region | null>(null);

  const shown = $derived(draft ?? region); // rectangle being drawn wins over the saved one

  // Draws each received frame on the canvas, resizing it if the window changes.
  function draw(frame: PreviewFrame) {
    if (!canvas) return;
    if (canvas.width !== frame.image.width || canvas.height !== frame.image.height) {
      canvas.width = frame.image.width;
      canvas.height = frame.image.height;
    }
    canvas.getContext("2d")!.putImageData(frame.image, 0, 0);
    sourceWidth = frame.sourceWidth;
    sourceHeight = frame.sourceHeight;
    live = true;
  }

  // (Re)starts capture whenever the chosen game changes; cleanup stops the previous one.
  $effect(() => {
    const handle = windowHandle;
    let stop: (() => Promise<void>) | null = null;
    let cancelled = false;
    live = false;
    error = null;

    startPreview(handle, MAX_WIDTH, draw)
      .then((fn) => {
        if (cancelled) fn();
        else stop = fn;
      })
      .catch((e) => (error = t(`errors.${errorKind(e)}`)));

    return () => {
      cancelled = true;
      stop?.();
    };
  });

  // Converts a pointer position into capture pixels, clamped to the image.
  function toSource(e: PointerEvent): [number, number] {
    const box = stage!.getBoundingClientRect();
    const fx = Math.min(1, Math.max(0, (e.clientX - box.left) / box.width));
    const fy = Math.min(1, Math.max(0, (e.clientY - box.top) / box.height));
    return [Math.round(fx * sourceWidth), Math.round(fy * sourceHeight)];
  }

  // Builds a normalized [x, y, w, h] from two corners, whatever the drag direction.
  function rectFrom(a: [number, number], b: [number, number]): Region {
    return [Math.min(a[0], b[0]), Math.min(a[1], b[1]), Math.abs(b[0] - a[0]), Math.abs(b[1] - a[1])];
  }

  // Starts a drag; pointer capture keeps the events coming even outside the image.
  function onDown(e: PointerEvent) {
    if (!selectable || !live || e.button !== 0) return;
    stage!.setPointerCapture(e.pointerId);
    dragStart = toSource(e);
    draft = [...dragStart, 0, 0];
  }

  // Updates the rectangle while dragging.
  function onMove(e: PointerEvent) {
    if (!dragStart) return;
    draft = rectFrom(dragStart, toSource(e));
  }

  // Ends the drag; keeps the rectangle only if it's big enough to be intentional.
  function onUp() {
    if (!dragStart) return;
    if (draft && draft[2] >= MIN_SIZE && draft[3] >= MIN_SIZE) region = draft;
    dragStart = null;
    draft = null;
  }

  // Percent-based style so the rectangle follows the canvas at any size.
  function boxStyle(r: Region): string {
    const pct = (v: number, total: number) => `${(v / total) * 100}%`;
    return `left:${pct(r[0], sourceWidth)};top:${pct(r[1], sourceHeight)};width:${pct(r[2], sourceWidth)};height:${pct(r[3], sourceHeight)}`;
  }
</script>

<figure class="preview" class:live>
  <div
    class="stage"
    class:selectable={selectable && live}
    bind:this={stage}
    onpointerdown={onDown}
    onpointermove={onMove}
    onpointerup={onUp}
    onpointercancel={onUp}
    role="presentation"
  >
    <canvas bind:this={canvas} aria-label={t("play.area.previewLabel")}></canvas>
    {#if selectable && shown && sourceWidth > 0}
      <div class="area" style={boxStyle(shown)}></div>
    {/if}
  </div>
  {#if error}
    <figcaption class="error" role="alert">{error}</figcaption>
  {:else if !live}
    <figcaption class="waiting">{t("play.area.previewWaiting")}</figcaption>
  {/if}
</figure>

<style>
  .preview {
    position: relative;
    margin: 0;
    aspect-ratio: 16 / 9;
    border-radius: var(--radius-control);
    border: 1px solid var(--line);
    background: var(--night-deep);
    overflow: hidden;
    display: grid;
    place-items: center;
  }
  /* Once there is an image, the aspect ratio becomes the game's own */
  .preview.live {
    aspect-ratio: auto;
  }

  .stage {
    position: relative;
    width: 100%;
    touch-action: none;
    user-select: none;
  }
  .stage.selectable {
    cursor: crosshair;
  }

  canvas {
    display: block;
    width: 100%;
    height: auto;
  }
  .preview:not(.live) .stage {
    position: absolute;
    inset: 0;
    opacity: 0;
  }

  /* Marked area: gold outline, and everything outside it dimmed by a huge shadow */
  .area {
    position: absolute;
    border: 2px solid var(--gold);
    border-radius: 3px;
    box-shadow: 0 0 0 9999px rgb(8 10 22 / 0.55);
    pointer-events: none;
  }

  figcaption {
    padding: 0 24px;
    text-align: center;
    font-size: var(--text-sm);
  }
  .waiting {
    color: var(--ink-faint);
  }
  .error {
    color: var(--danger);
  }
</style>
