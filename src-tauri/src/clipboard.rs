use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub const EVENT_CLIPBOARD_UPDATED: &str = "lux://clipboard-updated";
pub const DEFAULT_MAX_CLIPBOARD_ENTRIES: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClipboardKind {
    Text,
    Image,
    Link,
    Color,
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardEntry {
    pub id: String,
    pub kind: ClipboardKind,
    pub name: String,
    pub preview: Option<String>,
    pub source_app: Option<String>,
    pub created_at: i64,
    pub is_pinned: bool,
    pub char_count: Option<usize>,
    pub word_count: Option<usize>,
    pub link_url: Option<String>,
    pub link_title: Option<String>,
    pub link_desc: Option<String>,
    pub file_path: Option<String>,
    pub file_size: Option<String>,
    pub file_type: Option<String>,
    pub image_dims: Option<String>,
    pub image_size: Option<String>,
    pub color_hex: Option<String>,
    pub color_rgb: Option<String>,
    pub color_hsl: Option<String>,
    #[serde(default)]
    pub is_filtered: bool,
    #[serde(default)]
    pub matched_filter: Option<String>,
}

fn current_timestamp_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn hash_bytes(data: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

fn to_base64(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len() * 4 / 3 + 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

pub fn format_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        let kb = bytes as f64 / 1024.0;
        if (kb - kb.round()).abs() < 0.05 {
            format!("{} KB", kb.round() as usize)
        } else {
            format!("{kb:.1} KB")
        }
    } else {
        let mb = bytes as f64 / (1024.0 * 1024.0);
        if (mb - mb.round()).abs() < 0.05 {
            format!("{} MB", mb.round() as usize)
        } else {
            format!("{mb:.1} MB")
        }
    }
}

pub fn is_hex_color(s: &str) -> bool {
    let s = s.trim();
    if !s.starts_with('#') {
        return false;
    }
    let hex_part = &s[1..];
    (hex_part.len() == 3 || hex_part.len() == 6 || hex_part.len() == 8)
        && hex_part.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn hex_to_rgb(hex: &str) -> Option<(u8, u8, u8)> {
    let clean = hex.trim().strip_prefix('#')?;
    if clean.len() == 3 {
        let r = u8::from_str_radix(&clean[0..1].repeat(2), 16).ok()?;
        let g = u8::from_str_radix(&clean[1..2].repeat(2), 16).ok()?;
        let b = u8::from_str_radix(&clean[2..3].repeat(2), 16).ok()?;
        Some((r, g, b))
    } else if clean.len() >= 6 {
        let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
        let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
        let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
        Some((r, g, b))
    } else {
        None
    }
}

pub fn rgb_to_hsl(r: u8, g: u8, b: u8) -> String {
    let rf = r as f64 / 255.0;
    let gf = g as f64 / 255.0;
    let bf = b as f64 / 255.0;

    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let l = (max + min) / 2.0;
    let d = max - min;

    let (h, s) = if d.abs() < 1e-9 {
        (0.0, 0.0)
    } else {
        let s = d / (1.0 - (2.0 * l - 1.0).abs());
        let mut h = if (max - rf).abs() < 1e-9 {
            (gf - bf) / d + if gf < bf { 6.0 } else { 0.0 }
        } else if (max - gf).abs() < 1e-9 {
            (bf - rf) / d + 2.0
        } else {
            (rf - gf) / d + 4.0
        };
        h *= 60.0;
        (h, s)
    };

    format!(
        "{}°, {}%, {}%",
        h.round() as i64,
        (s * 100.0).round() as i64,
        (l * 100.0).round() as i64
    )
}

pub fn is_url(s: &str) -> bool {
    let s = s.trim();
    if s.contains('\n') || s.contains('\r') || s.contains(' ') {
        return false;
    }
    s.starts_with("http://") || s.starts_with("https://") || s.starts_with("ftp://")
}

pub fn extract_domain(url_str: &str) -> Option<String> {
    let without_proto = url_str
        .strip_prefix("https://")
        .or_else(|| url_str.strip_prefix("http://"))
        .or_else(|| url_str.strip_prefix("ftp://"))?;
    let domain = without_proto.split(['/', '?', '#', ':']).next()?;
    if domain.is_empty() {
        None
    } else {
        Some(domain.to_string())
    }
}

pub fn classify_text(text: &str) -> (ClipboardKind, String, Option<String>) {
    let trimmed = text.trim();

    if is_hex_color(trimmed) {
        let upper_hex = format!("#{}", trimmed[1..].to_uppercase());
        return (ClipboardKind::Color, upper_hex, Some("Hex Color".to_string()));
    }

    if is_url(trimmed) {
        let domain = extract_domain(trimmed).unwrap_or_else(|| "Web Link".to_string());
        return (ClipboardKind::Link, trimmed.to_string(), Some(domain));
    }

    let candidate_path = trimmed.trim_matches('"').trim_matches('\'');
    let candidate_path = candidate_path.strip_prefix("file://").unwrap_or(candidate_path);
    if (candidate_path.starts_with('/') || (candidate_path.len() >= 3 && candidate_path.as_bytes()[1] == b':'))
        && !candidate_path.contains('\n')
        && !candidate_path.contains('\r')
    {
        let p = Path::new(candidate_path);
        if p.exists() {
            let file_name = p
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| candidate_path.to_string());
            return (ClipboardKind::File, candidate_path.to_string(), Some(file_name));
        }
    }

    let first_line = trimmed.lines().next().unwrap_or("").trim();
    let preview = if first_line.chars().count() > 100 {
        let truncated: String = first_line.chars().take(100).collect();
        format!("{truncated}…")
    } else {
        first_line.to_string()
    };

    (ClipboardKind::Text, preview, None)
}

pub struct ClipboardHistory {
    entries: Vec<ClipboardEntry>,
    max_entries: usize,
    last_content_hash: Option<u64>,
    counter: u64,
}

impl ClipboardHistory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries: if max_entries == 0 { DEFAULT_MAX_CLIPBOARD_ENTRIES } else { max_entries },
            last_content_hash: None,
            counter: 0,
        }
    }

    pub fn list(&self) -> Vec<ClipboardEntry> {
        self.entries.iter().filter(|e| !e.is_filtered).cloned().collect()
    }

    pub fn list_filtered(&self) -> Vec<ClipboardEntry> {
        self.entries.iter().filter(|e| e.is_filtered).cloned().collect()
    }

    pub fn set_last_content_hash(&mut self, hash: u64) {
        self.last_content_hash = Some(hash);
    }

    pub fn get(&self, id: &str) -> Option<ClipboardEntry> {
        self.entries.iter().find(|e| e.id == id).cloned()
    }

    pub fn add_text(&mut self, text: String, app_source: Option<String>) -> Option<ClipboardEntry> {
        self.add_text_internal(text, app_source, false, None)
    }

    pub fn add_text_filtered(
        &mut self,
        text: String,
        app_source: Option<String>,
        matched_filter: String,
    ) -> Option<ClipboardEntry> {
        self.add_text_internal(text, app_source, true, Some(matched_filter))
    }

    fn add_text_internal(
        &mut self,
        text: String,
        app_source: Option<String>,
        is_filtered: bool,
        matched_filter: Option<String>,
    ) -> Option<ClipboardEntry> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }

        let hash = hash_bytes(trimmed.as_bytes());
        if self.last_content_hash == Some(hash) {
            return None;
        }
        self.last_content_hash = Some(hash);

        // Remove previous duplicate if present to bump to top
        self.entries.retain(|e| e.name != trimmed && e.file_path.as_deref() != Some(trimmed));

        self.counter += 1;
        let created_at = current_timestamp_ms();
        let id = format!("clip_{created_at}_{}", self.counter);

        let entry = if is_hex_color(trimmed) {
            let upper_hex = format!("#{}", trimmed[1..].to_uppercase());
            let (color_rgb, color_hsl) = if let Some((r, g, b)) = hex_to_rgb(&upper_hex) {
                (Some(format!("{r}, {g}, {b}")), Some(rgb_to_hsl(r, g, b)))
            } else {
                (None, None)
            };
            ClipboardEntry {
                id,
                kind: ClipboardKind::Color,
                name: upper_hex.clone(),
                preview: None,
                source_app: app_source,
                created_at,
                is_pinned: false,
                char_count: None,
                word_count: None,
                link_url: None,
                link_title: None,
                link_desc: None,
                file_path: None,
                file_size: None,
                file_type: None,
                image_dims: None,
                image_size: None,
                color_hex: Some(upper_hex),
                color_rgb,
                color_hsl,
                is_filtered,
                matched_filter: matched_filter.clone(),
            }
        } else if is_url(trimmed) {
            let domain = extract_domain(trimmed);
            ClipboardEntry {
                id,
                kind: ClipboardKind::Link,
                name: trimmed.to_string(),
                preview: None,
                source_app: app_source,
                created_at,
                is_pinned: false,
                char_count: None,
                word_count: None,
                link_url: Some(trimmed.to_string()),
                link_title: domain,
                link_desc: Some(trimmed.to_string()),
                file_path: None,
                file_size: None,
                file_type: None,
                image_dims: None,
                image_size: None,
                color_hex: None,
                color_rgb: None,
                color_hsl: None,
                is_filtered,
                matched_filter: matched_filter.clone(),
            }
        } else {
            let candidate_path = trimmed.trim_matches('"').trim_matches('\'');
            let candidate_path = candidate_path.strip_prefix("file://").unwrap_or(candidate_path);
            let p = Path::new(candidate_path);
            let is_path_syntax = (candidate_path.starts_with('/')
                || (candidate_path.len() >= 3 && candidate_path.as_bytes()[1] == b':'))
                && !candidate_path.contains('\n')
                && !candidate_path.contains('\r');

            if is_path_syntax && p.exists() {
                let file_name = p
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| candidate_path.to_string());
                let file_type = p
                    .extension()
                    .map(|ext| ext.to_string_lossy().to_string());
                let file_size = std::fs::metadata(p)
                    .ok()
                    .map(|m| format_size(m.len() as usize));

                ClipboardEntry {
                    id,
                    kind: ClipboardKind::File,
                    name: file_name,
                    preview: None,
                    source_app: app_source,
                    created_at,
                    is_pinned: false,
                    char_count: None,
                    word_count: None,
                    link_url: None,
                    link_title: None,
                    link_desc: None,
                    file_path: Some(candidate_path.to_string()),
                    file_size,
                    file_type,
                    image_dims: None,
                    image_size: None,
                    color_hex: None,
                    color_rgb: None,
                    color_hsl: None,
                    is_filtered,
                    matched_filter: matched_filter.clone(),
                }
            } else {
                let char_count = trimmed.chars().count();
                let word_count = trimmed.split_whitespace().count();
                ClipboardEntry {
                    id,
                    kind: ClipboardKind::Text,
                    name: trimmed.to_string(),
                    preview: None,
                    source_app: app_source,
                    created_at,
                    is_pinned: false,
                    char_count: Some(char_count),
                    word_count: Some(word_count),
                    link_url: None,
                    link_title: None,
                    link_desc: None,
                    file_path: None,
                    file_size: None,
                    file_type: None,
                    image_dims: None,
                    image_size: None,
                    color_hex: None,
                    color_rgb: None,
                    color_hsl: None,
                    is_filtered,
                    matched_filter,
                }
            }
        };

        self.entries.insert(0, entry.clone());
        self.prune();
        Some(entry)
    }

    pub fn add_image(
        &mut self,
        png_bytes: &[u8],
        width: u32,
        height: u32,
        app_source: Option<String>,
    ) -> Option<ClipboardEntry> {
        if png_bytes.is_empty() {
            return None;
        }

        let hash = hash_bytes(png_bytes);
        if self.last_content_hash == Some(hash) {
            return None;
        }
        self.last_content_hash = Some(hash);

        self.counter += 1;
        let created_at = current_timestamp_ms();
        let id = format!("clip_img_{created_at}_{}", self.counter);
        let base64_png = format!("data:image/png;base64,{}", to_base64(png_bytes));
        let dimensions = format!("{width} × {height}");
        let size_str = format_size(png_bytes.len());

        let entry = ClipboardEntry {
            id,
            kind: ClipboardKind::Image,
            name: format!("Screenshot {dimensions}"),
            preview: Some(base64_png),
            source_app: app_source,
            created_at,
            is_pinned: false,
            char_count: None,
            word_count: None,
            link_url: None,
            link_title: None,
            link_desc: None,
            file_path: None,
            file_size: Some(size_str.clone()),
            file_type: Some("png".to_string()),
            image_dims: Some(dimensions),
            image_size: Some(size_str),
            color_hex: None,
            color_rgb: None,
            color_hsl: None,
            is_filtered: false,
            matched_filter: None,
        };

        self.entries.insert(0, entry.clone());
        self.prune();
        Some(entry)
    }

    pub fn delete(&mut self, id: &str) -> bool {
        let initial_len = self.entries.len();
        self.entries.retain(|e| e.id != id);
        let removed = self.entries.len() < initial_len;
        if self.entries.is_empty() {
            self.last_content_hash = None;
        }
        removed
    }

    pub fn clear(&mut self) {
        // Keep pinned entries on clear
        self.entries.retain(|e| e.is_pinned);
        if self.entries.is_empty() {
            self.last_content_hash = None;
        }
    }

    pub fn toggle_pin(&mut self, id: &str) -> Option<bool> {
        for entry in &mut self.entries {
            if entry.id == id {
                entry.is_pinned = !entry.is_pinned;
                return Some(entry.is_pinned);
            }
        }
        None
    }

    fn prune(&mut self) {
        if self.entries.len() <= self.max_entries {
            return;
        }

        // Try to remove oldest unpinned entries first
        while self.entries.len() > self.max_entries {
            if let Some(idx) = self.entries.iter().rposition(|e| !e.is_pinned) {
                self.entries.remove(idx);
            } else {
                // If all are pinned, remove oldest anyway
                self.entries.pop();
            }
        }
    }
}

