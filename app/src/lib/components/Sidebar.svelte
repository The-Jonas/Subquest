<!-- Sidebar — main navigation, the guided tour, language switch and whether the screen reader (core) loaded. -->
<script lang="ts">
  import { app, persistSettings, type Screen } from "../stores/app.svelte";
  import { t, localeNames, type Locale } from "../i18n/index.svelte";
  import { session } from "../stores/session.svelte";
  import { startTour } from "../stores/tour.svelte";

  // The menu: few items, big narrow type, like a game's pause menu.
  const items: Screen[] = ["play", "games", "history", "settings"];
  const TOUR_ICON = "M10 3.5a6.5 6.5 0 1 0 0 13a6.5 6.5 0 0 0 0-13zM8 8a2 2 0 1 1 2.8 1.8c-.5.3-.8.7-.8 1.2v.5M10 13.5h.01";

  // Switches the UI language and saves the choice.
  async function changeLocale(event: Event) {
    app.settings.uiLanguage = (event.currentTarget as HTMLSelectElement).value;
    await persistSettings();
  }
</script>

<nav class="sidebar">
  <div class="brand">
    <svg viewBox="0 0 32 32" width="20" height="20" aria-hidden="true">
      <rect x="3" y="9" width="26" height="18" rx="4" fill="var(--panel)" stroke="var(--ink)" stroke-width="2" />
      <rect x="6" y="5.5" width="11" height="6" rx="2" fill="var(--gold)" stroke="var(--night-deep)" stroke-width="1.5" />
      <path d="M21 19h5l-2.5 3.5z" fill="var(--gold)" />
    </svg>
    <span>Subquest</span>
  </div>

  <ul>
    {#each items as id (id)}
      <li>
        <button
          class:active={app.screen === id}
          onclick={() => (app.screen = id)}
          aria-current={app.screen === id ? "page" : undefined}
          data-tour="nav-{id}"
        >
          {t(`nav.${id}`)}
          {#if id === "play" && session.running}
            <!-- The session keeps running on other screens: this dot says so -->
            <span class="live-dot" title={t("play.start.running", { game: session.game?.title ?? "" })}></span>
          {/if}
          {#if id === "history" && session.unseenMissed > 0}
            <!-- Lines that passed before their translation was ready -->
            <span class="badge" title={t("history.missedBadge")}>{session.unseenMissed}</span>
          {/if}
        </button>
      </li>
    {/each}
  </ul>

  <div class="foot">
    <!-- The guided tour: opens each screen and points at what matters -->
    <button class="tour-btn" onclick={startTour} data-tour="nav-tour">
      <svg viewBox="0 0 20 20" width="20" height="20" aria-hidden="true">
        <path d={TOUR_ICON} fill="none" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round" stroke-linecap="round" />
      </svg>
      {t("nav.tour")}
    </button>
    <label class="lang">
      <span class="sr-only">{t("settings.uiLanguage")}</span>
      <select class="input" value={app.settings.uiLanguage} onchange={changeLocale}>
        {#each Object.entries(localeNames) as [tag, name] (tag)}
          <option value={tag as Locale}>{name}</option>
        {/each}
      </select>
    </label>
    <!-- The screen reader (capture + OCR, the core DLL): is it there? -->
    <p class="core" class:ok={app.core.loaded} title={app.core.loaded ? t("core.okHelp") : t("core.missingHelp")}>
      <span class="dot"></span>
      {app.core.loaded ? t("core.ok") : t("core.missing")}
    </p>
  </div>
</nav>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 26px;
    padding: 22px 14px 18px 0;
    background: var(--night-deep);
    border-right: 1px solid var(--panel);
  }

  /* The name once, small: the menu is what matters here */
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-left: 22px;
    color: var(--ink-dim);
    font-family: var(--font-display);
    font-weight: 600;
    font-size: var(--text-sm);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }

  /* Menu items: big narrow words; the cursor is a bar on the left edge */
  li button {
    position: relative;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 0 4px 22px;
    border: 0;
    background: transparent;
    color: var(--ink-faint);
    cursor: pointer;
    text-align: left;
    font-family: var(--font-display);
    font-size: 30px;
    font-weight: 750;
    font-stretch: 70%;
    line-height: 1.1;
    transition: color 0.12s ease;
  }
  li button::before {
    content: "";
    position: absolute;
    left: 0;
    top: 8px;
    bottom: 8px;
    width: 4px;
    border-radius: 0 3px 3px 0;
    background: var(--gold);
    transform: scaleY(0);
    transition: transform 0.15s ease;
  }
  li button:hover {
    color: var(--ink-dim);
  }
  /* Keyboard focus: a quiet ring inside the item, not a box around the menu */
  li button:focus-visible {
    outline: 2px solid var(--gold);
    outline-offset: -2px;
    border-radius: 6px;
  }
  li button.active {
    color: var(--ink);
  }
  li button.active::before {
    transform: scaleY(1);
  }
  @media (prefers-reduced-motion: reduce) {
    li button,
    li button::before {
      transition: none;
    }
  }

  .foot {
    margin-top: auto;
    display: grid;
    gap: 12px;
    padding-left: 18px;
  }

  .tour-btn {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 10px;
    border: 1px dashed var(--line);
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--ink-dim);
    cursor: pointer;
    text-align: left;
  }
  .tour-btn:hover {
    color: var(--ink);
    border-color: var(--gold);
  }

  .lang select {
    font-size: var(--text-sm);
    padding: 7px 10px;
  }

  .core {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--text-sm);
    color: var(--ink-faint);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--danger);
  }
  .core.ok .dot {
    background: var(--live);
  }

  .live-dot {
    width: 8px;
    height: 8px;
    margin-left: 4px;
    border-radius: 50%;
    background: var(--live);
    box-shadow: 0 0 0 3px rgb(92 200 184 / 0.18);
  }

  .badge {
    margin-left: 4px;
    font-family: var(--font-body);
    min-width: 20px;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--gold);
    color: var(--gold-ink);
    font-size: 11px;
    font-weight: 700;
    text-align: center;
  }

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
