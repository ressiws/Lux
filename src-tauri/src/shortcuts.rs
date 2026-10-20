use std::str::FromStr;

use serde::Serialize;
use tauri::{AppHandle, Runtime};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

pub const DEFAULT_LAUNCHER_SHORTCUT: &str = "Alt+Space";
pub const SHORTCUT_ENV_VAR: &str = "LUX_SHORTCUT";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutStatus {
    pub shortcut: String,
    pub registered: bool,
    pub error: Option<String>,
}

pub fn resolve_configured_shortcut() -> String {
    match std::env::var(SHORTCUT_ENV_VAR) {
        Ok(val) if !val.trim().is_empty() => val.trim().to_string(),
        _ => DEFAULT_LAUNCHER_SHORTCUT.to_string(),
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

pub fn register_launcher_shortcut<R: Runtime>(
    app: &AppHandle<R>,
    raw_shortcut: &str,
) -> ShortcutStatus {
    let display = format_shortcut_display(raw_shortcut);
    let shortcut = match parse_launcher_shortcut(raw_shortcut) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("Warning: {err}");
            return ShortcutStatus {
                shortcut: display,
                registered: false,
                error: Some(err),
            };
        }
    };

    let global_shortcut = app.global_shortcut();
    if global_shortcut.is_registered(shortcut) {
        return ShortcutStatus {
            shortcut: display,
            registered: true,
            error: None,
        };
    }

    match global_shortcut.on_shortcut(shortcut, move |app_handle, _shortcut, event| {
        if event.state == ShortcutState::Pressed
            && let Err(err) = crate::window::toggle_launcher_window(app_handle)
        {
            eprintln!("Warning: failed to toggle launcher window: {err}");
        }
    }) {
        Ok(()) => ShortcutStatus {
            shortcut: display,
            registered: true,
            error: None,
        },
        Err(err) => {
            let message = format!(
                "Could not register global shortcut '{display}' ({err}). Use the system tray icon to open Lux."
            );
            eprintln!("Warning: {message}");
            ShortcutStatus {
                shortcut: display,
                registered: false,
                error: Some(message),
            }
        }
    }
}

pub fn unregister_all_shortcuts<R: Runtime>(app: &AppHandle<R>) {
    if let Err(err) = app.global_shortcut().unregister_all() {
        eprintln!("Warning: failed to unregister global shortcuts on exit: {err}");
    }
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
    }

    #[test]
    fn parses_default_launcher_shortcut() {
        let parsed = parse_launcher_shortcut(DEFAULT_LAUNCHER_SHORTCUT);
        assert!(parsed.is_ok());
        let spaced = parse_launcher_shortcut("Alt + Space");
        assert_eq!(parsed, spaced);
    }

    #[test]
    fn rejects_empty_or_invalid_shortcuts() {
        assert!(parse_launcher_shortcut("").is_err());
        assert!(parse_launcher_shortcut("   ").is_err());
        assert!(parse_launcher_shortcut("NotAModifier+NotAKey").is_err());
    }
}
