use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub const DEFAULT_LAUNCHER_SHORTCUT: &str = "Alt+Space";
pub const DEFAULT_CLIPBOARD_SHORTCUT: &str = "Alt+Shift+V";
pub const SHORTCUT_ENV_VAR: &str = "LUX_SHORTCUT";
pub const CLIPBOARD_SHORTCUT_ENV_VAR: &str = "LUX_CLIPBOARD_SHORTCUT";
pub const SHORTCUTS_CONFIG_FILE: &str = "shortcuts.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PersistedShortcuts {
    pub launcher: String,
    pub clipboard: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutStatus {
    pub shortcut: String,
    pub registered: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutConfigStatus {
    pub launcher: ShortcutStatus,
    pub clipboard: ShortcutStatus,
}

pub struct ShortcutsState {
    pub launcher_shortcut: Mutex<String>,
    pub clipboard_shortcut: Mutex<String>,
    pub launcher_status: Mutex<ShortcutStatus>,
    pub clipboard_status: Mutex<ShortcutStatus>,
}

impl ShortcutsState {
    pub fn new(launcher_status: ShortcutStatus, clipboard_status: ShortcutStatus) -> Self {
        Self {
            launcher_shortcut: Mutex::new(launcher_status.shortcut.clone()),
            clipboard_shortcut: Mutex::new(clipboard_status.shortcut.clone()),
            launcher_status: Mutex::new(launcher_status),
            clipboard_status: Mutex::new(clipboard_status),
        }
    }

    pub fn status(&self) -> ShortcutConfigStatus {
        ShortcutConfigStatus {
            launcher: self.launcher_status.lock().unwrap_or_else(|p| p.into_inner()).clone(),
            clipboard: self.clipboard_status.lock().unwrap_or_else(|p| p.into_inner()).clone(),
        }
    }
}

pub fn get_shortcuts_config_path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|dir| dir.join(SHORTCUTS_CONFIG_FILE))
}

pub fn load_persisted_shortcuts_from_path(path: &Path) -> Option<PersistedShortcuts> {
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<PersistedShortcuts>(&content).ok()
}

pub fn save_persisted_shortcuts_to_path(
    path: &Path,
    shortcuts: &PersistedShortcuts,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let data = serde_json::to_string_pretty(shortcuts)
        .map_err(|e| format!("Failed to serialize shortcuts: {e}"))?;
    std::fs::write(path, data)
        .map_err(|e| format!("Failed to write shortcuts file '{}': {e}", path.display()))
}

pub fn resolve_configured_shortcuts<R: Runtime>(app: &AppHandle<R>) -> (String, String) {
    let env_launcher = std::env::var(SHORTCUT_ENV_VAR).ok().and_then(|v| {
        let t = v.trim().to_string();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    });
    let env_clipboard = std::env::var(CLIPBOARD_SHORTCUT_ENV_VAR).ok().and_then(|v| {
        let t = v.trim().to_string();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    });

    let persisted = get_shortcuts_config_path(app)
        .and_then(|path| load_persisted_shortcuts_from_path(&path));

    let launcher = env_launcher
        .or_else(|| persisted.as_ref().map(|p| p.launcher.clone()))
        .unwrap_or_else(|| DEFAULT_LAUNCHER_SHORTCUT.to_string());

    let clipboard = env_clipboard
        .or_else(|| persisted.as_ref().map(|p| p.clipboard.clone()))
        .unwrap_or_else(|| DEFAULT_CLIPBOARD_SHORTCUT.to_string());

    if let Err(err) = validate_shortcut_combination(&launcher, &clipboard) {
        eprintln!(
            "Warning: configured shortcuts '{launcher}' and '{clipboard}' are invalid ({err}); falling back to defaults."
        );
        (
            DEFAULT_LAUNCHER_SHORTCUT.to_string(),
            DEFAULT_CLIPBOARD_SHORTCUT.to_string(),
        )
    } else {
        (launcher, clipboard)
    }
}

pub fn resolve_configured_shortcut() -> String {
    match std::env::var(SHORTCUT_ENV_VAR) {
        Ok(val) if !val.trim().is_empty() => val.trim().to_string(),
        _ => DEFAULT_LAUNCHER_SHORTCUT.to_string(),
    }
}

pub fn resolve_configured_clipboard_shortcut() -> String {
    match std::env::var(CLIPBOARD_SHORTCUT_ENV_VAR) {
        Ok(val) if !val.trim().is_empty() => val.trim().to_string(),
        _ => DEFAULT_CLIPBOARD_SHORTCUT.to_string(),
    }
}

