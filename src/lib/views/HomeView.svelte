<script lang="ts">
  import { api, type LibraryGame, type Session } from "$lib/api";
  import { app } from "$lib/app.svelte";
  import { formatHours, formatPlaytime, formatRelative } from "$lib/format";
  import ActivityHeatmap from "$lib/components/ActivityHeatmap.svelte";
  import GameCard from "$lib/components/GameCard.svelte";
  import HeroBanner from "$lib/components/HeroBanner.svelte";
  import MostPlayedChart from "$lib/components/MostPlayedChart.svelte";
  import StoreIcon from "$lib/components/StoreIcon.svelte";
  import Thumb from "$lib/components/Thumb.svelte";

  const DAY = 86400;
  const now = () => Date.now() / 1000;

  const installed = $derived(app.games.filter((g) => g.installed));
  const recent = $derived(
    app.games.filter((g) => g.lastPlayed).sort((a, b) => (b.lastPlayed ?? 0) - (a.lastPlayed ?? 0)),
  );
  const played = $derived(app.games.filter((g) => g.playtimeSecs > 0));
  const mostPlayed = $derived([...played].sort((a, b) => b.playtimeSecs - a.playtimeSecs).slice(0, 8));
  const neverPlayed = $derived(installed.filter((g) => g.playtimeSecs === 0 && !g.lastPlayed));
  // Installed games with real time in them that haven't been touched for a
  // couple of months, skipping the ones the "Most played" chart already shows.
  const pickBackUp = $derived(
    played
      .filter((g) => g.installed && !mostPlayed.includes(g))
      .filter((g) => g.playtimeSecs >= 2 * 3600 && (g.lastPlayed ?? 0) < now() - 60 * DAY)
      .sort((a, b) => b.playtimeSecs - a.playtimeSecs)
      .slice(0, 5),
  );

  const totals = $derived.by(() => {
    const total = app.games.reduce((s, g) => s + g.playtimeSecs, 0);
    const store = app.games.reduce((s, g) => s + g.storePlaytimeSecs, 0);
    const recentSecs = app.games.reduce((s, g) => s + g.recentSecs, 0);
    const recentGames = app.games.filter((g) => g.recentSecs > 0).length;
    const bySource = app.sources
      .map((s) => ({ ...s, count: installed.filter((g) => g.source === s.key).length }))
      .filter((s) => s.count > 0);
    return { total, store, recentSecs, recentGames, bySource, owned: app.games.length - installed.length };
  });

  // The banner features whatever is running, else the most recent game; the
  // "also played" list beside it can swap the featured game.
  let picked = $state<string | null>(null);
  const featured = $derived(
    app.games.find((g) => g.running) ?? app.games.find((g) => g.id === picked) ?? recent[0] ?? mostPlayed[0] ?? null,
  );
  const alsoRecent = $derived(recent.filter((g) => g.id !== featured?.id).slice(0, 4));
  const launchingFeatured = $derived(featured ? app.launching.has(featured.id) : false);

  // Sessions for the activity calendar; reloaded when any session ends.
  let sessions = $state<Session[]>([]);
  const sessionTotal = $derived(app.games.reduce((s, g) => s + g.sessionCount, 0));
  $effect(() => {
    void sessionTotal;
    api
      .sessions(null, Math.floor(now()) - 371 * DAY)
      .then((s) => (sessions = s))
      .catch(console.error);
  });
  const titles = $derived(new Map(app.games.map((g) => [g.id, g.title])));

  const eyebrow = (g: LibraryGame) =>
    g.running ? "Playing now" : g.lastPlayed ? "Continue playing" : "Your most played";
</script>