pub struct ClipboardState {
    pub history: Arc<Mutex<ClipboardHistory>>,
    pub monitor_cancel: Arc<AtomicBool>,
    pub is_monitoring: Arc<AtomicBool>,
}

impl ClipboardState {
    pub fn new(max_entries: usize) -> Self {
        Self {
            history: Arc::new(Mutex::new(ClipboardHistory::new(max_entries))),
            monitor_cancel: Arc::new(AtomicBool::new(false)),
            is_monitoring: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start_monitor<R: Runtime>(&self, app: AppHandle<R>) {
        if self
            .is_monitoring
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            self.monitor_cancel.store(false, Ordering::SeqCst);
            let cancel = self.monitor_cancel.clone();
            let is_monitoring = self.is_monitoring.clone();
            let history = self.history.clone();
            start_clipboard_monitor(app, history, cancel, is_monitoring);
        }
    }

    pub fn stop_monitor(&self) {
        self.monitor_cancel.store(true, Ordering::SeqCst);
        self.is_monitoring.store(false, Ordering::SeqCst);
    }

    pub fn is_monitor_running(&self) -> bool {
        self.is_monitoring.load(Ordering::SeqCst)
    }
}

#[cfg(target_os = "windows")]
mod win_clipboard {
    use std::ffi::c_void;

    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetClipboardSequenceNumber() -> u32;
        fn GetForegroundWindow() -> *mut c_void;
        fn GetWindowThreadProcessId(hwnd: *mut c_void, lpdw_process_id: *mut u32) -> u32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(dw_desired_access: u32, b_inherit_handle: i32, dw_process_id: u32) -> *mut c_void;
        fn QueryFullProcessImageNameW(
            h_process: *mut c_void,
            dw_flags: u32,
            lp_exe_name: *mut u16,
            lpdw_size: *mut u32,
        ) -> i32;
        fn CloseHandle(h_object: *mut c_void) -> i32;
    }

    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

