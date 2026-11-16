import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/** Store key, e.g. "steam", "epic", "ubisoft", "xbox". */
export type Source = string;

export interface LibraryGame {
  id: string;
  source: Source;
  title: string;
  cover: string | null;
  hero: string | null;
  logo: string | null;
  addedAt: number;
  lastPlayed: number | null;
  launchCount: number;
  /** Total: imported store playtime plus Cabinet sessions the store hasn't counted yet. */
  playtimeSecs: number;
  /** The part of playtimeSecs imported from the store. */
  storePlaytimeSecs: number;
  /** Playtime over the last 14 days. */
  recentSecs: number;
  sessionCount: number;
  /** False for games owned through a store account but not installed. */
  installed: boolean;
  running: boolean;
  /** Frontend-only: bumped when artwork is re-fetched, to bust the image cache. */
  artRev?: number;
}

export interface SourceInfo {
  key: Source;
  name: string;
}

export interface GameDetails {
  game: LibraryGame;
  sourceName: string;
  sourceId: string;
  installDir: string | null;
  sizeBytes: number | null;
  storePage: string | null;
}

export interface Session {
  gameId: string;
  startedAt: number;
  endedAt: number;
  durationSecs: number;
}

export interface SyncInfo {
  at: number;
  count: number | null;
  error: string | null;
}

export interface AccountStatus {
  source: Source;
  connected: boolean;
  account: string | null;
  lastSync: SyncInfo | null;
}

export interface Theme {
  id: string;
  name: string;
  css: string;
  builtin: boolean;
}

export type GameState = "launching" | "running" | "stopped" | "not-detected";

export interface GameStatus {
  id: string;
  state: GameState;
  playtimeSecs: number;
  sessionCount: number;
  lastPlayed: number | null;
}

export type ArtKind = "cover" | "hero" | "logo";

export interface ArtUpdate {
  id: string;
  kind: ArtKind;
  path: string;
}

export const api = {
  library: () => invoke<LibraryGame[]>("get_library"),
  sources: () => invoke<SourceInfo[]>("get_sources"),
  rescan: (manual: boolean) => invoke<LibraryGame[]>("rescan", { manual }),
  launch: (id: string) => invoke<void>("launch_game", { id }),
  details: (id: string) => invoke<GameDetails>("game_details", { id }),
  sessions: (gameId: string | null, since: number) => invoke<Session[]>("get_sessions", { gameId, since }),
  openInstallDir: (id: string) => invoke<void>("open_install_dir", { id }),
  openStorePage: (id: string) => invoke<void>("open_store_page", { id }),
  accounts: () => invoke<AccountStatus[]>("get_accounts"),
  setSteamKey: (key: string | null) => invoke<void>("set_steam_api_key", { key }),
  epicLoginUrl: () => invoke<string>("epic_login_url"),
  epicLogin: (code: string) => invoke<string>("epic_login", { code }),
  epicLogout: () => invoke<void>("epic_logout"),
  syncAccounts: () => invoke<void>("sync_accounts"),
  themes: () => invoke<Theme[]>("list_themes"),
  openThemesDir: () => invoke<void>("open_themes_dir"),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  getSetting: (key: string) => invoke<string | null>("get_setting", { key }),
  setSetting: (key: string, value: string) => invoke<void>("set_setting", { key, value }),
};

export const onGameStatus = (cb: (s: GameStatus) => void): Promise<UnlistenFn> =>
  listen<GameStatus>("game-status", (e) => cb(e.payload));

export const onArtUpdated = (cb: (a: ArtUpdate) => void): Promise<UnlistenFn> =>
  listen<ArtUpdate>("art-updated", (e) => cb(e.payload));

export const onLibraryChanged = (cb: () => void): Promise<UnlistenFn> => listen("library-changed", () => cb());

/** Turns a cached artwork path into a URL the webview may load. */
export const artUrl = (path: string | null, rev?: number): string | null =>
  path ? convertFileSrc(path) + (rev ? `?v=${rev}` : "") : null;
