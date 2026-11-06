<!--
  Tour — the guided tour's layer over the app. Everything is dimmed except the
  part being explained (an element with data-tour="<target>"), and a bubble
  next to it says what it is. Steps without a target show the bubble centered.
  The screens stay real: the tour only opens them and points.
-->
<script lang="ts">
  import { tick } from "svelte";
  import { t } from "../i18n/index.svelte";
  import { tour, tourSteps, moveTour, endTour } from "../stores/tour.svelte";

  const PAD = 6; // space between the highlighted part and its outline
  const GAP = 14; // space between the outline and the bubble
  const BUBBLE_WIDTH = 340;

  let hole = $state<{ left: number; top: number; width: number; height: number } | null>(null);
  let bubble = $state<HTMLDivElement>();
  let place = $state<{ left: number; top: number } | null>(null);

  const step = $derived(tourSteps[tour.index]);
  const last = $derived(tour.index === tourSteps.length - 1);

  // Finds the step's part on screen (after the screen opened) and measures it.
  async function locate() {
    await tick();
    await new Promise((r) => requestAnimationFrame(r)); // the new screen is laid out
    const el = step.target ? document.querySelector<HTMLElement>(`[data-tour="${step.target}"]`) : null;
    if (!el) {
      hole = null;
      place = null;
      return;
    }
    // A tall part (a whole settings section) shows from its top: its heading matters most.
    const tall = el.getBoundingClientRect().height > window.innerHeight * 0.5;
    el.scrollIntoView({ block: tall ? "start" : "nearest", behavior: "instant" as ScrollBehavior });
    const r = el.getBoundingClientRect();
    hole = { left: r.left - PAD, top: r.top - PAD, width: r.width + PAD * 2, height: r.height + PAD * 2 };
    await tick();
    position();
  }

  // Puts the bubble beside the part: right of it if there is room (sidebar
  // items), else below, else above; always inside the window.
  function position() {
    if (!hole || !bubble) return;
    const b = bubble.getBoundingClientRect();
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    let left: number;
    let top: number;
    if (hole.left + hole.width + GAP + b.width <= vw && hole.width < vw / 3) {
      left = hole.left + hole.width + GAP;
      top = hole.top;
    } else if (hole.top + hole.height + GAP + b.height <= vh) {
      left = hole.left;
      top = hole.top + hole.height + GAP;
    } else if (hole.top - GAP - b.height >= 0) {
      left = hole.left;
      top = hole.top - GAP - b.height;
    } else {
      // Taller than the window: bottom right, over the part's less important end.
      left = hole.left + hole.width - b.width - GAP;
      top = vh - b.height - GAP;
    }
    place = {
      left: Math.min(Math.max(12, left), vw - b.width - 12),
      top: Math.min(Math.max(12, top), vh - b.height - 12),
    };
  }

  // Keyboard: arrows move, Esc closes.
  function onKeydown(e: KeyboardEvent) {
    if (!tour.active) return;
    if (e.key === "Escape") void endTour();
    else if (e.key === "ArrowRight") last ? void endTour() : moveTour(1);
    else if (e.key === "ArrowLeft") moveTour(-1);
  }

  $effect(() => {
    if (!tour.active) return;
    void tour.index; // every step measures again
    void locate();
  });
</script>

<svelte:window onresize={() => tour.active && locate()} onkeydown={onKeydown} />

{#if tour.active}
  <div class="tour" role="dialog" aria-modal="true" aria-labelledby="tour-title">
    {#if hole}
      <!-- The outline's huge shadow is the dimmed layer; the part inside stays bright -->
      <div
        class="hole"
        style:left="{hole.left}px"
        style:top="{hole.top}px"
        style:width="{hole.width}px"
        style:height="{hole.height}px"
      ></div>
    {:else}
      <div class="dim"></div>
    {/if}

    <div
      class="bubble"
      class:centered={!hole}
      bind:this={bubble}
      style:width="{BUBBLE_WIDTH}px"
      style:left={hole && place ? `${place.left}px` : null}
      style:top={hole && place ? `${place.top}px` : null}
      style:visibility={hole && !place ? "hidden" : null}
    >
      <p class="count">{t("tour.count", { n: String(tour.index + 1), total: String(tourSteps.length) })}</p>
      <h2 id="tour-title">{t(`tour.steps.${step.key}.title`)}</h2>
      <p class="text">{t(`tour.steps.${step.key}.text`)}</p>
      <div class="actions">
        {#if !last}<button class="btn btn-ghost skip" onclick={endTour}>{t("tour.skip")}</button>{/if}
        <span class="space"></span>
        {#if tour.index > 0}<button class="btn" onclick={() => moveTour(-1)}>{t("tour.back")}</button>{/if}
        {#if last}
          <button class="btn btn-primary" onclick={endTour}>{t("tour.done")}</button>
        {:else}
          <button class="btn btn-primary" onclick={() => moveTour(1)}>{t("tour.next")}</button>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .tour {
    position: fixed;
    inset: 0;
    z-index: 100;
  }
  /* Clicks on the dimmed part do nothing: the tour drives the screens */
  .dim,
  .hole {
    position: fixed;
  }
  .dim {
    inset: 0;
    background: rgb(5 7 16 / 0.72);
  }
  .hole {
    border: 2px solid var(--gold);
    border-radius: 10px;
    box-shadow: 0 0 0 200vmax rgb(5 7 16 / 0.72);
    transition:
      left 0.25s ease,
      top 0.25s ease,
      width 0.25s ease,
      height 0.25s ease;
  }

  /* The bubble: the app's dialogue box, small */
  .bubble {
    position: fixed;
    display: grid;
    gap: 8px;
    padding: 16px 18px 14px;
    border: 1.5px solid var(--gold);
    border-radius: var(--radius-box, 12px);
    background: var(--panel);
    box-shadow: 0 12px 32px rgb(0 0 0 / 0.45);
    transition:
      left 0.25s ease,
      top 0.25s ease;
  }
  .bubble.centered {
    left: 50%;
    top: 40%;
    transform: translate(-50%, -50%);
    width: 440px !important;
  }
  .count {
    margin: 0;
    color: var(--gold);
    font-size: var(--text-sm);
    font-weight: 700;
  }
  h2 {
    margin: 0;
    font-size: var(--text-md, 1.1rem);
  }
  .text {
    margin: 0;
    color: var(--ink-dim);
    line-height: 1.55;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 6px;
  }
  .space {
    flex: 1;
  }
  .skip {
    padding-left: 0;
  }
</style>
