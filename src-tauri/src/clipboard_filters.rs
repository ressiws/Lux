use std::path::{Path, PathBuf};
use std::sync::RwLock;
use regex::Regex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

pub const CLIPBOARD_FILTERS_CONFIG_FILE: &str = "clipboard_filters.json";
pub const MAX_INSPECT_CHARS: usize = 30_000;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ExclusionMode {
    #[default]
    Ignore,
    Hide,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CustomFilter {
    pub id: String,
    pub name: String,
    pub pattern: String,
    pub is_regex: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardFiltersConfig {
    pub mode: ExclusionMode,
    pub filter_secrets: bool,
    pub filter_camel_case: bool,
    pub filter_credit_cards: bool,
    pub custom_filters: Vec<CustomFilter>,
}

impl Default for ClipboardFiltersConfig {
    fn default() -> Self {
        Self {
            mode: ExclusionMode::Ignore,
            filter_secrets: true,
            filter_camel_case: false,
            filter_credit_cards: true,
            custom_filters: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FilterValidationResult {
    pub valid: bool,
    pub error: Option<String>,
    pub matches_test: Option<bool>,
}

pub struct ActiveFilterItem {
    pub name: String,
    pub regex: Regex,
}

pub struct CompiledFilters {
    pub mode: ExclusionMode,
    pub items: Vec<ActiveFilterItem>,
}

impl CompiledFilters {
    pub fn from_config(config: &ClipboardFiltersConfig) -> Self {
        let mut items = Vec::new();

        if config.filter_secrets {
            let secrets_pattern = r"(?i)(?:-----BEGIN (?:[A-Z0-9_-]+ )?PRIVATE KEY-----|gh[pousr]_[A-Za-z0-9_]{36,}|AKIA[0-9A-Z]{16}|bearer\s+[a-z0-9_\-\.]{20,}|(?:password|passwd|secret|api_key|apikey|access_token|private_key)\s*[:=]\s*\S+)";
            if let Ok(re) = Regex::new(secrets_pattern) {
                items.push(ActiveFilterItem {
                    name: "Secrets & Tokens".to_string(),
                    regex: re,
                });
            }
        }

        if config.filter_camel_case {
            let camel_pattern = r"\b[a-z]+(?:[A-Z][a-z0-9]+)+\b";
            if let Ok(re) = Regex::new(camel_pattern) {
                items.push(ActiveFilterItem {
                    name: "camelCase".to_string(),
                    regex: re,
                });
            }
        }

        if config.filter_credit_cards {
            let cc_pattern = r"\b(?:4[0-9]{12}(?:[0-9]{3})?|5[1-5][0-9]{14}|3[47][0-9]{13}|3(?:0[0-5]|[68][0-9])[0-9]{11}|6(?:011|5[0-9]{2})[0-9]{12})\b|\b(?:\d{4}[ -]?){3}\d{4}\b";
            if let Ok(re) = Regex::new(cc_pattern) {
                items.push(ActiveFilterItem {
                    name: "Credit Card".to_string(),
                    regex: re,
                });
            }
        }

        for custom in &config.custom_filters {
            if !custom.enabled || custom.pattern.trim().is_empty() {
                continue;
            }
            let pattern = if custom.is_regex {
                custom.pattern.clone()
            } else {
                regex::escape(&custom.pattern)
            };
            if let Ok(re) = Regex::new(&pattern) {
                items.push(ActiveFilterItem {
                    name: if custom.name.trim().is_empty() {
                        custom.pattern.clone()
                    } else {
                        custom.name.clone()
                    },
                    regex: re,
                });
            }
        }

        Self {
            mode: config.mode,
            items,
        }
    }

    /// Evaluates text against all active filters up to MAX_INSPECT_CHARS.
    /// Returns the name of the first matched filter if any.
    pub fn matches(&self, text: &str) -> Option<String> {
        if self.items.is_empty() {
            return None;
        }

        // Bound evaluation length to protect background thread
        let slice = if text.len() > MAX_INSPECT_CHARS {
            &text[..MAX_INSPECT_CHARS]
        } else {
            text
        };

        for item in &self.items {
            if item.regex.is_match(slice) {
                return Some(item.name.clone());
            }
        }

        None
    }
}

pub struct ClipboardFiltersState {
    pub config: RwLock<ClipboardFiltersConfig>,
    pub compiled: RwLock<CompiledFilters>,
}

impl ClipboardFiltersState {
    pub fn new(config: ClipboardFiltersConfig) -> Self {
        let compiled = CompiledFilters::from_config(&config);
        Self {
            config: RwLock::new(config),
            compiled: RwLock::new(compiled),
        }
    }

    pub fn get_config(&self) -> ClipboardFiltersConfig {
        self.config.read().unwrap_or_else(|p| p.into_inner()).clone()
    }

    pub fn update_config(&self, new_config: ClipboardFiltersConfig) {
        let new_compiled = CompiledFilters::from_config(&new_config);
        *self.config.write().unwrap_or_else(|p| p.into_inner()) = new_config;
        *self.compiled.write().unwrap_or_else(|p| p.into_inner()) = new_compiled;
    }

    pub fn matches(&self, text: &str) -> Option<(ExclusionMode, String)> {
        let compiled = self.compiled.read().unwrap_or_else(|p| p.into_inner());
        let mode = compiled.mode;
        compiled.matches(text).map(|name| (mode, name))
    }
}

pub fn get_clipboard_filters_path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join(CLIPBOARD_FILTERS_CONFIG_FILE))
}

pub fn load_persisted_filters_from_path(path: &Path) -> Option<ClipboardFiltersConfig> {
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<ClipboardFiltersConfig>(&content).ok()
}

pub fn save_persisted_filters_to_path(
    path: &Path,
    config: &ClipboardFiltersConfig,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let data = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize clipboard filters config: {e}"))?;
    std::fs::write(path, data)
        .map_err(|e| format!("Failed to write clipboard filters file '{}': {e}", path.display()))
}

pub fn load_or_default_filters_config<R: Runtime>(app: &AppHandle<R>) -> ClipboardFiltersConfig {
    get_clipboard_filters_path(app)
        .and_then(|path| load_persisted_filters_from_path(&path))
        .unwrap_or_default()
}

#[tauri::command]
pub fn get_clipboard_filters_config<R: Runtime>(
    app: AppHandle<R>,
) -> Result<ClipboardFiltersConfig, String> {
    app.try_state::<ClipboardFiltersState>()
        .map(|s| s.get_config())
        .ok_or_else(|| "ClipboardFiltersState not initialized".to_string())
}

#[tauri::command]
pub fn save_clipboard_filters_config<R: Runtime>(
    app: AppHandle<R>,
    config: ClipboardFiltersConfig,
) -> Result<(), String> {
    if let Some(state) = app.try_state::<ClipboardFiltersState>() {
        state.update_config(config.clone());
    }

    if let Some(path) = get_clipboard_filters_path(&app) {
        save_persisted_filters_to_path(&path, &config)?;
    }

    Ok(())
}

#[tauri::command]
pub fn validate_clipboard_filter(
    pattern: String,
    is_regex: bool,
    test_text: Option<String>,
) -> FilterValidationResult {
    let re_pattern = if is_regex {
        pattern
    } else {
        regex::escape(&pattern)
    };

    match Regex::new(&re_pattern) {
        Ok(re) => {
            let matches_test = test_text.as_deref().map(|t| re.is_match(t));
            FilterValidationResult {
                valid: true,
                error: None,
                matches_test,
            }
        }
        Err(err) => FilterValidationResult {
            valid: false,
            error: Some(err.to_string()),
            matches_test: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predefined_secrets() {
        let config = ClipboardFiltersConfig {
            filter_secrets: true,
            filter_camel_case: false,
            filter_credit_cards: false,
            ..Default::default()
        };
        let compiled = CompiledFilters::from_config(&config);

        assert!(compiled.matches("-----BEGIN RSA PRIVATE KEY-----").is_some());
        assert!(compiled.matches("AKIAIOSFODNN7EXAMPLE").is_some());
        assert!(compiled.matches("ghp_1234567890abcdefghijklmnopqrstuvwxyz").is_some());
        assert!(compiled.matches("password = mySecretPass123!").is_some());
        assert!(compiled.matches("regular benign text without secrets").is_none());
    }

    #[test]
    fn test_predefined_camel_case() {
        let config = ClipboardFiltersConfig {
            filter_secrets: false,
            filter_camel_case: true,
            filter_credit_cards: false,
            ..Default::default()
        };
        let compiled = CompiledFilters::from_config(&config);

        assert!(compiled.matches("myVariableName").is_some());
        assert!(compiled.matches("anotherCoolIdentifier").is_some());
        assert!(compiled.matches("normal text lowercase").is_none());
    }

    #[test]
    fn test_predefined_credit_card() {
        let config = ClipboardFiltersConfig {
            filter_secrets: false,
            filter_camel_case: false,
            filter_credit_cards: true,
            ..Default::default()
        };
        let compiled = CompiledFilters::from_config(&config);

        assert!(compiled.matches("4111 2222 3333 4444").is_some());
        assert!(compiled.matches("5500-0000-0000-0004").is_some());
        assert!(compiled.matches("not a card 12345").is_none());
    }

    #[test]
    fn test_custom_literal_filter() {
        let config = ClipboardFiltersConfig {
            filter_secrets: false,
            filter_camel_case: false,
            filter_credit_cards: false,
            custom_filters: vec![CustomFilter {
                id: "1".into(),
                name: "Internal Secret".into(),
                pattern: "SUPER_INTERNAL_KEY".into(),
                is_regex: false,
                enabled: true,
            }],
            ..Default::default()
        };
        let compiled = CompiledFilters::from_config(&config);

        assert_eq!(
            compiled.matches("Contains SUPER_INTERNAL_KEY here"),
            Some("Internal Secret".to_string())
        );
        assert!(compiled.matches("Normal stuff").is_none());
    }

    #[test]
    fn test_custom_regex_validation() {
        let valid = validate_clipboard_filter(r"^\d{3}-\d{2}-\d{4}$".into(), true, Some("123-45-6789".into()));
        assert!(valid.valid);
        assert_eq!(valid.matches_test, Some(true));

        let invalid = validate_clipboard_filter("[unclosed bracket".into(), true, None);
        assert!(!invalid.valid);
        assert!(invalid.error.is_some());
    }

    #[test]
    fn test_massive_payload_truncation() {
        let config = ClipboardFiltersConfig {
            filter_secrets: true,
            ..Default::default()
        };
        let compiled = CompiledFilters::from_config(&config);
        let massive = "a".repeat(100_000);
        assert!(compiled.matches(&massive).is_none());
    }
}
