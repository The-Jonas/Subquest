<!--
  Styles — the fonts of a game, set by hand. Take a print of a phrase in the
  game (toolbar or Ctrl+Shift+S), pick the font that matches it, and from then
  on every line written in that color uses that font, whatever its size.
-->
<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { app } from "../stores/app.svelte";
  import { session } from "../stores/session.svelte";
  import { styles, showStyles } from "../stores/styles.svelte";
  import {
    beginSelection,
    profileKey,
    removeStyle,
    stylePrint,
    updateStyle,
    errorKind,
    type ImportedFont,
    type LetterCase,
    type TextStyle,
  } from "../api";
  import { applyCase, cssFamily, ensureFont, loadAllImported, styleCase } from "../fonts";
  import FontPicker from "../components/FontPicker.svelte";
  import FontDetect from "../components/FontDetect.svelte";

  let imported = $state<ImportedFont[]>([]);
  let prints = $state<Record<string, string>>({}); // "game/id" -> object URL of its print
  let open = $state<string | null>(null); // style whose font search is open

  // Games that have a profile, the running one first.
  const games = $derived(
    Object.entries(app.settings.profiles).sort(([a], [b]) => {
      const running = session.game ? profileKey(session.game) : "";
      return (b === running ? 1 : 0) - (a === running ? 1 : 0) || a.localeCompare(b);
    }),
  );
  const translatingThis = $derived(!!session.running && !!session.game && profileKey(session.game) === styles.key);

  // CSS color of a style's letters.
  function hex(rgb: number): string {
    return "#" + rgb.toString(16).padStart(6, "0");
  }

  // Loads the prints that aren't loaded yet. Keys include the game, so
  // switching games never shows another game's picture.
  async function loadPrints(key: string, ids: string[]) {
    for (const id of ids) {
      if (prints[`${key}/${id}`]) continue;
      const blob = await stylePrint(key, id).catch(() => null);
      if (blob) prints[`${key}/${id}`] = URL.createObjectURL(blob);
    }
  }

  // Saves a field of one style; the overlay restyles itself right away.
  async function save(s: TextStyle, change: { name?: string; font?: string; caps?: LetterCase }) {
    if (!styles.key) return;
    if (change.font) await ensureFont(change.font).catch(() => {});
    Object.assign(s, change);
    await updateStyle(styles.key, s.id, change).catch(errorKind);
  }

  // Forgets a style and its print.
  async function forget(s: TextStyle) {
    if (!styles.key) return;
    await removeStyle(styles.key, s.id).catch(errorKind);
    styles.list = styles.list.filter((x) => x.id !== s.id);
  }

  // Prints follow the list (they are never freed while the screen is open, so
  // nothing blinks when the list reloads).
  $effect(() => {
    const key = styles.key;
    const ids = styles.list.map((s) => s.id);
    if (key) void loadPrints(key, ids);
  });

  // A print just taken in the game: its font search opens by itself.
  $effect(() => {
    if (styles.focus && styles.list.some((s) => s.id === styles.focus)) {
      open = styles.focus;
      styles.focus = null;
    }
  });

  // No game picked yet: show the first one that has a profile.
  $effect(() => {
    if (!styles.key && games.length) void showStyles(games[0][0]);
  });
  loadAllImported()
    .then(async (fonts) => {
      imported = fonts;
    })
    .catch(errorKind);
  $effect(() => () => Object.values(prints).forEach((url) => URL.revokeObjectURL(url)));
</script>

