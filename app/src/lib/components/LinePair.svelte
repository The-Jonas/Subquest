<!--
  LinePair — one line read from the game: the original and its translation,
  grouped in one block. Used by the Play feed and History so both read alike.

  The translation is what the player came for, so it is the larger, brighter
  text, marked by the theme's accent; the original sits above it, quieter, in
  the same font, there to check what the OCR read. Line breaks that are only
  the edge of the game's box are joined back into paragraphs (paragraphs.ts).
-->
<script lang="ts">
  import { asParagraphs } from "../text/paragraphs";
  import { withAllIcons } from "../text/icons";
  import IconText from "./IconText.svelte";
  import type { IconPicture } from "../api";

  interface Props {
    original: string;
    translation?: string; // empty while waiting
    streaming?: boolean; // the translation is still arriving
    failed?: string; // message shown instead of the translation
    large?: boolean; // the newest line, at full size
    icons?: IconPicture[]; // pictures inside the line ("{1}" in the texts)
  }
  let { original, translation = "", streaming = false, failed = "", large = false, icons = [] }: Props = $props();

  const text = $derived(asParagraphs(original, translation && withAllIcons(original, translation)));
</script>

<div class="pair" class:large>
  <p class="original"><IconText text={text.original} {icons} /></p>
  {#if failed}
    <p class="translation failed">{failed}</p>
  {:else if translation}
    <p class="translation" class:streaming><IconText text={text.translation} {icons} /></p>
  {:else}
    <p class="translation pending" aria-label="…"><span></span><span></span><span></span></p>
  {/if}
</div>

<style>
  .pair {
    display: grid;
    gap: 8px;
    padding: 12px 16px 14px;
    border-radius: var(--radius-control);
    background: var(--panel-raised);
    border-left: 3px solid var(--gold);
    font-family: var(--font-body);
  }
  p {
    margin: 0;
    white-space: pre-line; /* breaks that are kept: menu items, new sentences */
    overflow-wrap: anywhere;
  }
  .original {
    color: var(--ink-dim);
    font-size: var(--text-sm);
    line-height: 1.45;
  }
  .translation {
    color: var(--ink);
    font-size: var(--text-md);
    line-height: 1.4;
  }
  .large .translation {
    font-size: var(--text-lg);
  }
  .large .original {
    font-size: var(--text-base);
  }
  .failed {
    color: var(--danger);
    font-size: var(--text-sm);
  }
  /* While the translation is still arriving, a caret in the accent color follows the text */
  .streaming::after {
    content: "▍";
    margin-left: 2px;
    color: var(--gold);
  }
  /* Waiting for the translation: three dots that breathe */
  .pending {
    display: flex;
    gap: 6px;
    padding: 6px 0;
  }
  .pending span {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ink-faint);
    animation: breathe 1.2s ease-in-out infinite;
  }
  .pending span:nth-child(2) {
    animation-delay: 0.2s;
  }
  .pending span:nth-child(3) {
    animation-delay: 0.4s;
  }
  @keyframes breathe {
    0%,
    100% {
      opacity: 0.3;
    }
    50% {
      opacity: 1;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .pending span {
      animation: none;
    }
  }
</style>
