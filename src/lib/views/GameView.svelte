<script lang="ts">
  import { api, type GameDetails, type Session } from "$lib/api";
  import { app } from "$lib/app.svelte";
  import {
    formatBytes,
    formatDate,
    formatHours,
    formatPlaytime,
    formatRelative,
  } from "$lib/format";
  import ActivityHeatmap from "$lib/components/ActivityHeatmap.svelte";
  import HeroBanner from "$lib/components/HeroBanner.svelte";
  import StoreIcon from "$lib/components/StoreIcon.svelte";

  const YEAR = 366 * 86400;

  let details = $state<GameDetails | null>(null);
  let sessions = $state<Session[]>([]);
  let error = $state<string | null>(null);

  // Live fields (running, playtime) come from the shared library; the rest
  // from the details call.
  const game = $derived(app.games.find((g) => g.id === app.gameId) ?? details?.game ?? null);

  async function load(id: string) {
    error = null;
    try {
      const [d, s] = await Promise.all([api.details(id), api.sessions(id, Math.floor(Date.now() / 1000) - YEAR)]);
      if (app.gameId === id) {
        details = d;
        sessions = s;
      }
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => {
    if (app.gameId) load(app.gameId);
  });

  // Refresh the session list when a tracked session ends.
  let lastCount = -1;
  $effect(() => {
    const count = game?.sessionCount ?? -1;
    if (lastCount !== -1 && count !== lastCount && app.gameId) load(app.gameId);
    lastCount = count;
  });

  const recentSessions = $derived([...sessions].reverse().slice(0, 12));
  const tracked = $derived(sessions.reduce((s, x) => s + x.durationSecs, 0));
  const avgSession = $derived(sessions.length ? tracked / sessions.length : 0);
  const launching = $derived(game ? app.launching.has(game.id) : false);
  const titles = $derived(new Map(game ? [[game.id, game.title]] : []));

  const timeFmt = (t: number) =>
    new Date(t * 1000).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });

  function openFolder() {
    if (game) api.openInstallDir(game.id).catch((e) => app.toast(String(e), "error"));
  }
  function openStore() {
    if (game) api.openStorePage(game.id).catch((e) => app.toast(String(e), "error"));
  }
</script>

