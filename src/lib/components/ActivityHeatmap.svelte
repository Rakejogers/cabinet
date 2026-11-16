<!--
  GitHub-style calendar of daily playtime for the past year. One sequential
  hue (theme --heat-1..4) encodes how long was played each day; empty days use
  --heat-empty. Hover a day for detail, or switch to the list view.
-->
<script lang="ts">
  import type { Session } from "$lib/api";
  import { formatPlaytime } from "$lib/format";

  interface Props {
    sessions: Session[];
    /** Game titles by id, to name what was played each day. */
    titles?: Map<string, string>;
    weeks?: number;
  }
  let { sessions, titles, weeks = 53 }: Props = $props();

  const DAY_MS = 86_400_000;
  /** Upper bounds (seconds) for heat levels 1-3; anything above is level 4. */
  const LEVELS = [30 * 60, 90 * 60, 3 * 3600];
  const LEVEL_LABELS = ["None", "Under 30m", "30m–1.5h", "1.5–3h", "3h+"];

  type Day = { date: Date; key: string; secs: number; games: Map<string, number> };

  const keyOf = (d: Date) => `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`;
  const level = (secs: number) => (secs <= 0 ? 0 : LEVELS.findIndex((l) => secs < l) + 1 || 4);

  const grid = $derived.by(() => {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    // Columns are weeks starting on Sunday; the last column holds today.
    const start = new Date(today);
    start.setDate(start.getDate() - start.getDay() - (weeks - 1) * 7);

    const days = new Map<string, Day>();
    const columns: (Day | null)[][] = [];
    for (let w = 0; w < weeks; w++) {
      const col: (Day | null)[] = [];
      for (let d = 0; d < 7; d++) {
        const date = new Date(start);
        date.setDate(start.getDate() + w * 7 + d);
        if (date > today) {
          col.push(null);
          continue;
        }
        const day: Day = { date, key: keyOf(date), secs: 0, games: new Map() };
        days.set(day.key, day);
        col.push(day);
      }
      columns.push(col);
    }

    // Spread each session over the local days it covers.
    for (const s of sessions) {
      let cur = s.startedAt * 1000;
      const end = s.endedAt * 1000;
      while (cur < end) {
        const midnight = new Date(cur);
        midnight.setHours(24, 0, 0, 0);
        const segEnd = Math.min(end, midnight.getTime());
        const day = days.get(keyOf(new Date(cur)));
        if (day) {
          const secs = (segEnd - cur) / 1000;
          day.secs += secs;
          day.games.set(s.gameId, (day.games.get(s.gameId) ?? 0) + secs);
        }
        cur = segEnd;
      }
    }

    // Month labels sit above the first week that contains the 1st.
    const months: { col: number; label: string }[] = [];
    columns.forEach((col, i) => {
      const anchor = i === 0 ? col.find(Boolean) : col.find((d) => d?.date.getDate() === 1);
      if (anchor) months.push({ col: i, label: anchor.date.toLocaleDateString(undefined, { month: "short" }) });
    });
    // Drop a leading label that would collide with the next one.
    if (months.length > 1 && months[1].col - months[0].col < 3) months.shift();

    const ordered = [...days.values()];
    const active = ordered.filter((d) => d.secs > 0);
    const total = active.reduce((s, d) => s + d.secs, 0);
    let longest = 0;
    let run = 0;
    for (const d of ordered) {
      run = d.secs > 0 ? run + 1 : 0;
      longest = Math.max(longest, run);
    }
    // The current streak may continue today, so an empty today doesn't break it.
    let current = 0;
    for (let i = ordered.length - 1; i >= 0; i--) {
      if (ordered[i].secs > 0) current++;
      else if (i !== ordered.length - 1) break;
    }
    const busiest = active.reduce<Day | null>((b, d) => (d.secs > (b?.secs ?? 0) ? d : b), null);
    return { columns, months, active, total, longest, current, busiest };
  });

  const firstTracked = $derived(sessions.length ? new Date(sessions[0].startedAt * 1000) : null);

  let hovered = $state<{ day: Day; x: number; y: number } | null>(null);
  let wrapEl: HTMLDivElement | undefined = $state();
  let showList = $state(false);

  function hover(day: Day, el: HTMLElement) {
    if (!wrapEl) return;
    const cell = el.getBoundingClientRect();
    const box = wrapEl.getBoundingClientRect();
    hovered = { day, x: cell.left - box.left + cell.width / 2, y: cell.top - box.top };
  }

  const fmtDay = (d: Date) =>
    d.toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric", year: "numeric" });

  function topGames(day: Day) {
    return [...day.games.entries()]
      .sort((a, b) => b[1] - a[1])
      .slice(0, 3)
      .map(([id, secs]) => ({ title: titles?.get(id) ?? id, secs }));
  }
