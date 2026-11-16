<!--
  Horizontal bar ranking of total playtime. One series, so no legend: the
  section title names it. Thin bars grow from a shared baseline with a rounded
  data end and the value printed at the tip; hover or focus a row for detail.
-->
<script lang="ts">
  import type { LibraryGame } from "$lib/api";
  import { app } from "$lib/app.svelte";
  import { formatDate, formatHours, formatPlaytime } from "$lib/format";
  import Thumb from "./Thumb.svelte";

  let { games, total }: { games: LibraryGame[]; total: number } = $props();

  const max = $derived(Math.max(1, ...games.map((g) => g.playtimeSecs)));

  let active = $state<{ game: LibraryGame; top: number } | null>(null);
  let chartEl: HTMLDivElement | undefined = $state();

  function show(game: LibraryGame, row: HTMLElement) {
    if (!chartEl) return;
    const top = row.getBoundingClientRect().bottom - chartEl.getBoundingClientRect().top + 6;
    active = { game, top };
  }
</script>

<div class="chart" bind:this={chartEl} role="list" onmouseleave={() => (active = null)}>
  {#each games as game, i (game.id)}
    {@const pct = game.playtimeSecs / max}
    <!-- Rows are focusable so keyboard users can reach the detail tooltip. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div
      class="row"
      class:dim={active && active.game.id !== game.id}
      role="listitem"
      tabindex="0"
      aria-label={`${i + 1}. ${game.title}, ${formatPlaytime(game.playtimeSecs)}`}
      onmouseenter={(e) => show(game, e.currentTarget)}
      onfocus={(e) => show(game, e.currentTarget)}
      onblur={() => (active = null)}
    >
      <span class="rank">{i + 1}</span>
      <Thumb {game} width={26} />
      <span class="name">{game.title}</span>
      <span class="plot">
        <span class="bar" style:--pct={pct}></span>
        <span class="value">{formatHours(game.playtimeSecs)}</span>
      </span>
    </div>
  {/each}

  {#if active}
    {@const g = active.game}
    <div class="tooltip" style:top={`${active.top}px`} role="tooltip">
      <div class="tt-title">{g.title}</div>
      <dl>
        <dt>Total</dt>
        <dd>{formatPlaytime(g.playtimeSecs)}</dd>
        <dt>Share</dt>
        <dd>{total ? Math.round((g.playtimeSecs / total) * 100) : 0}% of all playtime</dd>
        {#if g.storePlaytimeSecs > 0}
          <dt>From {app.sourceName(g.source)}</dt>
          <dd>{formatPlaytime(g.storePlaytimeSecs)}</dd>
        {/if}
        {#if g.recentSecs > 0}
          <dt>Last 2 weeks</dt>
          <dd>{formatPlaytime(g.recentSecs)}</dd>
        {/if}
        {#if g.lastPlayed}
          <dt>Last played</dt>
          <dd>{formatDate(g.lastPlayed)}</dd>
        {/if}
      </dl>
    </div>
  {/if}
</div>

<style>
  .chart {
    position: relative;
    display: flex;
    flex-direction: column;
  }

  .row {
    display: grid;
    grid-template-columns: 18px auto minmax(90px, 0.9fr) minmax(120px, 1.6fr);
    align-items: center;
    gap: 10px;
    padding: 5px 8px;
    margin: 0 -8px;
    border-radius: 8px;
    transition:
      background 120ms ease,
      opacity 120ms ease;
    outline: none;
  }
  .row:hover,
  .row:focus-visible {
    background: var(--surface-hover);
  }
  .row:focus-visible {
    box-shadow: inset 0 0 0 2px var(--accent);
  }
  .row.dim {
    opacity: 0.55;
  }

  .rank {
    font-size: 0.78rem;
    color: var(--text-faint);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .name {
    font-weight: 550;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* The plot area: a hairline baseline on the left, bars grow rightwards and
     leave room at the end for the value label. */
  .plot {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 100%;
    border-left: 1px solid var(--border-strong);
    min-width: 0;
  }
  .bar {
    height: 10px;
    width: calc((100% - 56px) * var(--pct));
    min-width: 2px;
    background: var(--chart-bar);
    border-radius: 0 4px 4px 0;
  }
  .value {
    flex: none;
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .tooltip {
    position: absolute;
    right: 0;
    z-index: 5;
    min-width: 220px;
    padding: 10px 12px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: 10px;
    box-shadow: var(--shadow-card-hover);
    pointer-events: none;
  }
  .tt-title {
    font-weight: 650;
    margin-bottom: 6px;
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 3px 14px;
    margin: 0;
    font-size: 0.82rem;
  }
  dt {
    color: var(--text-faint);
  }
  dd {
    margin: 0;
    color: var(--text);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
