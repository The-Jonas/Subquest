<!--
  Games — everything Subquest keeps about a game, in one place: where its text
  boxes are, and its styles (a print of a phrase + the font for it). Take a
  print in the game (toolbar or shortcut), pick its font here, and from then on
  every line whose letters look like it uses that font, whatever its size.
-->
<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { app, persistSettings } from "../stores/app.svelte";
  import { session } from "../stores/session.svelte";
  import { styles, showStyles } from "../stores/styles.svelte";
  import {
    beginSelection,
    forgetStyles,
    profileKey,
    removeStyle,
    stylePrint,
    updateStyle,
    errorKind,
    type GameArea,
    type ImportedFont,
    type LetterCase,
    type TextStyle,
  } from "../api";
  import { applyCase, builtInFonts, cssFamily, ensureFont, importedOption, loadAllImported, styleCase, systemFonts } from "../fonts";
  import FontPicker from "../components/FontPicker.svelte";
  import FontDetect from "../components/FontDetect.svelte";
  import RemoveButton from "../components/RemoveButton.svelte";

  let imported = $state<ImportedFont[]>([]);
  let prints = $state<Record<string, string>>({}); // "game/id" -> object URL of its print
  let open = $state<string | null>(null); // style whose font search is open

  // Games that have a profile, the running one first, then by title.
  const games = $derived(
    Object.entries(app.settings.profiles).sort(([a, pa], [b, pb]) => {
      const running = session.game ? profileKey(session.game) : "";
      return (b === running ? 1 : 0) - (a === running ? 1 : 0) || pa.title.localeCompare(pb.title);
    }),
  );
  const profile = $derived(styles.key ? app.settings.profiles[styles.key] : undefined);
  const translatingThis = $derived(!!session.running && !!session.game && profileKey(session.game) === styles.key);

  // Name shown for an area: the user's, or "Area N".
  function areaName(a: GameArea, i: number): string {
    return a.name.trim() || t("play.area.defaultName", { n: String(i + 1) });
  }

  // Readable name of a font value ("imported:Cinzel-700.ttf" -> its label).
  function fontName(value: string): string {
    const all = [...builtInFonts, ...systemFonts, ...imported.map(importedOption)];
    return all.find((f) => f.value === value)?.label ?? value.replace(/^imported:/, "").replace(/\.[^.]+$/, "");
  }

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

  // Forgets the whole game: its areas, styles and prints. Shows the next game.
  async function forgetGame() {
    const key = styles.key;
    if (!key) return;
    try {
      await forgetStyles(key);
      delete app.settings.profiles[key];
      await persistSettings();
    } catch (e) {
      errorKind(e);
    }
    styles.key = null;
    styles.list = [];
  }

  // Prints follow the list (never freed while the screen is open, so nothing
  // blinks when the list reloads).
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

  // No game picked yet (or it was forgotten): show the first one.
  $effect(() => {
    if ((!styles.key || !app.settings.profiles[styles.key]) && games.length) void showStyles(games[0][0]);
  });

  loadAllImported()
    .then((fonts) => (imported = fonts))
    .catch(errorKind);
  $effect(() => () => Object.values(prints).forEach((url) => URL.revokeObjectURL(url)));
</script>

