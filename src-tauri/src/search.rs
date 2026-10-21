use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, RwLock};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};

const MAX_DIR_DEPTH: usize = 6;
const MAX_INDEXED_FILES: usize = 50000;
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
    ".idea",
    ".vscode",
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
    "$windows.~bt",
    "$windows.~ws",
    "program files",
    "program files (x86)",
    "programdata",
    "perflogs",
    "inetpub",
    "recovery",
    "msocache",
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
    pub location: String,
    pub icon: Option<String>,
    pub thumbnail: Option<String>,
    pub color: Option<String>,
    pub is_image: bool,
    pub right_label: String,
    pub size: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IndexedApp {
    pub name: String,
    pub subtitle: String,
    pub path: PathBuf,
    pub location: String,
    pub exec: Option<String>,
    pub icon: Option<String>,
    pub color: String,
    pub keywords: String,
    pub size: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IndexedFile {
    pub name: String,
    pub subtitle: String,
    pub path: PathBuf,
    pub location: String,
    pub is_image: bool,
    pub extension: String,
    pub right_label: String,
    pub size: Option<String>,
    pub icon: Option<String>,
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

pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if bytes < KB {
        format!("{bytes} B")
    } else if bytes < MB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    }
}

static ICON_CACHE: LazyLock<RwLock<HashMap<PathBuf, Option<String>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static THUMBNAIL_CACHE: LazyLock<RwLock<HashMap<PathBuf, Option<String>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static EXTENSION_ICON_CACHE: LazyLock<RwLock<HashMap<String, Option<String>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static FOLDER_ICON_CACHE: LazyLock<RwLock<Option<String>>> =
    LazyLock::new(|| RwLock::new(None));

pub fn generate_image_thumbnail_base64(path: &Path, max_dim: u32) -> Option<String> {
    if !path.exists() || !path.is_file() {
        return None;
    }

    if let Some(ext) = path.extension() && ext.eq_ignore_ascii_case("svg") {
        if let Ok(svg_bytes) = fs::read(path) {
            return Some(format!("data:image/svg+xml;base64,{}", to_base64(&svg_bytes)));
        }
    }

    let reader = image::ImageReader::open(path).ok()?.with_guessed_format().ok()?;
    let img = reader.decode().ok()?;
    let thumb = img.thumbnail(max_dim, max_dim);
    let mut png_bytes = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    thumb.write_to(&mut cursor, image::ImageFormat::Png).ok()?;
    Some(format!("data:image/png;base64,{}", to_base64(&png_bytes)))
}

pub fn get_or_generate_thumbnail(path: &Path) -> Option<String> {
    if let Ok(guard) = THUMBNAIL_CACHE.read()
        && let Some(cached) = guard.get(path)
    {
        return cached.clone();
    }

    let thumb = generate_image_thumbnail_base64(path, 256);

    if let Ok(mut guard) = THUMBNAIL_CACHE.write() {
        guard.insert(path.to_path_buf(), thumb.clone());
    }

    thumb
}

pub fn get_image_thumbnail(path: String) -> Option<String> {
    let p = Path::new(&path);
    if p.exists() && p.is_file() {
        get_or_generate_thumbnail(p)
    } else {
        None
    }
}

pub fn get_or_extract_icon(path: &Path) -> Option<String> {
    if let Ok(guard) = ICON_CACHE.read()
        && let Some(cached) = guard.get(path)
    {
        return cached.clone();
    }

    #[cfg(target_os = "windows")]
    let extracted = extract_windows_icon_base64(path);

    #[cfg(not(target_os = "windows"))]
    let extracted = None;

    if let Ok(mut guard) = ICON_CACHE.write() {
        guard.insert(path.to_path_buf(), extracted.clone());
    }

    extracted
}

#[cfg(target_os = "windows")]
#[allow(clippy::upper_case_acronyms)]
mod win_icons {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    #[repr(C)]
    pub struct SHFILEINFOW {
        pub h_icon: *mut c_void,
        pub i_icon: i32,
        pub dw_attributes: u32,
        pub sz_display_name: [u16; 260],
        pub sz_type_name: [u16; 80],
    }

    #[repr(C)]
    pub struct ICONINFO {
        pub f_icon: i32,
        pub x_hotspot: u32,
        pub y_hotspot: u32,
        pub hbm_mask: *mut c_void,
        pub hbm_color: *mut c_void,
    }

    #[repr(C)]
    pub struct BITMAP {
        pub bm_type: i32,
        pub bm_width: i32,
        pub bm_height: i32,
        pub bm_width_bytes: i32,
        pub bm_planes: u16,
        pub bm_bits_pixel: u16,
        pub bm_bits: *mut c_void,
    }

    #[repr(C)]
    pub struct BITMAPINFOHEADER {
        pub bi_size: u32,
        pub bi_width: i32,
        pub bi_height: i32,
        pub bi_planes: u16,
        pub bi_bit_count: u16,
        pub bi_compression: u32,
        pub bi_size_image: u32,
        pub bi_x_pels_per_meter: i32,
        pub bi_y_pels_per_meter: i32,
        pub bi_clr_used: u32,
        pub bi_clr_important: u32,
    }

    #[link(name = "shell32")]
    unsafe extern "system" {
        pub fn SHGetFileInfoW(
            psz_path: *const u16,
            dw_file_attributes: u32,
            psfi: *mut SHFILEINFOW,
            cb_file_info: u32,
            u_flags: u32,
        ) -> usize;
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        pub fn GetIconInfo(h_icon: *mut c_void, p_icon_info: *mut ICONINFO) -> i32;
        pub fn DestroyIcon(h_icon: *mut c_void) -> i32;
    }

