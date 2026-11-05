<!--
  Shortcuts — the keys that work inside the game while it is translated.
  Click a key, press the new combination; it is saved right away and, mid-game,
  works at once. Shortcuts only exist during a session, so the rest of the
  time these keys belong to other programs again.
-->
<script lang="ts">
  import { t } from "../i18n/index.svelte";
  import { app, persistSettings } from "../stores/app.svelte";
  import { errorKind } from "../api";
  import {
    defaultShortcuts,
    recordKey,
    shortcutActions,
    takenBy,
    type ShortcutAction,
  } from "../shortcuts";

  let recording = $state<ShortcutAction | null>(null);
  let problem = $state<{ action: ShortcutAction; text: string } | null>(null);

  // Saves the settings; failing only loses the change on restart.
  async function save() {
    await persistSettings().catch(errorKind);
  }

  // Starts listening for the new keys of an action (Esc cancels).
  function record(action: ShortcutAction) {
    problem = null;
    recording = recording === action ? null : action;
  }

  // Key presses while recording: Esc cancels, the rest becomes the shortcut.
  function onKeydown(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      recording = null;
      return;
    }
    const action = recording;
    const result = recordKey(e);
    if ("waiting" in result) return;
    if ("error" in result) {
      problem = { action, text: t(`shortcuts.errors.${result.error}`) };
      return;
    }
    const other = takenBy(app.settings.shortcuts, result.keys, action);
    if (other) {
      problem = { action, text: t("shortcuts.errors.taken", { action: t(`shortcuts.actions.${other}.name`) }) };
      return;
    }
    app.settings.shortcuts[action] = result.keys;
    recording = null;
    problem = null;
    void save();
  }

  // Leaves an action without a shortcut (the toolbar still has its button).
  function clear(action: ShortcutAction) {
    app.settings.shortcuts[action] = "";
    recording = null;
    void save();
  }

  // Puts every key back to how Subquest comes.
  function restoreAll() {
    app.settings.shortcuts = { ...defaultShortcuts };
    recording = null;
    problem = null;
    void save();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="shortcuts">
  <header>
    <h1>{t("shortcuts.title")}</h1>
    <p class="sub">{t("shortcuts.intro")}</p>
  </header>

  <ul class="list">
    {#each shortcutActions as action (action)}
      {@const keys = app.settings.shortcuts[action]}
      <li>
        <div class="what">
          <strong>{t(`shortcuts.actions.${action}.name`)}</strong>
          <span>{t(`shortcuts.actions.${action}.help`)}</span>
          {#if problem?.action === action}<span class="problem" role="alert">{problem.text}</span>{/if}
        </div>
        <div class="keys">
          <!-- The key itself is the button: click, then press the new combination -->
          <button class="key" class:recording={recording === action} onclick={() => record(action)}>
            {#if recording === action}
              {t("shortcuts.press")}
            {:else if keys}
              {#each keys.split("+") as part, i (i)}{#if i > 0}<span class="plus">+</span>{/if}<kbd>{part}</kbd>{/each}
            {:else}
              {t("shortcuts.none")}
            {/if}
          </button>
          {#if keys && recording !== action}
            <button class="btn btn-ghost small" onclick={() => clear(action)}>{t("shortcuts.clear")}</button>
          {/if}
        </div>
      </li>
    {/each}
  </ul>

  <div class="foot">
    <p>{t("shortcuts.note")}</p>
    <button class="btn btn-ghost" onclick={restoreAll}>{t("shortcuts.restore")}</button>
  </div>
</div>

<style>
  .shortcuts {
    max-width: 760px;
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
    max-width: 60ch;
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    border: 1px solid var(--line);
    border-radius: var(--radius-control);
    background: var(--night-deep);
  }
  .list li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    padding: 14px 18px;
  }
  .list li + li {
    border-top: 1px solid var(--line);
  }
  .what {
    display: grid;
    gap: 2px;
  }
  .what span {
    color: var(--ink-dim);
    font-size: var(--text-sm);
  }
  .what .problem {
    color: var(--danger);
  }
  .keys {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }
  .key {
    min-width: 170px;
    display: inline-flex;
    justify-content: center;
    align-items: center;
    gap: 4px;
    padding: 6px 10px;
    border: 1px solid var(--line);
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--ink-dim);
    font: inherit;
    font-size: var(--text-sm);
    cursor: pointer;
  }
  .key:hover {
    border-color: var(--gold);
  }
  /* Waiting for keys: the gold of "your move" */
  .key.recording {
    border-color: var(--gold);
    color: var(--gold);
    box-shadow: inset 0 0 0 1px var(--gold);
  }
  kbd {
    padding: 2px 7px;
    border: 1px solid var(--line);
    border-bottom-width: 2px;
    border-radius: 5px;
    background: var(--panel);
    color: var(--ink);
    font: inherit;
    font-size: 13px;
  }
  .plus {
    color: var(--ink-faint);
  }
  .small {
    padding: 6px 10px;
    font-size: var(--text-sm);
  }
  .foot {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 20px;
    color: var(--ink-dim);
    font-size: var(--text-sm);
  }
  .foot p {
    margin: 0;
    max-width: 52ch;
  }
</style>
