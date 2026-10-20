use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

const MAX_DIR_DEPTH: usize = 3;
const MAX_INDEXED_FILES: usize = 15000;
const MAX_RESULTS_PER_CATEGORY: usize = 8;

const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "webp", "gif", "bmp", "svg", "ico", "tiff", "tif", "avif",
];

const IGNORED_DIR_NAMES: &[&str] = &[
    ".git",
    ".svn",
    ".hg",
    ".cache",
    ".config",
    "node_modules",
    "target",
    "build",
    "dist",
    "out",
    "bin",
    "obj",
    "appdata",
    "local settings",
    "__pycache__",
    "venv",
    ".venv",
    "env",
    ".env",
    "vendor",
    "tmp",
    "temp",
    "windows",
    "system volume information",
    "$recycle.bin",
];

const EXCLUDED_APP_PATTERNS: &[&str] = &[
    "uninstall",
    "unins000",
    "remove",
    "readme",
    "documentation",
    "manual",
    "release notes",
    "visit website",
    "help",
    "license",
    "about",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub apps: Vec<SearchResultItem>,
    pub files: Vec<SearchResultItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultItem {
    pub id: String,
    pub category: String, // "app" | "file"
    pub name: String,
    pub subtitle: String,
    pub path: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub is_image: bool,
    pub right_label: String,
}

#[derive(Debug, Clone)]
pub struct IndexedApp {
    pub name: String,
    pub subtitle: String,
    pub path: PathBuf,
    pub exec: Option<String>,
    pub icon: Option<String>,
    pub color: String,
    pub keywords: String,
}

#[derive(Debug, Clone)]
pub struct IndexedFile {
    pub name: String,
    pub subtitle: String,
    pub path: PathBuf,
    pub is_image: bool,
    pub extension: String,
    pub right_label: String,
}

#[derive(Default)]
pub struct SearchIndex {
    pub apps: Vec<IndexedApp>,
    pub files: Vec<IndexedFile>,
    pub is_indexed: bool,
}

pub struct SearchState {
    pub index: Arc<RwLock<SearchIndex>>,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            index: Arc::new(RwLock::new(SearchIndex::default())),
        }
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}

pub fn is_image_extension(ext: &str) -> bool {
    let lower = ext.to_lowercase();
    IMAGE_EXTENSIONS.contains(&lower.as_str())
}

pub fn format_display_path(path: &Path, home_dir: Option<&Path>) -> String {
    if let Some(home) = home_dir
        && let Ok(relative) = path.strip_prefix(home)
    {
        let rel_str = relative.to_string_lossy().replace('\\', "/");
        if rel_str.is_empty() {
            return "~".to_string();
        }
        return format!("~/{}", rel_str);
    }
    path.to_string_lossy().replace('\\', "/")
}

fn assign_app_color(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.contains("discord") {
        return "#5865f2".to_string();
    }
    if lower.contains("firefox") {
        return "#e8663d".to_string();
    }
    if lower.contains("code") || lower.contains("visual studio") {
        return "#2f80d1".to_string();
    }
    if lower.contains("spotify") {
        return "#1db954".to_string();
    }
    if lower.contains("chrome") {
        return "#ea4335".to_string();
    }
    if lower.contains("terminal") || lower.contains("powershell") || lower.contains("cmd") {
        return "#3a3a40".to_string();
    }
    if lower.contains("explorer") || lower.contains("files") {
        return "#4a8fd8".to_string();
    }
    if lower.contains("gimp") {
        return "#7a6a55".to_string();
    }
    if lower.contains("slack") {
        return "#611f69".to_string();
    }

    // Palette of balanced, modern launcher colors
    const PALETTE: &[&str] = &[
        "#4a8fd8", "#e8663d", "#2f9e5b", "#c9892b", "#8b5cf6", "#ec4899", "#06b6d4", "#f59e0b",
        "#10b981", "#6366f1", "#14b8a6", "#f43f5e",
    ];

    let hash: usize = lower
        .bytes()
        .fold(0usize, |acc, b| acc.wrapping_add(b as usize));
    PALETTE[hash % PALETTE.len()].to_string()
}

pub fn should_exclude_app_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    EXCLUDED_APP_PATTERNS
        .iter()
        .any(|pattern| lower.contains(pattern))
}