    #[link(name = "gdi32")]
    unsafe extern "system" {
        pub fn GetObjectW(h: *mut c_void, c: i32, pv: *mut c_void) -> i32;
        pub fn CreateCompatibleDC(hdc: *mut c_void) -> *mut c_void;
        pub fn DeleteDC(hdc: *mut c_void) -> i32;
        pub fn DeleteObject(ho: *mut c_void) -> i32;
        pub fn GetDIBits(
            hdc: *mut c_void,
            hbm: *mut c_void,
            start: u32,
            c_lines: u32,
            lpv_bits: *mut c_void,
            lpbmi: *mut BITMAPINFOHEADER,
            usage: u32,
        ) -> i32;
    }

    pub const SHGFI_ICON: u32 = 0x000000100;
    pub const SHGFI_LARGEICON: u32 = 0x000000000;
    pub const SHGFI_USEFILEATTRIBUTES: u32 = 0x000000010;
    pub const DIB_RGB_COLORS: u32 = 0;

    pub unsafe fn hicon_to_base64_png(h_icon: *mut c_void) -> Option<String> {
        let mut icon_info = unsafe { std::mem::zeroed::<ICONINFO>() };
        if unsafe { GetIconInfo(h_icon, &mut icon_info) } == 0 {
            unsafe {
                let _ = DestroyIcon(h_icon);
            }
            return None;
        }

        let hbm = if !icon_info.hbm_color.is_null() {
            icon_info.hbm_color
        } else {
            icon_info.hbm_mask
        };

        let mut bmp = unsafe { std::mem::zeroed::<BITMAP>() };
        if unsafe { GetObjectW(hbm, std::mem::size_of::<BITMAP>() as i32, (&raw mut bmp).cast()) } == 0 {
            unsafe {
                if !icon_info.hbm_color.is_null() {
                    let _ = DeleteObject(icon_info.hbm_color);
                }
                if !icon_info.hbm_mask.is_null() {
                    let _ = DeleteObject(icon_info.hbm_mask);
                }
                let _ = DestroyIcon(h_icon);
            }
            return None;
        }

        let width = bmp.bm_width as u32;
        let height = bmp.bm_height.unsigned_abs();
        if width == 0 || height == 0 || width > 256 || height > 256 {
            unsafe {
                if !icon_info.hbm_color.is_null() {
                    let _ = DeleteObject(icon_info.hbm_color);
                }
                if !icon_info.hbm_mask.is_null() {
                    let _ = DeleteObject(icon_info.hbm_mask);
                }
                let _ = DestroyIcon(h_icon);
            }
            return None;
        }

        let hdc = unsafe { CreateCompatibleDC(std::ptr::null_mut()) };
        let mut header = BITMAPINFOHEADER {
            bi_size: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            bi_width: width as i32,
            bi_height: -(height as i32), // negative for top-down
            bi_planes: 1,
            bi_bit_count: 32,
            bi_compression: 0,
            bi_size_image: width * height * 4,
            bi_x_pels_per_meter: 0,
            bi_y_pels_per_meter: 0,
            bi_clr_used: 0,
            bi_clr_important: 0,
        };

        let mut raw_pixels = vec![0u8; (width * height * 4) as usize];
        let lines = unsafe {
            GetDIBits(
                hdc,
                hbm,
                0,
                height,
                raw_pixels.as_mut_ptr().cast(),
                &mut header,
                DIB_RGB_COLORS,
            )
        };

        unsafe {
            let _ = DeleteDC(hdc);
            if !icon_info.hbm_color.is_null() {
                let _ = DeleteObject(icon_info.hbm_color);
            }
            if !icon_info.hbm_mask.is_null() {
                let _ = DeleteObject(icon_info.hbm_mask);
            }
            let _ = DestroyIcon(h_icon);
        }

        if lines == 0 {
            return None;
        }

        // Convert BGRA to RGBA
        let mut has_alpha = false;
        for chunk in raw_pixels.chunks_exact_mut(4) {
            let b = chunk[0];
            let r = chunk[2];
            chunk[0] = r;
            chunk[2] = b;
            if chunk[3] > 0 {
                has_alpha = true;
            }
        }

        if !has_alpha {
            for chunk in raw_pixels.chunks_exact_mut(4) {
                chunk[3] = 255;
            }
        }

        let img = image::RgbaImage::from_raw(width, height, raw_pixels)?;
        let mut png_bytes = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut png_bytes);
        img.write_to(&mut cursor, image::ImageFormat::Png).ok()?;

        Some(format!("data:image/png;base64,{}", super::to_base64(&png_bytes)))
    }

    pub fn extract_windows_icon_base64(path: &Path) -> Option<String> {
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        unsafe {
            let mut shfi = std::mem::zeroed::<SHFILEINFOW>();
            let res = SHGetFileInfoW(
                wide.as_ptr(),
                0,
                &mut shfi,
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON,
            );
            if res == 0 || shfi.h_icon.is_null() {
                return None;
            }
            hicon_to_base64_png(shfi.h_icon)
        }
    }

    pub fn extract_shell_icon_attributes(dummy_name: &str, file_attributes: u32) -> Option<String> {
        let wide: Vec<u16> = std::ffi::OsStr::new(dummy_name)
            .encode_wide()
            .chain(Some(0))
            .collect();
        unsafe {
            let mut shfi = std::mem::zeroed::<SHFILEINFOW>();
            let res = SHGetFileInfoW(
                wide.as_ptr(),
                file_attributes,
                &mut shfi,
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON | SHGFI_USEFILEATTRIBUTES,
            );
            if res == 0 || shfi.h_icon.is_null() {
                return None;
            }
            hicon_to_base64_png(shfi.h_icon)
        }
    }
}

