<!--
  Wide artwork banner with the game's logo (or title), used on Home and on the
  game page. Content goes in the default slot; an optional aside sits right.
-->
<script lang="ts">
  import type { Snippet } from "svelte";
  import { artUrl, type LibraryGame } from "$lib/api";
  import { titleHue } from "$lib/format";

  interface Props {
    game: LibraryGame;
    eyebrow?: string;
    live?: boolean;
    tall?: boolean;
    top?: Snippet;
    children?: Snippet;
    aside?: Snippet;
  }
  let { game, eyebrow, live = false, tall = false, top, children, aside }: Props = $props();

  let heroFailed = $state(false);
  let logoFailed = $state(false);
  const heroSrc = $derived(artUrl(game.hero, game.artRev));
  const logoSrc = $derived(artUrl(game.logo, game.artRev));
  const coverSrc = $derived(artUrl(game.cover, game.artRev));
  $effect(() => {
    void game.id;
    void game.artRev;
    heroFailed = false;
    logoFailed = false;
  });
</script>

<section class="hero" class:tall style:--hue={titleHue(game.title)}>
  <div class="hero-art" aria-hidden="true">
    {#if heroSrc && !heroFailed}
      {#key heroSrc}
        <img class="hero-img" src={heroSrc} alt="" onerror={() => (heroFailed = true)} />
      {/key}
    {:else if coverSrc}
      <img class="hero-img blurred" src={coverSrc} alt="" />
    {/if}
  </div>
  <div class="scrim" aria-hidden="true"></div>

  {#if top}<div class="top">{@render top()}</div>{/if}

  <div class="hero-body">
    {#if eyebrow}
      <span class="eyebrow">
        {#if live}<span class="dot"></span>{/if}{eyebrow}
      </span>
    {/if}
    {#if logoSrc && !logoFailed}
      <img class="logo" src={logoSrc} alt={game.title} onerror={() => (logoFailed = true)} />
    {:else}
      <h1>{game.title}</h1>
    {/if}
    {@render children?.()}
  </div>

  {@render aside?.()}
</section>

<style>
  .hero {
    position: relative;
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 24px;
    min-height: 340px;
    padding: 32px;
    border-radius: calc(var(--radius-card) + 4px);
    overflow: hidden;
    background: hsl(calc(var(--hue) * 1deg) var(--placeholder-saturation) var(--placeholder-lightness));
    isolation: isolate;
    outline: 1px solid var(--border);
    outline-offset: -1px;
    /* Text over artwork uses the theme's banner tokens. */
    --text: var(--hero-text);
    --text-muted: var(--hero-text-muted);
    --text-faint: var(--hero-text-faint);
    --surface-hover: rgba(255, 255, 255, 0.08);
    --source-epic: var(--hero-text-muted);
    color: var(--text);
  }
  .hero.tall {
    min-height: 400px;
  }
  .hero-art,
  .scrim {
    position: absolute;
    inset: 0;
    z-index: -1;
  }
  .hero-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    object-position: center 30%;
    animation: fade 400ms ease;
  }
  .hero-img.blurred {
    filter: blur(36px) saturate(1.3);
    transform: scale(1.3);
  }
  .scrim {
    background: var(--hero-scrim);
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }

  .top {
    position: absolute;
    top: 18px;
    left: 18px;
    right: 18px;
    display: flex;
    justify-content: space-between;
    gap: 12px;
  }

  .hero-body {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 14px;
    max-width: min(600px, 62%);
  }
  .eyebrow {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 0.75rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--text-muted);
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--success);
    animation: pulse 2s ease-in-out infinite;
  }
  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }
  .logo {
    max-width: min(420px, 100%);
    max-height: 120px;
    object-fit: contain;
    object-position: left bottom;
    filter: drop-shadow(0 4px 18px rgba(0, 0, 0, 0.45));
  }
  h1 {
    margin: 0;
    font-size: clamp(1.8rem, 3.2vw, 2.6rem);
    line-height: 1.05;
    letter-spacing: -0.02em;
  }

  @media (max-width: 860px) {
    .hero-body {
      max-width: 100%;
    }
  }
</style>