pub fn normalize_shortcut(raw: &str) -> String {
    raw.split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("+")
}

pub fn format_shortcut_display(raw: &str) -> String {
    let parts: Vec<&str> = raw
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        raw.trim().to_string()
    } else {
        parts.join(" + ")
    }
}

pub fn parse_launcher_shortcut(raw: &str) -> Result<Shortcut, String> {
    let normalized = normalize_shortcut(raw);
    if normalized.is_empty() {
        return Err("Shortcut cannot be empty.".to_string());
    }
    Shortcut::from_str(&normalized)
        .map_err(|err| format!("Invalid shortcut '{raw}': {err}"))
}

pub fn validate_shortcut_combination(launcher_raw: &str, clipboard_raw: &str) -> Result<(), String> {
    let launcher_norm = normalize_shortcut(launcher_raw);
    let clipboard_norm = normalize_shortcut(clipboard_raw);

    if launcher_norm.is_empty() {
        return Err("Launcher shortcut cannot be empty.".to_string());
    }
    if clipboard_norm.is_empty() {
        return Err("Clipboard shortcut cannot be empty.".to_string());
    }

    if launcher_norm.eq_ignore_ascii_case(&clipboard_norm) {
        return Err("Launcher and Clipboard shortcuts cannot be identical.".to_string());
    }

    // Windows Win+V reservation check
    #[cfg(target_os = "windows")]
    {
        for (name, s) in [("Launcher", &launcher_norm), ("Clipboard", &clipboard_norm)] {
            let lower = s.to_lowercase();
            if (lower.contains("super") || lower.contains("win")) && lower.ends_with("v") {
                return Err(format!(
                    "{name} shortcut cannot be 'Win + V'. Windows reserves this shortcut for system clipboard history. Please choose an alternative like 'Alt + Shift + V'."
                ));
            }
        }
    }

    parse_launcher_shortcut(&launcher_norm)?;
    parse_launcher_shortcut(&clipboard_norm)?;

    Ok(())
}

pub fn register_all_shortcuts<R: Runtime>(
    app: &AppHandle<R>,
    launcher_raw: &str,
    clipboard_raw: &str,
) -> ShortcutConfigStatus {
    let global_shortcut = app.global_shortcut();
    let _ = global_shortcut.unregister_all();

    let launcher_display = format_shortcut_display(launcher_raw);
    let clipboard_display = format_shortcut_display(clipboard_raw);

    // Register launcher shortcut
    let launcher_status = match parse_launcher_shortcut(launcher_raw) {
        Ok(parsed) => {
            match global_shortcut.on_shortcut(parsed, move |app_handle, _shortcut, event| {
                if event.state == ShortcutState::Pressed
                    && let Err(err) = crate::window::toggle_launcher_window(app_handle)
                {
                    eprintln!("Warning: failed to toggle launcher window: {err}");
                }
            }) {
                Ok(()) => ShortcutStatus {
                    shortcut: launcher_display,
                    registered: true,
                    error: None,
                },
                Err(err) => ShortcutStatus {
                    shortcut: launcher_display,
                    registered: false,
                    error: Some(format!("Failed to register launcher shortcut: {err}")),
                },
            }
        }
        Err(err) => ShortcutStatus {
            shortcut: launcher_display,
            registered: false,
            error: Some(err),
        },
    };

    // Register clipboard shortcut
    let clipboard_status = match parse_launcher_shortcut(clipboard_raw) {
        Ok(parsed) => {
            match global_shortcut.on_shortcut(parsed, move |app_handle, _shortcut, event| {
                if event.state == ShortcutState::Pressed
                    && let Err(err) = crate::window::toggle_clipboard_window(app_handle)
                {
                    eprintln!("Warning: failed to toggle clipboard window: {err}");
                }
            }) {
                Ok(()) => ShortcutStatus {
                    shortcut: clipboard_display,
                    registered: true,
                    error: None,
                },
                Err(err) => ShortcutStatus {
                    shortcut: clipboard_display,
                    registered: false,
                    error: Some(format!("Failed to register clipboard shortcut: {err}")),
                },
            }
        }
        Err(err) => ShortcutStatus {
            shortcut: clipboard_display,
            registered: false,
            error: Some(err),
        },
    };

    ShortcutConfigStatus {
        launcher: launcher_status,
        clipboard: clipboard_status,
    }
}

pub fn unregister_all_shortcuts<R: Runtime>(app: &AppHandle<R>) {
    if let Err(err) = app.global_shortcut().unregister_all() {
        eprintln!("Warning: failed to unregister global shortcuts on exit: {err}");
    }
}