#[cfg(target_os = "windows")]
pub fn extract_windows_icon_base64(path: &Path) -> Option<String> {
    win_icons::extract_windows_icon_base64(path)
}

#[cfg(target_os = "windows")]
pub fn extract_shell_icon_attributes(dummy_name: &str, file_attributes: u32) -> Option<String> {
    win_icons::extract_shell_icon_attributes(dummy_name, file_attributes)
}

pub fn get_folder_icon() -> Option<String> {
    if let Ok(guard) = FOLDER_ICON_CACHE.read()
        && guard.is_some()
    {
        return guard.clone();
    }

    #[cfg(target_os = "windows")]
    let icon = extract_shell_icon_attributes("folder", 0x00000010 /* FILE_ATTRIBUTE_DIRECTORY */);

    #[cfg(not(target_os = "windows"))]
    let icon = None;

    if let Ok(mut guard) = FOLDER_ICON_CACHE.write() {
        *guard = icon.clone();
    }
    icon
}

pub fn get_file_type_icon(ext: &str) -> Option<String> {
    let lower = ext.to_lowercase();
    if lower.is_empty() {
        return None;
    }

    if let Ok(guard) = EXTENSION_ICON_CACHE.read()
        && let Some(cached) = guard.get(&lower)
    {
        return cached.clone();
    }

    #[cfg(target_os = "windows")]
    let icon = {
        let dummy = format!("dummy.{}", lower);
        extract_shell_icon_attributes(&dummy, 0x00000080 /* FILE_ATTRIBUTE_NORMAL */)
    };

    #[cfg(not(target_os = "windows"))]
    let icon = None;

    if let Ok(mut guard) = EXTENSION_ICON_CACHE.write() {
        guard.insert(lower, icon.clone());
    }
    icon
}

#[cfg(target_os = "windows")]
#[allow(clippy::upper_case_acronyms)]
pub fn resolve_lnk_target(lnk_path: &Path) -> Option<PathBuf> {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;

    #[repr(C)]
    struct GUID {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    const CLSID_SHELL_LINK: GUID = GUID {
        data1: 0x00021401,
        data2: 0x0000,
        data3: 0x0000,
        data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
    };

    const IID_ISHELL_LINK_W: GUID = GUID {
        data1: 0x000214F9,
        data2: 0x0000,
        data3: 0x0000,
        data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
    };

    const IID_IPERSIST_FILE: GUID = GUID {
        data1: 0x0000010b,
        data2: 0x0000,
        data3: 0x0000,
        data4: [0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x46],
    };

    #[repr(C)]
    struct IShellLinkWVtbl {
        query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
        add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
        release: unsafe extern "system" fn(*mut c_void) -> u32,
        get_path: unsafe extern "system" fn(*mut c_void, *mut u16, i32, *mut c_void, u32) -> i32,
    }

    #[repr(C)]
    struct IPersistFileVtbl {
        query_interface: unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> i32,
        add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
        release: unsafe extern "system" fn(*mut c_void) -> u32,
        get_class_id: unsafe extern "system" fn(*mut c_void, *mut GUID) -> i32,
        is_dirty: unsafe extern "system" fn(*mut c_void) -> i32,
        load: unsafe extern "system" fn(*mut c_void, *const u16, u32) -> i32,
    }

    #[link(name = "ole32")]
    unsafe extern "system" {
        fn CoInitializeEx(pv_reserved: *mut c_void, dw_co_init: u32) -> i32;
        fn CoCreateInstance(
            rclsid: *const GUID,
            p_unk_outer: *mut c_void,
            dw_cls_context: u32,
            riid: *const GUID,
            ppv: *mut *mut c_void,
        ) -> i32;
    }

    unsafe {
        let _ = CoInitializeEx(std::ptr::null_mut(), 0);

        let mut shell_link: *mut c_void = std::ptr::null_mut();
        if CoCreateInstance(
            &CLSID_SHELL_LINK,
            std::ptr::null_mut(),
            1,
            &IID_ISHELL_LINK_W,
            &mut shell_link,
        ) != 0 || shell_link.is_null() {
            return None;
        }

        let link_vtbl = &**(shell_link as *mut *mut IShellLinkWVtbl);
        let mut persist_file: *mut c_void = std::ptr::null_mut();
        if (link_vtbl.query_interface)(shell_link, &IID_IPERSIST_FILE, &mut persist_file) != 0
            || persist_file.is_null()
        {
            let _ = (link_vtbl.release)(shell_link);
            return None;
        }

        let persist_vtbl = &**(persist_file as *mut *mut IPersistFileVtbl);
        let wide_path: Vec<u16> = lnk_path.as_os_str().encode_wide().chain(Some(0)).collect();
        let load_res = (persist_vtbl.load)(persist_file, wide_path.as_ptr(), 0);
        let _ = (persist_vtbl.release)(persist_file);

        if load_res != 0 {
            let _ = (link_vtbl.release)(shell_link);
            return None;
        }

        let mut target_buf = [0u16; 1024];
        let get_res = (link_vtbl.get_path)(
            shell_link,
            target_buf.as_mut_ptr(),
            target_buf.len() as i32,
            std::ptr::null_mut(),
            0,
        );
        let _ = (link_vtbl.release)(shell_link);

        if get_res == 0 {
            let len = target_buf.iter().position(|&c| c == 0).unwrap_or(0);
            if len > 0 {
                let s = String::from_utf16_lossy(&target_buf[..len]);
                let p = PathBuf::from(s);
                if p.exists() {
                    return Some(p);
                }
            }
        }
    }
    None
}

#[cfg(target_os = "windows")]
pub fn get_pe_file_version(path: &Path) -> Option<String> {
    use std::ffi::c_void;
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "version")]
    unsafe extern "system" {
        fn GetFileVersionInfoSizeW(lptstr_filename: *const u16, lpdw_handle: *mut u32) -> u32;
        fn GetFileVersionInfoW(
            lptstr_filename: *const u16,
            dw_handle: u32,
            dw_len: u32,
            lp_data: *mut c_void,
        ) -> i32;
        fn VerQueryValueW(
            p_block: *const c_void,
            lp_sub_block: *const u16,
            lplp_buffer: *mut *mut c_void,
            pu_len: *mut u32,
        ) -> i32;
    }

    #[repr(C)]
    struct VS_FIXEDFILEINFO {
        dw_signature: u32,
        dw_struc_version: u32,
        dw_file_version_ms: u32,
        dw_file_version_ls: u32,
        dw_product_version_ms: u32,
        dw_product_version_ls: u32,
        dw_file_flags_mask: u32,
        dw_file_flags: u32,
        dw_file_os: u32,
        dw_file_type: u32,
        dw_file_subtype: u32,
        dw_file_date_ms: u32,
        dw_file_date_ls: u32,
    }

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        let mut handle = 0u32;
        let size = GetFileVersionInfoSizeW(wide.as_ptr(), &mut handle);
        if size == 0 {
            return None;
        }

        let mut data = vec![0u8; size as usize];
        if GetFileVersionInfoW(wide.as_ptr(), handle, size, data.as_mut_ptr().cast()) == 0 {
            return None;
        }

        let sub_block: Vec<u16> = "\\\0".encode_utf16().collect();
        let mut buffer: *mut c_void = std::ptr::null_mut();
        let mut len = 0u32;

        if VerQueryValueW(
            data.as_ptr().cast(),
            sub_block.as_ptr(),
            &mut buffer,
            &mut len,
        ) != 0
            && !buffer.is_null()
            && len as usize >= std::mem::size_of::<VS_FIXEDFILEINFO>()
        {
            let info = &*(buffer as *const VS_FIXEDFILEINFO);
            let major = (info.dw_file_version_ms >> 16) & 0xffff;
            let minor = info.dw_file_version_ms & 0xffff;
            let patch = (info.dw_file_version_ls >> 16) & 0xffff;
            let build = info.dw_file_version_ls & 0xffff;
            if build > 0 {
                return Some(format!("{major}.{minor}.{patch}.{build}"));
            } else {
                return Some(format!("{major}.{minor}.{patch}"));
            }
        }
    }
    None
}