pub fn score_match(name: &str, keywords: &str, query: &str) -> u32 {
    let name_low = name.to_lowercase();
    let query_low = query.to_lowercase();

    if name_low == query_low {
        return 100;
    }
    if name_low.starts_with(&query_low) {
        return 80;
    }
    let words = name_low.split(|c: char| !c.is_alphanumeric());
    if words.into_iter().any(|w| w.starts_with(&query_low)) {
        return 60;
    }
    if name_low.contains(&query_low) {
        return 40;
    }
    if !keywords.is_empty() {
        let kw_low = keywords.to_lowercase();
        let kw_words = kw_low.split(|c: char| !c.is_alphanumeric());
        if kw_words.into_iter().any(|w| w.starts_with(&query_low)) {
            return 25;
        }
        if kw_low.contains(&query_low) {
            return 15;
        }
    }
    0
}

fn walk_directory_files(
    dir: &Path,
    current_depth: usize,
    max_depth: usize,
    home_dir: Option<&Path>,
    indexed_files: &mut Vec<IndexedFile>,
) {
    if current_depth > max_depth || indexed_files.len() >= MAX_INDEXED_FILES {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(read_dir) => read_dir,
        Err(_) => return, // Permission denied or inaccessible folder
    };

    for entry in entries.flatten() {
        if indexed_files.len() >= MAX_INDEXED_FILES {
            break;
        }

        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();

        if name_str.starts_with('.') {
            continue;
        }

        let path = entry.path();

        if file_type.is_dir() {
            let lower_name = name_str.to_lowercase();
            if IGNORED_DIR_NAMES.contains(&lower_name.as_str()) {
                continue;
            }
            walk_directory_files(
                &path,
                current_depth + 1,
                max_depth,
                home_dir,
                indexed_files,
            );
        } else if file_type.is_file() {
            let ext = path
                .extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default();

            let is_image = is_image_extension(&ext);
            let right_label = if is_image {
                "Image".to_string()
            } else if !ext.is_empty() {
                ext.to_uppercase()
            } else {
                "File".to_string()
            };

            let subtitle = format_display_path(&path, home_dir);

            indexed_files.push(IndexedFile {
                name: name_str.to_string(),
                subtitle,
                path,
                is_image,
                extension: ext,
                right_label,
            });
        }
    }
}

pub fn get_user_home_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

pub fn get_user_search_directories(home_dir: &Option<PathBuf>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(home) = home_dir {
        let desktop = home.join("Desktop");
        let documents = home.join("Documents");
        let downloads = home.join("Downloads");
        let pictures = home.join("Pictures");

        if desktop.exists() {
            dirs.push(desktop);
        }
        if documents.exists() {
            dirs.push(documents);
        }
        if downloads.exists() {
            dirs.push(downloads);
        }
        if pictures.exists() {
            dirs.push(pictures);
        }
    }

    dirs
}

#[cfg(target_os = "windows")]
fn scan_windows_start_menu_apps(home_dir: Option<&Path>) -> Vec<IndexedApp> {
    let mut apps = Vec::new();
    let mut seen_names = HashSet::new();

    let mut start_menu_roots = Vec::new();

    if let Some(appdata) = std::env::var_os("APPDATA") {
        start_menu_roots.push(
            PathBuf::from(appdata).join("Microsoft\\Windows\\Start Menu\\Programs"),
        );
    }
    if let Some(programdata) = std::env::var_os("PROGRAMDATA") {
        start_menu_roots.push(
            PathBuf::from(programdata).join("Microsoft\\Windows\\Start Menu\\Programs"),
        );
    }

    fn walk_shortcuts(
        dir: &Path,
        depth: usize,
        home_dir: Option<&Path>,
        seen_names: &mut HashSet<String>,
        apps: &mut Vec<IndexedApp>,
    ) {
        if depth > 4 {
            return;
        }
        let entries = match fs::read_dir(dir) {
            Ok(rd) => rd,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            let path = entry.path();
            let name_str = entry.file_name().to_string_lossy().to_string();

            if file_type.is_dir() {
                let lower = name_str.to_lowercase();
                if !IGNORED_DIR_NAMES.contains(&lower.as_str()) {
                    walk_shortcuts(&path, depth + 1, home_dir, seen_names, apps);
                }
            } else if file_type.is_file()
                && let Some(ext) = path.extension()
                && ext.eq_ignore_ascii_case("lnk")
            {
                let stem = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| name_str.clone());

                if should_exclude_app_name(&stem) {
                    continue;
                }

                let norm_name = stem.to_lowercase();
                if seen_names.insert(norm_name) {
                    let color = assign_app_color(&stem);
                    let subtitle = format_display_path(&path, home_dir);
                    apps.push(IndexedApp {
                        name: stem,
                        subtitle,
                        path,
                        exec: None,
                        icon: None,
                        color,
                        keywords: String::new(),
                    });
                }
            }
        }
    }

    for root in start_menu_roots {
        if root.exists() {
            walk_shortcuts(&root, 0, home_dir, &mut seen_names, &mut apps);
        }
    }

    apps.sort_by(|a, b| a.name.cmp(&b.name));
    apps
}

