# Cabinet

A fast, lightweight game launcher that puts every store's installed games in one grid. Built with Tauri 2, Svelte 5 and Rust.

## Running

Prerequisites: Rust (MSVC toolchain on Windows), Node 20+, Visual Studio C++ Build Tools, WebView2, and the Tauri CLI (`cargo install tauri-cli --version "^2"`).

```sh
npm install
cargo tauri dev        # or: npm run tauri dev
```

Rust unit tests: `cd src-tauri && cargo test` (add `-- --ignored` for the network test).

## Features

- **Home**: banner for the current or last-played game, stat tiles, recently played, a GitHub-style activity calendar, most played chart, "pick back up" and never-played shelves.
- **Library**: grid of every installed game with store filters, search, sort and cover size. Optionally includes owned games that aren't installed (greyed, with an Install button).
- **Game page**: banner, Play/Install, open folder, store page, playtime stats, per-game activity calendar, recent sessions and install details.
- **Stores**: Steam, Epic Games, Ubisoft Connect and Xbox (PC Game Pass / GDK games).
- **Accounts** (Settings): Steam via your own Web API key (official API); Epic via sign-in on epicgames.com (unofficial, see below). Both add owned-but-not-installed games and their playtime.

## Layout

```
src/                      Svelte frontend
  routes/+page.svelte     shell: top bar + current view
  lib/app.svelte.ts       shared state and actions (library, themes, launching, toasts)
  lib/views/              HomeView, LibraryView, GameView (detail page), SettingsView
  lib/components/         GameCard, HeroBanner, ActivityHeatmap, MostPlayedChart, StoreIcon, TopBar, ...
  lib/api.ts              typed wrappers around the Tauri commands and events
src-tauri/
  src/sources/            one GameSource per store (steam, epic + epic_api, ubisoft, xbox) + VDF parser
  src/accounts.rs         store credentials in the OS credential store
  src/jobs.rs             background artwork fetching and owned-library sync
  src/net.rs              HTTP helpers (OS trust store, readable API errors)
  src/platform/           ALL OS-specific code: store locations, path rules
  src/db.rs               SQLite: games, play sessions, settings
  src/covers.rs           artwork kinds and the local art cache
  src/tracker.rs          playtime tracking through sysinfo
  src/themes.rs           built-in and user themes
  themes/*.css            the built-in Dark and Light themes
```

### Adding a store

Implement `sources::GameSource` (`key`, `name`, `scan`, `launch`, and optionally `install`, `store_page`, `owned`, `fetch_art`) and register it in `sources::all()`. Put install-location lookups in `platform/{windows,macos,other}.rs` rather than adding `#[cfg]` to the source.

## Data

Everything lives in the OS app-data folders for the identifier `com.cabinet.app`:

- `%LOCALAPPDATA%\com.cabinet.app\cabinet.db`: library, sessions, settings
- `%LOCALAPPDATA%\com.cabinet.app\covers\`: cached cover art
- `%APPDATA%\com.cabinet.app\themes\`: user themes (any `*.css` file here shows up in the switcher)

## Themes

A theme is a CSS file that sets custom properties on `:root`. The first-line comment `/* name: My Theme */` gives it a display name. Use **Theme → Open themes folder…**: it contains `dark.css.example` and `light.css.example` to copy from. The rescan button reloads themes.

## How it works

- **Steam**: finds Steam through the registry, reads `steamapps/libraryfolders.vdf` and every `appmanifest_*.acf`, and only lists apps with the fully installed flag (redistributables and Proton are skipped). Launches via `steam://rungameid/<id>`. Covers come from Steam's local `appcache/librarycache` first, then the Steam CDN, then the public store asset API (needed for newer titles with hashed asset paths).
- **Epic**: reads `C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests\*.item`, skipping DLC and incomplete installs. Launches via `com.epicgames.launcher://apps/<ns>:<id>:<app>?action=launch`. Covers come from the launcher's local catalog cache (`Data/Catalog/catcache.bin`), resized by Epic's CDN.
- **Imported playtime**: on each scan, Steam's own per-game playtime and last-played date are read from `userdata/<account>/config/localconfig.vdf` for the most recently signed-in account. Steam also counts the sessions Cabinet launches, so the total shown is Steam's figure plus only the Cabinet sessions that ended after Steam's figure last changed. Epic keeps playtime server-side, so it is only imported once you sign in to Epic in Settings. Store figures only ever grow, so when the local file and the web API disagree the larger one wins.
- **Playtime**: after a launch, Cabinet polls the process list for up to 3 minutes for any executable inside the game's install folder. While the game runs, the session is written to SQLite every 5 seconds, so a crash loses almost nothing. The session ends after 3 consecutive empty polls.
- **Ubisoft Connect**: installs from the registry (`Ubisoft\Launcher\Installs`), launches via `uplay://launch/<id>/0`. Ubisoft has no public art service, so artwork comes from the same game's Steam listing when the titles match.
- **Xbox**: GDK games in the Xbox library folders listed by each drive's `.GamingRoot` file, read from their `MicrosoftGame.config`. Launches through the shell as `shell:AppsFolder\<PackageFamilyName>!<AppId>` (the family name's publisher hash is computed locally). Art comes from the public Microsoft Store catalog.
- **Activity calendar**: built from Cabinet's own play sessions, split across local days. Stores only report totals, so the calendar starts on the first session Cabinet tracked.
- **Owned games**: Steam uses `IPlayerService/GetOwnedGames` with your key and SteamID (sent only to Steam). Epic signs in as the Epic Games Launcher's public OAuth client (the method Legendary/Heroic use), then reads the library, catalog and playtime services. It is unofficial and may break. Secrets (the Steam key, the Epic refresh token) are stored in the OS credential store (Windows Credential Manager / macOS Keychain), never in the database.
- **HTTPS** uses the OS certificate store, so TLS-inspecting antivirus software (such as Avast Web Shield) and corporate proxies work.
