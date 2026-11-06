<!--
  Profiles — what Subquest keeps about each game: where its text boxes are and
  the styles taken from its prints. The look of the translation comes from the
  styles now, so there is nothing else to set here: this screen shows what is
  saved, and lets a game be forgotten (areas, styles and prints).
-->
<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { app, persistSettings } from "../stores/app.svelte";
  import { showStyles } from "../stores/styles.svelte";
  import { errorKind, forgetStyles, listStyles, stylePrint, type GameArea, type TextStyle } from "../api";
  import RemoveButton from "../components/RemoveButton.svelte";

  let gameStyles = $state<Record<string, TextStyle[]>>({}); // game key -> its styles
  let prints = $state<Record<string, string>>({}); // "game/id" -> object URL of the print

  // Profiles sorted by game title, so the list reads like a library.
  const entries = $derived(
    Object.entries(app.settings.profiles).sort(([, a], [, b]) => a.title.localeCompare(b.title)),
  );

  // Name shown for an area: the user's, or "Area N".
  function areaName(a: GameArea, i: number): string {
    return a.name.trim() || t("play.area.defaultName", { n: String(i + 1) });
  }

  // Loads each game's styles and their prints (once per game).
  async function load(keys: string[]) {
    for (const key of keys) {
      if (gameStyles[key]) continue;
      const list = await listStyles(key).catch(() => [] as TextStyle[]);
      gameStyles[key] = list;
      for (const s of list) {
        const blob = await stylePrint(key, s.id).catch(() => null);
        if (blob) prints[`${key}/${s.id}`] = URL.createObjectURL(blob);
      }
    }
  }

  // Opens the Styles screen on this game, to change a style's font.
  function openStyles(key: string) {
    void showStyles(key);
    app.screen = "styles";
  }

  // Forgets the game: its areas and every style and print taken from it.
  async function forget(key: string) {
    try {
      await forgetStyles(key);
      delete app.settings.profiles[key];
      delete gameStyles[key];
      await persistSettings();
    } catch (e) {
      errorKind(e);
    }
  }

  $effect(() => void load(entries.map(([key]) => key)));
  $effect(() => () => Object.values(prints).forEach((url) => URL.revokeObjectURL(url)));
</script>

<div class="profiles">
  <header>
    <h1>{t("profiles.title")}</h1>
    <p class="sub">{t("profiles.intro")}</p>
  </header>

  {#if entries.length === 0}
    <div class="empty">
      <p>{t("profiles.empty")}</p>
      <button class="btn btn-primary" onclick={() => (app.screen = "play")}>{t("nav.play")}</button>
    </div>
  {:else}
    <ul class="games">
      {#each entries as [key, profile] (key)}
        {@const list = gameStyles[key] ?? []}
        <li class="game">
          <RemoveButton label={t("profiles.forget")} onconfirm={() => forget(key)} />
          <div class="title">
            <strong>{profile.title}</strong>
            <span class="exe">{key}</span>
          </div>

          <!-- Where the text is read -->
          <section>
            <h2>{t("profiles.areas")}</h2>
            <ul class="chips">
              {#each profile.areas as a, i (i)}
                <li class="chip">
                  {#if a.mode === "screen"}
                    {t("profiles.wholeScreen")}
                  {:else}
                    <span class="num">{i + 1}</span>
                    {areaName(a, i)}
                    <small>{a.region ? `${a.region[2]} × ${a.region[3]}` : t("profiles.unmarked")}</small>
                  {/if}
                </li>
              {/each}
            </ul>
          </section>

          <!-- How the translation looks: one print per style, with its font -->
          <section>
            <h2>{t("profiles.styles")}</h2>
            {#if list.length}
              <ul class="prints">
                {#each list as s (s.id)}
                  <li>
                    <button class="print" onclick={() => openStyles(key)} title={t("profiles.editStyle")}>
                      <span class="pic">
                        {#if prints[`${key}/${s.id}`]}<img src={prints[`${key}/${s.id}`]} alt="" />{/if}
                      </span>
                      <span class="caption">
                        <strong>{s.name || t("profiles.unnamed")}</strong>
                        <small class:missing={!s.font}>
                          {s.font ? s.font.replace(/^imported:/, "").replace(/\.[^.]+$/, "") : t("profiles.noFont")}
                        </small>
                      </span>
                    </button>
                  </li>
                {/each}
              </ul>
            {:else}
              <p class="none">{t("profiles.noStyles")}</p>
            {/if}
          </section>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .profiles {
    max-width: 900px;
    margin: 0 auto;
    display: grid;
    gap: 20px;
    padding-bottom: 40px;
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

  .games {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 12px;
  }
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
    align-items: baseline;
    gap: 10px;
    padding-right: 40px; /* room for the X */
    min-width: 0;
  }
  .title strong {
    font-size: var(--text-md, 1rem);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .exe {
    color: var(--ink-faint);
    font-size: var(--text-sm);
  }

  section {
    display: grid;
    gap: 8px;
  }
  h2 {
    margin: 0;
    color: var(--ink-dim);
    font-size: var(--text-sm);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
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

  /* Styles: the prints themselves, the clearest way to tell them apart */
  .prints {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 8px;
  }
  .print {
    width: 100%;
    display: grid;
    gap: 6px;
    padding: 6px;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: transparent;
    color: var(--ink);
    cursor: pointer;
    text-align: left;
  }
  .print:hover {
    border-color: var(--gold);
  }
  .pic {
    height: 64px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 5px;
    background: #0d1018;
    overflow: hidden;
  }
  .pic img {
    max-width: 100%;
    max-height: 64px;
    object-fit: contain;
  }
  .caption {
    display: grid;
    padding: 0 2px;
    min-width: 0;
  }
  .caption strong,
  .caption small {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--text-sm);
  }
  .caption small {
    color: var(--ink-faint);
  }
  .caption small.missing {
    color: var(--gold);
  }
  .none {
    margin: 0;
    color: var(--ink-faint);
    font-size: var(--text-sm);
  }
</style>