#[cfg(not(target_os = "windows"))]
fn scan_linux_desktop_apps(home_dir: Option<&Path>) -> Vec<IndexedApp> {
    let mut apps = Vec::new();
    let mut seen_ids = HashSet::new();

    let mut roots = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
    ];

    if let Some(home) = home_dir {
        roots.push(home.join(".local/share/applications"));
        roots.push(home.join(".local/share/flatpak/exports/share/applications"));
    }

    for root in roots {
        if !root.exists() {
            continue;
        }
        let entries = match fs::read_dir(&root) {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "desktop") {
                let stem = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();

                if should_exclude_app_name(&stem) {
                    continue;
                }

                if let Ok(content) = fs::read_to_string(&path) {
                    if let Some(app) = parse_desktop_entry(&path, &content, home_dir) {
                        let norm = app.name.to_lowercase();
                        if seen_ids.insert(norm) {
                            apps.push(app);
                        }
                    }
                }
            }
        }
    }

    apps.sort_by(|a, b| a.name.cmp(&b.name));
    apps
}

#[cfg(not(target_os = "windows"))]
fn parse_desktop_entry(path: &Path, content: &str, home_dir: Option<&Path>) -> Option<IndexedApp> {
    let mut in_desktop_entry = false;
    let mut name: Option<String> = None;
    let mut comment: Option<String> = None;
    let mut exec: Option<String> = None;
    let mut icon: Option<String> = None;
    let mut is_app = false;
    let mut no_display = false;
    let mut hidden = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_desktop_entry = trimmed == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = trimmed.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "Type" => is_app = value == "Application",
                "Name" if name.is_none() => name = Some(value.to_string()),
                "Comment" if comment.is_none() => comment = Some(value.to_string()),
                "GenericName" if comment.is_none() => comment = Some(value.to_string()),
                "Exec" if exec.is_none() => exec = Some(value.to_string()),
                "Icon" if icon.is_none() => icon = Some(value.to_string()),
                "NoDisplay" => no_display = value.eq_ignore_ascii_case("true"),
                "Hidden" => hidden = value.eq_ignore_ascii_case("true"),
                _ => {}
            }
        }
    }

    if !is_app || no_display || hidden {
        return None;
    }

    let app_name = name?;
    let color = assign_app_color(&app_name);
    let subtitle = comment.unwrap_or_else(|| format_display_path(path, home_dir));

    Some(IndexedApp {
        name: app_name,
        subtitle,
        path: path.to_path_buf(),
        exec,
        icon,
        color,
        keywords: String::new(),
    })
}

pub fn build_full_search_index() -> SearchIndex {
    let home_dir = get_user_home_dir();

    #[cfg(target_os = "windows")]
    let apps = scan_windows_start_menu_apps(home_dir.as_deref());

    #[cfg(not(target_os = "windows"))]
    let apps = scan_linux_desktop_apps(home_dir.as_deref());

    let mut files = Vec::new();
    let search_roots = get_user_search_directories(&home_dir);

    for root in search_roots {
        walk_directory_files(
            &root,
            0,
            MAX_DIR_DEPTH,
            home_dir.as_deref(),
            &mut files,
        );
    }

    SearchIndex {
        apps,
        files,
        is_indexed: true,
    }
}