#[cfg(not(target_os = "windows"))]
fn get_or_resolve_linux_icon(app: &IndexedApp) -> Option<String> {
    if let Some(ref icon_name) = app.icon {
        if let Ok(guard) = ICON_CACHE.read()
            && let Some(cached) = guard.get(&app.path)
        {
            return cached.clone();
        }

        let icon_path = Path::new(icon_name);
        let data = if icon_path.is_absolute() && icon_path.exists() {
            load_image_or_svg_base64(icon_path)
        } else {
            let candidates = [
                format!("/usr/share/pixmaps/{icon_name}.png"),
                format!("/usr/share/pixmaps/{icon_name}.svg"),
                format!("/usr/share/icons/hicolor/48x48/apps/{icon_name}.png"),
                format!("/usr/share/icons/hicolor/scalable/apps/{icon_name}.svg"),
                format!("/usr/share/icons/hicolor/32x32/apps/{icon_name}.png"),
            ];
            candidates
                .iter()
                .map(Path::new)
                .find(|p| p.exists())
                .and_then(load_image_or_svg_base64)
        };

        if let Ok(mut guard) = ICON_CACHE.write() {
            guard.insert(app.path.clone(), data.clone());
        }

        data
    } else {
        None
    }
}

#[cfg(not(target_os = "windows"))]
fn load_image_or_svg_base64(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    if ext == "svg" {
        Some(format!("data:image/svg+xml;base64,{}", to_base64(&bytes)))
    } else if ext == "png" {
        Some(format!("data:image/png;base64,{}", to_base64(&bytes)))
    } else {
        None
    }
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

pub fn score_app_alias(app_name: &str, query: &str) -> u32 {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return 0;
    }
    let name_low = app_name.to_lowercase();

    // Check prefix expansions like "vs 2022" or "vscode insiders"
    if let Some(rest) = q.strip_prefix("vs ") {
        let rest = rest.trim();
        if !rest.is_empty() {
            let expanded = format!("visual studio {rest}");
            let s = score_match(app_name, "", &expanded);
            if s > 0 {
                return s;
            }
        }
    }
    if let Some(rest) = q.strip_prefix("vscode ") {
        let rest = rest.trim();
        if !rest.is_empty() {
            let expanded = format!("visual studio code {rest}");
            let s = score_match(app_name, "", &expanded);
            if s > 0 {
                return s;
            }
        }
    }
    if let Some(rest) = q.strip_prefix("code ") {
        let rest = rest.trim();
        if !rest.is_empty() {
            let expanded = format!("visual studio code {rest}");
            let s = score_match(app_name, "", &expanded);
            if s > 0 {
                return s;
            }
        }
    }

    match q.as_str() {
        "vs" => {
            if name_low.contains("visual studio") {
                if name_low.contains("code") {
                    70
                } else {
                    78
                }
            } else {
                0
            }
        }
        "code" => {
            if name_low.contains("visual studio code") || name_low.contains("vs code") {
                75
            } else {
                0
            }
        }
        "vscode" => {
            if name_low.contains("visual studio code")
                || name_low.contains("vs code")
                || name_low.contains("vscodium")
            {
                85
            } else {
                0
            }
        }
        "cmd" => {
            if name_low.contains("command prompt") {
                85
            } else {
                0
            }
        }
        "ps" | "pwsh" => {
            if name_low.contains("powershell") {
                80
            } else {
                0
            }
        }
        "calc" if name_low.contains("calculator") => 85,
        _ => 0,
    }
}

