<script lang="ts">
  import { onMount } from "svelte";
  import { app } from "$lib/app.svelte";
  import TopBar from "$lib/components/TopBar.svelte";
  import Toasts from "$lib/components/Toasts.svelte";
  import HomeView from "$lib/views/HomeView.svelte";
  import GameView from "$lib/views/GameView.svelte";
  import LibraryView from "$lib/views/LibraryView.svelte";
  import SettingsView from "$lib/views/SettingsView.svelte";

  let content: HTMLElement | undefined = $state();

  onMount(() => app.init());

  // Each view (and each game page) starts at the top.
  $effect(() => {
    void app.view;
    void app.gameId;
    content?.scrollTo({ top: 0 });
  });
</script>

<div class="app">
  <TopBar />
  <main class="content" bind:this={content}>
    {#if app.view === "home"}
      <HomeView />
    {:else if app.view === "game"}
      <GameView />
    {:else if app.view === "settings"}
      <SettingsView />
    {:else}
      <LibraryView />
    {/if}
  </main>
  <Toasts />
</div>

<style>
  .app {
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  .content {
    flex: 1;
    overflow-y: auto;
    scrollbar-gutter: stable;
  }
</style>
