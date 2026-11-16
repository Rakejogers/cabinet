// Shared application state and actions, used by every view.

import {
  api,
  onArtUpdated,
  onGameStatus,
  onLibraryChanged,
  type LibraryGame,
  type SourceInfo,
  type Theme,
} from "$lib/api";
import { formatPlaytime } from "$lib/format";

/** Top-level views; "game" and "settings" are reached from the others. */
export type View = "home" | "library" | "game" | "settings";
export type SortKey = "title" | "recent" | "playtime" | "added";
/** "all" or a store key. */
export type SourceFilter = string;
export type ToastKind = "info" | "error";

const THEME_CACHE = "cabinet.theme-css";

function applyThemeCss(css: string) {
  let el = document.getElementById("cabinet-theme") as HTMLStyleElement | null;
  if (!el) {
    el = document.createElement("style");
    el.id = "cabinet-theme";
    document.head.appendChild(el);
  }
  el.textContent = css;
  try {
    localStorage.setItem(THEME_CACHE, css);
  } catch {}
}

class App {
  games = $state<LibraryGame[]>([]);
  loaded = $state(false);
  scanning = $state(false);
  view = $state<View>("home");
  /** The game shown by the detail view. */
  gameId = $state<string | null>(null);
  /** Where the back button on the detail and settings views returns to. */
  private previous: View = "home";
  sources = $state<SourceInfo[]>([]);
  /** Also show games owned through a store account but not installed. */
  showUninstalled = $state(false);
  query = $state("");
  sort = $state<SortKey>("title");
  filter = $state<SourceFilter>("all");
  cardSize = $state(168);
  themes = $state<Theme[]>([]);
  themeId = $state("dark");
  launching = $state(new Set<string>());
  toasts = $state<{ id: number; text: string; kind: ToastKind }[]>([]);

  constructor() {
    // Apply the last-used theme before first paint; the backend copy follows.
    try {
      const cached = localStorage.getItem(THEME_CACHE);
      if (cached) applyThemeCss(cached);
    } catch {}
  }

  /** Loads settings and the saved library, starts a rescan, and subscribes to
   *  backend events. Returns a cleanup function. */
  init(): () => void {
    const unlisten = [
      onArtUpdated(({ id, kind, path }) => this.patch(id, { [kind]: path, artRev: Date.now() })),
      onLibraryChanged(async () => {
        try {
          this.games = this.mergeArt(await api.library());
        } catch (e) {
          console.error(e);
        }
      }),
      onGameStatus((s) => {
        const game = this.games.find((g) => g.id === s.id);
        const stats = {
          playtimeSecs: s.playtimeSecs,
          sessionCount: s.sessionCount,
          ...(s.lastPlayed ? { lastPlayed: s.lastPlayed } : {}),
        };
        if (s.state === "running") {
          this.clearLaunching(s.id);
          this.patch(s.id, { ...stats, running: true });
        } else if (s.state === "stopped") {
          this.patch(s.id, { ...stats, running: false });
          if (game) this.toast(`${game.title}: session recorded (${formatPlaytime(s.playtimeSecs)} total)`);
        } else if (s.state === "not-detected") {
          this.clearLaunching(s.id);
          this.patch(s.id, { running: false });
          if (game) this.toast(`Couldn't detect ${game.title} running, so playtime wasn't tracked`, "error");
        }
      }),
    ];

    (async () => {
      try {
        await this.loadThemes();
        const view = await api.getSetting("view");
        if (view === "home" || view === "library") this.view = view;
        const size = Number(await api.getSetting("cardSize"));
        if (size >= 120 && size <= 280) this.cardSize = size;
        this.showUninstalled = (await api.getSetting("showUninstalled")) === "true";
        this.sources = await api.sources();
      } catch (e) {
        console.error(e);
      }
      // Show the persisted library instantly, then refresh from disk.
      try {
        this.games = await api.library();
        if (this.games.length) this.loaded = true;
      } catch (e) {
        console.error(e);
      }
      await this.rescan();
    })();

    return () => unlisten.forEach((p) => p.then((f) => f()));
  }

