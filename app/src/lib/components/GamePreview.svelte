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

  const MAX_WIDTH = 1024; // preview width: big enough to mark the area precisely, light enough for 8 fps
  const FULL_WIDTH = 8192; // asks for frames at the window's real size (never upscaled)
  const SNAPSHOT_WAIT_MS = 2000; // after this, a snapshot settles for the preview frame
  const MAX_CROP = 1600; // a crop wider than this is scaled down before it is sent
  const MIN_SIZE = 12; // smallest area accepted, in capture pixels (filters out accidental clicks)

  let canvas = $state<HTMLCanvasElement>();
  let stage = $state<HTMLDivElement>();
  let live = $state(false);
  let error = $state<string | null>(null);
  let sourceWidth = $state(0);
  let sourceHeight = $state(0);
  let dragStart: [number, number] | null = null;
  let draft = $state<Region | null>(null);
  let width = $state(MAX_WIDTH); // preview width asked from the core
  let latest: PreviewFrame | null = null;
  let shownHandle = 0; // game whose frames are on the canvas
  let waitingFull: ((frame: PreviewFrame) => void) | null = null;

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
    latest = frame;
    if (waitingFull && frame.image.width === frame.sourceWidth) waitingFull(frame);
  }

  // PNG of the game at full resolution, cropped to `area` (or the whole window).
  // For a moment the preview runs at the window's real size, then goes back.
  export async function snapshot(area: Region | null): Promise<Blob> {
    width = FULL_WIDTH;
    const frame = await new Promise<PreviewFrame | null>((resolve) => {
      waitingFull = resolve;
      setTimeout(() => resolve(latest), SNAPSHOT_WAIT_MS);
    });
    waitingFull = null;
    width = MAX_WIDTH;
    if (!frame) throw new Error("no frame yet");

    // Area in this frame's pixels (the fallback frame may be downscaled).
    const scale = frame.image.width / frame.sourceWidth;
    const [x, y, w, h] = (area ?? [0, 0, frame.sourceWidth, frame.sourceHeight]).map((v) => Math.round(v * scale));
    const source = document.createElement("canvas");
    source.width = frame.image.width;
    source.height = frame.image.height;
    source.getContext("2d")!.putImageData(frame.image, 0, 0);

    const shrink = Math.min(1, MAX_CROP / w);
    const crop = document.createElement("canvas");
    crop.width = Math.max(1, Math.round(w * shrink));
    crop.height = Math.max(1, Math.round(h * shrink));
    crop.getContext("2d")!.drawImage(source, x, y, w, h, 0, 0, crop.width, crop.height);
    return new Promise((resolve, reject) =>
      crop.toBlob((blob) => (blob ? resolve(blob) : reject(new Error("PNG encoding failed"))), "image/png"),
    );
  }

  // (Re)starts capture whenever the chosen game or the asked width changes;
  // cleanup stops the previous one.
  $effect(() => {
    const handle = windowHandle;
    const maxWidth = width;
    let stop: (() => Promise<void>) | null = null;
    let cancelled = false;
    if (handle !== shownHandle) {
      // Another game: forget the old image. Only a size switch keeps it on screen.
      shownHandle = handle;
      latest = null;
      live = false;
    }
    error = null;

    startPreview(handle, maxWidth, draw)
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

  // Rectangle edges as percentages, so everything follows the canvas at any size.
  function edges(r: Region) {
    const left = (r[0] / sourceWidth) * 100;
    const top = (r[1] / sourceHeight) * 100;
    return { left, top, right: left + (r[2] / sourceWidth) * 100, bottom: top + (r[3] / sourceHeight) * 100 };
  }

  // Position and size of the gold outline.
  function boxStyle(r: Region): string {
    const e = edges(r);
    return `left:${e.left}%;top:${e.top}%;width:${e.right - e.left}%;height:${e.bottom - e.top}%`;
  }

  // Dark layer over the whole image with a hole where the area is: the outer
  // square and the inner one are drawn as one polygon with the even-odd rule.
  function shadeStyle(r: Region): string {
    const e = edges(r);
    const hole = `${e.left}% ${e.top}%, ${e.left}% ${e.bottom}%, ${e.right}% ${e.bottom}%, ${e.right}% ${e.top}%, ${e.left}% ${e.top}%`;
    return `clip-path: polygon(evenodd, 0 0, 100% 0, 100% 100%, 0 100%, 0 0, ${hole})`;
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
      <div class="shade" style={shadeStyle(shown)}></div>
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

  /* The stage hugs the canvas exactly, so percentages map 1:1 onto the image */
  .stage {
    position: relative;
    width: fit-content;
    max-width: 100%;
    touch-action: none;
    user-select: none;
  }
  .stage.selectable {
    cursor: crosshair;
  }

  /* As big as the box allows, but short enough that the choices and buttons
     below still fit in the window (~460px is everything else on the screen) */
  canvas {
    display: block;
    max-width: 100%;
    max-height: max(240px, calc(100vh - 460px));
    width: auto;
    height: auto;
  }
  .preview:not(.live) .stage {
    position: absolute;
    inset: 0;
    opacity: 0;
  }

  /* Everything outside the marked area is dimmed; the area itself gets a gold outline */
  .shade {
    position: absolute;
    inset: 0;
    background: rgb(8 10 22 / 0.6);
    pointer-events: none;
  }
  .area {
    position: absolute;
    border: 2px solid var(--gold);
    border-radius: 3px;
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
