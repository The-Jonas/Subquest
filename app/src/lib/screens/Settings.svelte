<!--
  Settings — in parts, one tab each: translation (languages and the two
  translator layers), look of the translation, in-game shortcuts, fonts, and
  what is kept on disk. Shortcuts and fonts save by themselves; the first two
  parts save with the button at the bottom.
-->
<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { app, persistSettings, type SettingsTab } from "../stores/app.svelte";
  import Shortcuts from "./Shortcuts.svelte";
  import Fonts from "./Fonts.svelte";
  import {
    cacheUsage,
    clearTranslationCache,
    errorDetail,
    errorKind,
    listModels,
    testTranslator,
    translatorPresets,
  } from "../api";

  const gameLanguages = ["en", "ja", "zh-Hans", "ko"];
  const targetLanguages = ["pt-BR", "en", "es"];
  const quickEngines = ["google", "deepl", ""]; // "" = layer off
  const tabs: SettingsTab[] = ["translation", "look", "shortcuts", "fonts", "storage"];
  const looks = ["inPlace", "classic"] as const;
  const withSave = $derived(app.settingsTab === "translation" || app.settingsTab === "look");

  // Saved translations [count, bytes], shown in the Storage section.
  let usage = $state<[number, number] | null>(null);
  cacheUsage()
    .then((u) => (usage = u))
    .catch(errorKind);

  // "1.2 MB" for a size in bytes.
  function megabytes(bytes: number): string {
    return `${(bytes / 1_000_000).toLocaleString(app.settings.uiLanguage, { maximumFractionDigits: 1 })} MB`;
  }

  // Forgets every saved translation (they are made again when the lines come back).
  async function clearCache() {
    await clearTranslationCache().catch(errorKind);
    usage = await cacheUsage().catch(() => null);
  }

  let saved = $state(false);
  let error = $state<string | null>(null);

  // Which preset the current address belongs to ("custom" when typed by hand).
  const preset = $derived(
    translatorPresets.find((p) => p.baseUrl === app.settings.translator.baseUrl)?.id ?? "custom",
  );
  let models = $state<string[]>([]);
  let modelsState = $state<"idle" | "loading" | "error">("idle");
  let test = $state<{ status: "idle" | "running" | "ok" | "error"; text: string; ms: number; detail: string }>({
    status: "idle",
    text: "",
    ms: 0,
    detail: "",
  });

  // Picking a service fills in its address; "custom" keeps whatever is typed.
  function choosePreset(id: string) {
    const found = translatorPresets.find((p) => p.id === id);
    if (found) app.settings.translator.baseUrl = found.baseUrl;
    models = [];
    test.status = "idle";
  }

  // Models that can't translate text (speech, images, video, embeddings) only add noise to the list.
  const NOT_FOR_TEXT = /(embedding|tts|audio|image|imagen|veo|aqa|live|robotics|computer-use)/i;

  // Asks the service which models it has, so the user picks a real name.
  async function fetchModels() {
    modelsState = "loading";
    try {
      const all = await listModels($state.snapshot(app.settings.translator));
      models = all.filter((m) => !NOT_FOR_TEXT.test(m)).sort((a, b) => b.localeCompare(a)); // newest versions first
      modelsState = "idle";
      if (!app.settings.translator.model && models.length) {
        app.settings.translator.model = models.find((m) => m.includes("flash")) ?? models[0];
      }
    } catch (e) {
      errorKind(e);
      modelsState = "error";
    }
  }

  // Translates a sample line with the settings on screen (saved or not).
  async function runTest() {
    test = { status: "running", text: "", ms: 0, detail: "" };
    try {
      const result = await testTranslator(
        $state.snapshot(app.settings.translator),
        app.settings.sourceLanguage,
        app.settings.targetLanguage,
      );
      test = { status: "ok", text: result.text, ms: Math.round(result.elapsedMs), detail: "" };
    } catch (e) {
      test = { status: "error", text: t(`errors.${errorKind(e)}`), ms: 0, detail: errorDetail(e) };
    }
  }

  // Saves and shows the confirmation for a few seconds.
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

  <!-- One part at a time: the screen stays short and each part easy to find -->
  <div class="tabs" role="tablist">
    {#each tabs as tab (tab)}
      <button
        role="tab"
        class:active={app.settingsTab === tab}
        aria-selected={app.settingsTab === tab}
        onclick={() => (app.settingsTab = tab)}
      >
        {t(`settings.tabs.${tab}`)}
      </button>
    {/each}
  </div>

  {#if app.settingsTab === "translation"}
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

  <section data-tour="translator">
    <h2><span class="layer">1</span>{t("settings.translation")}</h2>
    <p class="intro">{t("settings.translationIntro")}</p>
    <div class="grid">
      <label class="field">
        <span>{t("settings.service")}</span>
        <select class="input" value={preset} onchange={(e) => choosePreset(e.currentTarget.value)}>
          {#each translatorPresets as p (p.id)}
            <option value={p.id}>{t(`settings.presets.${p.id}`)}</option>
          {/each}
          <option value="custom">{t("settings.presets.custom")}</option>
        </select>
      </label>
      <label class="field">
        <span>{t("settings.baseUrl")}</span>
        <input class="input" bind:value={app.settings.translator.baseUrl} spellcheck="false" />
      </label>
      <label class="field wide">
        <span>{t("settings.apiKey")}</span>
        <input class="input" type="password" bind:value={app.settings.translator.apiKey} autocomplete="off" />
        <small>{t(preset === "gemini" ? "settings.apiKeyGemini" : "settings.apiKeyHelp")}</small>
      </label>
      <div class="field wide">
        <label for="model">{t("settings.model")}</label>
        <div class="row">
          {#if models.length}
            <!-- A real list once we know the models (a datalist would filter by what's typed) -->
            <select id="model" class="input" bind:value={app.settings.translator.model}>
              {#if !models.includes(app.settings.translator.model)}
                <option value={app.settings.translator.model}>{app.settings.translator.model}</option>
              {/if}
              {#each models as m (m)}<option value={m}>{m}</option>{/each}
            </select>
          {:else}
            <input id="model" class="input" bind:value={app.settings.translator.model} spellcheck="false" />
          {/if}
          <button class="btn" onclick={fetchModels} disabled={modelsState === "loading"}>
            {t("settings.fetchModels")}
          </button>
        </div>
        <small class:bad={modelsState === "error"}>
          {modelsState === "error"
            ? t("settings.fetchModelsError")
            : models.length
              ? t("settings.modelsFound", { count: String(models.length) })
              : t("settings.modelHelp")}
        </small>
      </div>
      <!-- Font detection only: may be a stronger (slower) model than the translator -->
      <div class="field wide">
        <label for="vision-model">{t("settings.visionModel")}</label>
        {#if models.length}
          <select id="vision-model" class="input" bind:value={app.settings.translator.visionModel}>
            <option value="">{t("settings.visionModelSame")}</option>
            {#if app.settings.translator.visionModel && !models.includes(app.settings.translator.visionModel)}
              <option value={app.settings.translator.visionModel}>{app.settings.translator.visionModel}</option>
            {/if}
            {#each models as m (m)}<option value={m}>{m}</option>{/each}
          </select>
        {:else}
          <input
            id="vision-model"
            class="input"
            bind:value={app.settings.translator.visionModel}
            placeholder={t("settings.visionModelSame")}
            spellcheck="false"
          />
        {/if}
        <small>{t("settings.visionModelHelp")}</small>
      </div>
    </div>

    <!-- Test: one sample line through the real service, with the time it took -->
    <div class="test">
      <button class="btn" onclick={runTest} disabled={test.status === "running"}>
        {test.status === "running" ? t("settings.testing") : t("settings.test")}
      </button>
      {#if test.status === "ok"}
        <p class="ok">{test.text} <span class="ms">{t("settings.testTime", { ms: String(test.ms) })}</span></p>
      {:else if test.status === "error"}
        <div class="test-error">
          <p class="bad">{test.text}</p>
          <!-- The service's own words: usually says exactly what's wrong (model, quota, parameter) -->
          <code>{test.detail}</code>
        </div>
      {/if}
    </div>
  </section>

  <!-- Second layer: shown first, replaced by the main translation when it arrives -->
  <section data-tour="quick">
    <h2><span class="layer">2</span>{t("settings.quick.title")}</h2>
    <p class="intro">{t("settings.quick.intro")}</p>
    <div class="engines" role="radiogroup">
      {#each quickEngines as engine (engine)}
        <label class="engine" class:selected={app.settings.quick.engine === engine}>
          <input type="radio" value={engine} bind:group={app.settings.quick.engine} />
          <strong>{t(`settings.quick.engines.${engine || "off"}`)}</strong>
          <span>{t(`settings.quick.engines.${engine || "off"}Help`)}</span>
        </label>
      {/each}
    </div>
    {#if app.settings.quick.engine === "deepl"}
      <label class="field wide deepl-key">
        <span>{t("settings.quick.deeplKey")}</span>
        <input class="input" type="password" bind:value={app.settings.quick.deeplKey} autocomplete="off" />
        <small>{t("settings.quick.deeplKeyHelp")}</small>
      </label>
    {/if}
  </section>

  {:else if app.settingsTab === "look"}
  <!-- How translations are drawn over the game -->
  <section>
    <h2>{t("settings.look.title")}</h2>
    <div class="engines looks" role="radiogroup">
      {#each looks as look (look)}
        <label class="engine" class:selected={app.settings.overlay.look === look}>
          <input type="radio" value={look} bind:group={app.settings.overlay.look} />
          <strong>{t(`settings.look.${look}`)}</strong>
          <span>{t(`settings.look.${look}Help`)}</span>
        </label>
      {/each}
    </div>
  </section>

  {#if app.settings.overlay.look === "classic"}
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

    <!-- Preview: a striped "scene" to show the subtitle over a busy background -->
    <figure class="preview" aria-label={t("settings.preview")}>
      <div class="subtitle" style:font-size="{app.settings.overlay.fontSize}px" style:background="rgb(10 12 24 / {app.settings.overlay.backgroundOpacity})">
        {#if app.settings.overlay.showOriginal}
          <small>{t("settings.previewOriginal")}</small>
        {/if}
        {t("settings.previewText")}
      </div>
    </figure>
  </section>

  {:else}
    <p class="intro">{t("settings.look.inPlaceNote")}</p>
  {/if}

  {:else if app.settingsTab === "shortcuts"}
    <Shortcuts />
  {:else if app.settingsTab === "fonts"}
    <Fonts />
  {:else}
  <!-- What Subquest keeps on disk, and a way to free it -->
  <section>
    <h2>{t("settings.storage.title")}</h2>
    <p class="intro">{t("settings.storage.intro")}</p>
    <div class="storage">
      <p>
        {#if usage}
          {t("settings.storage.cache", { count: usage[0].toLocaleString(app.settings.uiLanguage), size: megabytes(usage[1]) })}
        {/if}
      </p>
      <button class="btn btn-ghost" onclick={clearCache} disabled={!usage || usage[0] === 0}>
        {t("settings.storage.clear")}
      </button>
    </div>
  </section>

  {/if}

  {#if withSave}
  <div class="bar">
    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if saved}<p class="ok" role="status">{t("settings.saved")}</p>{/if}
    <button class="btn btn-primary" onclick={save}>{t("settings.save")}</button>
  </div>
  {/if}
</div>

<style>
  .settings {
    max-width: 760px;
    margin: 0 auto;
    display: grid;
    gap: 34px;
    padding-bottom: 24px;
  }

  h1 {
    font-size: var(--text-xl);
    margin-bottom: -14px;
  }

  /* Tabs: a row of pills under the title; the open one is lit */
  .tabs {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding-bottom: 12px;
    border-bottom: 1px solid var(--panel-raised, var(--line));
  }
  .tabs button {
    padding: 7px 14px;
    border: 0;
    border-radius: 999px;
    background: transparent;
    color: var(--ink-dim);
    cursor: pointer;
    font: inherit;
  }
  .tabs button:hover {
    color: var(--ink);
  }
  .tabs button.active {
    background: var(--gold);
    color: var(--gold-ink);
    font-weight: 700;
  }

  section {
    display: grid;
    gap: 16px;
  }

  h2 {
    font-size: var(--text-md);
    color: var(--gold);
  }

  /* Layer number in front of the translator headings: 1 = main, 2 = quick */
  .layer {
    display: inline-grid;
    place-items: center;
    width: 22px;
    height: 22px;
    margin-right: 10px;
    border-radius: 50%;
    background: var(--gold);
    color: var(--gold-ink);
    font-size: 12px;
    vertical-align: 2px;
  }
  .intro {
    margin: -6px 0 14px;
    color: var(--ink-dim);
    font-size: var(--text-sm);
    max-width: 62ch;
  }
  .engines {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
  }
  .engine {
    position: relative; /* keeps the hidden radio inside the card */
    display: grid;
    gap: 4px;
    align-content: start;
    padding: 12px 14px;
    border: 1px solid var(--line);
    border-radius: var(--radius-control);
    background: var(--night-deep);
    cursor: pointer;
  }
  .engine span {
    color: var(--ink-dim);
    font-size: var(--text-sm);
  }
  .engine.selected {
    border-color: var(--gold);
    box-shadow: inset 0 0 0 1px var(--gold);
  }
  .engine input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .engine:has(input:focus-visible) {
    outline: 2px solid var(--gold);
    outline-offset: 2px;
  }
  .looks {
    grid-template-columns: 1fr 1fr;
  }
  .storage {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .storage p {
    margin: 0;
  }
  .deepl-key {
    margin-top: 12px;
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

  .row {
    display: flex;
    gap: 8px;
  }
  .row .input {
    flex: 1;
  }
  .bad {
    color: var(--danger);
  }

  .test {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .test p {
    margin: 0;
  }
  .test-error {
    display: grid;
    gap: 6px;
    min-width: 0;
  }
  .test-error code {
    font-size: 12px;
    color: var(--ink-faint);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 7.5em;
    overflow-y: auto;
  }
  .ms {
    color: var(--ink-faint);
    font-size: var(--text-sm);
    margin-left: 6px;
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