fn walk_directory_files(
    dir: &Path,
    current_depth: usize,
    max_depth: usize,
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

        if file_type.is_symlink() {
            continue;
        }

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

            let folder_parent = path
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Folder".to_string());

            let location = path.to_string_lossy().to_string();
            let icon = get_folder_icon();

            indexed_files.push(IndexedFile {
                name: name_str.to_string(),
                subtitle: folder_parent,
                path: path.clone(),
                location,
                is_image: false,
                extension: String::new(),
                right_label: "Folder".to_string(),
                size: None,
                icon,
            });

            walk_directory_files(
                &path,
                current_depth + 1,
                max_depth,
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

            let folder_name = path
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Files".to_string());

            let location = path
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());
            let size = entry.metadata().ok().map(|m| format_file_size(m.len()));
            let icon = get_file_type_icon(&ext);

            indexed_files.push(IndexedFile {
                name: name_str.to_string(),
                subtitle: folder_name,
                path,
                location,
                is_image,
                extension: ext,
                right_label,
                size,
                icon,
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

#[cfg(target_os = "windows")]
pub fn get_available_windows_drives() -> Vec<PathBuf> {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetLogicalDrives() -> u32;
        fn GetDriveTypeW(lp_root_path_name: *const u16) -> u32;
    }
    const DRIVE_REMOVABLE: u32 = 2;
    const DRIVE_FIXED: u32 = 3;

    let mut drives = Vec::new();
    unsafe {
        let bitmask = GetLogicalDrives();
        for i in 0..26 {
            if (bitmask & (1 << i)) != 0 {
                let letter = (b'A' + i as u8) as char;
                let root_str = format!("{}:\\", letter);
                let wide: Vec<u16> = root_str.encode_utf16().chain(Some(0)).collect();
                let drive_type = GetDriveTypeW(wide.as_ptr());
                if drive_type == DRIVE_FIXED || drive_type == DRIVE_REMOVABLE {
                    drives.push(PathBuf::from(root_str));
                }
            }
        }
    }
    if drives.is_empty() {
        drives.push(PathBuf::from("C:\\"));
    }
    drives
}

pub fn get_user_search_directories(home_dir: &Option<PathBuf>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut seen = HashSet::new();

    let mut add_dir = |p: PathBuf| {
        let norm = p.to_string_lossy().to_lowercase();
        if p.is_dir() && seen.insert(norm) {
            dirs.push(p);
        }
    };

    if let Some(home) = home_dir {
        add_dir(home.join("Desktop"));
        add_dir(home.join("Documents"));
        add_dir(home.join("Downloads"));
        add_dir(home.join("Pictures"));
        add_dir(home.join("Projects"));
        add_dir(home.join("projects"));
        add_dir(home.join("source"));
        add_dir(home.join("repos"));
        add_dir(home.join("dev"));
        add_dir(home.join("Work"));
    }

    #[cfg(target_os = "windows")]
    {
        let available_drives = get_available_windows_drives();

        // 1. Check common user folders on all drives first (e.g. D:\projects, C:\projects, etc.)
        for drive in &available_drives {
            let candidates = [
                "projects",
                "Projects",
                "Documents",
                "Downloads",
                "Pictures",
                "Desktop",
                "Work",
                "Media",
                "Files",
                "Data",
                "dev",
                "source",
                "repos",
            ];
            for cand in candidates {
                add_dir(drive.join(cand));
            }
        }

        // 2. Sort drives so non-system drives (D:\, etc.) are indexed before C:\ root
        let mut sorted_drives = available_drives.clone();
        sorted_drives.sort_by_key(|d| {
            let s = d.to_string_lossy().to_uppercase();
            if s.starts_with("C:") { 1 } else { 0 }
        });

        for drive in sorted_drives {
            if let Ok(entries) = fs::read_dir(&drive) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let name = entry.file_name();
                        let name_str = name.to_string_lossy().to_lowercase();
                        if !name_str.starts_with('$')
                            && !name_str.starts_with('.')
                            && !IGNORED_DIR_NAMES.contains(&name_str.as_str())
                        {
                            add_dir(path);
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Linux: check /media/$USER, /run/media/$USER, and /mnt
        if let Some(user) = std::env::var_os("USER") {
            let media = PathBuf::from("/media").join(&user);
            if let Ok(entries) = fs::read_dir(&media) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        add_dir(path);
                    }
                }
            }
            let run_media = PathBuf::from("/run/media").join(&user);
            if let Ok(entries) = fs::read_dir(&run_media) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        add_dir(path);
                    }
                }
            }
        }
    }

    dirs
}

