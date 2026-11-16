<script lang="ts">
  import type { LibraryGame } from "$lib/api";
  import { app, type SortKey } from "$lib/app.svelte";
  import GameCard from "$lib/components/GameCard.svelte";
  import StoreIcon from "$lib/components/StoreIcon.svelte";

  const SORTS: { key: SortKey; label: string }[] = [
    { key: "title", label: "Title" },
    { key: "recent", label: "Recently played" },
    { key: "playtime", label: "Most played" },
    { key: "added", label: "Recently added" },
  ];
  // Games in scope: installed only, or everything owned.
  const pool = $derived(app.showUninstalled ? app.games : app.games.filter((g) => g.installed));
  const ownedExtra = $derived(app.games.filter((g) => !g.installed).length);

  // One filter per store that actually has games in scope.
  const filters = $derived([
    { key: "all", name: "All", count: pool.length },
    ...app.sources
      .map((s) => ({ ...s, count: pool.filter((g) => g.source === s.key).length }))
      .filter((s) => s.count > 0),
  ]);
  $effect(() => {
    if (!filters.some((f) => f.key === app.filter)) app.filter = "all";
  });

  const visible = $derived.by(() => {
    const q = app.query.trim().toLowerCase();
    const list = pool.filter(
      (g) => (app.filter === "all" || g.source === app.filter) && (!q || g.title.toLowerCase().includes(q)),
    );
    const byTitle = (a: LibraryGame, b: LibraryGame) =>
      a.title.localeCompare(b.title, undefined, { sensitivity: "base", numeric: true });
    const cmp: Record<SortKey, (a: LibraryGame, b: LibraryGame) => number> = {
      title: byTitle,
      recent: (a, b) => (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0) || byTitle(a, b),
      playtime: (a, b) => b.playtimeSecs - a.playtimeSecs || byTitle(a, b),
      added: (a, b) => b.addedAt - a.addedAt || byTitle(a, b),
    };
    return list.sort(cmp[app.sort]);
  });

  // "Recently played" / "Most played" sorts show the matching detail line.
  const detail = $derived(app.sort === "recent" ? "recent" : "playtime");
</script>

<div class="library" style:--card-min-width={`${app.cardSize}px`}>
  <div class="toolbar">
    <div class="segmented" role="tablist" aria-label="Store">
      {#each filters as f (f.key)}
        <button
          role="tab"
          aria-selected={app.filter === f.key}
          class:active={app.filter === f.key}
          onclick={() => (app.filter = f.key)}
          title={f.name}
        >
          {#if f.key !== "all"}<StoreIcon source={f.key} size={14} />{/if}
          <span class="fname">{f.key === "all" ? "All" : f.name}</span>
          <span class="count">{f.count}</span>
        </button>
      {/each}
    </div>

    <div class="toolbar-right">
      {#if ownedExtra > 0}
        <div class="segmented small" role="tablist" aria-label="Installed or owned">
          <button role="tab" aria-selected={!app.showUninstalled} class:active={!app.showUninstalled} onclick={() => app.setShowUninstalled(false)}>Installed</button>
          <button role="tab" aria-selected={app.showUninstalled} class:active={app.showUninstalled} onclick={() => app.setShowUninstalled(true)}>
            All owned <span class="count">+{ownedExtra}</span>
          </button>
        </div>
      {/if}
      <label class="size" title="Cover size">
        <svg viewBox="0 0 24 24" width="14" height="14" aria-hidden="true">
          <rect x="4" y="4" width="7" height="7" rx="1.5" fill="currentColor" />
          <rect x="13" y="4" width="7" height="7" rx="1.5" fill="currentColor" />
          <rect x="4" y="13" width="7" height="7" rx="1.5" fill="currentColor" />
          <rect x="13" y="13" width="7" height="7" rx="1.5" fill="currentColor" />
        </svg>
        <input
          type="range"
          min="120"
          max="280"
          step="4"
          bind:value={app.cardSize}
          oninput={() => app.saveCardSize()}
          aria-label="Cover size"
        />
      </label>
      <label class="sort">
        <span>Sort</span>
        <select bind:value={app.sort}>
          {#each SORTS as s (s.key)}
            <option value={s.key}>{s.label}</option>
          {/each}
        </select>
      </label>
    </div>
  </div>

  {#if !app.loaded}
    <div class="grid">
      {#each Array(12) as _, i (i)}
        <div class="skeleton"></div>
      {/each}
    </div>
  {:else if visible.length === 0}
    <div class="empty">
      {#if app.games.length === 0}
        <h2>No games found</h2>
        <p>Cabinet looks for installed Steam and Epic Games Store titles. Install a game, then rescan.</p>
        <button class="primary" onclick={() => app.rescan(true)}>Rescan</button>
      {:else}
        <h2>No matches</h2>
        <p>Nothing matches “{app.query}”{app.filter !== "all" ? ` in ${app.sourceName(app.filter)}` : ""}.</p>
      {/if}
    </div>
  {:else}
    <div class="grid">
      {#each visible as game (game.id)}
        <GameCard {game} {detail} />
      {/each}
    </div>
  {/if}
</div>

<style>
  .library {
    padding: 22px 24px 48px;
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 20px;
  }
  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .segmented {
    display: inline-flex;
    padding: 3px;
    gap: 2px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
  }
  .segmented button {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 5px 12px;
    border: 0;
    border-radius: calc(var(--radius-control) - 3px);
    background: none;
    color: var(--text-muted);
    font-weight: 500;
  }
  .segmented button:hover {
    color: var(--text);
  }
  .segmented button.active {
    background: var(--accent-soft);
    color: var(--text);
  }
  .count {
    font-size: 0.75rem;
    color: var(--text-faint);
    font-variant-numeric: tabular-nums;
  }
  .segmented button.active .count {
    color: var(--accent);
  }
  .segmented.small button {
    padding: 4px 10px;
    font-size: 0.85rem;
  }
  @media (max-width: 1000px) {
    .fname {
      display: none;
    }
    .segmented button:first-child .fname {
      display: inline;
    }
  }

  .size {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-faint);
  }
  .size input {
    width: 96px;
    accent-color: var(--accent);
  }
  .sort {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-faint);
    font-size: 0.85rem;
  }
  .sort select {
    height: 32px;
    padding: 0 8px;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--surface);
    color: var(--text);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--card-min-width), 1fr));
    gap: 26px 20px;
  }

  .skeleton {
    aspect-ratio: 2 / 3;
    border-radius: var(--radius-card);
    background: linear-gradient(100deg, var(--surface) 40%, var(--surface-hover) 50%, var(--surface) 60%);
    background-size: 200% 100%;
    animation: shimmer 1.4s linear infinite;
  }
  @keyframes shimmer {
    from {
      background-position: 100% 0;
    }
    to {
      background-position: -100% 0;
    }
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 80px 20px;
    text-align: center;
    color: var(--text-muted);
  }
  .empty h2 {
    margin: 0 0 6px;
    color: var(--text);
    font-size: 1.15rem;
  }
  .empty p {
    margin: 0 0 18px;
    max-width: 420px;
  }
  .primary {
    padding: 8px 18px;
    border: 0;
    border-radius: var(--radius-control);
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
  }
  .primary:hover {
    background: var(--accent-hover);
  }
</style>
