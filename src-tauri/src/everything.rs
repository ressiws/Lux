use std::ffi::{c_char, c_void};
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::search::{SearchResultItem, IMAGE_EXTENSIONS};

const CREATE_NO_WINDOW: u32 = 0x08000000;

const EVERYTHING_REQUEST_FILE_NAME: u32 = 0x00000001;
const EVERYTHING_REQUEST_PATH: u32 = 0x00000002;
const EVERYTHING_REQUEST_SIZE: u32 = 0x00000010;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(lpLibFileName: *const u16) -> *mut c_void;
    fn FreeLibrary(hLibModule: *mut c_void) -> i32;
    fn GetProcAddress(hModule: *mut c_void, lpProcName: *const c_char) -> *mut c_void;
}

#[derive(Clone)]
struct RawEverythingDll {
    module: *mut c_void,
    set_search_w: unsafe extern "system" fn(*const u16),
    set_request_flags: unsafe extern "system" fn(u32),
    set_max: unsafe extern "system" fn(u32),
    query_w: unsafe extern "system" fn(i32) -> i32,
    get_last_error: unsafe extern "system" fn() -> u32,
    get_num_results: unsafe extern "system" fn() -> u32,
    get_result_file_name_w: unsafe extern "system" fn(u32) -> *const u16,
    get_result_path_w: unsafe extern "system" fn(u32) -> *const u16,
    is_folder_result: unsafe extern "system" fn(u32) -> i32,
    get_result_size: unsafe extern "system" fn(u32, *mut u64) -> i32,
    is_db_loaded: unsafe extern "system" fn() -> i32,
    reset: unsafe extern "system" fn(),
    clean_up: unsafe extern "system" fn(),
    exit: unsafe extern "system" fn() -> i32,
}

// SAFETY: All accesses through RawEverythingDll are synchronized via EverythingClient mutex.
unsafe impl Send for RawEverythingDll {}
unsafe impl Sync for RawEverythingDll {}

impl RawEverythingDll {
    unsafe fn load(dll_path: &Path) -> Result<Self, String> {
        let wide_path: Vec<u16> = dll_path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let module = unsafe { LoadLibraryW(wide_path.as_ptr()) };
        if module.is_null() {
            return Err(format!("Failed to load DLL from {}", dll_path.display()));
        }

        macro_rules! resolve {
            ($sym:literal, $t:ty) => {{
                let sym_name = concat!($sym, "\0").as_ptr() as *const c_char;
                let proc = unsafe { GetProcAddress(module, sym_name) };
                if proc.is_null() {
                    unsafe { FreeLibrary(module) };
                    return Err(format!("Failed to resolve symbol {}", $sym));
                }
                let func: $t = unsafe { std::mem::transmute(proc) };
                func
            }};
        }

        let set_search_w = resolve!(
            "Everything_SetSearchW",
            unsafe extern "system" fn(*const u16)
        );
        let set_request_flags = resolve!(
            "Everything_SetRequestFlags",
            unsafe extern "system" fn(u32)
        );
        let set_max = resolve!(
            "Everything_SetMax",
            unsafe extern "system" fn(u32)
        );
        let query_w = resolve!(
            "Everything_QueryW",
            unsafe extern "system" fn(i32) -> i32
        );
        let get_last_error = resolve!(
            "Everything_GetLastError",
            unsafe extern "system" fn() -> u32
        );
        let get_num_results = resolve!(
            "Everything_GetNumResults",
            unsafe extern "system" fn() -> u32
        );
        let get_result_file_name_w = resolve!(
            "Everything_GetResultFileNameW",
            unsafe extern "system" fn(u32) -> *const u16
        );
        let get_result_path_w = resolve!(
            "Everything_GetResultPathW",
            unsafe extern "system" fn(u32) -> *const u16
        );
        let is_folder_result = resolve!(
            "Everything_IsFolderResult",
            unsafe extern "system" fn(u32) -> i32
        );
        let get_result_size = resolve!(
            "Everything_GetResultSize",
            unsafe extern "system" fn(u32, *mut u64) -> i32
        );
        let is_db_loaded = resolve!(
            "Everything_IsDBLoaded",
            unsafe extern "system" fn() -> i32
        );
        let reset = resolve!(
            "Everything_Reset",
            unsafe extern "system" fn()
        );
        let clean_up = resolve!(
            "Everything_CleanUp",
            unsafe extern "system" fn()
        );
        let exit = resolve!(
            "Everything_Exit",
            unsafe extern "system" fn() -> i32
        );

        Ok(Self {
            module,
            set_search_w,
            set_request_flags,
            set_max,
            query_w,
            get_last_error,
            get_num_results,
            get_result_file_name_w,
            get_result_path_w,
            is_folder_result,
            get_result_size,
            is_db_loaded,
            reset,
            clean_up,
            exit,
        })
    }
}

