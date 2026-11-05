<!-- Sidebar — main navigation, language switch and core status. -->
<script lang="ts">
  import { app, persistSettings, type Screen } from "../stores/app.svelte";
  import { t, localeNames, type Locale } from "../i18n/index.svelte";

  // Simple stroke icons, hand-drawn on the same 20px grid.
  const items: { id: Screen; icon: string }[] = [
    { id: "play", icon: "M6 4.5v11l9-5.5z" },
    { id: "profiles", icon: "M5 3.5h10v13l-5-3.5-5 3.5z" },
    { id: "settings", icon: "M3 6h9M15 6h2M3 14h2M8 14h9M12 4v4M5 12v4" },
    { id: "tutorial", icon: "M3 5l5-2 4 2 5-2v12l-5 2-4-2-5 2zM8 3v12M12 5v12" },
  ];

  // Switches the UI language and saves the choice.
  async function changeLocale(event: Event) {
    app.settings.uiLanguage = (event.currentTarget as HTMLSelectElement).value;
    await persistSettings();
  }
</script>

<nav class="sidebar">
  <div class="brand">
    <svg viewBox="0 0 32 32" width="28" height="28" aria-hidden="true">
      <rect x="3" y="9" width="26" height="18" rx="4" fill="var(--panel)" stroke="var(--ink)" stroke-width="2" />
      <rect x="6" y="5.5" width="11" height="6" rx="2" fill="var(--gold)" stroke="var(--night-deep)" stroke-width="1.5" />
      <path d="M21 19h5l-2.5 3.5z" fill="var(--gold)" />
    </svg>
    <span>Subquest</span>
  </div>

  <ul>
    {#each items as item (item.id)}
      <li>
        <button class:active={app.screen === item.id} onclick={() => (app.screen = item.id)} aria-current={app.screen === item.id ? "page" : undefined}>
          <svg viewBox="0 0 20 20" width="20" height="20" aria-hidden="true">
            <path d={item.icon} fill="none" stroke="currentColor" stroke-width="1.7" stroke-linejoin="round" stroke-linecap="round" />
          </svg>
          {t(`nav.${item.id}`)}
        </button>
      </li>
    {/each}
  </ul>

  <div class="foot">
    <label class="lang">
      <span class="sr-only">{t("settings.uiLanguage")}</span>
      <select class="input" value={app.settings.uiLanguage} onchange={changeLocale}>
        {#each Object.entries(localeNames) as [tag, name] (tag)}
          <option value={tag as Locale}>{name}</option>
        {/each}
      </select>
    </label>
    <p class="core" class:ok={app.core.loaded} title={app.core.loaded ? "" : t("core.missingHelp")}>
      <span class="dot"></span>
      {app.core.loaded ? t("core.ok") : t("core.missing")}
    </p>
  </div>
</nav>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    gap: 28px;
    padding: 22px 14px;
    background: var(--night-deep);
    border-right: 1px solid var(--panel);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 8px;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: var(--text-md);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }

  li button {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 9px 10px;
    border: 0;
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--ink-dim);
    cursor: pointer;
    text-align: left;
  }
  li button:hover {
    color: var(--ink);
  }
  /* Active item: gold marker on the left, like a game menu cursor */
  li button.active {
    color: var(--ink);
    background: var(--panel);
    box-shadow: inset 3px 0 0 var(--gold);
  }

  .foot {
    margin-top: auto;
    display: grid;
    gap: 12px;
    padding: 0 4px;
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

  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
