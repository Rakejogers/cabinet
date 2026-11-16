<script lang="ts">
  import { app } from "$lib/app.svelte";
</script>

<div class="toasts" aria-live="polite">
  {#each app.toasts as t (t.id)}
    <div class="toast" class:error={t.kind === "error"}>{t.text}</div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    left: 50%;
    bottom: 20px;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    pointer-events: none;
    z-index: 20;
  }
  .toast {
    padding: 9px 16px;
    border-radius: 999px;
    background: var(--text);
    color: var(--bg);
    font-weight: 500;
    box-shadow: 0 10px 30px -10px rgba(0, 0, 0, 0.5);
    animation: rise 180ms ease-out;
  }
  .toast.error {
    background: var(--danger);
    color: #fff;
  }
  @keyframes rise {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
</style>