impl Drop for RawEverythingDll {
    fn drop(&mut self) {
        if !self.module.is_null() {
            unsafe {
                (self.clean_up)();
                FreeLibrary(self.module);
            }
            self.module = std::ptr::null_mut();
        }
    }
}

pub struct EverythingClient {
    dll: RawEverythingDll,
}

impl EverythingClient {
    pub fn new(dll_path: &Path) -> Result<Self, String> {
        let dll = unsafe { RawEverythingDll::load(dll_path)? };
        Ok(Self { dll })
    }

    pub fn is_db_loaded(&self) -> bool {
        unsafe { (self.dll.is_db_loaded)() != 0 }
    }

    pub fn query(&self, search_term: &str, max_results: u32) -> Result<Vec<SearchResultItem>, String> {
        let wide_search: Vec<u16> = search_term
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        unsafe {
            (self.dll.reset)();
            (self.dll.set_search_w)(wide_search.as_ptr());
            (self.dll.set_request_flags)(
                EVERYTHING_REQUEST_FILE_NAME | EVERYTHING_REQUEST_PATH | EVERYTHING_REQUEST_SIZE,
            );
            (self.dll.set_max)(max_results);

            let ok = (self.dll.query_w)(1);
            if ok == 0 {
                let err = (self.dll.get_last_error)();
                return Err(format!("Everything query failed with error code: {err}"));
            }

            let num_results = (self.dll.get_num_results)();
            let mut items = Vec::with_capacity(num_results as usize);

            for i in 0..num_results {
                let fname_ptr = (self.dll.get_result_file_name_w)(i);
                let path_ptr = (self.dll.get_result_path_w)(i);

                if fname_ptr.is_null() || path_ptr.is_null() {
                    continue;
                }

                let name = from_wide_ptr(fname_ptr);
                let dir_path = from_wide_ptr(path_ptr);
                let is_folder = (self.dll.is_folder_result)(i) != 0;

                let mut size_bytes: u64 = 0;
                let has_size = (self.dll.get_result_size)(i, &mut size_bytes) != 0;

                let full_path = if dir_path.ends_with('\\') {
                    format!("{}{}", dir_path, name)
                } else {
                    format!("{}\\{}", dir_path, name)
                };

                let extension = if is_folder {
                    String::new()
                } else {
                    Path::new(&name)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase()
                };

                let is_image = !is_folder && IMAGE_EXTENSIONS.contains(&extension.as_str());

                let right_label = if is_folder {
                    "Folder".to_string()
                } else if !extension.is_empty() {
                    extension.to_uppercase()
                } else {
                    "File".to_string()
                };

                let size_str = if is_folder {
                    None
                } else if has_size {
                    Some(format_size(size_bytes))
                } else {
                    None
                };

                items.push(SearchResultItem {
                    id: format!("file:{}", full_path),
                    category: "file".to_string(),
                    name,
                    subtitle: dir_path.clone(),
                    path: full_path,
                    location: dir_path,
                    icon: None,
                    thumbnail: None,
                    color: None,
                    is_image,
                    right_label,
                    size: size_str,
                    version: None,
                });
            }

            Ok(items)
        }
    }

    pub fn exit_engine(&self) -> bool {
        unsafe { (self.dll.exit)() != 0 }
    }
}

