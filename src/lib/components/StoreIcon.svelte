<!--
  Small monochrome store mark. Brand glyphs come from Simple Icons (CC0); the
  Xbox mark isn't distributed there, so Xbox and unknown stores get a generic
  gamepad glyph.
-->
<script lang="ts">
  import { siEpicgames, siSteam, siUbisoft } from "simple-icons";

  let { source, size = 14, title }: { source: string; size?: number; title?: string } = $props();

  const GAMEPAD =
    "M7.5 6h9a5.5 5.5 0 0 1 5.4 6.5l-.9 4.9a2.6 2.6 0 0 1-4.6 1.1L14.6 16H9.4l-1.8 2.5a2.6 2.6 0 0 1-4.6-1.1l-.9-4.9A5.5 5.5 0 0 1 7.5 6Zm-.5 3v1.5H5.5v1.5H7v1.5h1.5V12H10v-1.5H8.5V9H7Zm9.25.25a1 1 0 1 0 0 2 1 1 0 0 0 0-2Zm-1.75 2.5a1 1 0 1 0 0 2 1 1 0 0 0 0-2Z";

  const PATHS: Record<string, string> = {
    steam: siSteam.path,
    epic: siEpicgames.path,
    ubisoft: siUbisoft.path,
  };

  const path = $derived(PATHS[source] ?? GAMEPAD);
</script>

<svg
  class="store-icon"
  viewBox="0 0 24 24"
  width={size}
  height={size}
  role={title ? "img" : undefined}
  aria-hidden={title ? undefined : "true"}
  aria-label={title}
>
  {#if title}<title>{title}</title>{/if}
  <path d={path} fill="currentColor" />
</svg>

<style>
  .store-icon {
    flex: none;
    display: block;
  }
</style>