#[cfg(target_os = "windows")]
fn scan_windows_start_menu_apps() -> Vec<IndexedApp> {
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
                    walk_shortcuts(&path, depth + 1, seen_names, apps);
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
                    let subtitle = "Application".to_string();
                    let (icon, location, size, version) = resolve_app_metadata(&path, None, None);
                    apps.push(IndexedApp {
                        name: stem,
                        subtitle,
                        path,
                        location,
                        exec: None,
                        icon,
                        color,
                        keywords: String::new(),
                        size,
                        version,
                    });
                }
            }
        }
    }

    for root in start_menu_roots {
        if root.exists() {
            walk_shortcuts(&root, 0, &mut seen_names, &mut apps);
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
                    if let Some(app) = parse_desktop_entry(&path, &content) {
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
fn parse_desktop_entry(path: &Path, content: &str) -> Option<IndexedApp> {
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
    let subtitle = comment.unwrap_or_else(|| "Application".to_string());
    let (resolved_icon, location, size, version) =
        resolve_app_metadata(path, exec.as_deref(), icon.as_deref());

    Some(IndexedApp {
        name: app_name,
        subtitle,
        path: path.to_path_buf(),
        location,
        exec,
        icon: resolved_icon,
        color,
        keywords: String::new(),
        size,
        version,
    })
}

pub fn build_full_search_index() -> SearchIndex {
    let home_dir = get_user_home_dir();

    #[cfg(target_os = "windows")]
    let apps = scan_windows_start_menu_apps();

    #[cfg(not(target_os = "windows"))]
    let apps = scan_linux_desktop_apps(home_dir.as_deref());

    let mut files = Vec::new();
    let search_roots = get_user_search_directories(&home_dir);

    for root in search_roots {
        walk_directory_files(
            &root,
            0,
            MAX_DIR_DEPTH,
            &mut files,
        );
    }

    files.sort_by(|a, b| {
        let a_is_folder = a.right_label == "Folder";
        let b_is_folder = b.right_label == "Folder";
        b_is_folder
            .cmp(&a_is_folder)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    SearchIndex {
        apps,
        files,
        is_indexed: true,
    }
}

fn resolve_app_metadata(
    path: &Path,
    exec: Option<&str>,
    icon_name: Option<&str>,
) -> (Option<String>, String, Option<String>, Option<String>) {
    #[cfg(target_os = "windows")]
    {
        let _ = exec;
        let _ = icon_name;
        let target = if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("lnk")) {
            resolve_lnk_target(path).unwrap_or_else(|| path.to_path_buf())
        } else {
            path.to_path_buf()
        };

        let location = target.to_string_lossy().to_string();
        let icon = get_or_extract_icon(path).or_else(|| get_or_extract_icon(&target));
        let size = fs::metadata(&target)
            .or_else(|_| fs::metadata(path))
            .ok()
            .map(|m| format_file_size(m.len()));
        let version = get_pe_file_version(&target);

        (icon, location, size, version)
    }

    #[cfg(not(target_os = "windows"))]
    {
        let location = exec.map(|s| s.to_string()).unwrap_or_else(|| path.to_string_lossy().to_string());
        let icon = icon_name.and_then(|name| {
            let p = Path::new(name);
            if p.is_absolute() && p.exists() {
                load_image_or_svg_base64(p)
            } else {
                let candidates = [
                    format!("/usr/share/pixmaps/{name}.png"),
                    format!("/usr/share/pixmaps/{name}.svg"),
                    format!("/usr/share/icons/hicolor/48x48/apps/{name}.png"),
                    format!("/usr/share/icons/hicolor/scalable/apps/{name}.svg"),
                    format!("/usr/share/icons/hicolor/32x32/apps/{name}.png"),
                ];
                candidates
                    .iter()
                    .map(Path::new)
                    .find(|p| p.exists())
                    .and_then(load_image_or_svg_base64)
            }
        });
        let size = fs::metadata(path).ok().map(|m| format_file_size(m.len()));
        (icon, location, size, None)
    }
}

fn build_app_search_result(app: &IndexedApp) -> SearchResultItem {
    SearchResultItem {
        id: format!("app:{}", app.path.to_string_lossy()),
        category: "app".to_string(),
        name: app.name.clone(),
        subtitle: app.subtitle.clone(),
        path: app.path.to_string_lossy().to_string(),
        location: app.location.clone(),
        icon: app.icon.clone(),
        thumbnail: None,
        color: Some(app.color.clone()),
        is_image: false,
        right_label: "Application".to_string(),
        size: app.size.clone(),
        version: app.version.clone(),
    }
}

fn build_file_search_result(file: &IndexedFile) -> SearchResultItem {
    SearchResultItem {
        id: format!("file:{}", file.path.to_string_lossy()),
        category: "file".to_string(),
        name: file.name.clone(),
        subtitle: file.subtitle.clone(),
        path: file.path.to_string_lossy().to_string(),
        location: file.location.clone(),
        icon: file.icon.clone(),
        thumbnail: None,
        color: None,
        is_image: file.is_image,
        right_label: file.right_label.clone(),
        size: file.size.clone(),
        version: None,
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
            .map(build_app_search_result)
            .collect();

        return (app_suggestions, Vec::new());
    }

    // Search applications
    let mut matched_apps: Vec<(u32, &IndexedApp)> = index
        .apps
        .iter()
        .filter_map(|app| {
            let direct_score = score_match(&app.name, &app.keywords, trimmed);
            let alias_score = score_app_alias(&app.name, trimmed);
            let score = direct_score.max(alias_score);
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
        .map(|(_, app)| build_app_search_result(app))
        .collect();

    // Search files and folders
    let mut matched_files: Vec<(u32, &IndexedFile)> = index
        .files
        .iter()
        .filter_map(|file| {
            let mut score = score_match(&file.name, &file.subtitle, trimmed);

            // Allow path-fragment matches (e.g. "projects/lux" or "lux\page")
            if trimmed.contains('/') || trimmed.contains('\\') {
                let query_norm = trimmed.replace('\\', "/").to_lowercase();
                let path_norm = file.path.to_string_lossy().replace('\\', "/").to_lowercase();
                if path_norm.ends_with(&query_norm) {
                    score = score.max(90);
                } else if path_norm.contains(&query_norm) {
                    score = score.max(45);
                }
            }

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
        .map(|(_, file)| {
            let mut item = build_file_search_result(file);
            if file.is_image {
                item.thumbnail = get_or_generate_thumbnail(&file.path);
            }
            item
        })
        .collect();

    (app_results, file_results)
}

pub fn browse_index(
    index: &SearchIndex,
    category: &str,
) -> (Vec<SearchResultItem>, Vec<SearchResultItem>) {
    match category {
        "apps" => {
            let mut apps: Vec<SearchResultItem> =
                index.apps.iter().map(build_app_search_result).collect();
            apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
            (apps, Vec::new())
        }
        "files" => {
            const MAX_BROWSE_FILES: usize = 500;
            let files: Vec<SearchResultItem> = index
                .files
                .iter()
                .take(MAX_BROWSE_FILES)
                .map(|f| {
                    let mut item = build_file_search_result(f);
                    if f.is_image {
                        if let Ok(guard) = THUMBNAIL_CACHE.read() {
                            if let Some(cached) = guard.get(&f.path) {
                                item.thumbnail = cached.clone();
                            }
                        }
                    }
                    item
                })
                .collect();
            (Vec::new(), files)
        }
        _ => (Vec::new(), Vec::new()),
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexingStatusPayload {
    pub is_indexing: bool,
    pub total_apps: usize,
    pub total_files: usize,
}

pub const EVENT_INDEXING_STATUS: &str = "lux://indexing-status";

pub fn start_background_indexing<R: Runtime>(app_handle: AppHandle<R>) {
    std::thread::spawn(move || {
        use tauri::Emitter;

        let _ = app_handle.emit(
            EVENT_INDEXING_STATUS,
            IndexingStatusPayload {
                is_indexing: true,
                total_apps: 0,
                total_files: 0,
            },
        );

        let index = build_full_search_index();
        let total_apps = index.apps.len();
        let total_files = index.files.len();

        if let Some(state) = app_handle.try_state::<SearchState>()
            && let Ok(mut lock) = state.index.write()
        {
            *lock = index;
        }

        let _ = app_handle.emit(
            EVENT_INDEXING_STATUS,
            IndexingStatusPayload {
                is_indexing: false,
                total_apps,
                total_files,
            },
        );
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
                    location: "C:\\Program Files\\Mozilla Firefox\\firefox.exe".to_string(),
                    exec: None,
                    icon: None,
                    color: "#e8663d".to_string(),
                    keywords: "browser web".to_string(),
                    size: Some("120 MB".to_string()),
                    version: Some("130.0.0".to_string()),
                },
                IndexedApp {
                    name: "Discord".to_string(),
                    subtitle: "~/Start Menu/Discord".to_string(),
                    path: PathBuf::from("C:/Discord.lnk"),
                    location: "C:\\Users\\tester\\AppData\\Local\\Discord\\app.exe".to_string(),
                    exec: None,
                    icon: None,
                    color: "#5865f2".to_string(),
                    keywords: "chat voice".to_string(),
                    size: Some("85 MB".to_string()),
                    version: Some("1.0.9".to_string()),
                },
            ],
            files: vec![
                IndexedFile {
                    name: "wedding-photo.jpg".to_string(),
                    subtitle: "~/Pictures/wedding-photo.jpg".to_string(),
                    path: PathBuf::from("C:/Users/tester/Pictures/wedding-photo.jpg"),
                    location: "C:\\Users\\tester\\Pictures".to_string(),
                    is_image: true,
                    extension: "jpg".to_string(),
                    right_label: "Image".to_string(),
                    size: Some("4.2 MB".to_string()),
                    icon: None,
                },
                IndexedFile {
                    name: "contract.pdf".to_string(),
                    subtitle: "~/Documents/contract.pdf".to_string(),
                    path: PathBuf::from("C:/Users/tester/Documents/contract.pdf"),
                    location: "C:\\Users\\tester\\Documents".to_string(),
                    is_image: false,
                    extension: "pdf".to_string(),
                    right_label: "PDF".to_string(),
                    size: Some("250 KB".to_string()),
                    icon: None,
                },
                IndexedFile {
                    name: "Projects".to_string(),
                    subtitle: "D:\\".to_string(),
                    path: PathBuf::from("D:/Projects"),
                    location: "D:\\Projects".to_string(),
                    is_image: false,
                    extension: String::new(),
                    right_label: "Folder".to_string(),
                    size: None,
                    icon: None,
                },
                IndexedFile {
                    name: "page.svelte".to_string(),
                    subtitle: "routes".to_string(),
                    path: PathBuf::from("D:/projects/Lux/app/src/routes/page.svelte"),
                    location: "D:\\projects\\Lux\\app\\src\\routes".to_string(),
                    is_image: false,
                    extension: "svelte".to_string(),
                    right_label: "SVELTE".to_string(),
                    size: Some("12 KB".to_string()),
                    icon: None,
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

        // Test folder search
        let (_apps, files) = search_index(&index, "projects");
        assert!(!files.is_empty());
        assert_eq!(files[0].name, "Projects");
        assert_eq!(files[0].right_label, "Folder");

        // Test path fragment search
        let (_apps, files) = search_index(&index, "routes/page");
        assert!(!files.is_empty());
        assert_eq!(files[0].name, "page.svelte");

        // Test nested D:\projects files
        let (_apps, files) = search_index(&index, "page");
        assert!(!files.is_empty());
        assert_eq!(files[0].name, "page.svelte");
        assert_eq!(files[0].right_label, "SVELTE");
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

    #[test]
    fn test_application_search_aliases() {
        let make_app = |name: &str| IndexedApp {
            name: name.to_string(),
            subtitle: format!("~/Apps/{name}"),
            path: PathBuf::from(format!("C:/Apps/{name}.lnk")),
            location: format!("C:\\Apps\\{name}.exe"),
            exec: None,
            icon: None,
            color: "#5865f2".to_string(),
            keywords: String::new(),
            size: None,
            version: None,
        };

        let index = SearchIndex {
            apps: vec![
                make_app("Visual Studio 2022"),
                make_app("Visual Studio Code"),
                make_app("Firefox"),
                make_app("Discord"),
                make_app("Code"),
                make_app("Command Prompt"),
            ],
            files: vec![],
            is_indexed: true,
        };

        // 1. "vs" matches Visual Studio apps, ranking Visual Studio 2022 above Visual Studio Code
        let (apps, _) = search_index(&index, "vs");
        assert!(apps.len() >= 2);
        assert_eq!(apps[0].name, "Visual Studio 2022");
        assert_eq!(apps[1].name, "Visual Studio Code");

        // Case insensitivity
        let (apps_upper, _) = search_index(&index, "VS");
        assert_eq!(apps_upper[0].name, "Visual Studio 2022");

        // 2. "code" ranks exact match "Code" highest, followed by "Visual Studio Code"
        let (code_apps, _) = search_index(&index, "code");
        assert!(code_apps.len() >= 2);
        assert_eq!(code_apps[0].name, "Code"); // Exact match 100
        assert_eq!(code_apps[1].name, "Visual Studio Code"); // Alias match 75

        // Case insensitivity
        let (code_apps_upper, _) = search_index(&index, "CODE");
        assert_eq!(code_apps_upper[0].name, "Code");

        // 3. "vscode" directly matches Visual Studio Code
        let (vscode_apps, _) = search_index(&index, "vscode");
        assert_eq!(vscode_apps.len(), 1);
        assert_eq!(vscode_apps[0].name, "Visual Studio Code");

        let (vscode_apps_upper, _) = search_index(&index, "VSCode");
        assert_eq!(vscode_apps_upper.len(), 1);
        assert_eq!(vscode_apps_upper[0].name, "Visual Studio Code");

        // 4. "cmd" matches Command Prompt
        let (cmd_apps, _) = search_index(&index, "cmd");
        assert_eq!(cmd_apps.len(), 1);
        assert_eq!(cmd_apps[0].name, "Command Prompt");

        // 5. No false positives on unrelated applications
        let (apps_empty, _) = search_index(&index, "xyznotanapp");
        assert!(apps_empty.is_empty());
    }

    #[test]
    fn test_browse_index() {
        let index = SearchIndex {
            apps: vec![IndexedApp {
                name: "Firefox".to_string(),
                subtitle: String::new(),
                path: PathBuf::from("C:\\Firefox.lnk"),
                location: "C:\\Firefox.exe".to_string(),
                exec: None,
                icon: None,
                color: "#e8663d".to_string(),
                keywords: String::new(),
                size: None,
                version: None,
            }],
            files: vec![IndexedFile {
                name: "test.pdf".to_string(),
                subtitle: "Documents".to_string(),
                path: PathBuf::from("C:\\test.pdf"),
                location: "C:\\".to_string(),
                is_image: false,
                extension: "pdf".to_string(),
                right_label: "PDF".to_string(),
                size: None,
                icon: None,
            }],
            is_indexed: true,
        };

        let (apps, files) = browse_index(&index, "apps");
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Firefox");
        assert_eq!(files.len(), 0);

        let (apps, files) = browse_index(&index, "files");
        assert_eq!(apps.len(), 0);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "test.pdf");

        let (apps, files) = browse_index(&index, "actions");
        assert_eq!(apps.len(), 0);
        assert_eq!(files.len(), 0);
    }

    #[test]
    fn test_image_thumbnail_generation_and_error_handling() {
        // Test non-existent file returns None gracefully
        assert_eq!(
            generate_image_thumbnail_base64(Path::new("C:\\does_not_exist_xyz.png"), 128),
            None
        );

        // Test non-image file returns None gracefully
        let temp_dir = std::env::temp_dir();
        let non_image_path = temp_dir.join("lux_test_text.txt");
        let _ = fs::write(&non_image_path, "not an image content");
        assert_eq!(generate_image_thumbnail_base64(&non_image_path, 128), None);
        let _ = fs::remove_file(&non_image_path);

        // Test valid synthetic image generation
        let img_path = temp_dir.join("lux_test_image.png");
        let img = image::RgbaImage::new(32, 32);
        if img.save(&img_path).is_ok() {
            let thumb = generate_image_thumbnail_base64(&img_path, 16);
            assert!(thumb.is_some());
            let thumb_str = thumb.unwrap();
            assert!(thumb_str.starts_with("data:image/png;base64,"));

            let cached = get_or_generate_thumbnail(&img_path);
            assert!(cached.is_some());
            let _ = fs::remove_file(&img_path);
        }
    }
}