<div class="styles-screen">
  <header>
    <div>
      <h1>{t("styles.title")}</h1>
      <p class="sub">{t("styles.intro")}</p>
    </div>
    {#if games.length > 1}
      <select class="input game" value={styles.key} onchange={(e) => showStyles(e.currentTarget.value)}>
        {#each games as [key, profile] (key)}<option value={key}>{profile.title}</option>{/each}
      </select>
    {/if}
  </header>

  <div class="take">
    <button class="btn btn-primary" onclick={() => beginSelection("style")} disabled={!translatingThis}>
      {t("styles.take")}
    </button>
    <p>{translatingThis ? t("styles.takeHelp", { keys: app.settings.shortcuts.style || "—" }) : t("styles.takeNeedsGame")}</p>
  </div>

  {#if styles.list.length === 0}
    <p class="empty">{t("styles.empty")}</p>
  {:else}
    <ul class="cards">
      {#each styles.list as s (s.id)}
        {@const print = prints[`${styles.key}/${s.id}`]}
        <li class="card">
          <!-- The real thing: the phrase exactly as the game drew it -->
          <div class="print-box">
            {#if print}<img class="print" src={print} alt="" />{/if}
          </div>

          <div class="fields">
            <input
              class="input name"
              value={s.name}
              placeholder={t("styles.namePlaceholder")}
              onchange={(e) => save(s, { name: e.currentTarget.value })}
            />
            <span class="swatch" style:background={hex(s.textRgb)} title={t("styles.color")}></span>
          </div>

          <!-- The font in use, in the style's color, right under the print to compare -->
          <p
            class="sample"
            style:font-family={cssFamily(s.font)}
            style:color={hex(s.textRgb)}
            style:font-variant-caps={styleCase(s) === "smallCaps" ? "small-caps" : null}
          >
            {applyCase(s.name || t("fonts.sampleText"), styleCase(s))}
          </p>
          {#if !s.font}<p class="hint">{t("styles.noFont")}</p>{/if}

          <div class="row">
            <FontPicker value={s.font} {imported} onchange={(font) => save(s, { font })} />
            <select class="input caps" value={s.caps} onchange={(e) => save(s, { caps: e.currentTarget.value as LetterCase })}>
              <!-- Default: the case seen on the print, so the translation is written like the game -->
              <option value="">{t("styles.caseAuto", { seen: s.seenCase ? t(`fonts.detect.cases.${s.seenCase}`) : t("styles.seenNormal") })}</option>
              <option value="none">{t("styles.caseNone")}</option>
              <option value="upper">{t("fonts.detect.cases.upper")}</option>
              <option value="smallCaps">{t("fonts.detect.cases.smallCaps")}</option>
            </select>
            <button class="btn" onclick={() => (open = open === s.id ? null : s.id)}>{t("styles.search")}</button>
            <button class="btn btn-ghost" onclick={() => forget(s)}>{t("styles.forget")}</button>
          </div>

          {#if open === s.id && styles.key}
            {@const key = styles.key}
            <FontDetect
              snapshot={() => stylePrint(key, s.id)}
              value={s.font}
              caps={s.caps}
              title={t("styles.searchTitle")}
              help={t("styles.searchHelp")}
              showCrop={false}
              onchoose={(font) => {
                save(s, { font }); // the case stays "as in the game", measured on the print
                loadAllImported().then((fonts) => (imported = fonts));
              }}
            />
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .styles-screen {
    max-width: 900px;
    margin: 0 auto;
    display: grid;
    gap: 20px;
    padding-bottom: 40px;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 20px;
  }
  h1 {
    margin: 0;
  }
  .sub {
    margin: 4px 0 0;
    color: var(--ink-dim);
    font-size: var(--text-sm);
    max-width: 62ch;
  }
  .game {
    width: auto;
    min-width: 200px;
  }

  .take {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 14px 16px;
    border: 1px dashed var(--line);
    border-radius: var(--radius-control);
  }
  .take p {
    margin: 0;
    color: var(--ink-dim);
    font-size: var(--text-sm);
  }
  .take .btn {
    flex-shrink: 0;
  }

  .cards {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 12px;
  }
  .card {
    display: grid;
    gap: 10px;
    padding: 16px;
    border: 1px solid var(--line);
    border-radius: var(--radius-control);
    background: var(--night-deep);
  }
  /* Fixed-height box: the card keeps its size while the picture loads */
  .print-box {
    height: 96px;
    display: flex;
    align-items: center;
    border-radius: 6px;
    background: #0d1018;
    overflow: hidden;
  }
  .print {
    display: block;
    max-width: 100%;
    max-height: 96px;
    object-fit: contain;
  }
  .fields {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .name {
    flex: 1;
  }
  .swatch {
    width: 26px;
    height: 26px;
    border-radius: 6px;
    border: 1px solid var(--line);
    flex-shrink: 0;
  }
  .sample {
    margin: 0;
    padding: 8px 12px;
    border-radius: 6px;
    background: #0d1018;
    font-size: 26px;
    line-height: 1.25;
    overflow-wrap: anywhere;
  }
  .hint {
    margin: 0;
    color: var(--gold);
    font-size: var(--text-sm);
  }
  .row {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .row :global(select:first-child) {
    flex: 1;
  }
  .caps {
    width: auto;
  }
  .empty {
    color: var(--ink-dim);
  }
</style>