    pub fn get_sequence_number() -> u32 {
        unsafe { GetClipboardSequenceNumber() }
    }

    pub fn get_foreground_app_name() -> Option<String> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return None;
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &raw mut pid);
            if pid == 0 {
                return None;
            }

            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if process.is_null() {
                return None;
            }

            let mut buf = [0u16; 512];
            let mut size = buf.len() as u32;
            let success = QueryFullProcessImageNameW(process, 0, buf.as_mut_ptr(), &raw mut size);
            CloseHandle(process);

            if success != 0 && size > 0 {
                let full_path = String::from_utf16_lossy(&buf[..size as usize]);
                let file_name = std::path::Path::new(&full_path)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string());
                return file_name;
            }
            None
        }
    }
}

pub fn start_clipboard_monitor<R: Runtime>(
    app: AppHandle<R>,
    state: Arc<Mutex<ClipboardHistory>>,
    cancel: Arc<AtomicBool>,
    is_monitoring: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        #[cfg(target_os = "windows")]
        let mut last_seq = win_clipboard::get_sequence_number();
        #[cfg(not(target_os = "windows"))]
        let mut last_seq: u32 = 0;

        loop {
            std::thread::sleep(Duration::from_millis(400));

            if cancel.load(Ordering::SeqCst) {
                break;
            }

            #[cfg(target_os = "windows")]
            {
                let seq = win_clipboard::get_sequence_number();
                if seq == last_seq {
                    continue;
                }
                last_seq = seq;
            }

            let app_source = {
                #[cfg(target_os = "windows")]
                {
                    win_clipboard::get_foreground_app_name()
                }
                #[cfg(not(target_os = "windows"))]
                {
                    None
                }
            };

            // Attempt to read clipboard contents using arboard
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                // Check text first
                if let Ok(text) = clipboard.get_text() {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        let hash = hash_bytes(trimmed.as_bytes());
                        let filter_match = app
                            .try_state::<crate::clipboard_filters::ClipboardFiltersState>()
                            .and_then(|fs| fs.matches(trimmed));

                        let mut hist = match state.lock() {
                            Ok(guard) => guard,
                            Err(poisoned) => poisoned.into_inner(),
                        };

                        if let Some((mode, filter_name)) = filter_match {
                            match mode {
                                crate::clipboard_filters::ExclusionMode::Ignore => {
                                    // Ignore completely: never stored in history
                                    hist.set_last_content_hash(hash);
                                    continue;
                                }
                                crate::clipboard_filters::ExclusionMode::Hide => {
                                    // Marked as filtered: hidden from normal UI, available for safe inspection
                                    if let Some(_entry) = hist.add_text_filtered(text, app_source.clone(), filter_name) {
                                        let _ = app.emit(EVENT_CLIPBOARD_UPDATED, ());
                                    }
                                    continue;
                                }
                            }
                        }

                        if let Some(_entry) = hist.add_text(text, app_source.clone()) {
                            let _ = app.emit(EVENT_CLIPBOARD_UPDATED, ());
                        }
                        continue;
                    }
                }

                // If no text, check image
                if let Ok(img) = clipboard.get_image() {
                    let width = img.width as u32;
                    let height = img.height as u32;
                    if width > 0 && height > 0 {
                        let rgba_raw = img.bytes.into_owned();
                        if let Some(image_buffer) =
                            image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(width, height, rgba_raw)
                        {
                            let mut png_bytes = Vec::new();
                            let mut cursor = std::io::Cursor::new(&mut png_bytes);
                            if image_buffer.write_to(&mut cursor, image::ImageFormat::Png).is_ok() {
                                let mut hist = match state.lock() {
                                    Ok(guard) => guard,
                                    Err(poisoned) => poisoned.into_inner(),
                                };
                                if let Some(_entry) = hist.add_image(&png_bytes, width, height, app_source) {
                                    let _ = app.emit(EVENT_CLIPBOARD_UPDATED, ());
                                }
                            }
                        }
                    }
                }
            }
        }
        is_monitoring.store(false, Ordering::SeqCst);
    });
}