unsafe fn from_wide_ptr(ptr: *const u16) -> String {
    let mut len = 0;
    unsafe {
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(ptr, len);
        String::from_utf16_lossy(slice)
    }
}

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[derive(Debug, Clone)]
pub struct EverythingPaths {
    pub exe_path: PathBuf,
    pub dll_path: PathBuf,
    pub ini_path: PathBuf,
    pub db_path: PathBuf,
}

pub struct EverythingManager {
    paths: EverythingPaths,
    client: Option<Arc<Mutex<EverythingClient>>>,
    child_process: Option<Child>,
    managed_by_us: bool,
}

impl EverythingManager {
    pub fn new(resource_dir: Option<&Path>, app_data_dir: &Path) -> Result<Self, String> {
        let everything_data_dir = app_data_dir.join("everything");
        let bin_dir = everything_data_dir.join("bin");
        fs::create_dir_all(&bin_dir)
            .map_err(|e| format!("Failed to create everything app data dir: {e}"))?;

        let (src_exe, src_dll) = locate_resource_binaries(resource_dir)?;

        let dest_exe = bin_dir.join("everything.exe");
        let dest_dll = bin_dir.join("Everything64.dll");

        copy_if_needed(&src_exe, &dest_exe)?;
        copy_if_needed(&src_dll, &dest_dll)?;

        let ini_path = everything_data_dir.join("Everything.ini");
        let db_path = everything_data_dir.join("Everything.db");

        ensure_config_exists(&ini_path)?;

        let paths = EverythingPaths {
            exe_path: dest_exe,
            dll_path: dest_dll,
            ini_path,
            db_path,
        };

        Ok(Self {
            paths,
            client: None,
            child_process: None,
            managed_by_us: false,
        })
    }

    pub fn start_or_connect(&mut self) -> Result<(), String> {
        // Load the DLL client first
        let client = EverythingClient::new(&self.paths.dll_path)?;
        let client_arc = Arc::new(Mutex::new(client));

        // Check if an existing Everything instance is already running and loaded
        let db_loaded = client_arc.lock().map(|c| c.is_db_loaded()).unwrap_or(false);

        if db_loaded {
            self.client = Some(client_arc);
            self.managed_by_us = false;
            return Ok(());
        }

        // Not running or DB not loaded yet: spawn background child process
        let mut cmd = Command::new(&self.paths.exe_path);
        cmd.arg("-config")
            .arg(&self.paths.ini_path)
            .arg("-db")
            .arg(&self.paths.db_path)
            .arg("-startup")
            .creation_flags(CREATE_NO_WINDOW);

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn Everything process: {e}"))?;

        self.child_process = Some(child);
        self.managed_by_us = true;
        self.client = Some(client_arc.clone());

        // Wait up to 3 seconds for DB to load in background
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(3000) {
            std::thread::sleep(Duration::from_millis(50));
            if let Ok(c) = client_arc.lock() {
                if c.is_db_loaded() {
                    break;
                }
            }
        }

        Ok(())
    }

    pub fn is_available(&self) -> bool {
        if let Some(ref c) = self.client {
            if let Ok(guard) = c.lock() {
                return guard.is_db_loaded();
            }
        }
        false
    }

