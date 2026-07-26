//! WASM `SourcePlugin` for Epic Games, ported from the game-library-client's built-in
//! `epic.rs` + `src/plugins/epic/index.ts`. Epic Games Launcher writes one `*.item` JSON
//! manifest per installed game to `%PROGRAMDATA%\Epic\EpicGamesLauncher\Data\Manifests` -
//! confirmed against Playnite's EpicLibrary source (LauncherInstalled.cs/EpicLauncher.cs),
//! since Epic doesn't document this format itself. All filesystem access goes through the
//! `host` interface instead of `std::fs` directly, since guest code is sandboxed.
//!
//! `launch()` is implemented for contract-completeness but is dead code in practice, same
//! reasoning as the Steam/GOG ports: the host app's generic launch dispatch handles Epic's
//! real `com.epicgames.launcher://` URI scheme directly via `openUrl()`, never actually
//! calling a plugin's own `launch()` export.

#[allow(warnings)]
mod bindings;

use bindings::exports::gamelib::plugin::source_plugin::{GameEntry, Guest};
use bindings::gamelib::plugin::host;
use serde::Deserialize;

struct EpicPlugin;

#[derive(Deserialize)]
struct EpicManifestFile {
    #[serde(rename = "AppName")]
    app_name: Option<String>,
    #[serde(rename = "DisplayName")]
    display_name: Option<String>,
    #[serde(rename = "InstallLocation")]
    install_location: Option<String>,
}

#[derive(Debug, PartialEq)]
struct EpicApp {
    app_name: String,
    display_name: String,
    install_location: String,
}

/// `%PROGRAMDATA%` is a fixed Windows system directory - virtually always `C:\ProgramData`
/// and not something users relocate in practice, so this is hardcoded rather than needing a
/// new host primitive just to read one environment variable (matches the Steam port's own
/// hardcoded install-path fallback for the same reasoning: a narrow, well-known constant
/// isn't worth a new capability).
fn manifests_dir() -> String {
    r"C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests".to_string()
}

fn parse_manifest_content(content: &str) -> Option<EpicApp> {
    let manifest: EpicManifestFile = serde_json::from_str(content).ok()?;
    Some(EpicApp {
        app_name: manifest.app_name?,
        display_name: manifest.display_name?,
        install_location: manifest.install_location?,
    })
}

fn find_epic_apps() -> Vec<EpicApp> {
    let dir = manifests_dir();
    let Ok(entries) = host::list_dir(&dir) else {
        return Vec::new(); // Epic not installed / no games installed yet
    };

    let mut apps = Vec::new();
    for path in entries {
        let is_item_file = path
            .rsplit('.')
            .next()
            .map(|ext| ext.eq_ignore_ascii_case("item"))
            .unwrap_or(false);
        if !is_item_file {
            continue;
        }

        // Epic's client sometimes writes an empty/corrupt manifest; skip those rather than
        // failing the whole scan.
        if let Ok(content) = host::read_file(&path) {
            if let Some(app) = parse_manifest_content(&content) {
                apps.push(app);
            }
        }
    }

    apps
}

fn to_game_entry(app: &EpicApp) -> GameEntry {
    GameEntry {
        id: format!("epic-{}", app.app_name),
        title: app.display_name.clone(),
        // Epic doesn't expose the resolved launch executable/args locally, only the install
        // folder - the host app's generic launch dispatch handles this URI directly, not
        // this plugin's own launch() export (see the module doc comment).
        executable_path: format!(
            "com.epicgames.launcher://apps/{}?action=launch&silent=true",
            app.app_name
        ),
        platform: "epic".to_string(),
        cover_art_url: None,
        install_dir: Some(app.install_location.clone()),
    }
}

impl Guest for EpicPlugin {
    fn scan() -> Result<Vec<GameEntry>, String> {
        Ok(find_epic_apps().iter().map(to_game_entry).collect())
    }

    fn launch(entry: GameEntry) -> Result<(), String> {
        host::spawn_process(&entry.executable_path, &[])
    }

    fn get_install_status(entry: GameEntry) -> Result<bool, String> {
        Ok(find_epic_apps()
            .iter()
            .any(|app| format!("epic-{}", app.app_name) == entry.id))
    }
}

bindings::export!(EpicPlugin with_types_in bindings);

// Pure-Rust parsing logic (no host:: calls), so this can run under plain `cargo test`
// without needing a live wasmtime instantiation - same real manifest shape epic.rs's own
// test used (verified against Playnite's InstalledManifest model).
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_real_manifest_shape() {
        let content = r#"{
            "FormatVersion": 0,
            "bIsIncompleteInstall": false,
            "AppVersionString": "1.7.1.0",
            "LaunchCommand": "",
            "LaunchExecutable": "FarmingSimulator2019.exe",
            "ManifestLocation": "C:\\Games\\Epic\\FarmingSimulator19/.egstore",
            "bIsApplication": true,
            "bIsExecutable": true,
            "bRequiresAuth": true,
            "bCanRunOffline": true,
            "AppName": "Stellula",
            "DisplayName": "Farming Simulator 19",
            "InstallLocation": "C:\\Games\\Epic\\FarmingSimulator19"
        }"#;

        let app = parse_manifest_content(content).unwrap();
        assert_eq!(
            app,
            EpicApp {
                app_name: "Stellula".to_string(),
                display_name: "Farming Simulator 19".to_string(),
                install_location: "C:\\Games\\Epic\\FarmingSimulator19".to_string(),
            }
        );
    }

    #[test]
    fn rejects_manifest_missing_required_fields() {
        let content = r#"{ "AppName": "Stellula" }"#;
        assert!(parse_manifest_content(content).is_none());
    }

    #[test]
    fn rejects_empty_manifest() {
        assert!(parse_manifest_content("").is_none());
    }
}