pub fn search_index(
    index: &SearchIndex,
    raw_query: &str,
) -> (Vec<SearchResultItem>, Vec<SearchResultItem>) {
    let trimmed = raw_query.trim();

    if trimmed.is_empty() {
        let app_suggestions = index
            .apps
            .iter()
            .take(3)
            .map(|app| SearchResultItem {
                id: format!("app:{}", app.path.to_string_lossy()),
                category: "app".to_string(),
                name: app.name.clone(),
                subtitle: app.subtitle.clone(),
                path: app.path.to_string_lossy().to_string(),
                icon: app.icon.clone(),
                color: Some(app.color.clone()),
                is_image: false,
                right_label: "Application".to_string(),
            })
            .collect();

        return (app_suggestions, Vec::new());
    }

    // Search applications
    let mut matched_apps: Vec<(u32, &IndexedApp)> = index
        .apps
        .iter()
        .filter_map(|app| {
            let score = score_match(&app.name, &app.keywords, trimmed);
            if score > 0 {
                Some((score, app))
            } else {
                None
            }
        })
        .collect();

    matched_apps.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.name.len().cmp(&b.1.name.len()))
            .then_with(|| a.1.name.cmp(&b.1.name))
    });

    let app_results: Vec<SearchResultItem> = matched_apps
        .into_iter()
        .take(MAX_RESULTS_PER_CATEGORY)
        .map(|(_, app)| SearchResultItem {
            id: format!("app:{}", app.path.to_string_lossy()),
            category: "app".to_string(),
            name: app.name.clone(),
            subtitle: app.subtitle.clone(),
            path: app.path.to_string_lossy().to_string(),
            icon: app.icon.clone(),
            color: Some(app.color.clone()),
            is_image: false,
            right_label: "Application".to_string(),
        })
        .collect();

    // Search files
    let mut matched_files: Vec<(u32, &IndexedFile)> = index
        .files
        .iter()
        .filter_map(|file| {
            let score = score_match(&file.name, "", trimmed);
            if score > 0 {
                Some((score, file))
            } else {
                None
            }
        })
        .collect();

    matched_files.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.1.name.len().cmp(&b.1.name.len()))
            .then_with(|| a.1.name.cmp(&b.1.name))
    });

    let file_results: Vec<SearchResultItem> = matched_files
        .into_iter()
        .take(MAX_RESULTS_PER_CATEGORY)
        .map(|(_, file)| SearchResultItem {
            id: format!("file:{}", file.path.to_string_lossy()),
            category: "file".to_string(),
            name: file.name.clone(),
            subtitle: file.subtitle.clone(),
            path: file.path.to_string_lossy().to_string(),
            icon: None,
            color: None,
            is_image: file.is_image,
            right_label: file.right_label.clone(),
        })
        .collect();

    (app_results, file_results)
}

#[cfg(target_os = "windows")]
#[link(name = "shell32")]
unsafe extern "system" {
    fn ShellExecuteW(
        hwnd: *mut std::ffi::c_void,
        lp_operation: *const u16,
        lp_file: *const u16,
        lp_parameters: *const u16,
        lp_directory: *const u16,
        n_show_cmd: i32,
    ) -> isize;
}

#[cfg(target_os = "windows")]
fn to_wide_null(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

pub fn launch_target(path: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let wide_op = to_wide_null("open");
        let wide_file = to_wide_null(path);
        let code = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                wide_op.as_ptr(),
                wide_file.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1, // SW_SHOWNORMAL
            )
        };
        if code > 32 {
            Ok(())
        } else {
            Err(format!(
                "Failed to open '{}' via ShellExecuteW (code: {})",
                path, code
            ))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let status = std::process::Command::new("xdg-open")
            .arg(path)
            .status()
            .map_err(|err| format!("Failed to spawn xdg-open for '{path}': {err}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("xdg-open exited with non-zero status for '{path}'"))
        }
    }
}

pub fn launch_target_as_admin(path: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let wide_op = to_wide_null("runas");
        let wide_file = to_wide_null(path);
        let code = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                wide_op.as_ptr(),
                wide_file.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            )
        };
        if code > 32 {
            Ok(())
        } else {
            Err(format!(
                "Failed to run '{}' as admin via ShellExecuteW (code: {})",
                path, code
            ))
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On Linux, running as root uses pkexec or sudo
        let status = std::process::Command::new("pkexec")
            .arg(path)
            .status()
            .map_err(|err| format!("Failed to spawn pkexec for '{path}': {err}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("pkexec exited with non-zero status for '{path}'"))
        }
    }
}

pub fn reveal_in_folder(path: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer.exe")
            .arg(format!("/select,{}", path))
            .spawn()
            .map_err(|err| format!("Failed to reveal item in Explorer: {err}"))?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let p = Path::new(path);
        let folder = if p.is_dir() {
            p
        } else {
            p.parent().unwrap_or(p)
        };
        std::process::Command::new("xdg-open")
            .arg(folder)
            .spawn()
            .map_err(|err| format!("Failed to open directory '{:?}': {err}", folder))?;
        Ok(())
    }
}

