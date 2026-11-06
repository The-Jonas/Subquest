<!--
  Fonts (a part of Settings) — import font files so the overlay can match a game's look. Files are
  copied into the app's own folder: nothing is installed in Windows.
-->
<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { errorDetail, errorKind, importFont, removeFont, type ImportedFont } from "../api";
  import { builtInFonts, importedOption, loadAllImported, loadImportedFont } from "../fonts";

  let fonts = $state<ImportedFont[]>([]);
  let error = $state<string | null>(null);
  let input = $state<HTMLInputElement>();
  let dragging = $state(false);

  // Reloads the list (and registers every imported font for the previews).
  async function refresh() {
    fonts = await loadAllImported();
  }

  // Imports each chosen file; one bad file doesn't stop the others.
  async function addFiles(files: FileList | null) {
    error = null;
    for (const file of Array.from(files ?? [])) {
      try {
        const entry = await importFont(file);
        await loadImportedFont(entry.file);
      } catch (e) {
        errorKind(e);
        error = t("fonts.importError", { name: file.name, detail: errorDetail(e) });
      }
    }
    await refresh();
  }

  // Deletes an imported font.
  async function remove(file: string) {
    try {
      await removeFont(file);
      await refresh();
    } catch (e) {
      error = errorDetail(e);
    }
  }

  // Drop zone handlers: highlight while dragging, import on drop.
  function onDrop(e: DragEvent) {
    e.preventDefault();
    dragging = false;
    addFiles(e.dataTransfer?.files ?? null);
  }

  refresh().catch(errorKind);
</script>

<div class="fonts">
  <p class="intro">{t("fonts.intro")}</p>

  <div
    class="drop"
    class:dragging
    role="button"
    tabindex="0"
    ondragover={(e) => {
      e.preventDefault();
      dragging = true;
    }}
    ondragleave={() => (dragging = false)}
    ondrop={onDrop}
    onclick={() => input?.click()}
    onkeydown={(e) => (e.key === "Enter" || e.key === " ") && input?.click()}
  >
    <strong>{t("fonts.import")}</strong>
    <span>{t("fonts.importHelp")}</span>
    <input
      bind:this={input}
      type="file"
      accept=".ttf,.otf,.woff,.woff2"
      multiple
      hidden
      onchange={(e) => addFiles(e.currentTarget.files)}
    />
  </div>
  {#if error}<p class="bad" role="alert">{error}</p>{/if}

  <section>
    <h2>{t("fonts.groups.imported")}</h2>
    {#if fonts.length === 0}
      <p class="muted">{t("fonts.none")}</p>
    {:else}
      <ul>
        {#each fonts as font (font.file)}
          <li>
            <span class="sample" style:font-family={importedOption(font).family}>{t("fonts.sampleText")}</span>
            <span class="file">{font.name}</span>
            <button class="btn btn-ghost" onclick={() => remove(font.file)}>{t("fonts.remove")}</button>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section>
    <h2>{t("fonts.groups.builtIn")}</h2>
    <ul>
      {#each builtInFonts as font (font.value)}
        <li>
          <span class="sample" style:font-family={font.family}>{t("fonts.sampleText")}</span>
          <span class="file">{font.label}</span>
        </li>
      {/each}
    </ul>
  </section>
</div>

<style>
  .fonts {
    max-width: 760px;
    margin: 0 auto;
    display: grid;
    gap: 22px;
    padding-bottom: 32px;
  }
  h2 {
    font-size: var(--text-md);
    color: var(--gold);
    margin-bottom: 8px;
  }
  .intro,
  .muted {
    color: var(--ink-dim);
    max-width: 62ch;
  }

  .drop {
    display: grid;
    gap: 4px;
    padding: 22px;
    border: 1px dashed var(--line);
    border-radius: var(--radius-box);
    text-align: center;
    cursor: pointer;
  }
  .drop span {
    color: var(--ink-faint);
    font-size: var(--text-sm);
  }
  .drop:hover,
  .drop.dragging {
    border-color: var(--gold);
    background: var(--night-deep);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    border-top: 1px solid var(--panel-raised);
  }
  li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: 16px;
    padding: 12px 4px;
    border-bottom: 1px solid var(--panel-raised);
  }
  /* The font speaks for itself: its own sample is the row's main content */
  .sample {
    font-size: 22px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .file {
    color: var(--ink-faint);
    font-size: var(--text-sm);
  }
  .bad {
    color: var(--danger);
    font-size: var(--text-sm);
  }
</style>
