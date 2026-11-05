<!-- App — layout: sidebar + current screen. -->
<script lang="ts">
  import Sidebar from "./lib/components/Sidebar.svelte";
  import Play from "./lib/screens/Play.svelte";
  import Profiles from "./lib/screens/Profiles.svelte";
  import Settings from "./lib/screens/Settings.svelte";
  import Tutorial from "./lib/screens/Tutorial.svelte";
  import { app, boot } from "./lib/stores/app.svelte";

  boot();
</script>

<div class="shell">
  <Sidebar />
  <main>
    {#if app.ready}
      {#if app.screen === "play"}
        <Play />
      {:else if app.screen === "profiles"}
        <Profiles />
      {:else if app.screen === "settings"}
        <Settings />
      {:else}
        <Tutorial />
      {/if}
    {/if}
  </main>
</div>

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
