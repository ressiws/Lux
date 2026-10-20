use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub const FEATURES_CONFIG_FILE: &str = "features.json";
pub const EVENT_FEATURES_CHANGED: &str = "lux://features-changed";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FeaturesConfig {
    #[serde(default = "default_true")]
    pub clipboard_history: bool,
}

fn default_true() -> bool {
    true
}

impl Default for FeaturesConfig {
    fn default() -> Self {
        Self {
            clipboard_history: true,
        }
    }
}

pub struct FeaturesState {
    pub clipboard_history: Arc<AtomicBool>,
}

impl FeaturesState {
    pub fn new(config: &FeaturesConfig) -> Self {
        Self {
            clipboard_history: Arc::new(AtomicBool::new(config.clipboard_history)),
        }
    }

    pub fn is_clipboard_history_enabled(&self) -> bool {
        self.clipboard_history.load(Ordering::SeqCst)
    }

    pub fn set_clipboard_history_enabled(&self, enabled: bool) {
        self.clipboard_history.store(enabled, Ordering::SeqCst);
    }
}

pub fn get_features_config_path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|dir| dir.join(FEATURES_CONFIG_FILE))
}

pub fn load_persisted_features_from_path(path: &Path) -> Option<FeaturesConfig> {
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<FeaturesConfig>(&content).ok()
}

pub fn save_persisted_features_to_path(
    path: &Path,
    config: &FeaturesConfig,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let data = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize features config: {e}"))?;
    std::fs::write(path, data)
        .map_err(|e| format!("Failed to write features config file '{}': {e}", path.display()))
}

pub fn load_or_default_features_config<R: Runtime>(app: &AppHandle<R>) -> FeaturesConfig {
    get_features_config_path(app)
        .and_then(|path| load_persisted_features_from_path(&path))
        .unwrap_or_default()
}

#[tauri::command]
pub fn get_features_config<R: Runtime>(app: AppHandle<R>) -> FeaturesConfig {
    let enabled = app
        .try_state::<FeaturesState>()
        .map(|s| s.is_clipboard_history_enabled())
        .unwrap_or(true);
    FeaturesConfig {
        clipboard_history: enabled,
    }
}

#[tauri::command]
pub fn set_feature_enabled<R: Runtime>(
    app: AppHandle<R>,
    feature: String,
    enabled: bool,
) -> Result<FeaturesConfig, String> {
    match feature.as_str() {
        "clipboardHistory" | "clipboard_history" => {
            if let Some(state) = app.try_state::<FeaturesState>() {
                state.set_clipboard_history_enabled(enabled);
            }

            let config = FeaturesConfig {
                clipboard_history: enabled,
            };

            if let Some(path) = get_features_config_path(&app) {
                let _ = save_persisted_features_to_path(&path, &config);
            }

            // Sync clipboard background monitor
            if let Some(clipboard_state) = app.try_state::<crate::clipboard::ClipboardState>() {
                if enabled {
                    clipboard_state.start_monitor(app.clone());
                } else {
                    clipboard_state.stop_monitor();
                }
            }

            // Sync shortcut registration
            if let Some(shortcuts_state) = app.try_state::<crate::shortcuts::ShortcutsState>() {
                let launcher = shortcuts_state
                    .launcher_shortcut
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .clone();
                let clipboard = shortcuts_state
                    .clipboard_shortcut
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .clone();
                let status = crate::shortcuts::register_all_shortcuts(&app, &launcher, &clipboard);
                *shortcuts_state
                    .launcher_status
                    .lock()
                    .unwrap_or_else(|p| p.into_inner()) = status.launcher.clone();
                *shortcuts_state
                    .clipboard_status
                    .lock()
                    .unwrap_or_else(|p| p.into_inner()) = status.clipboard.clone();
            }

            let _ = app.emit(EVENT_FEATURES_CHANGED, &config);
            Ok(config)
        }
        _ => Err(format!("Unknown feature: {feature}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_features_config_defaults() {
        let config = FeaturesConfig::default();
        assert!(config.clipboard_history);
    }

    #[test]
    fn test_features_config_deserialization_defaults() {
        let empty_json = "{}";
        let parsed: FeaturesConfig = serde_json::from_str(empty_json).expect("should parse empty json");
        assert!(parsed.clipboard_history);
    }

    #[test]
    fn test_features_config_roundtrip() {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("lux_test_features_{unique_id}"));
        let config_file = temp_dir.join("features.json");

        let initial = FeaturesConfig {
            clipboard_history: false,
        };

        save_persisted_features_to_path(&config_file, &initial).expect("should save");
        let loaded = load_persisted_features_from_path(&config_file).expect("should load");
        assert_eq!(initial, loaded);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_features_state_toggle() {
        let config = FeaturesConfig {
            clipboard_history: true,
        };
        let state = FeaturesState::new(&config);
        assert!(state.is_clipboard_history_enabled());

        state.set_clipboard_history_enabled(false);
        assert!(!state.is_clipboard_history_enabled());

        state.set_clipboard_history_enabled(true);
        assert!(state.is_clipboard_history_enabled());
    }
}