#[tauri::command]
pub fn update_shortcuts<R: Runtime>(
    app: AppHandle<R>,
    launcher: String,
    clipboard: String,
    state: tauri::State<'_, ShortcutsState>,
) -> Result<ShortcutConfigStatus, String> {
    validate_shortcut_combination(&launcher, &clipboard)?;
    let status = register_all_shortcuts(&app, &launcher, &clipboard);

    if let Some(config_path) = get_shortcuts_config_path(&app) {
        let persisted = PersistedShortcuts {
            launcher: launcher.clone(),
            clipboard: clipboard.clone(),
        };
        if let Err(err) = save_persisted_shortcuts_to_path(&config_path, &persisted) {
            eprintln!("Warning: failed to persist shortcuts: {err}");
        }
    }

    *state.launcher_shortcut.lock().unwrap_or_else(|p| p.into_inner()) = launcher;
    *state.clipboard_shortcut.lock().unwrap_or_else(|p| p.into_inner()) = clipboard;
    *state.launcher_status.lock().unwrap_or_else(|p| p.into_inner()) = status.launcher.clone();
    *state.clipboard_status.lock().unwrap_or_else(|p| p.into_inner()) = status.clipboard.clone();

    if let Some(launcher_state) = app.try_state::<crate::window::LauncherState>() {
        launcher_state.update_shortcut_status(status.launcher.clone());
    }

    Ok(status)
}

#[tauri::command]
pub fn get_shortcuts_status(state: tauri::State<'_, ShortcutsState>) -> ShortcutConfigStatus {
    state.status()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_and_formats_shortcut_strings() {
        assert_eq!(normalize_shortcut("Alt + Space"), "Alt+Space");
        assert_eq!(normalize_shortcut("  Ctrl + Shift + Space "), "Ctrl+Shift+Space");
        assert_eq!(format_shortcut_display("Alt+Space"), "Alt + Space");
        assert_eq!(format_shortcut_display("Alt + Space"), "Alt + Space");
        assert_eq!(format_shortcut_display("Alt+Shift+V"), "Alt + Shift + V");
    }

    #[test]
    fn parses_default_launcher_and_clipboard_shortcuts() {
        let parsed_launcher = parse_launcher_shortcut(DEFAULT_LAUNCHER_SHORTCUT);
        assert!(parsed_launcher.is_ok());
        let parsed_clipboard = parse_launcher_shortcut(DEFAULT_CLIPBOARD_SHORTCUT);
        assert!(parsed_clipboard.is_ok());
    }

    #[test]
    fn rejects_empty_or_invalid_shortcuts() {
        assert!(parse_launcher_shortcut("").is_err());
        assert!(parse_launcher_shortcut("   ").is_err());
        assert!(parse_launcher_shortcut("NotAModifier+NotAKey").is_err());
    }

    #[test]
    fn validates_combination_and_detects_conflicts() {
        // Valid distinct shortcuts
        assert!(validate_shortcut_combination("Alt+Space", "Alt+Shift+V").is_ok());

        // Identical shortcuts conflict
        assert!(validate_shortcut_combination("Alt+Space", "Alt+Space").is_err());
        assert!(validate_shortcut_combination("Alt + Space", "Alt+Space").is_err());

        // Empty shortcut rejected
        assert!(validate_shortcut_combination("", "Alt+Shift+V").is_err());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn rejects_win_v_on_windows() {
        assert!(validate_shortcut_combination("Alt+Space", "Super+V").is_err());
    }

    #[test]
    fn persists_shortcuts_round_trip() {
        let temp_dir = std::env::temp_dir().join(format!("lux_test_shortcuts_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);
        let file_path = temp_dir.join("shortcuts.json");

        // Non-existent path returns None
        assert_eq!(load_persisted_shortcuts_from_path(&file_path), None);

        let config = PersistedShortcuts {
            launcher: "Ctrl+Space".to_string(),
            clipboard: "Ctrl+Shift+C".to_string(),
        };

        // Save succeeds
        assert!(save_persisted_shortcuts_to_path(&file_path, &config).is_ok());

        // Load succeeds and matches
        let loaded = load_persisted_shortcuts_from_path(&file_path);
        assert_eq!(loaded, Some(config));

        // Corrupted content gracefully returns None
        std::fs::write(&file_path, "{ broken json }").unwrap();
        assert_eq!(load_persisted_shortcuts_from_path(&file_path), None);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