</script>

<div class="activity">
  <div class="summary">
    <div><span class="num">{formatPlaytime(grid.total)}</span><span class="cap">in the last year</span></div>
    <div><span class="num">{grid.active.length}</span><span class="cap">active days</span></div>
    <div><span class="num">{grid.current}</span><span class="cap">day streak</span></div>
    <div><span class="num">{grid.longest}</span><span class="cap">longest streak</span></div>
    <button class="toggle" onclick={() => (showList = !showList)} aria-pressed={showList}>
      {showList ? "Calendar" : "List"}
    </button>
  </div>

  {#if showList}
    {#if grid.active.length}
      <table class="list">
        <thead><tr><th>Day</th><th>Played</th><th>Games</th></tr></thead>
        <tbody>
          {#each [...grid.active].reverse() as day (day.key)}
            <tr>
              <td>{fmtDay(day.date)}</td>
              <td class="num-cell">{formatPlaytime(day.secs)}</td>
              <td>{topGames(day).map((g) => g.title).join(", ")}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else}
      <p class="empty">No tracked play in the last year yet.</p>
    {/if}
  {:else}
    <div class="scroll">
      <div
        class="calendar"
        bind:this={wrapEl}
        style:--weeks={weeks}
        role="img"
        aria-label={`Daily playtime for the last year: ${formatPlaytime(grid.total)} over ${grid.active.length} days`}
        onmouseleave={() => (hovered = null)}
      >
        <div class="months" aria-hidden="true">
          {#each grid.months as m (m.col)}
            <span style:grid-column={m.col + 1}>{m.label}</span>
          {/each}
        </div>
        <div class="weekdays" aria-hidden="true">
          <span style:grid-row="2">Mon</span>
          <span style:grid-row="4">Wed</span>
          <span style:grid-row="6">Fri</span>
        </div>
        <div class="cells">
          {#each grid.columns as col, w (w)}
            {#each col as day, d (d)}
              {#if day}
                <span
                  class="cell"
                  data-level={level(day.secs)}
                  style:grid-column={w + 1}
                  style:grid-row={d + 1}
                  onmouseenter={(e) => hover(day, e.currentTarget)}
                  role="presentation"
                ></span>
              {/if}
            {/each}
          {/each}
        </div>

        {#if hovered}
          {@const games = topGames(hovered.day)}
          <div class="tooltip" style:left={`${hovered.x}px`} style:top={`${hovered.y}px`} role="tooltip">
            <strong>{hovered.day.secs > 0 ? formatPlaytime(hovered.day.secs) : "No play"}</strong>
            <span class="tt-date">{fmtDay(hovered.day.date)}</span>
            {#each games as g (g.title)}
              <span class="tt-game"><span>{g.title}</span><span>{formatPlaytime(g.secs)}</span></span>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <div class="footer">
      <span class="note">
        {#if firstTracked}
          Tracked by Cabinet since {firstTracked.toLocaleDateString(undefined, { month: "long", day: "numeric", year: "numeric" })}.
        {:else}
          Cabinet starts filling this in as you play.
        {/if}
        Stores only report totals, so earlier play isn't shown day by day.
      </span>
      <span class="legend" aria-label="Legend">
        Less
        {#each [0, 1, 2, 3, 4] as l (l)}
          <span class="cell" data-level={l} title={LEVEL_LABELS[l]}></span>
        {/each}
        More
      </span>
    </div>
  {/if}
</div>

<style>
  .activity {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .summary {
    display: flex;
    align-items: flex-end;
    flex-wrap: wrap;
    gap: 10px 28px;
  }
  .summary > div {
    display: flex;
    flex-direction: column;
  }
  .num {
    font-size: 1.25rem;
    font-weight: 650;
  }
  .cap {
    font-size: 0.78rem;
    color: var(--text-muted);
  }
  .toggle {
    margin-left: auto;
    padding: 5px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: none;
    color: var(--text-muted);
    font-size: 0.82rem;
    font-weight: 550;
  }
  .toggle:hover {
    color: var(--text);
    background: var(--surface-hover);
  }

  .scroll {
    overflow-x: auto;
    padding-bottom: 4px;
  }
  .calendar {
    --cell: 12px;
    --gap: 3px;
    position: relative;
    display: grid;
    grid-template-columns: 30px auto;
    grid-template-rows: auto auto;
    column-gap: 6px;
    row-gap: 6px;
    width: max-content;
  }
  .months,
  .cells {
    grid-column: 2;
    display: grid;
    grid-template-columns: repeat(var(--weeks), var(--cell));
    column-gap: var(--gap);
  }
  .months {
    grid-row: 1;
    font-size: 0.72rem;
    color: var(--text-faint);
    height: 14px;
  }
  .months span {
    white-space: nowrap;
  }
  .weekdays {
    grid-column: 1;
    grid-row: 2;
    display: grid;
    grid-template-rows: repeat(7, var(--cell));
    row-gap: var(--gap);
    font-size: 0.68rem;
    color: var(--text-faint);
    line-height: var(--cell);
  }
  .cells {
    grid-row: 2;
    grid-template-rows: repeat(7, var(--cell));
    row-gap: var(--gap);
  }

  .cell {
    display: block;
    width: var(--cell, 12px);
    height: var(--cell, 12px);
    border-radius: 3px;
    background: var(--heat-empty);
  }
  .cell[data-level="1"] {
    background: var(--heat-1);
  }
  .cell[data-level="2"] {
    background: var(--heat-2);
  }
  .cell[data-level="3"] {
    background: var(--heat-3);
  }
  .cell[data-level="4"] {
    background: var(--heat-4);
  }
  .cells .cell:hover {
    outline: 2px solid var(--text);
    outline-offset: 1px;
  }

  .tooltip {
    position: absolute;
    z-index: 5;
    transform: translate(-50%, calc(-100% - 8px));
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 170px;
    padding: 8px 11px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: 9px;
    box-shadow: var(--shadow-card-hover);
    pointer-events: none;
    font-size: 0.8rem;
    white-space: nowrap;
  }
  .tt-date {
    color: var(--text-muted);
    margin-bottom: 2px;
  }
  .tt-game {
    display: flex;
    justify-content: space-between;
    gap: 14px;
  }
  .tt-game span:last-child {
    color: var(--text-muted);
    font-variant-numeric: tabular-nums;
  }

  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-wrap: wrap;
    gap: 10px 20px;
    font-size: 0.78rem;
    color: var(--text-faint);
  }
  .legend {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }
  .legend .cell {
    --cell: 11px;
  }

  .list {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }
  .list th {
    text-align: left;
    font-weight: 600;
    color: var(--text-faint);
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
  }
  .list td {
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
  }
  .num-cell {
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .empty {
    margin: 0;
    color: var(--text-muted);
  }
</style>
