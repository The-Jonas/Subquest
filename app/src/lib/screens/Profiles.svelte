<!-- Profiles — perfis por jogo (área, idioma, glossário). Vazio até a fase 3. -->
<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { app } from "../stores/app.svelte";
</script>

<div class="profiles">
  <h1>{t("profiles.title")}</h1>
  <div class="empty">
    <p>{t("profiles.empty")}</p>
    <button class="btn btn-primary" onclick={() => (app.screen = "play")}>{t("nav.play")}</button>
  </div>
</div>

<style>
  .profiles {
    max-width: 720px;
    margin: 0 auto;
    display: grid;
    gap: 24px;
  }
  h1 {
    font-size: var(--text-xl);
  }
  .empty {
    display: grid;
    justify-items: start;
    gap: 18px;
    padding: 28px;
    border: 1px dashed var(--line);
    border-radius: var(--radius-box);
    color: var(--ink-dim);
    max-width: 60ch;
  }
</style>
