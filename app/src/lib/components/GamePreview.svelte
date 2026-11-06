<!--
  GamePreview — live thumbnail of the game window.
  Starts capture on mount and stops it on unmount (or when the game changes).
  With `selectable`, the area works like the in-game editor: drag inside it to
  move it, drag a handle to resize it, or drag anywhere else to mark it again.
  The result goes into `region`, in real capture pixels (not preview pixels).
  Clicking another area's outline asks to switch to it (`onpick`).
-->
<script lang="ts">
  import { startPreview, errorKind, type PreviewFrame, type Region } from "../api";
  import { t } from "../i18n/index.svelte";

  let {
    windowHandle,
    selectable = false,
    region = $bindable(null),
    label = "",
    others = [],
    onpick,
  }: {
    windowHandle: number;
    selectable?: boolean;
    region?: Region | null;
    label?: string; // number shown on the area being marked
    others?: { region: Region | null; label: string }[]; // the game's other areas, shown dimmed
    onpick?: (label: string) => void; // another area's outline was clicked
  } = $props();

  // Resize handles: corners and edge midpoints, named by compass direction.
  const HANDLES = ["nw", "n", "ne", "e", "se", "s", "sw", "w"] as const;
  type Handle = (typeof HANDLES)[number];

  const MAX_WIDTH = 1024; // preview width: big enough to mark the area precisely, light enough for 8 fps
  const FULL_WIDTH = 8192; // asks for frames at the window's real size (never upscaled)
  const SNAPSHOT_WAIT_MS = 2000; // after this, a snapshot settles for the preview frame
  const MAX_CROP = 1600; // a crop wider than this is scaled down before it is sent
  const MIN_CROP_HEIGHT = 480; // a shorter crop is enlarged up to this...
  const MAX_GROW = 3; // ...but never more than 3x
  const MIN_SIZE = 12; // smallest area accepted, in capture pixels (filters out accidental clicks)

  let canvas = $state<HTMLCanvasElement>();
  let stage = $state<HTMLDivElement>();
  let live = $state(false);
  let error = $state<string | null>(null);
  let sourceWidth = $state(0);
  let sourceHeight = $state(0);
  // Current drag: what it does, where it started (capture pixels) and the area before it.
  let drag = $state.raw<{ kind: "new" | "move" | "resize"; handle?: Handle; from: [number, number]; start: Region | null } | null>(
    null,
  );
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

    // Small crops are enlarged (models read tiny letters badly), big ones reduced.
    const resize = Math.min(MAX_CROP / w, Math.max(1, Math.min(MAX_GROW, MIN_CROP_HEIGHT / h)));
    const crop = document.createElement("canvas");
    crop.width = Math.max(1, Math.round(w * resize));
    crop.height = Math.max(1, Math.round(h * resize));
    const ctx = crop.getContext("2d")!;
    ctx.imageSmoothingQuality = "high";
    ctx.drawImage(source, x, y, w, h, 0, 0, crop.width, crop.height);
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

  // Starts a drag. What was pressed decides the kind: a handle resizes, the area
  // moves, anything else draws a new one. Pointer capture keeps the events
  // coming even outside the image.
  function onDown(e: PointerEvent) {
    if (!selectable || !live || e.button !== 0) return;
    const target = e.target as HTMLElement;
    const handle = target.dataset.handle as Handle | undefined;
    const kind = handle && region ? "resize" : region && target.closest(".area.current") ? "move" : "new";
    stage!.setPointerCapture(e.pointerId);
    const from = toSource(e);
    drag = { kind, handle, from, start: region ? ([...region] as Region) : null };
    draft = kind === "new" ? [...from, 0, 0] : drag.start;
  }

  // Updates the rectangle while dragging, always kept inside the image.
  function onMove(e: PointerEvent) {
    if (!drag) return;
    const now = toSource(e);
    if (drag.kind === "new" || !drag.start) {
      draft = rectFrom(drag.from, now);
      return;
    }
    const dx = now[0] - drag.from[0];
    const dy = now[1] - drag.from[1];
    let [x, y, w, h] = drag.start;
    if (drag.kind === "move") {
      x = Math.min(Math.max(0, x + dx), sourceWidth - w);
      y = Math.min(Math.max(0, y + dy), sourceHeight - h);
    } else {
      const handle = drag.handle!;
      const right = x + w;
      const bottom = y + h;
      // Each dragged edge follows the pointer; the opposite edge stays put.
      if (handle.includes("w")) x = Math.min(Math.max(0, x + dx), right - MIN_SIZE);
      if (handle.includes("e")) w = Math.max(MIN_SIZE, Math.min(sourceWidth, right + dx) - x);
      else w = right - x;
      if (handle.includes("n")) y = Math.min(Math.max(0, y + dy), bottom - MIN_SIZE);
      if (handle.includes("s")) h = Math.max(MIN_SIZE, Math.min(sourceHeight, bottom + dy) - y);
      else h = bottom - y;
    }
    draft = [x, y, w, h];
  }

  // Ends the drag; a new rectangle is kept only if it's big enough to be intentional.
  function onUp() {
    if (!drag) return;
    if (draft && draft[2] >= MIN_SIZE && draft[3] >= MIN_SIZE) region = draft;
    drag = null;
    draft = null;
  }

  // Clicking another area's outline (outside the current one) switches to it.
  function pick(e: PointerEvent, label: string) {
    if (!onpick || e.button !== 0) return;
    e.stopPropagation(); // not the start of a new rectangle
    onpick(label);
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
    class:dragging={!!drag}
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
      <div class="area current" style={boxStyle(shown)}>
        {#if label}<span class="tag">{label}</span>{/if}
        {#each HANDLES as handle (handle)}
          <i class="handle {handle}" data-handle={handle}></i>
        {/each}
      </div>
    {/if}
    <!-- Drawn over the shade: the other areas stay readable while this one is marked -->
    {#if sourceWidth > 0}
      {#each others as o (o.label)}
        {#if o.region}
          <div
            class="area other"
            class:pickable={!!onpick}
            style={boxStyle(o.region)}
            onpointerdown={(e) => pick(e, o.label)}
            title={onpick ? t("play.area.pick") : undefined}
            role="presentation"
          >
            <span class="tag">{o.label}</span>
          </div>
        {/if}
      {/each}
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

  /* As big as the box allows, but short enough that the mode switch, tabs and
     buttons around it still fit in the window (~380px is everything else) */
  canvas {
    display: block;
    max-width: 100%;
    max-height: max(300px, calc(100vh - 380px));
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
  /* The area being edited: grab it to move it (handles resize it) */
  .area.current {
    pointer-events: auto;
    cursor: move;
  }
  .stage.dragging,
  .stage.dragging * {
    cursor: grabbing;
  }
  /* Small gold squares on the corners and edge midpoints */
  .handle {
    position: absolute;
    width: 10px;
    height: 10px;
    margin: -6px 0 0 -6px;
    border: 1px solid var(--gold-ink);
    border-radius: 2px;
    background: var(--gold);
  }
  .handle.nw { left: 0; top: 0; cursor: nwse-resize; }
  .handle.n { left: 50%; top: 0; cursor: ns-resize; }
  .handle.ne { left: 100%; top: 0; cursor: nesw-resize; }
  .handle.e { left: 100%; top: 50%; cursor: ew-resize; }
  .handle.se { left: 100%; top: 100%; cursor: nwse-resize; }
  .handle.s { left: 50%; top: 100%; cursor: ns-resize; }
  .handle.sw { left: 0; top: 100%; cursor: nesw-resize; }
  .handle.w { left: 0; top: 50%; cursor: ew-resize; }

  /* Number tag in the corner, matching the area tabs above the preview */
  .area .tag {
    position: absolute;
    top: -2px;
    left: -2px;
    min-width: 18px;
    padding: 1px 5px;
    border-radius: 3px 0 3px 0;
    background: var(--gold);
    color: var(--gold-ink);
    font-size: 11px;
    font-weight: 700;
    text-align: center;
  }
  /* The game's other areas: visible for reference, but quiet */
  .area.other {
    border: 1px dashed rgb(234 228 211 / 0.55);
  }
  /* Another area can be clicked to edit it instead */
  .area.other.pickable {
    pointer-events: auto;
    cursor: pointer;
  }
  .area.other.pickable:hover {
    border-color: var(--ink);
  }
  .area.other .tag {
    background: rgb(234 228 211 / 0.55);
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