#[tauri::command]
pub fn get_clipboard_history(state: tauri::State<'_, ClipboardState>) -> Vec<ClipboardEntry> {
    state.history.lock().unwrap_or_else(|p| p.into_inner()).list()
}

#[tauri::command]
pub fn get_filtered_clipboard_history(state: tauri::State<'_, ClipboardState>) -> Vec<ClipboardEntry> {
    state.history.lock().unwrap_or_else(|p| p.into_inner()).list_filtered()
}

#[tauri::command]
pub fn copy_clipboard_entry(id: String, state: tauri::State<'_, ClipboardState>) -> Result<(), String> {
    let entry = {
        let hist = state.history.lock().unwrap_or_else(|p| p.into_inner());
        hist.get(&id).ok_or_else(|| "Entry not found".to_string())?
    };

    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Failed to access clipboard: {e}"))?;

    match entry.kind {
        ClipboardKind::Image => {
            // For images, decode base64 PNG stored in preview
            if let Some(data_url) = &entry.preview {
                let stripped = data_url.strip_prefix("data:image/png;base64,").unwrap_or(data_url);
                if let Ok(png_bytes) = decode_base64(stripped)
                    && let Ok(decoded_image) = image::load_from_memory(&png_bytes)
                {
                    let rgba = decoded_image.to_rgba8();
                    let img_data = arboard::ImageData {
                        width: rgba.width() as usize,
                        height: rgba.height() as usize,
                        bytes: std::borrow::Cow::Owned(rgba.into_raw()),
                    };
                    clipboard.set_image(img_data).map_err(|e| format!("Failed to set image: {e}"))?;
                    return Ok(());
                }
            }
            clipboard.set_text(&entry.name).map_err(|e| format!("Failed to set text: {e}"))?;
        }
        ClipboardKind::File => {
            let to_copy = entry.file_path.as_deref().unwrap_or(&entry.name);
            clipboard.set_text(to_copy).map_err(|e| format!("Failed to set text: {e}"))?;
        }
        _ => {
            clipboard.set_text(&entry.name).map_err(|e| format!("Failed to set text: {e}"))?;
        }
    }

    Ok(())
}

