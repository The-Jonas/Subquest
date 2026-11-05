<!-- Tutorial — usage walkthrough, shown on first run and always available in the menu. -->
<script lang="ts">
  import DialogueBox from "../components/DialogueBox.svelte";
  import { t, tRaw } from "../i18n/index.svelte";
  import { app, persistSettings } from "../stores/app.svelte";

  type Page = { title: string; text: string };

  let index = $state(0);
  const pages = $derived(tRaw<Page[]>("tutorial.pages"));
  const page = $derived(pages[index]);
  const last = $derived(index === pages.length - 1);

  // Marks the tutorial as seen and goes to the Play screen.
  async function finish() {
    app.settings.tutorialDone = true;
    await persistSettings();
    app.screen = "play";
  }
</script>

<div class="tutorial">
  <DialogueBox speaker={t("tutorial.title")} line={page.title}>
    <p class="text">{page.text}</p>
    <div class="actions">
      <button class="btn btn-ghost skip" onclick={finish}>{t("tutorial.skip")}</button>
      <span class="count" aria-label="{index + 1} / {pages.length}">{index + 1} / {pages.length}</span>
      <button class="btn" onclick={() => index--} disabled={index === 0}>{t("tutorial.back")}</button>
      {#if last}
        <button class="btn btn-primary" onclick={finish}>{t("tutorial.done")}</button>
      {:else}
        <button class="btn btn-primary" onclick={() => index++}>{t("tutorial.next")}</button>
      {/if}
    </div>
  </DialogueBox>
</div>

<style>
  .tutorial {
    max-width: 680px;
    margin: 0 auto;
    padding-top: 28px;
  }
  .text {
    font-size: var(--text-md);
    color: var(--ink-dim);
    max-width: 60ch;
  }
  .actions {
    margin-top: 26px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .skip {
    margin-right: auto;
    padding-left: 0;
  }
  .count {
    color: var(--ink-faint);
    font-size: var(--text-sm);
    margin-right: 8px;
  }
</style>
