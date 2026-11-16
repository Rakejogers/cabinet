# Cabinet: first version brief

Build the first clean version of Cabinet, a custom cross-store game launcher, as a Tauri 2 desktop app.

## Context

- Developing on Windows 11 right now. The app must later also build on macOS from the same codebase, so keep all OS-specific code isolated.
- Goals: very fast and light, easy to set up, themeable, and showing games from every launcher in one library with stats.
- The owner dislikes Playnite's UI, so don't copy it. Aim for a clean, modern grid-based library.
- The project lives in this folder, `G:\CS\Projects\cabinet`. It contains only this brief so far. Scaffold into it and initialise a git repo here.

## Setup

- First check that Rust, Node, the Visual Studio C++ Build Tools and WebView2 are installed. Report what's missing before installing anything.
- Scaffold with create-tauri-app: Tauri 2, Svelte + TypeScript frontend. App and package name: `cabinet`.

## Architecture

- Rust backend with a `GameSource` trait (`scan() -> Vec<Game>`, `launch(&Game) -> Result<()>`), one implementation per store, with `#[cfg(target_os = ...)]` only where paths or launch methods differ.
- SQLite (rusqlite or sqlx) for the library and playtime data.
- Themes as CSS variables, loaded from theme files, with a switcher in the UI.

## Scope of the first version

1. Steam scanner: parse `libraryfolders.vdf` and `appmanifest_*.acf`, launch via `steam://rungameid/<id>`.
2. Epic scanner: read the `.item` JSON manifests in `C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests`.
3. Library persisted in SQLite, rescanned on startup and on demand.
4. Game grid UI with cover art, search, and click to launch. Cache covers locally.
5. Two built-in themes (dark and light) plus the switcher.
6. Basic playtime tracking by watching the launched game's process (`sysinfo`), shown per game.

Out of scope for now: other stores, owned-but-not-installed games, achievements, macOS builds, installers and signing.

## Done means

`cargo tauri dev` runs on this machine, the installed Steam and Epic games appear with covers, launching works, and the theme switcher works. Run it and verify against the real library before reporting back, and report anything that didn't work.