    pub fn search(&self, query: &str, max_results: u32) -> Result<Vec<SearchResultItem>, String> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| "Everything client not initialized".to_string())?;

        let guard = client
            .lock()
            .map_err(|_| "Failed to lock Everything client".to_string())?;

        guard.query(query, max_results)
    }

    pub fn stop(&mut self) {
        if self.managed_by_us {
            if let Some(ref client) = self.client {
                if let Ok(c) = client.lock() {
                    let _ = c.exit_engine();
                }
            }

            if let Some(mut child) = self.child_process.take() {
                // Give child process up to 500ms to exit gracefully
                let start = Instant::now();
                let mut exited = false;
                while start.elapsed() < Duration::from_millis(500) {
                    if let Ok(Some(_)) = child.try_wait() {
                        exited = true;
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }

                if !exited {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
            self.managed_by_us = false;
        }
        self.client = None;
    }
}

impl Drop for EverythingManager {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn locate_resource_binaries(resource_dir: Option<&Path>) -> Result<(PathBuf, PathBuf), String> {
    let mut candidate_dirs: Vec<PathBuf> = Vec::new();

    if let Some(res) = resource_dir {
        candidate_dirs.push(res.join("resources").join("everything"));
        candidate_dirs.push(res.join("everything"));
        candidate_dirs.push(res.to_path_buf());
    }

    if let Ok(cur_exe) = std::env::current_exe() {
        if let Some(exe_dir) = cur_exe.parent() {
            candidate_dirs.push(exe_dir.join("resources").join("everything"));
            candidate_dirs.push(exe_dir.join("everything"));
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        candidate_dirs.push(cwd.join("resources").join("everything"));
        candidate_dirs.push(cwd.join("src-tauri").join("resources").join("everything"));
        candidate_dirs.push(cwd.join("..").join("src-tauri").join("resources").join("everything"));
    }

    for dir in &candidate_dirs {
        let exe = dir.join("everything.exe");
        let dll = dir.join("Everything64.dll");
        if exe.exists() && dll.exists() {
            return Ok((exe, dll));
        }
    }

    Err(format!(
        "Could not locate Everything bundled binaries in any candidate location: {:?}",
        candidate_dirs
    ))
}

fn copy_if_needed(src: &Path, dst: &Path) -> Result<(), String> {
    let need_copy = if dst.exists() {
        let src_meta = fs::metadata(src).map_err(|e| e.to_string())?;
        let dst_meta = fs::metadata(dst).map_err(|e| e.to_string())?;
        src_meta.len() != dst_meta.len()
    } else {
        true
    };

    if need_copy {
        fs::copy(src, dst).map_err(|e| {
            format!(
                "Failed to copy {} to {}: {}",
                src.display(),
                dst.display(),
                e
            )
        })?;
    }
    Ok(())
}

fn ensure_config_exists(ini_path: &Path) -> Result<(), String> {
    if !ini_path.exists() {
        let home_dir = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\Users".to_string());
        let programs = "C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs";
        let ini_content = format!(
            "; Lux managed Everything configuration\r\n\
run_as_admin=0\r\n\
show_tray_icon=0\r\n\
run_in_background=1\r\n\
close_on_exit=1\r\n\
check_for_updates_on_startup=0\r\n\
allow_http_server=0\r\n\
allow_etp_server=0\r\n\
hide_empty_search_results=0\r\n\
folders=\"{}\",\"{}\"\r\n",
            programs, home_dir
        );
        fs::write(ini_path, ini_content)
            .map_err(|e| format!("Failed to create Everything.ini at {}: {}", ini_path.display(), e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1 KB");
        assert_eq!(format_size(1536), "2 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
        assert_eq!(format_size(1073741824), "1.0 GB");
        assert_eq!(format_size(2684354560), "2.5 GB");
    }

    #[test]
    fn test_locate_resource_binaries() {
        let res = locate_resource_binaries(None);
        assert!(res.is_ok(), "Expected to find bundled Everything binaries: {:?}", res.err());
        let (exe, dll) = res.unwrap();
        assert!(exe.exists());
        assert!(dll.exists());
    }

    #[test]
    fn test_dll_load_and_query() {
        let (_, dll) = locate_resource_binaries(None).expect("Find DLL");
        let client = EverythingClient::new(&dll).expect("Load DLL");
        let _loaded = client.is_db_loaded();
    }

    #[test]
    fn test_manager_lifecycle_and_search() {
        let temp_dir = std::env::temp_dir().join("lux_test_everything_mgr");
        let _ = fs::remove_dir_all(&temp_dir);
        let mut mgr = EverythingManager::new(None, &temp_dir).expect("init manager");
        mgr.start_or_connect().expect("start or connect");
        println!("is_available: {}", mgr.is_available());
        let res = mgr.search("windows", 5);
        println!("Search result: {:?}", res);
        mgr.stop();
        assert!(!mgr.is_available());
        let _ = fs::remove_dir_all(&temp_dir);
    }
}
