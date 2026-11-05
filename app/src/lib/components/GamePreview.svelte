<!--
  GamePreview — live thumbnail of the game window.
  Starts capture on mount and stops it on unmount (or when the game changes).
-->
<script lang="ts">
  import { startPreview, errorKind, type PreviewFrame } from "../api";
  import { t } from "../i18n/index.svelte";

  let { windowHandle }: { windowHandle: number } = $props();

  const MAX_WIDTH = 640; // preview width: enough to make out the dialogue box

  let canvas = $state<HTMLCanvasElement>();
  let live = $state(false);
  let error = $state<string | null>(null);

  // Draws each received frame on the canvas, resizing it if the window changes.
  function draw(frame: PreviewFrame) {
    if (!canvas) return;
    if (canvas.width !== frame.image.width || canvas.height !== frame.image.height) {
      canvas.width = frame.image.width;
      canvas.height = frame.image.height;
    }
    canvas.getContext("2d")!.putImageData(frame.image, 0, 0);
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
</script>

<figure class="preview" class:live>
  <canvas bind:this={canvas} aria-label={t("play.area.previewLabel")}></canvas>
  {#if error}
    <figcaption class="error" role="alert">{error}</figcaption>
  {:else if !live}
    <figcaption class="waiting">{t("play.area.previewWaiting")}</figcaption>
  {/if}
</figure>

<style>
  .preview {
    position: relative;
    margin: 0 0 18px;
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

  canvas {
    display: block;
    width: 100%;
    height: auto;
  }
  .preview:not(.live) canvas {
    position: absolute;
    inset: 0;
    opacity: 0;
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
