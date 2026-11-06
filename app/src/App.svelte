<!-- App — layout: sidebar + current screen. -->
<script lang="ts">
  import Sidebar from "./lib/components/Sidebar.svelte";
  import Play from "./lib/screens/Play.svelte";
  import History from "./lib/screens/History.svelte";
  import Games from "./lib/screens/Games.svelte";
  import Settings from "./lib/screens/Settings.svelte";
  import Tour from "./lib/components/Tour.svelte";
  import { app, boot } from "./lib/stores/app.svelte";
  import { watchSession } from "./lib/stores/session.svelte";

  boot();
  watchSession(); // a game that closes ends its session on its own
</script>

<div class="shell">
  <Sidebar />
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
</div>
<Tour />

<style>
  .shell {
    display: grid;
    grid-template-columns: 210px 1fr;
    height: 100%;
  }
  main {
    overflow-y: auto;
    padding: 40px 40px 0;
  }
</style>
