<script lang="ts">
  import { artUrl, type LibraryGame } from "$lib/api";
  import { initials, titleHue } from "$lib/format";

  let { game, width = 32 }: { game: LibraryGame; width?: number } = $props();
  let failed = $state(false);
  const src = $derived(artUrl(game.cover, game.artRev));
</script>

<span class="thumb" style:width={`${width}px`} style:--hue={titleHue(game.title)} aria-hidden="true">
  {#if src && !failed}
    <img {src} alt="" loading="lazy" decoding="async" onerror={() => (failed = true)} />
  {:else}
    <span class="ph">{initials(game.title)}</span>
  {/if}
</span>

<style>
  .thumb {
    position: relative;
    flex: none;
    aspect-ratio: 2 / 3;
    border-radius: 5px;
    overflow: hidden;
    background: hsl(calc(var(--hue) * 1deg) var(--placeholder-saturation) var(--placeholder-lightness));
    outline: 1px solid var(--border);
    outline-offset: -1px;
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .ph {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    font-size: 0.65rem;
    font-weight: 700;
    color: var(--text);
    opacity: 0.7;
  }
</style>
