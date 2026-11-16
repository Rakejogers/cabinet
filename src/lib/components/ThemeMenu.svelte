<script lang="ts">
  import { api } from "$lib/api";
  import { app } from "$lib/app.svelte";

  let open = $state(false);
  const current = $derived(app.themes.find((t) => t.id === app.themeId));
</script>

<svelte:window
  onclick={() => (open = false)}
  onkeydown={(e) => {
    if (e.key === "Escape") open = false;
  }}
/>

<div class="theme-menu">
  <button
    class="pill-btn"
    onclick={(e) => {
      e.stopPropagation();
      open = !open;
    }}
    aria-haspopup="menu"
    aria-expanded={open}
  >
    <svg viewBox="0 0 24 24" width="15" height="15" aria-hidden="true">
      <circle cx="12" cy="12" r="8.5" fill="none" stroke="currentColor" stroke-width="2" />
      <path d="M12 3.5a8.5 8.5 0 0 1 0 17z" fill="currentColor" />
    </svg>
    {current?.name ?? "Theme"}
  </button>
  {#if open}
    <div class="menu" role="menu" tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={() => {}}>
      {#each app.themes as theme (theme.id)}
        <button
          role="menuitemradio"
          aria-checked={theme.id === app.themeId}
          onclick={() => {
            open = false;
            app.selectTheme(theme);
          }}
        >
          <span class="check">{theme.id === app.themeId ? "✓" : ""}</span>
          {theme.name}
          {#if !theme.builtin}<span class="tag">custom</span>{/if}
        </button>
      {/each}
      <div class="sep"></div>
      <button
        role="menuitem"
        onclick={() => {
          open = false;
          api.openThemesDir().catch((e) => app.toast(String(e), "error"));
        }}
      >
        <span class="check"></span>Open themes folder…
      </button>
    </div>
  {/if}
</div>

<style>
  .theme-menu {
    position: relative;
  }
  .pill-btn {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    height: 36px;
    padding: 0 12px;
    border: 1px solid var(--border);
    background: var(--surface);
    border-radius: var(--radius-control);
    color: var(--text-muted);
    font-weight: 500;
    transition:
      background 120ms ease,
      color 120ms ease;
  }
  .pill-btn:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .menu {
    position: absolute;
    right: 0;
    top: calc(100% + 6px);
    min-width: 210px;
    padding: 5px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: calc(var(--radius-control) + 2px);
    box-shadow: var(--shadow-card-hover);
  }
  .menu button {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 7px 10px 7px 6px;
    border: 0;
    border-radius: 6px;
    background: none;
    text-align: left;
  }
  .menu button:hover {
    background: var(--surface-hover);
  }
  .check {
    width: 16px;
    text-align: center;
    color: var(--accent);
  }
  .tag {
    margin-left: auto;
    font-size: 0.7rem;
    color: var(--text-faint);
  }
  .sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--border);
  }
</style>