<div class="games">
  <header>
    <h1>{t("games.title")}</h1>
    <p class="sub">{t("games.intro")}</p>
  </header>

  {#if !profile || !styles.key}
    <div class="empty">
      <p>{t("games.empty")}</p>
      <button class="btn btn-primary" onclick={() => (app.screen = "play")}>{t("nav.play")}</button>
    </div>
  {:else}
    {@const key = styles.key}
    <!-- The game: name, switcher and the X that forgets everything about it -->
    <section class="game">
      <RemoveButton label={t("games.forget")} onconfirm={forgetGame} />
      <div class="title">
        {#if games.length > 1}
          <select class="input switch" value={key} onchange={(e) => showStyles(e.currentTarget.value)} aria-label={t("games.switch")}>
            {#each games as [k, p] (k)}<option value={k}>{p.title}</option>{/each}
          </select>
        {:else}
          <strong>{profile.title}</strong>
        {/if}
        <span class="exe">{key}</span>
        {#if translatingThis}<span class="live">{t("games.live")}</span>{/if}
      </div>

      <div class="block">
        <h2>{t("games.areas")}</h2>
        <ul class="chips">
          {#each profile.areas as a, i (i)}
            <li class="chip">
              {#if a.mode === "screen"}
                {t("games.wholeScreen")}
              {:else}
                <span class="num">{i + 1}</span>
                {areaName(a, i)}
                <small>{a.region ? `${a.region[2]} × ${a.region[3]}` : t("games.unmarked")}</small>
              {/if}
            </li>
          {/each}
          {#if translatingThis}
            <li>
              <button class="chip action" onclick={() => beginSelection("edit")}>{t("games.editAreas")}</button>
            </li>
          {/if}
        </ul>
        {#if !translatingThis}<p class="hint">{t("games.areasHint")}</p>{/if}
      </div>
    </section>

    <section class="block">
      <div class="styles-head">
        <h2>{t("games.styles")}</h2>
        <button class="btn btn-primary" onclick={() => beginSelection("style")} disabled={!translatingThis} data-tour="games-take">
          {t("styles.take")}
        </button>
      </div>
      <p class="hint">
        {translatingThis ? t("styles.takeHelp", { keys: app.settings.shortcuts.style || "—" }) : t("styles.takeNeedsGame")}
      </p>

      {#if styles.list.length === 0}
        <p class="none">{t("styles.empty")}</p>
      {:else}
        <ul class="cards">
          {#each styles.list as s (s.id)}
            {@const print = prints[`${key}/${s.id}`]}
            <li class="card" class:searching={open === s.id}>
              <RemoveButton label={t("styles.forget")} onconfirm={() => forget(s)} />
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

              <!-- The chosen font, in the style's color, right under the print to compare -->
              <p
                class="sample"
                style:font-family={cssFamily(s.font)}
                style:color={hex(s.textRgb)}
                style:font-variant-caps={styleCase(s) === "smallCaps" ? "small-caps" : null}
              >
                {applyCase(s.name || t("fonts.sampleText"), styleCase(s))}
              </p>

              <div class="controls">
                <label class="field">
                  <span class:missing={!s.font}>{s.font ? t("styles.font") : t("styles.noFont")}</span>
                  <FontPicker value={s.font} {imported} onchange={(font) => save(s, { font })} />
                </label>
                <label class="field">
                  <span>{t("styles.letters")}</span>
                  <select class="input" value={s.caps} onchange={(e) => save(s, { caps: e.currentTarget.value as LetterCase })}>
                    <!-- Default: the case seen on the print, so the translation is written like the game -->
                    <option value="">{t("styles.caseAuto", { seen: t(`styles.caseShort.${s.seenCase || "normal"}`) })}</option>
                    <option value="none">{t("styles.caseNone")}</option>
                    <option value="upper">{t("styles.caseShort.upper")}</option>
                    <option value="smallCaps">{t("styles.caseShort.smallCaps")}</option>
                  </select>
                </label>
              </div>
              <button class="btn btn-ghost search" onclick={() => (open = open === s.id ? null : s.id)}>
                {open === s.id ? t("games.closeSearch") : t("styles.search")}
              </button>

              {#if open === s.id}
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
        <p class="hint">{t("games.fallback")}</p>
      {/if}
    </section>
  {/if}
</div>

<style>
  .games {
    max-width: 980px;
    margin: 0 auto;
    display: grid;
    gap: 22px;
    padding-bottom: 40px;
  }
  h1 {
    margin: 0;
  }
  .sub {
    margin: 4px 0 0;
    color: var(--ink-dim);
    font-size: var(--text-sm);
    max-width: 66ch;
  }
  h2 {
    margin: 0;
    color: var(--ink-dim);
    font-size: var(--text-sm);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }
  .hint,
  .none {
    margin: 0;
    color: var(--ink-faint);
    font-size: var(--text-sm);
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

  /* The game card: name on top, its areas below */
  .game {
    position: relative; /* holds the X in its corner */
    display: grid;
    gap: 16px;
    padding: 16px 18px 18px;
    border: 1px solid var(--line);
    border-radius: var(--radius-control);
    background: var(--night-deep);
  }
  .title {
    display: flex;
    align-items: center;
    gap: 12px;
    padding-right: 40px; /* room for the X */
    min-width: 0;
  }
  .title strong {
    font-size: var(--text-lg, 1.25rem);
  }
  .switch {
    width: auto;
    min-width: 220px;
    font-weight: 700;
  }
  .exe {
    color: var(--ink-faint);
    font-size: var(--text-sm);
  }
  .live {
    padding: 2px 10px;
    border-radius: 999px;
    background: rgb(92 200 184 / 0.15);
    color: var(--live);
    font-size: var(--text-sm);
  }
  .block {
    display: grid;
    gap: 10px;
  }

  /* Areas: numbered pills, like the tabs in Play */
  .chips {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 5px 12px 5px 6px;
    border: 1px solid var(--line);
    border-radius: 999px;
    font-size: var(--text-sm);
  }
  .chip:not(:has(.num)) {
    padding-left: 12px;
  }
  .chip small {
    color: var(--ink-faint);
  }
  .chip.action {
    border-style: dashed;
    background: transparent;
    color: var(--ink-dim);
    cursor: pointer;
  }
  .chip.action:hover {
    color: var(--ink);
    border-color: var(--gold);
  }
  .num {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: var(--gold);
    color: var(--gold-ink);
    font-size: 11px;
    font-weight: 700;
  }

  .styles-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  /* Style cards side by side; the one searching fonts takes the whole row */
  .cards {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(380px, 1fr));
    gap: 12px;
  }
  .card {
    position: relative; /* holds the X in its corner */
    display: grid;
    gap: 10px;
    align-content: start;
    padding: 14px;
    border: 1px solid var(--line);
    border-radius: var(--radius-control);
    background: var(--night-deep);
  }
  .card.searching {
    grid-column: 1 / -1;
  }
  /* Fixed-height box: the card keeps its size while the picture loads */
  .print-box {
    height: 84px;
    margin-right: 34px; /* room for the X */
    display: flex;
    align-items: center;
    border-radius: 6px;
    background: #0d1018;
    overflow: hidden;
  }
  .print {
    display: block;
    max-width: 100%;
    max-height: 84px;
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
    font-size: 24px;
    line-height: 1.25;
    overflow-wrap: anywhere;
  }
  .controls {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 190px);
    gap: 10px;
  }
  .field {
    display: grid;
    gap: 4px;
    color: var(--ink-dim);
    font-size: var(--text-sm);
  }
  .field :global(select) {
    width: 100%;
  }
  /* No font yet: the style does nothing, so the label says it in the accent */
  .missing {
    color: var(--gold);
  }
  .search {
    justify-self: start;
    padding-left: 0;
  }
</style>
