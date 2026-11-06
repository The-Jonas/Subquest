<!-- App — layout: sidebar + current screen. -->
<script lang="ts">
  import Sidebar from "./lib/components/Sidebar.svelte";
  import Play from "./lib/screens/Play.svelte";
  import History from "./lib/screens/History.svelte";
  import Games from "./lib/screens/Games.svelte";
  import Settings from "./lib/screens/Settings.svelte";
  import Tour from "./lib/components/Tour.svelte";
  import SubtitleStrip from "./lib/components/SubtitleStrip.svelte";
  import { app, boot } from "./lib/stores/app.svelte";
  import { watchSession } from "./lib/stores/session.svelte";

  boot();
  watchSession(); // a game that closes ends its session on its own
</script>

<div class="shell">
  <Sidebar />
  <div class="column">
  <main>
    {#if app.ready}
      {#if app.screen === "play"}
        <Play />
      {:else if app.screen === "games"}
        <Games />
      {:else if app.screen === "history"}
        <History />
      {:else}
        <Settings />
      {/if}
    {/if}
  </main>
  <SubtitleStrip />
  </div>
</div>
<Tour />

<style>
  .shell {
    display: grid;
    grid-template-columns: 220px minmax(0, 1fr);
    height: 100%;
  }
  /* The screen scrolls; the subtitle strip stays at the bottom */
  .column {
    display: grid;
    grid-template-rows: minmax(0, 1fr) auto;
    min-height: 0;
  }
  main {
    overflow-y: auto;
    padding: 40px 40px 0;
  }
</style>
