<!--
  DialogueBox — the card of the Play flow: a plain heading (`line`) and the
  step's content below it. No animation: the text is readable at once.
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  let { line, children }: { line: string; children?: Snippet } = $props();
</script>

<section class="dialogue">
  <h1 class="line">{line}</h1>
  {#if children}
    <div class="body">{@render children()}</div>
  {/if}
</section>

<style>
  .dialogue {
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--radius-box);
    padding: 28px 30px 26px;
  }

  .line {
    font-size: var(--text-xl);
    min-height: 1.1em;
  }

  .body {
    margin-top: 18px;
  }
</style>
