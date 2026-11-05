<!--
  Styles — the album of a game's text styles. Every way the game writes text
  (menus, dialogue, names, songs...) shows up here with a print of the real
  line, the font Subquest is using for it, and a way to find a closer one on
  Google Fonts. Works the same with marked areas or the whole screen.
-->
<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { app } from "../stores/app.svelte";
  import { session } from "../stores/session.svelte";
  import { styles, showStyles } from "../stores/styles.svelte";
  import { markStylesSeen, profileKey, removeStyle, stylePrint, updateStyle, errorKind, type ImportedFont, type LetterCase, type TextStyle } from "../api";
  import { cssFamily, ensureFont, loadAllImported, builtInFonts, systemFonts, importedOption } from "../fonts";
  import FontPicker from "../components/FontPicker.svelte";
  import FontDetect from "../components/FontDetect.svelte";

  let imported = $state<ImportedFont[]>([]);
  let prints = $state<Record<string, string>>({}); // style id -> object URL of its print
  let open = $state<string | null>(null); // style whose font search is open

  // Games that have a profile, the running one first.
  const games = $derived(
    Object.entries(app.settings.profiles).sort(([a], [b]) => {
      const running = session.game ? profileKey(session.game) : "";
      return (b === running ? 1 : 0) - (a === running ? 1 : 0) || a.localeCompare(b);
    }),
  );

  // Most seen styles first: those are the ones that matter most on screen.
  const list = $derived([...styles.list].sort((a, b) => b.seen - a.seen));

  // Name of a font value, for "in use" lines.
  function fontName(value: string): string {
    const all = [...builtInFonts, ...systemFonts, ...imported.map(importedOption)];
    return all.find((f) => f.value === value)?.label ?? value.replace(/^imported:/, "").replace(/\.[^.]+$/, "");
  }

  // Font the overlay uses for a style: the chosen one, else the automatic one.
  function inUse(s: TextStyle): string {
    return s.font || s.autoFont;
  }

  // CSS color of a style's letters.
  function hex(rgb: number): string {
    return "#" + rgb.toString(16).padStart(6, "0");
  }

  // Loads the prints of the shown game (and frees the old ones).
  async function loadPrints(key: string, ids: string[]) {
    for (const id of ids) {
      if (prints[id]) continue;
      const blob = await stylePrint(key, id).catch(() => null);
      if (blob && styles.key === key) prints[id] = URL.createObjectURL(blob);
    }
  }

  // Switches the album to another game.
  async function pickGame(key: string) {
    Object.values(prints).forEach((url) => URL.revokeObjectURL(url));
    prints = {};
    open = null;
    await showStyles(key);
  }

  // Saves a font chosen for one style (from the list or from a suggestion).
  async function choose(s: TextStyle, font: string, caps: LetterCase) {
    if (!styles.key) return;
    await ensureFont(font).catch(() => {});
    s.font = font;
    s.caps = caps;
    await updateStyle(styles.key, s.id, { font, caps }).catch(errorKind);
  }

  // Forgets a style (it comes back with a new print if the game shows it again).
  async function forget(s: TextStyle) {
    if (!styles.key) return;
    await removeStyle(styles.key, s.id).catch(errorKind);
    styles.list = styles.list.filter((x) => x.id !== s.id);
  }

  // Prints follow the list; opening the screen marks the styles as seen.
  $effect(() => {
    const key = styles.key;
    const ids = styles.list.map((s) => s.id);
    if (key) void loadPrints(key, ids);
  });
  $effect(() => {
    if (styles.key && styles.unseen > 0) {
      void markStylesSeen(styles.key);
      styles.unseen = 0; // clear the sidebar badge right away
    }
  });

  // First visit without a session: show the first game that has a profile.
  $effect(() => {
    if (!styles.key && games.length) void pickGame(games[0][0]);
  });
  loadAllImported()
    .then((fonts) => (imported = fonts))
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
      <select class="input game" value={styles.key} onchange={(e) => pickGame(e.currentTarget.value)}>
        {#each games as [key, profile] (key)}<option value={key}>{profile.title}</option>{/each}
      </select>
    {/if}
  </header>

  {#if list.length === 0}
    <div class="empty">
      <p>{t("styles.empty")}</p>
      {#if !session.running}
        <button class="btn btn-primary" onclick={() => (app.screen = "play")}>{t("nav.play")}</button>
      {/if}
    </div>
  {:else}
    <ul class="cards">
      {#each list as s (s.id)}
        <li class="card" class:new={s.new}>
          <!-- The real thing first: the line exactly as the game drew it -->
          {#if prints[s.id]}
            <img class="print" src={prints[s.id]} alt={s.sample} />
          {:else}
            <div class="print placeholder"></div>
          {/if}

          <div class="meta">
            <span class="swatch" style:background={hex(s.textRgb)}></span>
            <span>{t("styles.size", { px: String(s.height) })}</span>
            <span>{t("styles.seen", { count: String(s.seen) })}</span>
            {#if s.new}<span class="tag">{t("styles.new")}</span>{/if}
          </div>

          <!-- The same line in the font in use, right under the print, to compare -->
          <p
            class="sample"
            style:font-family={cssFamily(inUse(s))}
            style:color={hex(s.textRgb)}
            style:text-transform={s.font && s.caps === "upper" ? "uppercase" : null}
            style:font-variant-caps={s.font && s.caps === "smallCaps" ? "small-caps" : null}
          >
            {s.sample}
          </p>
          <p class="using">
            {#if s.font}
              {t("styles.chosen", { font: fontName(s.font) })}
            {:else if s.autoFont}
              {t("styles.auto", { font: fontName(s.autoFont) })}
            {:else}
              {t("styles.matching")}
            {/if}
          </p>

          <div class="row">
            <FontPicker
              value={s.font}
              {imported}
              onchange={(font) => choose(s, font, s.caps)}
            />
            <button class="btn" onclick={() => (open = open === s.id ? null : s.id)}>
              {t("styles.search")}
            </button>
            <button class="btn btn-ghost" onclick={() => forget(s)}>{t("styles.forget")}</button>
          </div>

          {#if open === s.id && styles.key}
            <FontDetect
              snapshot={() => stylePrint(styles.key!, s.id)}
              value={s.font}
              caps={s.caps}
              title={t("styles.searchTitle")}
              help={t("styles.searchHelp")}
              showCrop={false}
              onchoose={(font, caps) => choose(s, font, caps)}
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
    max-width: 60ch;
  }
  .game {
    width: auto;
    min-width: 200px;
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
  /* A style the user hasn't looked at yet */
  .card.new {
    border-left: 3px solid var(--gold);
  }
  .print {
    display: block;
    max-width: 100%;
    max-height: 110px;
    object-fit: contain;
    object-position: left center;
    border-radius: 6px;
    border: 1px solid var(--line);
    image-rendering: auto;
  }
  .placeholder {
    height: 48px;
    background: var(--panel);
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 12px;
    color: var(--ink-faint);
    font-size: 12px;
  }
  .swatch {
    width: 14px;
    height: 14px;
    border-radius: 3px;
    border: 1px solid var(--line);
  }
  .tag {
    padding: 1px 7px;
    border: 1px solid var(--gold);
    border-radius: 999px;
    color: var(--gold);
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
  .using {
    margin: 0;
    color: var(--ink-dim);
    font-size: var(--text-sm);
  }
  .row {
    display: flex;
    gap: 8px;
    align-items: center;
  }
  .row :global(select) {
    flex: 1;
  }
  .empty {
    display: grid;
    justify-items: start;
    gap: 12px;
    color: var(--ink-dim);
  }
</style>