{#if game}
  <div class="page">
    <HeroBanner
      {game}
      tall
      eyebrow={game.running ? "Playing now" : game.installed ? undefined : "Not installed"}
      live={game.running}
    >
      {#snippet top()}
        <button class="btn btn-glass back" onclick={() => app.back()}>
          <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
            <path d="M15 5l-7 7 7 7" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
          Back
        </button>
      {/snippet}

      <div class="meta">
        <span class="store">
          <StoreIcon source={game.source} size={15} />
          {details?.sourceName ?? app.sourceName(game.source)}
        </span>
        {#if game.lastPlayed}<span>Last played {formatRelative(game.lastPlayed)}</span>{/if}
        {#if details?.sizeBytes}<span>{formatBytes(details.sizeBytes)}</span>{/if}
      </div>

      <div class="actions">
        <button
          class="btn btn-primary btn-large"
          disabled={game.running || launching}
          onclick={() => game && app.launch(game)}
        >
          {#if game.running}
            Running
          {:else if launching}
            Starting…
          {:else if game.installed}
            <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true"><path d="M8 5.5v13l10.5-6.5z" fill="currentColor" /></svg>
            Play
          {:else}
            <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
              <path d="M12 4v11m0 0-4.5-4.5M12 15l4.5-4.5M5 19.5h14" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
            Install
          {/if}
        </button>
        {#if details?.installDir}
          <button class="btn btn-glass" onclick={openFolder} title={details.installDir}>
            <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
              <path d="M3.5 7.5a2 2 0 0 1 2-2h4l2 2.5h7a2 2 0 0 1 2 2v7.5a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linejoin="round" />
            </svg>
            Open folder
          </button>
        {/if}
        {#if details?.storePage}
          <button class="btn btn-glass" onclick={openStore}>
            <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
              <path d="M14 4h6v6M20 4l-9 9M18 14v4.5a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 4 18.5v-11A1.5 1.5 0 0 1 5.5 6H10" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
            Store page
          </button>
        {/if}
      </div>
    </HeroBanner>

    {#if error}<p class="error">{error}</p>{/if}

    <section class="stats" aria-label="Playtime">
      <div class="tile primary-tile">
        <span class="label">Total playtime</span>
        <span class="hero-figure">{game.playtimeSecs > 0 ? formatHours(game.playtimeSecs) : "—"}</span>
        <span class="sub">
          {#if game.storePlaytimeSecs > 0}
            {formatPlaytime(game.storePlaytimeSecs)} from {details?.sourceName ?? "the store"}
            {#if game.playtimeSecs > game.storePlaytimeSecs}+ {formatPlaytime(game.playtimeSecs - game.storePlaytimeSecs)} tracked by Cabinet{/if}
          {:else if game.playtimeSecs > 0}
            all tracked by Cabinet
          {:else}
            Not played yet
          {/if}
        </span>
      </div>
      <div class="tile">
        <span class="label">Last 2 weeks</span>
        <span class="figure">{game.recentSecs > 0 ? formatPlaytime(game.recentSecs) : "—"}</span>
      </div>
      <div class="tile">
        <span class="label">Sessions in Cabinet</span>
        <span class="figure">{game.sessionCount}</span>
        <span class="sub">{game.launchCount} launch{game.launchCount === 1 ? "" : "es"}</span>
      </div>
      <div class="tile">
        <span class="label">Average session</span>
        <span class="figure">{avgSession > 0 ? formatPlaytime(avgSession) : "—"}</span>
      </div>
    </section>

    <section class="panel">
      <header class="section-head"><h2>Activity</h2></header>
      <ActivityHeatmap {sessions} {titles} />
    </section>

    <div class="split">
      <section class="panel">
        <header class="section-head"><h2>Recent sessions</h2></header>
        {#if recentSessions.length}
          <table class="sessions">
            <thead><tr><th>Date</th><th>Time</th><th class="right">Length</th></tr></thead>
            <tbody>
              {#each recentSessions as s (s.startedAt)}
                <tr>
                  <td>{formatDate(s.startedAt)}</td>
                  <td class="muted">{timeFmt(s.startedAt)} – {timeFmt(s.endedAt)}</td>
                  <td class="right">{formatPlaytime(s.durationSecs)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {:else}
          <p class="empty">No sessions tracked yet. Launch the game from Cabinet and they'll appear here.</p>
        {/if}
      </section>

      <section class="panel">
        <header class="section-head"><h2>Details</h2></header>
        <dl class="info">
          <dt>Store</dt>
          <dd><StoreIcon source={game.source} size={13} /> {details?.sourceName ?? game.source}</dd>
          <dt>Store ID</dt>
          <dd class="mono">{details?.sourceId ?? "—"}</dd>
          <dt>Status</dt>
          <dd>{game.installed ? "Installed" : "Owned, not installed"}</dd>
          {#if details?.installDir}
            <dt>Folder</dt>
            <dd class="mono path" title={details.installDir}>{details.installDir}</dd>
          {/if}
          {#if details?.sizeBytes}
            <dt>Size</dt>
            <dd>{formatBytes(details.sizeBytes)}</dd>
          {/if}
          <dt>In Cabinet since</dt>
          <dd>{formatDate(game.addedAt)}</dd>
        </dl>
      </section>
    </div>
  </div>
{:else}
  <div class="page"><p class="empty">Loading…</p></div>
{/if}

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 28px;
    padding: 22px 24px 56px;
    max-width: 1600px;
    margin: 0 auto;
  }

  .back {
    height: 34px;
    padding: 0 14px 0 10px;
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 14px;
    color: var(--text-muted);
    font-size: 0.9rem;
  }
  .meta > span + span::before {
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
  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 10px;
    margin-top: 4px;
  }

  .error {
    margin: 0;
    color: var(--danger);
  }

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

  .panel {
    padding: 18px 20px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    min-width: 0;
  }
  .section-head {
    margin-bottom: 14px;
  }
  h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 650;
  }
  .split {
    display: grid;
    grid-template-columns: 1.3fr 1fr;
    gap: 18px;
    align-items: start;
  }

  .sessions {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.88rem;
  }
  .sessions th {
    text-align: left;
    font-weight: 600;
    color: var(--text-faint);
    padding: 0 8px 8px;
    border-bottom: 1px solid var(--border);
  }
  .sessions td {
    padding: 8px;
    border-bottom: 1px solid var(--border);
    font-variant-numeric: tabular-nums;
  }
  .sessions tr:last-child td {
    border-bottom: 0;
  }
  .right {
    text-align: right;
  }
  .muted {
    color: var(--text-muted);
  }

  .info {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 10px 18px;
    margin: 0;
    font-size: 0.88rem;
  }
  .info dt {
    color: var(--text-faint);
  }
  .info dd {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    min-width: 0;
  }
  .mono {
    font-family: ui-monospace, "Cascadia Code", Consolas, monospace;
    font-size: 0.82rem;
  }
  .path {
    display: block !important;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .empty {
    margin: 0;
    color: var(--text-muted);
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
</style>
