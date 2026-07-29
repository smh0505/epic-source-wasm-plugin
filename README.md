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

`launch()` is implemented for contract-completeness (the WIT `source-plugin` interface
requires it) but is architecturally dead code in practice - the host app's generic launch
dispatch handles Epic's real `com.epicgames.launcher://` URI scheme directly via `openUrl()`,
never actually calling a plugin's own `launch()` export. It returns a documented error instead
of calling `host::spawn-process` on the URI, which would just fail anyway - a URI can't be
spawned as a process. No `run-programs` capability needed (Milestone 13's gating) since
`spawn-process` is never actually called.

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