<div class="home">
  {#if featured}
    <HeroBanner game={featured} eyebrow={eyebrow(featured)} live={featured.running}>
      <div class="hero-meta">
        <span class="store"><StoreIcon source={featured.source} size={14} />{app.sourceName(featured.source)}</span>
        {#if featured.lastPlayed}<span>Last played {formatRelative(featured.lastPlayed)}</span>{/if}
        {#if featured.playtimeSecs > 0}<span>{formatPlaytime(featured.playtimeSecs)} played</span>{/if}
        {#if featured.recentSecs > 0}<span>{formatPlaytime(featured.recentSecs)} in the last 2 weeks</span>{/if}
      </div>
      <div class="hero-actions">
        <button
          class="btn btn-primary btn-large"
          disabled={featured.running || launchingFeatured}
          onclick={() => featured && app.launch(featured)}
        >
          {#if featured.running}
            Running
          {:else if launchingFeatured}
            Starting…
          {:else if featured.installed}
            <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
              <path d="M8 5.5v13l10.5-6.5z" fill="currentColor" />
            </svg>
            Play
          {:else}
            Install
          {/if}
        </button>
        <button class="btn btn-glass" onclick={() => featured && app.openGame(featured.id)}>Details</button>
      </div>

      {#snippet aside()}
        {#if alsoRecent.length}
          <aside class="also">
            <span class="also-title">Also recently played</span>
            {#each alsoRecent as game (game.id)}
              <button class="also-item" onclick={() => (picked = game.id)} title={`Show ${game.title}`}>
                <Thumb {game} width={30} />
                <span class="also-text">
                  <span class="also-name">{game.title}</span>
                  <span class="also-when">{game.lastPlayed ? formatRelative(game.lastPlayed) : ""}</span>
                </span>
              </button>
            {/each}
          </aside>
        {/if}
      {/snippet}
    </HeroBanner>
  {/if}

  <section class="stats" aria-label="Overview">
    <div class="tile primary-tile">
      <span class="label">Total playtime</span>
      <span class="hero-figure">{formatHours(totals.total)}</span>
      <span class="sub">
        across {played.length} game{played.length === 1 ? "" : "s"}
        {#if totals.store > 0}· {formatHours(totals.store)} imported from stores{/if}
      </span>
    </div>
    <div class="tile">
      <span class="label">Last 2 weeks</span>
      <span class="figure">{totals.recentSecs > 0 ? formatPlaytime(totals.recentSecs) : "—"}</span>
      <span class="sub">
        {totals.recentGames
          ? `${totals.recentGames} game${totals.recentGames === 1 ? "" : "s"} played`
          : "Nothing played lately"}
      </span>
    </div>
    <div class="tile">
      <span class="label">Installed</span>
      <span class="figure">{installed.length}</span>
      <span class="sub sources">
        {#each totals.bySource as s (s.key)}
          <span class="src"><StoreIcon source={s.key} size={12} title={s.name} />{s.count}</span>
        {/each}
        {#if totals.owned > 0}<span>· {totals.owned} more owned</span>{/if}
      </span>
    </div>
    <div class="tile">
      <span class="label">Never played</span>
      <span class="figure">{neverPlayed.length}</span>
      <span class="sub">{neverPlayed.length ? "installed and waiting" : "You've tried everything"}</span>
    </div>
  </section>

  {#if recent.length}
    <section>
      <header class="section-head">
        <h2>Recently played</h2>
        <button class="link" onclick={() => app.openLibrary("recent")}>See all</button>
      </header>
      <div class="shelf">
        {#each recent.slice(0, 12) as game (game.id)}
          <div class="shelf-item"><GameCard {game} detail="recent" /></div>
        {/each}
      </div>
    </section>
  {/if}

  <section class="panel">
    <header class="section-head">
      <div>
        <h2>Activity</h2>
        <p class="caption">Daily playtime across all games</p>
      </div>
    </header>
    <ActivityHeatmap {sessions} {titles} />
  </section>

  <div class="split">
    <section class="panel">
      <header class="section-head">
        <div>
          <h2>Most played</h2>
          <p class="caption">Top {mostPlayed.length} by total playtime</p>
        </div>
        <button class="link" onclick={() => app.openLibrary("playtime")}>See all</button>
      </header>
      {#if mostPlayed.length}
        <MostPlayedChart games={mostPlayed} total={totals.total} />
      {:else}
        <p class="empty">Play something and your top games will show up here.</p>
      {/if}
    </section>

    <section class="panel">
      <header class="section-head">
        <div>
          <h2>Pick back up</h2>
          <p class="caption">Games you've sunk time into but haven't opened in 2+ months</p>
        </div>
      </header>
      {#if pickBackUp.length}
        <ul class="list">
          {#each pickBackUp as game (game.id)}
            <li>
              <button class="list-open" onclick={() => app.openGame(game.id)}>
                <Thumb {game} width={34} />
                <span class="list-text">
                  <span class="list-name">{game.title}</span>
                  <span class="list-sub">
                    {formatPlaytime(game.playtimeSecs)}
                    · {game.lastPlayed ? `last played ${formatRelative(game.lastPlayed)}` : "never launched here"}
                  </span>
                </span>
              </button>
              <button class="mini-play" onclick={() => app.launch(game)} aria-label={`Play ${game.title}`}>
                <svg viewBox="0 0 24 24" width="15" height="15" aria-hidden="true">
                  <path d="M8 5.5v13l10.5-6.5z" fill="currentColor" />
                </svg>
              </button>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="empty">Nothing gathering dust right now.</p>
      {/if}
    </section>
  </div>

  {#if neverPlayed.length}
    <section>
      <header class="section-head">
        <div>
          <h2>Never played</h2>
          <p class="caption">Installed, waiting for a first launch</p>
        </div>
      </header>
      <div class="shelf">
        {#each neverPlayed as game (game.id)}
          <div class="shelf-item"><GameCard {game} /></div>
        {/each}
      </div>
    </section>
  {/if}
</div>

<style>
  .home {
    display: flex;
    flex-direction: column;
    gap: 34px;
    padding: 22px 24px 56px;
    max-width: 1600px;
    margin: 0 auto;
  }

  .hero-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 14px;
    color: var(--text-muted);
    font-size: 0.9rem;
  }
  .hero-meta > span + span::before {
    content: "·";
    margin-right: 14px;
    color: var(--text-faint);
  }
  .store {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    color: var(--text);
    font-weight: 600;
  }
  .hero-actions {
    display: flex;
    gap: 10px;
    margin-top: 4px;
  }

  .also {
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 250px;
    flex: none;
    padding: 12px;
    border-radius: var(--radius-card);
    background: var(--hero-panel);
    backdrop-filter: blur(14px);
    border: 1px solid rgba(255, 255, 255, 0.08);
  }
  .also-title {
    padding: 0 6px 4px;
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--text-faint);
  }
  .also-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px;
    border: 0;
    border-radius: 8px;
    background: none;
    text-align: left;
    min-width: 0;
  }
  .also-item:hover {
    background: var(--surface-hover);
  }
  .also-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .also-name {
    font-weight: 600;
    font-size: 0.88rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .also-when {
    font-size: 0.76rem;
    color: var(--text-muted);
  }

  /* ---- Stat tiles ---- */
  .stats {
    display: grid;
    grid-template-columns: 1.6fr repeat(3, 1fr);
    gap: 14px;
  }
  .tile {
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    gap: 4px;
    padding: 18px 20px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    min-width: 0;
  }
  .label {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-muted);
  }
  .hero-figure {
    font-size: 3.2rem;
    font-weight: 700;
    letter-spacing: -0.03em;
    line-height: 1.05;
  }
  .figure {
    font-size: 1.9rem;
    font-weight: 650;
    letter-spacing: -0.02em;
    line-height: 1.15;
  }
  .sub {
    font-size: 0.82rem;
    color: var(--text-muted);
  }
  .sources {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 2px 12px;
  }
  .src {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  /* ---- Sections ---- */
  .section-head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 14px;
  }
  h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 650;
    letter-spacing: -0.01em;
  }
  .caption {
    margin: 2px 0 0;
    font-size: 0.82rem;
    color: var(--text-faint);
  }
  .link {
    padding: 4px 8px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--text-muted);
    font-weight: 550;
    font-size: 0.85rem;
  }
  .link:hover {
    color: var(--text);
    background: var(--surface-hover);
  }

  .shelf {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 152px;
    gap: 18px;
    overflow-x: auto;
    padding: 6px 2px 12px;
    margin: -6px -2px 0;
    scroll-snap-type: x proximity;
  }
  .shelf-item {
    scroll-snap-align: start;
    min-width: 0;
  }
  .shelf::-webkit-scrollbar {
    height: 10px;
  }

  .split {
    display: grid;
    grid-template-columns: 1.35fr 1fr;
    gap: 18px;
    align-items: start;
  }
  .panel {
    padding: 18px 20px 16px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    min-width: 0;
  }
  .empty {
    margin: 8px 0;
    color: var(--text-muted);
  }

  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .list li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 8px;
    margin: 0 -8px;
    border-radius: 8px;
  }
  .list li:hover {
    background: var(--surface-hover);
  }
  .list-open {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
    padding: 0;
    border: 0;
    background: none;
    text-align: left;
  }
  .list-text {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .list-name {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .list-sub {
    font-size: 0.8rem;
    color: var(--text-muted);
  }
  .mini-play {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    flex: none;
    border: 0;
    border-radius: 50%;
    background: var(--accent-soft);
    color: var(--accent);
    opacity: 0.8;
    transition:
      background 120ms ease,
      color 120ms ease,
      opacity 120ms ease;
  }
  .list li:hover .mini-play,
  .mini-play:focus-visible {
    opacity: 1;
    background: var(--accent);
    color: var(--accent-text);
  }

  @media (max-width: 1100px) {
    .stats {
      grid-template-columns: repeat(3, 1fr);
    }
    .primary-tile {
      grid-column: 1 / -1;
    }
    .split {
      grid-template-columns: 1fr;
    }
  }
  @media (max-width: 860px) {
    .also {
      display: none;
    }
  }
</style>
