<!--
  Profiles — what Subquest remembers about each game: where the text is and how
  the translation should look there (font and style), per area.
-->
<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { app, persistSettings } from "../stores/app.svelte";
  import { errorKind, type GameArea, type ImportedFont } from "../api";
  import { cssFamily, ensureFont, loadAllImported } from "../fonts";
  import FontPicker from "../components/FontPicker.svelte";

  const styles = ["inPlace", "classic"] as const;

  let imported = $state<ImportedFont[]>([]);
  let open = $state<string | null>(null); // profile being edited
  let saved = $state<string | null>(null);

  // Profiles sorted by game title, so the list reads like a library.
  const entries = $derived(
    Object.entries(app.settings.profiles).sort(([, a], [, b]) => a.title.localeCompare(b.title)),
  );

  // One-line description of where Subquest reads this game.
  function describe(area: GameArea | undefined): string {
    if (area?.mode === "dialogue" && area.region) {
      return t("profiles.dialogueArea", { width: String(area.region[2]), height: String(area.region[3]) });
    }
    return t("profiles.wholeScreen");
  }

  // Saves after a change in the editor and flashes a confirmation on that row.
  async function save(key: string) {
    try {
      await persistSettings();
      saved = key;
      setTimeout(() => (saved = saved === key ? null : saved), 1800);
    } catch (e) {
      errorKind(e);
    }
  }

  // Removes the profile; next time the game starts from scratch.
  async function forget(key: string) {
    delete app.settings.profiles[key];
    if (open === key) open = null;
    await save(key);
  }

  loadAllImported()
    .then((fonts) => (imported = fonts))
    .catch(errorKind);
</script>

<div class="profiles">
  <h1>{t("profiles.title")}</h1>

  {#if entries.length === 0}
    <div class="empty">
      <p>{t("profiles.empty")}</p>
      <button class="btn btn-primary" onclick={() => (app.screen = "play")}>{t("nav.play")}</button>
    </div>
  {:else}
    <ul>
      {#each entries as [key, profile] (key)}
        {@const area = profile.areas[0]}
        <li class:open={open === key}>
          <div class="row">
            <div class="name">
              <strong>{profile.title}</strong>
              <span class="exe">{key}</span>
            </div>
            <p class="where">{describe(area)}</p>
            <div class="actions">
              <button class="btn btn-ghost" onclick={() => (open = open === key ? null : key)} aria-expanded={open === key}>
                {t("profiles.look")}
              </button>
              <button class="btn btn-ghost" onclick={() => forget(key)}>{t("profiles.forget")}</button>
            </div>
          </div>

          {#if open === key && area}
            <!-- Look of the translation in this area: font + style, with a live sample -->
            <div class="editor">
              <label class="field">
                <span>{t("profiles.font")}</span>
                <FontPicker
                  bind:value={area.font}
                  {imported}
                  onchange={() => ensureFont(area.font).then(() => save(key))}
                />
              </label>
              <label class="field">
                <span>{t("fonts.detect.case")}</span>
                <select class="input" bind:value={area.caps} onchange={() => save(key)}>
                  {#each ["", "upper", "smallCaps"] as c (c)}
                    <option value={c}>{t(`fonts.detect.cases.${c || "normal"}`)}</option>
                  {/each}
                </select>
              </label>
              <div class="field">
                <span>{t("profiles.style")}</span>
                <div class="styles" role="radiogroup">
                  {#each styles as s (s)}
                    <label class="choice" class:selected={area.style === s}>
                      <input type="radio" value={s} bind:group={area.style} onchange={() => save(key)} />
                      <strong>{t(`profiles.styles.${s}`)}</strong>
                      <span>{t(`profiles.styles.${s}Help`)}</span>
                    </label>
                  {/each}
                </div>
              </div>
              <figure
                class="sample"
                class:in-place={area.style === "inPlace"}
                style:font-family={cssFamily(area.font)}
                style:text-transform={area.caps === "upper" ? "uppercase" : null}
                style:font-variant-caps={area.caps === "smallCaps" ? "small-caps" : null}
              >
                {t("profiles.sample")}
              </figure>
              {#if saved === key}<p class="ok" role="status">{t("settings.saved")}</p>{/if}
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .profiles {
    max-width: 760px;
    margin: 0 auto;
    display: grid;
    gap: 24px;
    padding-bottom: 32px;
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

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    border-top: 1px solid var(--panel-raised);
  }
  li {
    border-bottom: 1px solid var(--panel-raised);
  }
  li.open {
    background: var(--night-deep);
  }
  /* Rows like a save-file list: title on the left, how it's read, then the actions */
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr) auto;
    align-items: center;
    gap: 16px;
    padding: 14px 8px;
  }
  .name {
    display: grid;
    min-width: 0;
  }
  .name strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .exe,
  .where {
    color: var(--ink-faint);
    font-size: var(--text-sm);
  }
  .where {
    color: var(--ink-dim);
  }
  .actions {
    display: flex;
    gap: 4px;
  }

  .editor {
    display: grid;
    gap: 16px;
    padding: 4px 8px 20px;
  }
  .styles {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }
  .choice {
    display: grid;
    gap: 4px;
    padding: 12px 14px;
    border: 1px solid var(--line);
    border-radius: var(--radius-control);
    cursor: pointer;
  }
  .choice span {
    color: var(--ink-dim);
    font-size: var(--text-sm);
  }
  .choice.selected {
    border-color: var(--gold);
    box-shadow: inset 0 0 0 1px var(--gold);
  }
  .choice input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .choice:has(input:focus-visible) {
    outline: 2px solid var(--gold);
    outline-offset: 2px;
  }

  /* Sample: classic = the dark subtitle box; game = an example of sampled colors */
  .sample {
    margin: 0;
    padding: 14px 18px;
    border-radius: 8px;
    font-size: 22px;
    color: #fff;
    background: rgb(10 12 24 / 0.8);
    text-shadow: 0 1px 2px rgb(0 0 0 / 0.9);
  }
  .sample.in-place {
    border-radius: 2px;
    background: #1b2a24;
    color: #8fd19e;
  }
  .ok {
    color: var(--live);
    font-size: var(--text-sm);
  }
</style>