pub fn start_background_indexing<R: Runtime>(app_handle: AppHandle<R>) {
    std::thread::spawn(move || {
        let index = build_full_search_index();
        if let Some(state) = app_handle.try_state::<SearchState>()
            && let Ok(mut lock) = state.index.write()
        {
            *lock = index;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_extensions() {
        assert!(is_image_extension("png"));
        assert!(is_image_extension("JPG"));
        assert!(is_image_extension("jpeg"));
        assert!(is_image_extension("webp"));
        assert!(is_image_extension("gif"));
        assert!(is_image_extension("svg"));
        assert!(!is_image_extension("pdf"));
        assert!(!is_image_extension("txt"));
        assert!(!is_image_extension("rs"));
    }

    #[test]
    fn test_format_display_path() {
        let home = Path::new("C:/Users/tester");
        let file = Path::new("C:/Users/tester/Documents/notes.txt");
        assert_eq!(
            format_display_path(file, Some(home)),
            "~/Documents/notes.txt"
        );

        let outside = Path::new("D:/projects/Lux/app");
        assert_eq!(
            format_display_path(outside, Some(home)),
            "D:/projects/Lux/app"
        );
    }

    #[test]
    fn test_should_exclude_app_name() {
        assert!(should_exclude_app_name("Uninstall Discord"));
        assert!(should_exclude_app_name("7-Zip Help"));
        assert!(should_exclude_app_name("Documentation"));
        assert!(should_exclude_app_name("ReadMe"));
        assert!(!should_exclude_app_name("Discord"));
        assert!(!should_exclude_app_name("Visual Studio Code"));
        assert!(!should_exclude_app_name("Firefox"));
    }

    #[test]
    fn test_score_match() {
        assert_eq!(score_match("Discord", "", "Discord"), 100);
        assert_eq!(score_match("Discord", "", "disc"), 80);
        assert_eq!(score_match("Visual Studio Code", "", "Studio"), 60);
        assert_eq!(score_match("Visual Studio Code", "", "vis"), 80);
        assert_eq!(score_match("LightCraft", "", "craft"), 40);
        assert_eq!(score_match("LightCraft", "photo editor", "photo"), 25);
        assert_eq!(score_match("Calculator", "", "zebra"), 0);
    }

    #[test]
    fn test_search_index_query() {
        let index = SearchIndex {
            apps: vec![
                IndexedApp {
                    name: "Firefox".to_string(),
                    subtitle: "~/Start Menu/Firefox".to_string(),
                    path: PathBuf::from("C:/Firefox.lnk"),
                    exec: None,
                    icon: None,
                    color: "#e8663d".to_string(),
                    keywords: "browser web".to_string(),
                },
                IndexedApp {
                    name: "Discord".to_string(),
                    subtitle: "~/Start Menu/Discord".to_string(),
                    path: PathBuf::from("C:/Discord.lnk"),
                    exec: None,
                    icon: None,
                    color: "#5865f2".to_string(),
                    keywords: "chat voice".to_string(),
                },
            ],
            files: vec![
                IndexedFile {
                    name: "wedding-photo.jpg".to_string(),
                    subtitle: "~/Pictures/wedding-photo.jpg".to_string(),
                    path: PathBuf::from("C:/Users/tester/Pictures/wedding-photo.jpg"),
                    is_image: true,
                    extension: "jpg".to_string(),
                    right_label: "Image".to_string(),
                },
                IndexedFile {
                    name: "contract.pdf".to_string(),
                    subtitle: "~/Documents/contract.pdf".to_string(),
                    path: PathBuf::from("C:/Users/tester/Documents/contract.pdf"),
                    is_image: false,
                    extension: "pdf".to_string(),
                    right_label: "PDF".to_string(),
                },
            ],
            is_indexed: true,
        };

        let (apps, files) = search_index(&index, "fire");
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Firefox");
        assert_eq!(files.len(), 0);

        let (apps, files) = search_index(&index, "photo");
        assert_eq!(apps.len(), 0);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "wedding-photo.jpg");
        assert!(files[0].is_image);
    }

    #[test]
    fn test_live_index_build_finds_real_apps_and_files() {
        let index = build_full_search_index();
        assert!(!index.apps.is_empty(), "Expected Start Menu shortcuts to be indexed");
        // Verify query matching against real host apps
        let (apps, _files) = search_index(&index, "Discord");
        assert!(!apps.is_empty(), "Expected to find Discord on development machine");
        assert!(apps[0].name.to_lowercase().contains("discord"));
    }
}
