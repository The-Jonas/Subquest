<!-- Settings — idiomas, serviço de tradução e aparência da legenda, com prévia ao vivo. -->
<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { app, persistSettings } from "../stores/app.svelte";
  import { errorKind } from "../api";

  const gameLanguages = ["en", "ja", "zh-Hans", "ko"];
  const targetLanguages = ["pt-BR", "en", "es"];

  let saved = $state(false);
  let error = $state<string | null>(null);

  // Grava e mostra a confirmação por alguns segundos.
  async function save() {
    error = null;
    try {
      await persistSettings();
      saved = true;
      setTimeout(() => (saved = false), 2400);
    } catch (e) {
      error = t(`errors.${errorKind(e)}`);
    }
  }
</script>

<div class="settings">
  <h1>{t("settings.title")}</h1>

  <section>
    <h2>{t("settings.languages")}</h2>
    <div class="grid">
      <label class="field">
        <span>{t("settings.sourceLanguage")}</span>
        <select class="input" bind:value={app.settings.sourceLanguage}>
          {#each gameLanguages as tag (tag)}<option value={tag}>{t(`languages.${tag}`)}</option>{/each}
        </select>
      </label>
      <label class="field">
        <span>{t("settings.targetLanguage")}</span>
        <select class="input" bind:value={app.settings.targetLanguage}>
          {#each targetLanguages as tag (tag)}<option value={tag}>{t(`languages.${tag}`)}</option>{/each}
        </select>
      </label>
    </div>
  </section>

  <section>
    <h2>{t("settings.translation")}</h2>
    <div class="grid">
      <label class="field wide">
        <span>{t("settings.service")}</span>
        <select class="input" bind:value={app.settings.translator.kind}>
          <option value="openai-compatible">{t("settings.serviceLlm")}</option>
          <option value="deepl">{t("settings.serviceDeepl")}</option>
        </select>
      </label>
      {#if app.settings.translator.kind === "openai-compatible"}
        <label class="field">
          <span>{t("settings.baseUrl")}</span>
          <input class="input" bind:value={app.settings.translator.baseUrl} spellcheck="false" />
          <small>{t("settings.baseUrlHelp")}</small>
        </label>
        <label class="field">
          <span>{t("settings.model")}</span>
          <input class="input" bind:value={app.settings.translator.model} placeholder="qwen2.5:7b" spellcheck="false" />
        </label>
      {/if}
      <label class="field wide">
        <span>{t("settings.apiKey")}</span>
        <input class="input" type="password" bind:value={app.settings.translator.apiKey} autocomplete="off" />
        <small>{t("settings.apiKeyHelp")}</small>
      </label>
    </div>
  </section>

  <section>
    <h2>{t("settings.overlay")}</h2>
    <div class="grid">
      <label class="field">
        <span>{t("settings.fontSize")} <output>{app.settings.overlay.fontSize}px</output></span>
        <input type="range" min="14" max="40" bind:value={app.settings.overlay.fontSize} />
      </label>
      <label class="field">
        <span>{t("settings.opacity")} <output>{Math.round(app.settings.overlay.backgroundOpacity * 100)}%</output></span>
        <input type="range" min="0" max="1" step="0.02" bind:value={app.settings.overlay.backgroundOpacity} />
      </label>
      <label class="check wide">
        <input type="checkbox" bind:checked={app.settings.overlay.showOriginal} />
        {t("settings.showOriginal")}
      </label>
    </div>

    <!-- Prévia: um "cenário" listrado para mostrar a legenda sobre fundo irregular -->
    <figure class="preview" aria-label={t("settings.preview")}>
      <div class="subtitle" style:font-size="{app.settings.overlay.fontSize}px" style:background="rgb(10 12 24 / {app.settings.overlay.backgroundOpacity})">
        {#if app.settings.overlay.showOriginal}
          <small>{t("settings.previewOriginal")}</small>
        {/if}
        {t("settings.previewText")}
      </div>
    </figure>
  </section>

  <div class="bar">
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if saved}<p class="ok" role="status">{t("settings.saved")}</p>{/if}
    <button class="btn btn-primary" onclick={save}>{t("settings.save")}</button>
  </div>
</div>

<style>
  .settings {
    max-width: 720px;
    margin: 0 auto;
    display: grid;
    gap: 34px;
    padding-bottom: 24px;
  }

  h1 {
    font-size: var(--text-xl);
  }

  section {
    display: grid;
    gap: 16px;
  }

  h2 {
    font-size: var(--text-md);
    color: var(--gold);
  }

  .grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    align-items: start;
    gap: 16px 20px;
  }
  .wide {
    grid-column: 1 / -1;
  }

  output {
    color: var(--ink-faint);
    font-weight: 400;
    margin-left: 6px;
  }

  input[type="range"] {
    accent-color: var(--gold);
  }

  .check {
    display: flex;
    align-items: center;
    gap: 10px;
    cursor: pointer;
  }
  .check input {
    accent-color: var(--gold);
    width: 16px;
    height: 16px;
  }

  .preview {
    margin: 0;
    height: 170px;
    border-radius: var(--radius-box);
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding: 18px;
    overflow: hidden;
    background:
      repeating-linear-gradient(115deg, #3b5d4a 0 38px, #6c8a52 38px 76px, #d7c58a 76px 92px, #2c3f52 92px 140px);
  }
  .subtitle {
    max-width: 90%;
    padding: 8px 16px;
    border-radius: var(--radius-small);
    color: #fff;
    text-align: center;
    line-height: 1.35;
    text-shadow: 0 1px 2px rgb(0 0 0 / 0.8);
  }
  .subtitle small {
    display: block;
    font-size: 0.62em;
    opacity: 0.75;
    margin-bottom: 2px;
  }

  .bar {
    position: sticky;
    bottom: 0;
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 16px;
    padding: 14px 0;
    background: linear-gradient(transparent, var(--night) 35%);
  }
  .ok {
    color: var(--live);
  }
  .error {
    color: var(--danger);
  }
</style>
