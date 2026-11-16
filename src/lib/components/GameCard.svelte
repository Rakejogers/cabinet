<!--
  Library tile: cover, title, store mark and playtime. Clicking opens the game's
  page; the round button on the cover plays (or installs) straight away.
-->
<script lang="ts">
  import { artUrl, type LibraryGame } from "$lib/api";
  import { app } from "$lib/app.svelte";
  import { formatHoursDecimal, formatPlaytime, formatRelative, initials, titleHue } from "$lib/format";
  import StoreIcon from "./StoreIcon.svelte";

  interface Props {
    game: LibraryGame;
    /** What the second line shows: total playtime, or when it was last played. */
    detail?: "playtime" | "recent";
  }
  let { game, detail = "playtime" }: Props = $props();

  let loaded = $state(false);
  let failed = $state(false);
  let src = $derived(artUrl(game.cover, game.artRev));

  // Reset image state when the cover changes (e.g. after a background fetch).
  $effect(() => {
    void src;
    loaded = false;
    failed = false;
  });

  const storeName = $derived(app.sourceName(game.source));
  const status = $derived(game.running ? "Playing" : app.launching.has(game.id) ? "Starting…" : null);
  const subtitle = $derived(
    detail === "recent" && game.lastPlayed
      ? formatRelative(game.lastPlayed)
      : game.playtimeSecs > 0
        ? formatHoursDecimal(game.playtimeSecs)
        : game.installed
          ? "Not played"
          : "Not installed",
  );
  const tooltip = $derived.by(() => {
    let text = game.title;
    if (game.playtimeSecs > 0) text += `\n${formatPlaytime(game.playtimeSecs)} played`;
    if (game.storePlaytimeSecs > 0) text += ` (includes ${formatPlaytime(game.storePlaytimeSecs)} from ${storeName})`;
    if (!game.installed) text += "\nNot installed";
    return text;
  });
</script>

<div class="card" class:running={game.running} class:uninstalled={!game.installed}>
  <button class="open" onclick={() => app.openGame(game.id)} title={tooltip}>
    <span class="cover" style:--hue={titleHue(game.title)}>
      {#if src && !failed}
        <img
          {src}
          alt=""
          loading="lazy"
          decoding="async"
          class:loaded
          onload={() => (loaded = true)}
          onerror={() => (failed = true)}
        />
      {/if}
      {#if !src || failed || !loaded}
        <span class="placeholder" aria-hidden="true">
          <span class="initials">{initials(game.title)}</span>
          <span class="ph-title">{game.title}</span>
        </span>
      {/if}
      {#if status}
        <span class="status" class:live={game.running}><span class="dot"></span>{status}</span>
      {/if}
    </span>
    <span class="meta">
      <span class="title">{game.title}</span>
      <span class="sub">
        <StoreIcon source={game.source} size={13} title={storeName} />
        <span>{subtitle}</span>
      </span>
    </span>
  </button>

  <!-- Same box as the cover, so the action button sits on the artwork. -->
  {#if !game.running}
    <div class="cover-layer">
      <button
        class="action"
        onclick={() => app.launch(game)}
        aria-label={`${game.installed ? "Play" : "Install"} ${game.title}`}
        title={game.installed ? "Play" : `Install with ${storeName}`}
      >
        {#if game.installed}
          <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true">
            <path d="M9 5.5v13l10.5-6.5z" fill="currentColor" />
          </svg>
        {:else}
          <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
            <path
              d="M12 4v11m0 0-4.5-4.5M12 15l4.5-4.5M5 19.5h14"
              fill="none"
              stroke="currentColor"
              stroke-width="2.2"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
        {/if}
      </button>
    </div>
  {/if}
</div>

<style>
  .card {
    position: relative;
    min-width: 0;
    width: 100%;
  }

  .open {
    display: flex;
    flex-direction: column;
    gap: 9px;
    width: 100%;
    padding: 0;
    border: 0;
    background: none;
    text-align: left;
    border-radius: var(--radius-card);
  }
  .open:focus-visible {
    outline: none;
  }

  .cover,
  .cover-layer {
    transition:
      transform 180ms cubic-bezier(0.2, 0.8, 0.2, 1),
      box-shadow 180ms ease,
      filter 180ms ease;
  }
  .cover {
    position: relative;
    display: block;
    aspect-ratio: 2 / 3;
    border-radius: var(--radius-card);
    overflow: hidden;
    background: var(--surface);
    box-shadow: var(--shadow-card);
    outline: 1px solid var(--border);
    outline-offset: -1px;
  }
  .card:hover .cover,
  .card:hover .cover-layer,
  .card:focus-within .cover,
  .card:focus-within .cover-layer {
    transform: translateY(-4px);
  }
  .card:hover .cover,
  .card:focus-within .cover {
    box-shadow: var(--shadow-card-hover);
  }
  .open:focus-visible .cover {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .card.running .cover {
    outline: 2px solid var(--success);
    outline-offset: 2px;
  }
  .card.uninstalled .cover {
    filter: grayscale(0.85) brightness(0.7);
  }
  .card.uninstalled:hover .cover {
    filter: grayscale(0.3) brightness(0.9);
  }

  img {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    opacity: 0;
    transition: opacity 220ms ease;
  }
  img.loaded {
    opacity: 1;
  }

  .placeholder {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    padding: 14px;
    background:
      radial-gradient(
        120% 80% at 20% 0%,
        hsl(calc(var(--hue) * 1deg) var(--placeholder-saturation) calc(var(--placeholder-lightness) + 10%)),
        transparent 70%
      ),
      hsl(calc((var(--hue) + 40) * 1deg) var(--placeholder-saturation) var(--placeholder-lightness));
    color: var(--text);
  }
  .initials {
    font-size: 2.2rem;
    font-weight: 700;
    letter-spacing: -0.02em;
    opacity: 0.55;
  }
  .ph-title {
    font-weight: 600;
    font-size: 0.95rem;
    line-height: 1.25;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .status {
    position: absolute;
    top: 8px;
    left: 8px;
    z-index: 1;
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 4px 9px 4px 8px;
    border-radius: 999px;
    font-size: 0.75rem;
    font-weight: 600;
    background: rgba(10, 11, 14, 0.78);
    color: #fff;
    backdrop-filter: blur(6px);
  }
  .status .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1.2s ease-in-out infinite;
  }
  .status.live .dot {
    background: var(--success);
    animation-duration: 2s;
  }
  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }

  .cover-layer {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    aspect-ratio: 2 / 3;
    pointer-events: none;
  }
  .action {
    position: absolute;
    right: 10px;
    bottom: 10px;
    display: grid;
    place-items: center;
    width: 42px;
    height: 42px;
    border: 0;
    border-radius: 50%;
    background: var(--accent);
    color: var(--accent-text);
    box-shadow: 0 8px 22px -6px rgba(0, 0, 0, 0.6);
    pointer-events: auto;
    opacity: 0;
    transform: translateY(6px) scale(0.9);
    transition:
      opacity 150ms ease,
      transform 180ms cubic-bezier(0.2, 0.8, 0.2, 1),
      background 120ms ease;
  }
  .card:hover .action,
  .card:focus-within .action {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
  .action:hover {
    background: var(--accent-hover);
  }

  .meta {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 0 2px;
    min-width: 0;
  }
  .title {
    font-weight: 650;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sub {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.8rem;
    color: var(--text-muted);
    white-space: nowrap;
  }
</style>
