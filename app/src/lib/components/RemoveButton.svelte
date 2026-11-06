<!--
  RemoveButton — the "X" in the top corner of a card that deletes it.
  Deleting can't be undone, so the first click only asks ("Delete?") and the
  second one confirms; moving the mouse away or clicking "No" calls it off.
  The parent card needs `position: relative`.
-->
<script lang="ts">
  import { t } from "../i18n/index.svelte";

  let { label, onconfirm }: { label: string; onconfirm: () => unknown } = $props();

  let asking = $state(false);
  let busy = $state(false);

  // Second click: really deletes.
  async function confirm() {
    busy = true;
    try {
      await onconfirm();
    } finally {
      busy = false;
      asking = false;
    }
  }
</script>

<div class="remove" role="group" onmouseleave={() => (asking = false)}>
  {#if asking}
    <span class="ask">{t("remove.ask")}</span>
    <button class="yes" onclick={confirm} disabled={busy}>{t("remove.yes")}</button>
    <button class="no" onclick={() => (asking = false)} disabled={busy}>{t("remove.no")}</button>
  {:else}
    <button class="x" onclick={() => (asking = true)} aria-label={label} title={label}>
      <svg viewBox="0 0 20 20" aria-hidden="true"><path d="M5.5 5.5l9 9M14.5 5.5l-9 9" /></svg>
    </button>
  {/if}
</div>

<style>
  .remove {
    position: absolute;
    top: 8px;
    right: 8px;
    display: flex;
    align-items: center;
    gap: 4px;
    z-index: 1;
  }
  button {
    border: 0;
    cursor: pointer;
    font: inherit;
  }
  .x {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--ink-faint);
  }
  .x:hover {
    background: rgb(224 108 108 / 0.15);
    color: var(--danger);
  }
  svg {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
  }
  /* The question takes the place of the X, in the danger color */
  .ask {
    color: var(--danger);
    font-size: var(--text-sm);
    margin-right: 2px;
  }
  .yes,
  .no {
    padding: 4px 10px;
    border-radius: 999px;
    font-size: var(--text-sm);
  }
  .yes {
    background: var(--danger);
    color: #fff;
  }
  .no {
    background: transparent;
    color: var(--ink-dim);
  }
  .no:hover {
    color: var(--ink);
  }
</style>
