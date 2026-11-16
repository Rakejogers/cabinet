<script lang="ts">
  import { onMount } from "svelte";
  import { api, type AccountStatus } from "$lib/api";
  import { app } from "$lib/app.svelte";
  import { formatRelative } from "$lib/format";
  import StoreIcon from "$lib/components/StoreIcon.svelte";

  let accounts = $state<AccountStatus[]>([]);
  let steamKey = $state("");
  let epicCode = $state("");
  let busy = $state<string | null>(null);
  let epicStep = $state(false);

  const steam = $derived(accounts.find((a) => a.source === "steam"));
  const epic = $derived(accounts.find((a) => a.source === "epic"));

  const installedBySource = $derived(
    app.sources.map((s) => ({
      ...s,
      installed: app.games.filter((g) => g.source === s.key && g.installed).length,
      owned: app.games.filter((g) => g.source === s.key && !g.installed).length,
    })),
  );

  async function refresh() {
    try {
      accounts = await api.accounts();
    } catch (e) {
      app.toast(String(e), "error");
    }
  }
  onMount(refresh);

  async function run(label: string, task: () => Promise<void>) {
    busy = label;
    try {
      await task();
    } catch (e) {
      app.toast(String(e), "error");
    } finally {
      busy = null;
      await refresh();
    }
  }

  const saveSteamKey = () =>
    run("steam", async () => {
      await api.setSteamKey(steamKey.trim());
      steamKey = "";
      app.toast("Steam connected");
    });

  const removeSteamKey = () =>
    run("steam", async () => {
      await api.setSteamKey(null);
      app.toast("Steam key removed");
    });

  const openEpicLogin = async () => {
    epicStep = true;
    await api.openUrl(await api.epicLoginUrl());
  };

  const connectEpic = () =>
    run("epic", async () => {
      const name = await api.epicLogin(epicCode);
      epicCode = "";
      epicStep = false;
      app.toast(`Signed in to Epic as ${name}`);
    });

  const signOutEpic = () => run("epic", () => api.epicLogout());
  const syncNow = () => run("sync", () => api.syncAccounts());

  function syncLine(a: AccountStatus | undefined): string {
    const s = a?.lastSync;
    if (!s) return "Not synced yet";
    if (s.error) return `Last sync failed ${formatRelative(s.at)}`;
    return `${s.count ?? 0} owned games · synced ${formatRelative(s.at)}`;
  }
</script>

