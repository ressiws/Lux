use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Runtime};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardEntry {
    pub id: String,
    pub kind: ClipboardKind,
    pub content: String,
    pub preview: String,
    pub details: Option<String>,
    pub image_data: Option<String>,
    pub image_dimensions: Option<String>,
    pub file_size: Option<String>,
    pub copied_at_ms: i64,
    pub app_source: Option<String>,
    pub pinned: bool,
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
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
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
        return (ClipboardKind::Color, upper_hex.clone(), Some("Hex Color".to_string()));
    }

    if is_url(trimmed) {
        let domain = extract_domain(trimmed).unwrap_or_else(|| "Web Link".to_string());
        return (ClipboardKind::Link, trimmed.to_string(), Some(domain));
    }

    if (trimmed.starts_with('/') || (trimmed.len() >= 3 && trimmed.as_bytes()[1] == b':'))
        && !trimmed.contains('\n')
    {
        let p = Path::new(trimmed);
        if p.exists() {
            let file_name = p
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| trimmed.to_string());
            return (ClipboardKind::File, trimmed.to_string(), Some(file_name));
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
        self.entries.clone()
    }

    pub fn get(&self, id: &str) -> Option<ClipboardEntry> {
        self.entries.iter().find(|e| e.id == id).cloned()
    }

    pub fn add_text(&mut self, text: String, app_source: Option<String>) -> Option<ClipboardEntry> {
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
        self.entries.retain(|e| e.content != trimmed);

        self.counter += 1;
        let id = format!("clip_{}_{}", current_timestamp_ms(), self.counter);
        let (kind, preview, details) = classify_text(trimmed);

        let entry = ClipboardEntry {
            id,
            kind,
            content: trimmed.to_string(),
            preview: if preview.is_empty() { trimmed.to_string() } else { preview },
            details,
            image_data: None,
            image_dimensions: None,
            file_size: Some(format_size(trimmed.len())),
            copied_at_ms: current_timestamp_ms(),
            app_source,
            pinned: false,
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
        let id = format!("clip_img_{}_{}", current_timestamp_ms(), self.counter);
        let base64_png = format!("data:image/png;base64,{}", to_base64(png_bytes));
        let dimensions = format!("{width} × {height}");
        let size_str = format_size(png_bytes.len());

        let entry = ClipboardEntry {
            id,
            kind: ClipboardKind::Image,
            content: format!("Screenshot {dimensions}"),
            preview: format!("Image ({dimensions})"),
            details: Some(format!("{dimensions} · {size_str}")),
            image_data: Some(base64_png),
            image_dimensions: Some(dimensions),
            file_size: Some(size_str),
            copied_at_ms: current_timestamp_ms(),
            app_source,
            pinned: false,
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
        self.entries.retain(|e| e.pinned);
        if self.entries.is_empty() {
            self.last_content_hash = None;
        }
    }

    pub fn toggle_pin(&mut self, id: &str) -> Option<bool> {
        for entry in &mut self.entries {
            if entry.id == id {
                entry.pinned = !entry.pinned;
                return Some(entry.pinned);
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
            if let Some(idx) = self.entries.iter().rposition(|e| !e.pinned) {
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
}

impl ClipboardState {
    pub fn new(max_entries: usize) -> Self {
        Self {
            history: Arc::new(Mutex::new(ClipboardHistory::new(max_entries))),
        }
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

pub fn start_clipboard_monitor<R: Runtime>(app: AppHandle<R>, state: Arc<Mutex<ClipboardHistory>>) {
    std::thread::spawn(move || {
        #[cfg(target_os = "windows")]
        let mut last_seq = win_clipboard::get_sequence_number();
        #[cfg(not(target_os = "windows"))]
        let mut last_seq: u32 = 0;

        loop {
            std::thread::sleep(Duration::from_millis(400));

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
                        let mut hist = match state.lock() {
                            Ok(guard) => guard,
                            Err(poisoned) => poisoned.into_inner(),
                        };
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
    });
}

#[tauri::command]
pub fn get_clipboard_history(state: tauri::State<'_, ClipboardState>) -> Vec<ClipboardEntry> {
    state.history.lock().unwrap_or_else(|p| p.into_inner()).list()
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
            // For images, if we have raw image data
            if let Some(base64_data) = &entry.image_data {
                let stripped = base64_data.strip_prefix("data:image/png;base64,").unwrap_or(base64_data);
                // Decode base64 PNG
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
            clipboard.set_text(entry.content).map_err(|e| format!("Failed to set text: {e}"))?;
        }
        _ => {
            clipboard.set_text(entry.content).map_err(|e| format!("Failed to set text: {e}"))?;
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
        assert_eq!(history.list()[0].content, "4");
        assert_eq!(history.list()[2].content, "2");
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
}
