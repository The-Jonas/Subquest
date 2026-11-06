<!--
  SubtitleStrip — while a game is translated, the newest line sits at the
  bottom of every screen like a subtitle: the original small and quiet, the
  translation in the accent color. So the session is never out of sight, even
  in Settings. Hidden when nothing is being translated.
-->
<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { app } from "../stores/app.svelte";
  import { session } from "../stores/session.svelte";

  const line = $derived(session.lines[0]);
  const translation = $derived(line ? session.translations[line.frameId] : undefined);
  const text = $derived(translation?.errorKind ? "" : (translation?.text ?? ""));
</script>

{#if session.running && app.screen !== "play"}
  <!-- Play already shows the lines; elsewhere this strip keeps them in view -->
  <aside class="strip" aria-live="polite" aria-label={t("strip.label")}>
    {#if line}
      <span class="original">{line.text}</span>
      <span class="translation" class:waiting={!text}>{text || "…"}</span>
    {:else}
      <span class="original">{t("play.start.waiting")}</span>
    {/if}
  </aside>
{/if}

<style>
  .strip {
    display: flex;
    align-items: baseline;
    gap: 16px;
    padding: 12px 40px 14px;
    border-top: 1px solid var(--panel);
    background: var(--night-deep);
    min-width: 0;
  }
  .original {
    flex: 0 1 auto;
    max-width: 40%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--ink-faint);
    font-size: var(--text-sm);
  }
  /* The translation reads like a subtitle: accent color, a hard shadow */
  .translation {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--gold);
    font-size: var(--text-md);
    font-weight: 600;
    text-shadow: 0 1px 0 rgb(0 0 0 / 0.6);
  }
  .translation.waiting {
    color: var(--ink-faint);
  }
</style>
