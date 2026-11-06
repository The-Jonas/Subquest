<!--
  IconText — a text with the game's pictures in it: each "{n}" placeholder is
  drawn as the picture cut from the game (a gamepad button), at the size of
  the letters around it (over the game: at the size it had there). Words in
  "<c1>...</c1>" tags take that tag's color. Text without placeholders is
  just text.
-->
<script lang="ts">
  import type { IconPicture } from "../api";
  import { pieces } from "../text/icons";

  interface Props {
    text: string;
    icons?: IconPicture[];
    scale?: number; // CSS px per game window pixel: icons keep their size in the game (else: the letters' size)
    colors?: string[]; // colors of the text's "<c1>" tags (none: tagged words keep the text's color)
  }
  let { text, icons = [], scale, colors = [] }: Props = $props();

  const parts = $derived(pieces(text));
</script>

{#each parts as part, i (i)}{#if "icon" in part}{#if icons[part.icon]}<img
        class="icon"
        class:game-size={scale != null}
        src={icons[part.icon].url}
        style:height={scale != null ? `${icons[part.icon].rect[3] * scale}px` : null}
        alt=""
      />{:else}<span class="lost">◆</span>{/if}{:else if part.mark !== undefined && colors[part.mark]}<span
      style:color={colors[part.mark]}>{part.text}</span>{:else}{part.text}{/if}{/each}

<style>
  /* A picture sits in the line like a letter: as tall as the text, on its middle */
  .icon {
    height: 1.15em;
    width: auto;
    vertical-align: -0.22em;
    margin: 0 0.08em;
  }
  /* Over the game: the picture at its own size, centered on the line */
  .icon.game-size {
    vertical-align: middle;
  }
  .lost {
    opacity: 0.6;
  }
</style>
