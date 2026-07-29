# epic-source-wasm-plugin

A `SourcePlugin` for [Concourse](https://github.com/smh0505/Concourse) implemented as a WASM component, ported from that
project's built-in `epic.rs` + `src/plugins/epic/index.ts`. Reads Epic Games Launcher's
`*.item` JSON manifests from `%PROGRAMDATA%\Epic\EpicGamesLauncher\Data\Manifests` for
installed games - same logic as the built-in version, just running sandboxed via `wasmtime`
instead of compiled directly into the host app.

This is a real, separate repo on purpose - same reasoning as `steam-source-wasm-plugin`/
`gog-source-wasm-plugin`: a plugin whose source lives inside the host app's own repo isn't
genuinely exercising the "install arbitrary third-party code" model the WASM plugin system is
for.

`%PROGRAMDATA%` is hardcoded to `C:\ProgramData` rather than read via an environment variable
- it's a fixed Windows system directory users don't relocate in practice, and reading it
properly would need a new host primitive (`env-var`) just for one well-known constant. Same
reasoning as the Steam port's own hardcoded install-path fallback.

`launch()` is implemented for contract-completeness but is dead code in practice - the host
app's generic launch dispatch handles Epic's real `com.epicgames.launcher://` URI scheme
directly via `openUrl()`, never actually calling a plugin's own `launch()` export.

## Permissions

Declares the `run-programs` capability (`plugin.json`'s `capabilities` field) since `launch()`
calls `host::spawn-process` - even though, per the note above, that path is currently
unreachable via the host's own `com.epicgames.launcher://` URI dispatch. Concourse gates
`spawn-process`/`run-and-wait` behind an explicit, visible per-plugin grant (Milestone 13) -
installing via URL prompts for it in the confirm dialog; an already-installed copy (dropped in
manually) shows a "Permission needed" row with a Grant button in Settings until granted once.

## Building

```sh
rustup target add wasm32-wasip1   # once
cargo install cargo-component     # once
cargo component build
```

Output: `target/wasm32-wasip1/debug/epic_source_wasm_plugin.wasm`.

Manifest-parsing logic is pure Rust (no `host::` calls) and has its own `cargo test` suite,
using the same real manifest field shape `epic.rs`'s own test used.

## Installing into a running Concourse

Either build locally (above) or grab the prebuilt `.wasm` + `plugin.json` from this repo's
[Releases](https://github.com/smh0505/epic-source-wasm-plugin/releases) - CI (`.github/workflows/publish.yml`) publishes a new release
automatically whenever `plugin.json`'s `version` is bumped on `main`. Concourse's Settings ->
Source tab -> Add Plugin also accepts a Release's `plugin.json` URL directly (source-kind
plugins install by URL) - the latest one always lives at:

```
https://github.com/smh0505/epic-source-wasm-plugin/releases/latest/download/plugin.json
```

Copy the compiled `.wasm` and `plugin.json` into
`<app data dir>/wasm-plugins/source/epic-wasm/` (Windows:
`%APPDATA%\com.bloppy.concourse\wasm-plugins\source\epic-wasm\`). It'll show up in Settings' Plugins
panel next time the app starts, as **Epic Games**.

## Versioning

Plain SemVer (`Cargo.toml` + `plugin.json`'s `version`), independent of Concourse's own
milestone-tracked version - patch for fixes, minor for backward-compatible new capabilities,
major for breaking manifest/WIT interface changes. Full convention:
[`.claude/CLAUDE.md`](https://github.com/smh0505/Concourse/blob/main/.claude/CLAUDE.md) (Plugin Versioning) in the main [Concourse](https://github.com/smh0505/Concourse) repo.