<div class="page">
  <header class="head">
    <button class="btn btn-ghost" onclick={() => app.back()}>
      <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true">
        <path d="M15 5l-7 7 7 7" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round" />
      </svg>
      Back
    </button>
    <h1>Settings</h1>
  </header>

  <section>
    <div class="section-head">
      <div>
        <h2>Store accounts</h2>
        <p class="caption">Connect accounts to see games you own but haven't installed, with their playtime.</p>
      </div>
      <button class="btn btn-ghost" onclick={syncNow} disabled={busy !== null || !(steam?.connected || epic?.connected)}>
        {busy === "sync" ? "Syncing…" : "Sync now"}
      </button>
    </div>

    <div class="cards">
      <!-- Steam -->
      <article class="card">
        <div class="card-head">
          <span class="badge"><StoreIcon source="steam" size={20} /></span>
          <div class="who">
            <strong>Steam</strong>
            <span class="status" class:ok={steam?.connected}>
              {steam?.connected ? "Connected" : "Not connected"}{steam?.account
                ? ` · signed in to Steam as ${steam.account}`
                : ""}
            </span>
          </div>
        </div>

        {#if steam?.connected}
          <p class="line">{syncLine(steam)}</p>
          {#if steam.lastSync?.error}<p class="err">{steam.lastSync.error}</p>{/if}
          <div class="row">
            <input type="password" placeholder="Replace API key" bind:value={steamKey} autocomplete="off" spellcheck="false" />
            <button class="btn btn-ghost" onclick={saveSteamKey} disabled={!steamKey.trim() || busy !== null}>Save</button>
            <button class="btn btn-ghost danger" onclick={removeSteamKey} disabled={busy !== null}>Disconnect</button>
          </div>
        {:else}
          <p class="line">
            Uses Steam's official Web API with your own free key. Cabinet sends the key and your Steam ID only to Steam.
          </p>
          <ol class="steps">
            <li>
              <button class="link" onclick={() => api.openUrl("https://steamcommunity.com/dev/apikey")}>Get a Web API key</button>
              (any domain name works, e.g. <code>localhost</code>).
            </li>
            <li>Paste it here:</li>
          </ol>
          <div class="row">
            <input type="password" placeholder="Steam Web API key" bind:value={steamKey} autocomplete="off" spellcheck="false" />
            <button class="btn btn-primary" onclick={saveSteamKey} disabled={!steamKey.trim() || busy !== null}>
              {busy === "steam" ? "Connecting…" : "Connect"}
            </button>
          </div>
          {#if steam?.lastSync?.error}<p class="err">{steam.lastSync.error}</p>{/if}
        {/if}
      </article>

      <!-- Epic -->
      <article class="card">
        <div class="card-head">
          <span class="badge"><StoreIcon source="epic" size={20} /></span>
          <div class="who">
            <strong>Epic Games</strong>
            <span class="status" class:ok={epic?.connected}>
              {epic?.connected ? `Signed in · ${epic.account}` : "Not signed in"}
            </span>
          </div>
        </div>

        {#if epic?.connected}
          <p class="line">{syncLine(epic)}</p>
          {#if epic.lastSync?.error}<p class="err">{epic.lastSync.error}</p>{/if}
          <div class="row">
            <button class="btn btn-ghost danger" onclick={signOutEpic} disabled={busy !== null}>Sign out</button>
          </div>
        {:else}
          <p class="line">
            Epic has no public API, so this signs in the same way the Heroic and Legendary launchers do. It's unofficial
            and could stop working if Epic changes things.
          </p>
          <ol class="steps">
            <li><button class="link" onclick={openEpicLogin}>Sign in on epicgames.com</button>.</li>
            <li>
              The page then shows some text containing <code>"authorizationCode"</code>. Copy that whole text (or just
              the code) and paste it here:
            </li>
          </ol>
          <div class="row">
            <input
              type="password"
              placeholder={epicStep ? "Paste the authorization code" : "Authorization code"}
              bind:value={epicCode}
              autocomplete="off"
              spellcheck="false"
            />
            <button class="btn btn-primary" onclick={connectEpic} disabled={!epicCode.trim() || busy !== null}>
              {busy === "epic" ? "Signing in…" : "Connect"}
            </button>
          </div>
          {#if epic?.lastSync?.error}<p class="err">{epic.lastSync.error}</p>{/if}
        {/if}
      </article>
    </div>
    <p class="fine">Keys and sign-in tokens are kept in the system credential store, not in Cabinet's database.</p>
  </section>

  <section>
    <div class="section-head"><h2>Library</h2></div>
    <label class="switch">
      <input type="checkbox" checked={app.showUninstalled} onchange={(e) => app.setShowUninstalled(e.currentTarget.checked)} />
      <span class="track"><span class="thumb"></span></span>
      <span>Show owned games that aren't installed</span>
    </label>
  </section>

  <section>
    <div class="section-head"><h2>Stores on this PC</h2></div>
    <ul class="stores">
      {#each installedBySource as s (s.key)}
        <li>
          <StoreIcon source={s.key} size={16} />
          <span class="store-name">{s.name}</span>
          <span class="muted">
            {s.installed ? `${s.installed} installed` : "No games found"}{s.owned ? ` · ${s.owned} more owned` : ""}
          </span>
        </li>
      {/each}
    </ul>
  </section>

  <section>
    <div class="section-head"><h2>Appearance</h2></div>
    <div class="themes">
      {#each app.themes as theme (theme.id)}
        <button class="btn btn-ghost" class:selected={theme.id === app.themeId} onclick={() => app.selectTheme(theme)}>
          {theme.name}
        </button>
      {/each}
      <button class="btn btn-ghost" onclick={() => api.openThemesDir().catch((e) => app.toast(String(e), "error"))}>
        Open themes folder…
      </button>
    </div>
  </section>
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: 34px;
    max-width: 980px;
    margin: 0 auto;
    padding: 22px 24px 56px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  h1 {
    margin: 0;
    font-size: 1.5rem;
    letter-spacing: -0.01em;
  }
  h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 650;
  }
  .section-head {
    display: flex;
    align-items: flex-end;
    justify-content: space-between;
    gap: 16px;
    margin-bottom: 14px;
  }
  .caption {
    margin: 3px 0 0;
    font-size: 0.85rem;
    color: var(--text-muted);
  }

  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(360px, 1fr));
    gap: 16px;
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 18px 20px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
  }
  .card-head {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .badge {
    display: grid;
    place-items: center;
    width: 38px;
    height: 38px;
    border-radius: 10px;
    background: var(--surface-hover);
  }
  .who {
    display: flex;
    flex-direction: column;
  }
  .status {
    font-size: 0.82rem;
    color: var(--text-muted);
  }
  .status.ok {
    color: var(--success);
  }
  .line {
    margin: 0;
    font-size: 0.88rem;
    color: var(--text-muted);
  }
  .err {
    margin: 0;
    font-size: 0.82rem;
    color: var(--danger);
    word-break: break-word;
  }
  .steps {
    margin: 0;
    padding-left: 20px;
    font-size: 0.88rem;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  code {
    font-family: ui-monospace, "Cascadia Code", Consolas, monospace;
    font-size: 0.82rem;
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--surface-hover);
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    font-weight: 600;
    text-decoration: underline;
    text-underline-offset: 2px;
  }
  .row {
    display: flex;
    gap: 8px;
  }
  .row input {
    flex: 1;
    min-width: 0;
    height: 38px;
    padding: 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--bg);
    outline: none;
  }
  .row input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }
  .danger {
    color: var(--danger);
  }
  .fine {
    margin: 10px 0 0;
    font-size: 0.78rem;
    color: var(--text-faint);
  }

  .switch {
    display: inline-flex;
    align-items: center;
    gap: 12px;
    cursor: pointer;
  }
  .switch input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .track {
    position: relative;
    width: 40px;
    height: 22px;
    border-radius: 999px;
    background: var(--border-strong);
    transition: background 150ms ease;
  }
  .thumb {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #fff;
    transition: transform 150ms ease;
  }
  .switch input:checked + .track {
    background: var(--accent);
  }
  .switch input:checked + .track .thumb {
    transform: translateX(18px);
  }
  .switch input:focus-visible + .track {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .stores {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
  }
  .stores li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 18px;
  }
  .stores li + li {
    border-top: 1px solid var(--border);
  }
  .store-name {
    font-weight: 600;
    min-width: 140px;
  }
  .muted {
    color: var(--text-muted);
    font-size: 0.88rem;
  }

  .themes {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .themes .selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-soft);
  }
</style>
