<script lang="ts">
  import { onMount } from "svelte";
  import { app, type View } from "$lib/app.svelte";
  import ThemeMenu from "./ThemeMenu.svelte";

  let searchEl: HTMLInputElement | undefined = $state();

  const TABS: { id: View; label: string }[] = [
    { id: "home", label: "Home" },
    { id: "library", label: "Library" },
  ];

  // Searching always happens in the library.
  function onSearchInput() {
    if (app.query && app.view !== "library") app.setView("library");
  }

  onMount(() => {
    const onKey = (e: KeyboardEvent) => {
      const typing = document.activeElement === searchEl;
      if ((e.key === "/" && !typing) || (e.key.toLowerCase() === "f" && (e.ctrlKey || e.metaKey))) {
        e.preventDefault();
        searchEl?.focus();
        searchEl?.select();
      } else if (e.key === "Escape" && typing) {
        app.query = "";
        searchEl?.blur();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });
</script>

<header class="topbar">
  <div class="left">
    <div class="brand">
      <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
        <rect x="3" y="3" width="8" height="8" rx="2" fill="var(--accent)" />
        <rect x="13" y="3" width="8" height="8" rx="2" fill="currentColor" opacity="0.35" />
        <rect x="3" y="13" width="8" height="8" rx="2" fill="currentColor" opacity="0.35" />
        <rect x="13" y="13" width="8" height="8" rx="2" fill="currentColor" opacity="0.6" />
      </svg>
      <span>Cabinet</span>
    </div>
    <nav class="tabs" aria-label="Sections">
      {#each TABS as tab (tab.id)}
        <button
          class:active={app.view === tab.id}
          aria-current={app.view === tab.id ? "page" : undefined}
          onclick={() => app.setView(tab.id)}
        >
          {tab.label}
          {#if tab.id === "library" && app.games.length}<span class="count">{app.games.length}</span>{/if}
        </button>
      {/each}
    </nav>
  </div>

  <label class="search">
    <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
      <circle cx="11" cy="11" r="6.5" fill="none" stroke="currentColor" stroke-width="2" />
      <path d="m16 16 4.5 4.5" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
    </svg>
    <input
      bind:this={searchEl}
      bind:value={app.query}
      oninput={onSearchInput}
      type="search"
      placeholder="Search {app.games.length} games"
      spellcheck="false"
      autocomplete="off"
    />
    <kbd>/</kbd>
  </label>

  <div class="actions">
    <button
      class="icon-btn"
      class:spinning={app.scanning}
      onclick={() => app.rescan(true)}
      disabled={app.scanning}
      title="Rescan libraries"
      aria-label="Rescan libraries"
    >
      <svg viewBox="0 0 24 24" width="17" height="17" aria-hidden="true">
        <path
          d="M20 11a8 8 0 0 0-14.6-4.5M4 13a8 8 0 0 0 14.6 4.5M5 3v4h4M19 21v-4h-4"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>
    </button>
    <ThemeMenu />
    <button
      class="icon-btn"
      class:active={app.view === "settings"}
      onclick={() => app.setView("settings")}
      title="Settings and accounts"
      aria-label="Settings"
    >
      <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
        <path
          d="M12 15.2a3.2 3.2 0 1 0 0-6.4 3.2 3.2 0 0 0 0 6.4Zm7.4-2.1.2-1.1-.2-1.1 2-1.6-2-3.4-2.4.9a7.6 7.6 0 0 0-1.9-1.1L14.7 3h-4l-.4 2.6a7.6 7.6 0 0 0-1.9 1.1L6 5.8 4 9.2l2 1.6-.1 1.2.1 1.1-2 1.6 2 3.4 2.4-.9c.6.5 1.2.8 1.9 1.1l.4 2.6h4l.4-2.6c.7-.3 1.3-.6 1.9-1.1l2.4.9 2-3.4z"
          fill="none"
          stroke="currentColor"
          stroke-width="1.7"
          stroke-linejoin="round"
        />
      </svg>
    </button>
  </div>
</header>

<style>
  .topbar {
    position: relative;
    z-index: 10;
    display: grid;
    grid-template-columns: 1fr minmax(200px, 460px) 1fr;
    align-items: center;
    gap: 16px;
    padding: 10px 24px;
    background: var(--bg-elevated);
    backdrop-filter: blur(16px) saturate(140%);
    border-bottom: 1px solid var(--border);
  }
  .left {
    display: flex;
    align-items: center;
    gap: 22px;
    min-width: 0;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 9px;
    font-weight: 700;
    font-size: 1.05rem;
    letter-spacing: -0.01em;
  }

  .tabs {
    display: flex;
    gap: 2px;
  }
  .tabs button {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 36px;
    padding: 0 12px;
    border: 0;
    border-radius: var(--radius-control);
    background: none;
    color: var(--text-muted);
    font-weight: 550;
    transition:
      color 120ms ease,
      background 120ms ease;
  }
  .tabs button:hover {
    color: var(--text);
    background: var(--surface-hover);
  }
  .tabs button.active {
    color: var(--text);
  }
  .tabs button.active::after {
    content: "";
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: -11px;
    height: 2px;
    border-radius: 2px;
    background: var(--accent);
  }
  .count {
    font-size: 0.72rem;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--surface);
    border: 1px solid var(--border);
    color: var(--text-faint);
    font-variant-numeric: tabular-nums;
  }

  .search {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 38px;
    padding: 0 10px 0 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    color: var(--text-faint);
    transition:
      border-color 120ms ease,
      box-shadow 120ms ease;
  }
  .search:focus-within {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: 0;
    background: none;
    color: var(--text);
  }
  .search input::placeholder {
    color: var(--text-faint);
  }
  .search input::-webkit-search-cancel-button {
    display: none;
  }
  kbd {
    font-family: inherit;
    font-size: 0.72rem;
    padding: 1px 6px;
    border-radius: 5px;
    border: 1px solid var(--border-strong);
    color: var(--text-faint);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 8px;
  }
  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    border: 1px solid var(--border);
    background: var(--surface);
    border-radius: var(--radius-control);
    color: var(--text-muted);
    transition:
      background 120ms ease,
      color 120ms ease;
  }
  .icon-btn:hover:not(:disabled) {
    background: var(--surface-hover);
    color: var(--text);
  }
  .icon-btn.active {
    color: var(--accent);
    border-color: var(--accent);
  }
  .icon-btn:disabled {
    cursor: default;
  }
  .spinning svg {
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 860px) {
    .brand span {
      display: none;
    }
  }
</style>