fn decode_base64(s: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut val = 0u32;
    let mut valb = -8;
    for c in s.chars() {
        if c == '=' || c.is_whitespace() {
            continue;
        }
        let v = match c {
            'A'..='Z' => c as u32 - 'A' as u32,
            'a'..='z' => c as u32 - 'a' as u32 + 26,
            '0'..='9' => c as u32 - '0' as u32 + 52,
            '+' => 62,
            '/' => 63,
            _ => return Err(format!("Invalid base64 character: {c}")),
        };
        val = (val << 6) | v;
        valb += 6;
        if valb >= 0 {
            out.push(((val >> valb) & 0xFF) as u8);
            valb -= 8;
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn delete_clipboard_entry(
    id: String,
    app: AppHandle,
    state: tauri::State<'_, ClipboardState>,
) -> Result<(), String> {
    let removed = {
        let mut hist = state.history.lock().unwrap_or_else(|p| p.into_inner());
        hist.delete(&id)
    };
    if removed {
        let _ = app.emit(EVENT_CLIPBOARD_UPDATED, ());
    }
    Ok(())
}

#[tauri::command]
pub fn clear_clipboard_history(
    app: AppHandle,
    state: tauri::State<'_, ClipboardState>,
) -> Result<(), String> {
    {
        let mut hist = state.history.lock().unwrap_or_else(|p| p.into_inner());
        hist.clear();
    }
    let _ = app.emit(EVENT_CLIPBOARD_UPDATED, ());
    Ok(())
}

#[tauri::command]
pub fn toggle_pin_clipboard_entry(
    id: String,
    app: AppHandle,
    state: tauri::State<'_, ClipboardState>,
) -> Result<bool, String> {
    let pinned = {
        let mut hist = state.history.lock().unwrap_or_else(|p| p.into_inner());
        hist.toggle_pin(&id).ok_or_else(|| "Entry not found".to_string())?
    };
    let _ = app.emit(EVENT_CLIPBOARD_UPDATED, ());
    Ok(pinned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_hex_color() {
        let (kind, preview, details) = classify_text("#3b3550");
        assert_eq!(kind, ClipboardKind::Color);
        assert_eq!(preview, "#3B3550");
        assert_eq!(details, Some("Hex Color".to_string()));

        let (kind2, _, _) = classify_text("#FFF");
        assert_eq!(kind2, ClipboardKind::Color);

        let (kind3, _, _) = classify_text("not a color");
        assert_eq!(kind3, ClipboardKind::Text);
    }

    #[test]
    fn test_classify_url() {
        let (kind, preview, details) = classify_text("https://swisser.pt/portfolio?ref=lux");
        assert_eq!(kind, ClipboardKind::Link);
        assert_eq!(preview, "https://swisser.pt/portfolio?ref=lux");
        assert_eq!(details, Some("swisser.pt".to_string()));
    }

    #[test]
    fn test_classify_plain_text() {
        let (kind, preview, details) = classify_text("Hello World from Lux Clipboard");
        assert_eq!(kind, ClipboardKind::Text);
        assert_eq!(preview, "Hello World from Lux Clipboard");
        assert_eq!(details, None);
    }

    #[test]
    fn test_clipboard_history_deduplication() {
        let mut history = ClipboardHistory::new(10);
        assert!(history.add_text("First copy".to_string(), None).is_some());
        assert_eq!(history.list().len(), 1);

        // Consecutive duplicate is ignored
        assert!(history.add_text("First copy".to_string(), None).is_none());
        assert_eq!(history.list().len(), 1);

        // Different copy is added
        assert!(history.add_text("Second copy".to_string(), None).is_some());
        assert_eq!(history.list().len(), 2);
    }

    #[test]
    fn test_clipboard_history_capacity_and_pruning() {
        let mut history = ClipboardHistory::new(3);
        history.add_text("1".to_string(), None);
        history.add_text("2".to_string(), None);
        history.add_text("3".to_string(), None);
        assert_eq!(history.list().len(), 3);

        // Adding 4th prunes oldest
        history.add_text("4".to_string(), None);
        assert_eq!(history.list().len(), 3);
        assert_eq!(history.list()[0].name, "4");
        assert_eq!(history.list()[2].name, "2");
    }

    #[test]
    fn test_clipboard_history_delete_and_clear() {
        let mut history = ClipboardHistory::new(10);
        let e1 = history.add_text("Keep me".to_string(), None).unwrap();
        let e2 = history.add_text("Delete me".to_string(), None).unwrap();
        assert_eq!(history.list().len(), 2);

        assert!(history.delete(&e2.id));
        assert_eq!(history.list().len(), 1);
        assert_eq!(history.list()[0].id, e1.id);

        history.clear();
        assert_eq!(history.list().len(), 0);
    }

    #[test]
    fn test_base64_encode_decode_roundtrip() {
        let original = b"Hello, Lux Clipboard Base64 Test!";
        let encoded = to_base64(original);
        let decoded = decode_base64(&encoded).expect("Decode should succeed");
        assert_eq!(original.to_vec(), decoded);
    }

    #[test]
    fn test_hex_to_rgb_and_rgb_to_hsl() {
        let (r, g, b) = hex_to_rgb("#3B3550").expect("Valid 6-digit hex");
        assert_eq!((r, g, b), (59, 53, 80));
        let hsl = rgb_to_hsl(r, g, b);
        assert_eq!(hsl, "253°, 20%, 26%");

        let (r2, g2, b2) = hex_to_rgb("#FFF").expect("Valid 3-digit hex");
        assert_eq!((r2, g2, b2), (255, 255, 255));
        let hsl2 = rgb_to_hsl(r2, g2, b2);
        assert_eq!(hsl2, "0°, 0%, 100%");

        let (r3, g3, b3) = hex_to_rgb("#E8663D").expect("Valid hex");
        assert_eq!((r3, g3, b3), (232, 102, 61));
        let hsl3 = rgb_to_hsl(r3, g3, b3);
        assert_eq!(hsl3, "14°, 79%, 57%");
    }

    #[test]
    fn test_text_entry_metadata() {
        let mut history = ClipboardHistory::new(10);
        let entry = history
            .add_text("The quick brown fox jumps over the lazy dog".to_string(), Some("Terminal".to_string()))
            .expect("Should create entry");

        assert_eq!(entry.kind, ClipboardKind::Text);
        assert_eq!(entry.name, "The quick brown fox jumps over the lazy dog");
        assert_eq!(entry.char_count, Some(43));
        assert_eq!(entry.word_count, Some(9));
        assert!(entry.created_at > 0);
        assert!(!entry.is_pinned);
        assert_eq!(entry.source_app, Some("Terminal".to_string()));
    }

    #[test]
    fn test_color_entry_metadata() {
        let mut history = ClipboardHistory::new(10);
        let entry = history
            .add_text("#3b3550".to_string(), Some("ColorPicker".to_string()))
            .expect("Should create color entry");

        assert_eq!(entry.kind, ClipboardKind::Color);
        assert_eq!(entry.name, "#3B3550");
        assert_eq!(entry.color_hex, Some("#3B3550".to_string()));
        assert_eq!(entry.color_rgb, Some("59, 53, 80".to_string()));
        assert_eq!(entry.color_hsl, Some("253°, 20%, 26%".to_string()));
        assert!(entry.created_at > 0);
    }

    #[test]
    fn test_url_entry_metadata() {
        let mut history = ClipboardHistory::new(10);
        let entry = history
            .add_text("https://swisser.pt/portfolio?ref=lux".to_string(), Some("Firefox".to_string()))
            .expect("Should create link entry");

        assert_eq!(entry.kind, ClipboardKind::Link);
        assert_eq!(entry.name, "https://swisser.pt/portfolio?ref=lux");
        assert_eq!(entry.link_url, Some("https://swisser.pt/portfolio?ref=lux".to_string()));
        assert_eq!(entry.link_title, Some("swisser.pt".to_string()));
        assert!(entry.created_at > 0);
    }

    #[test]
    fn test_image_entry_metadata() {
        let mut history = ClipboardHistory::new(10);
        let dummy_png = b"\x89PNG\r\n\x1a\nfakeimagebytes";
        let entry = history
            .add_image(dummy_png, 800, 600, Some("GIMP".to_string()))
            .expect("Should create image entry");

        assert_eq!(entry.kind, ClipboardKind::Image);
        assert_eq!(entry.name, "Screenshot 800 × 600");
        assert_eq!(entry.image_dims, Some("800 × 600".to_string()));
        assert!(entry.preview.unwrap().starts_with("data:image/png;base64,"));
        assert!(entry.created_at > 0);
    }

    #[test]
    fn test_camel_case_serialization_contract() {
        let mut history = ClipboardHistory::new(10);
        let entry = history
            .add_text("#3b3550".to_string(), Some("DevApp".to_string()))
            .unwrap();

        let json = serde_json::to_string(&entry).expect("Serialization must succeed");
        assert!(json.contains("\"createdAt\":"));
        assert!(json.contains("\"isPinned\":false"));
        assert!(json.contains("\"sourceApp\":\"DevApp\""));
        assert!(json.contains("\"colorHex\":\"#3B3550\""));
        assert!(json.contains("\"colorRgb\":\"59, 53, 80\""));
        assert!(json.contains("\"colorHsl\":\"253°, 20%, 26%\""));

        // Roundtrip deserialization
        let deserialized: ClipboardEntry = serde_json::from_str(&json).expect("Deserialization must succeed");
        assert_eq!(entry, deserialized);
    }

    #[test]
    fn test_filtered_entry_separation() {
        let mut history = ClipboardHistory::new(10);
        history.add_text("Normal entry 1".to_string(), None);
        history.add_text_filtered("Secret token".to_string(), None, "Secrets & Tokens".to_string());
        history.add_text("Normal entry 2".to_string(), None);

        // Standard list should never return filtered entries
        let normal = history.list();
        assert_eq!(normal.len(), 2);
        assert!(normal.iter().all(|e| !e.is_filtered));

        // Filtered list should return only the filtered entry
        let filtered = history.list_filtered();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "Secret token");
        assert_eq!(filtered[0].matched_filter.as_deref(), Some("Secrets & Tokens"));
        assert!(filtered[0].is_filtered);
    }
}
