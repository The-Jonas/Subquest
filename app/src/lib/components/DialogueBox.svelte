<!--
  DialogueBox — the RPG dialogue box that is Subquest's signature.
  The line (`line`) appears letter by letter, like in games, and ends with a
  bobbing ▼. It is the only automatic animation in the UI.
-->
<script lang="ts">
  import type { Snippet } from "svelte";

  let { speaker = "", line, children }: { speaker?: string; line: string; children?: Snippet } = $props();

  let typed = $state("");
  let done = $state(false);

  // Restarts the typing effect whenever the line changes.
  $effect(() => {
    const full = line;
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (reduced) {
      typed = full;
      done = true;
      return;
    }
    typed = "";
    done = false;
    let i = 0;
    const timer = setInterval(() => {
      i += 1;
      typed = full.slice(0, i);
      if (i >= full.length) {
        done = true;
        clearInterval(timer);
      }
    }, 22);
    return () => clearInterval(timer);
  });
</script>

<section class="dialogue">
  {#if speaker}<span class="nameplate">{speaker}</span>{/if}
  <h1 class="line" aria-label={line}>
    <span aria-hidden="true">{typed}</span><span class="continue" class:visible={done} aria-hidden="true">▼</span>
  </h1>
  {#if children}
    <div class="body">{@render children()}</div>
  {/if}
</section>

<style>
  .dialogue {
    position: relative;
    background: var(--panel);
    border: 1px solid var(--line);
    border-radius: var(--radius-box);
    padding: 28px 30px 26px;
  }

  /* Nameplate with the speaker's name, sitting on the top border */
  .nameplate {
    position: absolute;
    top: -16px;
    left: 26px;
    padding: 3px 14px;
    background: var(--gold);
    color: var(--gold-ink);
    font-family: var(--font-display);
    font-weight: 700;
    font-size: var(--text-sm);
    border-radius: var(--radius-small);
    box-shadow: 0 0 0 5px var(--night);
  }

  /* The question: big and narrow, typed out letter by letter */
  .line {
    font-size: 34px;
    font-weight: 800;
    font-stretch: 72%;
    min-height: 1.1em;
  }

  .continue {
    display: inline-block;
    margin-left: 10px;
    font-size: 0.55em;
    color: var(--gold);
    vertical-align: middle;
    opacity: 0;
  }
  .continue.visible {
    opacity: 1;
    animation: bob 1.1s ease-in-out infinite;
  }

  .body {
    margin-top: 22px;
  }

  @keyframes bob {
    50% {
      transform: translateY(3px);
    }
  }
</style>
