<!-- Profiles — what Subquest remembers about each game (area mode and dialogue area). -->
<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { app, persistSettings } from "../stores/app.svelte";
  import { errorKind, type GameProfile } from "../api";

  // Profiles sorted by game title, so the list reads like a library.
  const entries = $derived(
    Object.entries(app.settings.profiles).sort(([, a], [, b]) => a.title.localeCompare(b.title)),
  );

  // One-line description of how Subquest reads this game.
  function describe(p: GameProfile): string {
    if (p.areaMode === "dialogue" && p.region) {
      return t("profiles.dialogueArea", { width: String(p.region[2]), height: String(p.region[3]) });
    }
    return t("profiles.wholeScreen");
  }

  // Removes the profile; next time the game starts from scratch.
  async function forget(key: string) {
    delete app.settings.profiles[key];
    try {
      await persistSettings();
    } catch (e) {
      errorKind(e);
    }
  }
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
        <li>
          <div>
            <strong>{profile.title}</strong>
            <span class="exe">{key}</span>
          </div>
          <p>{describe(profile)}</p>
          <button class="btn btn-ghost" onclick={() => forget(key)}>{t("profiles.forget")}</button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .profiles {
    max-width: 720px;
    margin: 0 auto;
    display: grid;
    gap: 24px;
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
  /* Rows like a save-file list: title on the left, how it's read, then the action */
  li {
    display: grid;
    grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr) auto;
    align-items: center;
    gap: 16px;
    padding: 14px 4px;
    border-bottom: 1px solid var(--panel-raised);
  }
  li div {
    display: grid;
    min-width: 0;
  }
  li strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .exe {
    color: var(--ink-faint);
    font-size: var(--text-sm);
  }
  li p {
    color: var(--ink-dim);
    font-size: var(--text-sm);
  }
</style>