  setView(view: View) {
    if (view === "game" || view === "settings") {
      if (this.view !== "game" && this.view !== "settings") this.previous = this.view;
    } else {
      api.setSetting("view", view).catch(console.error);
    }
    this.view = view;
  }

  openGame(id: string) {
    this.gameId = id;
    this.setView("game");
  }

  back() {
    this.setView(this.previous);
  }

  setShowUninstalled(value: boolean) {
    this.showUninstalled = value;
    api.setSetting("showUninstalled", String(value)).catch(console.error);
  }

  sourceName(key: string): string {
    return this.sources.find((s) => s.key === key)?.name ?? key;
  }

  /** Jumps to the library with a given sort, e.g. from a home-screen section. */
  openLibrary(sort: SortKey) {
    this.sort = sort;
    this.filter = "all";
    this.query = "";
    this.setView("library");
  }

  private sizeTimer: ReturnType<typeof setTimeout> | undefined;
  saveCardSize() {
    clearTimeout(this.sizeTimer);
    this.sizeTimer = setTimeout(() => api.setSetting("cardSize", String(this.cardSize)), 400);
  }

  // ---- Themes -------------------------------------------------------------

  async loadThemes() {
    this.themes = await api.themes();
    const saved = (await api.getSetting("theme")) ?? "dark";
    const theme = this.themes.find((t) => t.id === saved) ?? this.themes[0];
    this.themeId = theme.id;
    applyThemeCss(theme.css);
  }

  async selectTheme(theme: Theme) {
    this.themeId = theme.id;
    applyThemeCss(theme.css);
    await api.setSetting("theme", theme.id);
  }

  // ---- Library ------------------------------------------------------------

  toast(text: string, kind: ToastKind = "info") {
    const id = Date.now() + Math.random();
    this.toasts = [...this.toasts, { id, text, kind }];
    setTimeout(() => (this.toasts = this.toasts.filter((t) => t.id !== id)), kind === "error" ? 6000 : 3500);
  }

  async rescan(manual = false) {
    if (this.scanning) return;
    this.scanning = true;
    try {
      const before = this.games.length;
      this.games = this.mergeArt(await api.rescan(manual));
      if (manual) {
        await this.loadThemes();
        const diff = this.games.length - before;
        this.toast(diff > 0 ? `Found ${diff} new game${diff === 1 ? "" : "s"}` : "Library is up to date");
      }
    } catch (e) {
      this.toast(`Scan failed: ${e}`, "error");
    } finally {
      this.scanning = false;
      this.loaded = true;
    }
  }

  async launch(game: LibraryGame) {
    if (!game.installed) {
      try {
        await api.launch(game.id);
        this.toast(`Opening ${this.sourceName(game.source)} to install ${game.title}…`);
      } catch (e) {
        this.toast(`Couldn't start the install: ${e}`, "error");
      }
      return;
    }
    if (game.running || this.launching.has(game.id)) {
      this.toast(`${game.title} is already ${game.running ? "running" : "starting"}`);
      return;
    }
    this.launching = new Set(this.launching).add(game.id);
    try {
      await api.launch(game.id);
      this.toast(`Launching ${game.title}…`);
    } catch (e) {
      this.clearLaunching(game.id);
      this.toast(`Couldn't launch ${game.title}: ${e}`, "error");
    }
  }

  private clearLaunching(id: string) {
    if (!this.launching.has(id)) return;
    const next = new Set(this.launching);
    next.delete(id);
    this.launching = next;
  }

  /** Art arrives through events that can overtake a command's response, so a
   *  fresh game list keeps any artwork the current one already has. */
  private mergeArt(fresh: LibraryGame[]): LibraryGame[] {
    const current = new Map(this.games.map((g) => [g.id, g]));
    return fresh.map((g) => {
      const old = current.get(g.id);
      if (!old) return g;
      return {
        ...g,
        cover: g.cover ?? old.cover,
        hero: g.hero ?? old.hero,
        logo: g.logo ?? old.logo,
        artRev: old.artRev,
      };
    });
  }

  private patch(id: string, fields: Partial<LibraryGame>) {
    const i = this.games.findIndex((g) => g.id === id);
    if (i >= 0) this.games[i] = { ...this.games[i], ...fields };
  }
}

export const app = new App();
