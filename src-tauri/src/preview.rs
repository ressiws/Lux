use std::fs::{self, File};


use std::io::{Read, Seek, SeekFrom};


use std::path::Path;


use std::time::UNIX_EPOCH;


use serde::{Deserialize, Serialize};





#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]


#[serde(rename_all = "lowercase")]


pub enum FileFormatKind {


    Pdf,


    Markdown,


    Text,


    Image,


    Video,


    Audio,


    Code,


    Archive,


    Document,


    Folder,


    Generic,


}





impl FileFormatKind {


    pub fn as_str(&self) -> &'static str {


        match self {


            Self::Pdf => "pdf",


            Self::Markdown => "markdown",


            Self::Text => "text",


            Self::Image => "image",


            Self::Video => "video",


            Self::Audio => "audio",


            Self::Code => "code",


            Self::Archive => "archive",


            Self::Document => "document",


            Self::Folder => "folder",


            Self::Generic => "generic",


        }


    }


}





pub fn detect_format_kind(path: &Path) -> FileFormatKind {


    if path.is_dir() {


        return FileFormatKind::Folder;


    }





    let ext = match path.extension().and_then(|e| e.to_str()) {


        Some(e) => e.to_lowercase(),


        None => return FileFormatKind::Generic,


    };





    match ext.as_str() {


        "pdf" => FileFormatKind::Pdf,





        "md" | "markdown" | "mdown" | "mkd" => FileFormatKind::Markdown,





        "txt" | "log" | "rtf" | "nfo" | "ini" | "cfg" | "conf" => FileFormatKind::Text,





        "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "svg" | "ico" | "tiff" | "tif"


        | "avif" | "heic" => FileFormatKind::Image,





        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "m4v" | "3gp" => {


            FileFormatKind::Video


        }





        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "wma" | "aac" | "opus" | "alac" => {


            FileFormatKind::Audio


        }





        "rs" | "ts" | "js" | "jsx" | "tsx" | "py" | "c" | "cpp" | "h" | "hpp" | "cs"


        | "go" | "java" | "html" | "css" | "scss" | "json" | "toml" | "yaml" | "yml"


        | "xml" | "sql" | "sh" | "bat" | "ps1" | "cmd" | "php" | "rb" | "swift" | "kt"


        | "dart" | "lua" | "vue" | "svelte" => FileFormatKind::Code,





        "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" | "iso" | "tgz" => {


            FileFormatKind::Archive


        }





        "doc" | "docx" | "xls" | "xlsx" | "csv" | "ppt" | "pptx" | "odt" | "ods" | "odp"


        | "epub" => FileFormatKind::Document,





        _ => FileFormatKind::Generic,


    }


}





pub fn get_format_label(path: &Path, kind: FileFormatKind) -> String {


    let ext_upper = path


        .extension()


        .and_then(|e| e.to_str())


        .map(|s| s.to_uppercase())


        .unwrap_or_default();





    match kind {


        FileFormatKind::Pdf => "PDF Document".to_string(),


        FileFormatKind::Markdown => "Markdown Document".to_string(),


        FileFormatKind::Text => {


            if ext_upper.is_empty() {


                "Text Document".to_string()


            } else {


                format!("{ext_upper} Document")


            }


        }


        FileFormatKind::Image => {


            if ext_upper.is_empty() {


                "Image".to_string()


            } else {


                format!("{ext_upper} Image")


            }


        }


        FileFormatKind::Video => {


            if ext_upper.is_empty() {


                "Video".to_string()


            } else {


                format!("{ext_upper} Video")


            }


        }


        FileFormatKind::Audio => {


            if ext_upper.is_empty() {


                "Audio".to_string()


            } else {


                format!("{ext_upper} Audio")


            }


        }


        FileFormatKind::Code => match ext_upper.as_str() {


            "RS" => "Rust Source File".to_string(),


            "TS" | "TSX" => "TypeScript File".to_string(),


            "JS" | "JSX" => "JavaScript File".to_string(),


            "PY" => "Python Source File".to_string(),


            "JSON" => "JSON Document".to_string(),


            "TOML" => "TOML Configuration".to_string(),


            "YAML" | "YML" => "YAML Configuration".to_string(),


            "HTML" => "HTML Document".to_string(),


            "CSS" | "SCSS" => "Stylesheet".to_string(),


            "C" | "H" => "C Source File".to_string(),


            "CPP" | "HPP" => "C++ Source File".to_string(),


            "GO" => "Go Source File".to_string(),


            "JAVA" => "Java Source File".to_string(),


            "CS" => "C# Source File".to_string(),


            "SH" | "BASH" => "Shell Script".to_string(),


            "BAT" | "CMD" => "Batch Script".to_string(),


            "PS1" => "PowerShell Script".to_string(),


            "SQL" => "SQL Script".to_string(),


            "XML" => "XML Document".to_string(),


            _ => format!("{ext_upper} Source File"),


        },


        FileFormatKind::Archive => {


            if ext_upper.is_empty() {


                "Archive".to_string()


            } else {


                format!("{ext_upper} Archive")


            }


        }


        FileFormatKind::Document => {


            if ext_upper.is_empty() {


                "Document".to_string()


            } else {


                format!("{ext_upper} Document")


            }


        }


        FileFormatKind::Folder => "Folder".to_string(),


        FileFormatKind::Generic => {


            if ext_upper.is_empty() {


                "File".to_string()


            } else {


                format!("{ext_upper} File")


            }


        }


    }


}





pub fn format_bytes(bytes: u64) -> String {


    crate::everything::format_size(bytes)


}





#[derive(Debug, Clone, Serialize, Deserialize)]


#[serde(rename_all = "camelCase")]


pub struct ArchiveEntry {


    pub name: String,


    pub is_dir: bool,


    pub size_bytes: u64,


    pub size_formatted: String,


}





#[derive(Debug, Clone, Serialize, Deserialize)]


#[serde(rename_all = "camelCase")]


pub struct FilePreviewInfo {


    pub path: String,


    pub filename: String,


    pub file_size_bytes: u64,


    pub file_size_formatted: String,


    pub format_kind: String,


    pub mime_or_type_label: String,


    pub modified_timestamp: Option<u64>,


    pub modified_formatted: Option<String>,


    pub text_content: Option<String>,


    pub is_truncated: bool,


    pub line_count: Option<usize>,


    pub dimensions: Option<(u32, u32)>,


    pub page_count: Option<usize>,


    pub archive_entries: Option<Vec<ArchiveEntry>>,


    pub error: Option<String>,


}





pub fn get_file_preview_info(path_str: &str) -> Result<FilePreviewInfo, String> {


    if path_str.trim().is_empty() {


        return Err("File path is empty".to_string());


    }





    let path = Path::new(path_str);


    let filename = path


        .file_name()


        .and_then(|n| n.to_str())


        .unwrap_or(path_str)


        .to_string();





    let kind = detect_format_kind(path);


    let type_label = get_format_label(path, kind);





    let metadata = match fs::metadata(path) {


        Ok(m) => m,


        Err(err) => {


            return Ok(FilePreviewInfo {


                path: path_str.to_string(),


                filename,


                file_size_bytes: 0,


                file_size_formatted: "0 B".to_string(),


                format_kind: kind.as_str().to_string(),


                mime_or_type_label: type_label,


                modified_timestamp: None,


                modified_formatted: None,


                text_content: None,


                is_truncated: false,


                line_count: None,


                dimensions: None,


                page_count: None,


                archive_entries: None,


                error: Some(format!("Could not access file: {err}")),


            });


        }


    };





    let file_size_bytes = metadata.len();


    let file_size_formatted = format_bytes(file_size_bytes);





    let (modified_timestamp, modified_formatted) = match metadata.modified() {


        Ok(t) => {


            let secs = t.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);


            (Some(secs), Some(format_timestamp(secs)))


        }


        Err(_) => (None, None),


    };





    let mut text_content = None;


    let mut is_truncated = false;


    let mut line_count = None;


    let mut dimensions = None;


    let mut page_count = None;


    let mut archive_entries = None;





    if kind == FileFormatKind::Pdf {


        page_count = detect_pdf_page_count(path);


    }





    match kind {


        FileFormatKind::Text | FileFormatKind::Markdown | FileFormatKind::Code => {


            const MAX_READ_BYTES: u64 = 512 * 1024; // 512 KB


            if let Ok(mut file) = File::open(path) {


                let to_read = file_size_bytes.min(MAX_READ_BYTES) as usize;


                let mut buf = vec![0u8; to_read];


                if file.read_exact(&mut buf).is_ok() {


                    let text = String::from_utf8_lossy(&buf).to_string();


                    line_count = Some(text.lines().count());


                    text_content = Some(text);


                    if file_size_bytes > MAX_READ_BYTES {


                        is_truncated = true;


                    }


                }


            }


        }


        FileFormatKind::Image => {


            if let Ok(reader) = image::ImageReader::open(path) {


                if let Ok(dims) = reader.into_dimensions() {


                    dimensions = Some(dims);


                }


            }


        }


        FileFormatKind::Archive => {


            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();


            if ext == "zip" {


                if let Ok(entries) = read_zip_entries(path, 100) {


                    archive_entries = Some(entries);


                }


            }


        }


        _ => {}


    }





    Ok(FilePreviewInfo {


        path: path_str.to_string(),


        filename,


        file_size_bytes,


        file_size_formatted,


        format_kind: kind.as_str().to_string(),


        mime_or_type_label: type_label,


        modified_timestamp,


        modified_formatted,


        text_content,


        is_truncated,


        line_count,


        dimensions,


        page_count,


        archive_entries,


        error: None,


    })


}





fn detect_pdf_page_count(path: &Path) -> Option<usize> {


    if let Ok(f) = File::open(path) {


        let mut buf = Vec::new();


        let _ = f.take(1024 * 1024).read_to_end(&mut buf);


        let s = String::from_utf8_lossy(&buf);


        for line in s.split('\n') {


            if let Some(idx) = line.find("/Count ") {


                let rest = &line[idx + 7..];


                let count_str: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();


                if let Ok(count) = count_str.parse::<usize>() {


                    if count > 0 {


                        return Some(count);


                    }


                }


            }


        }


    }


    None


}





fn format_timestamp(secs: u64) -> String {


    // Basic human-readable date format YYYY-MM-DD HH:MM


    let days = secs / 86400;


    let rem = secs % 86400;


    let hours = rem / 3600;


    let minutes = (rem % 3600) / 60;





    let mut y = 1970;


    let mut d = days;


    loop {


        let leap = if (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0) { 1 } else { 0 };


        let days_in_year = 365 + leap;


        if d < days_in_year {


            let mut m = 1;


            let month_days = [31, 28 + leap, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];


            for dim in &month_days {


                if d < *dim {


                    break;


                }


                d -= *dim;


                m += 1;


            }


            return format!("{:04}-{:02}-{:02} {:02}:{:02}", y, m, d + 1, hours, minutes);


        }


        d -= days_in_year;


        y += 1;


    }


}





pub fn read_zip_entries(path: &Path, max_entries: usize) -> Result<Vec<ArchiveEntry>, String> {


    let mut file = File::open(path).map_err(|e| format!("Open zip file failed: {e}"))?;


    let file_len = file.metadata().map_err(|e| format!("Metadata error: {e}"))?.len();


    if file_len < 22 {


        return Ok(Vec::new());


    }





    // Search for End of Central Directory Record (EOCD signature: 0x06054b50) in last 65557 bytes


    let search_len = file_len.min(65557);


    let mut buf = vec![0u8; search_len as usize];


    file.seek(SeekFrom::End(-(search_len as i64)))


        .map_err(|e| format!("Seek failed: {e}"))?;


    file.read_exact(&mut buf)


        .map_err(|e| format!("Read failed: {e}"))?;





    let mut eocd_pos = None;


    for i in (0..buf.len().saturating_sub(3)).rev() {


        if buf[i] == 0x50 && buf[i + 1] == 0x4b && buf[i + 2] == 0x05 && buf[i + 3] == 0x06 {


            eocd_pos = Some(i);


            break;


        }


    }





    let eocd_idx = match eocd_pos {


        Some(idx) => idx,


        None => return Ok(Vec::new()),


    };





    if buf.len() < eocd_idx + 20 {


        return Ok(Vec::new());


    }





    let cd_entries_count = u16::from_le_bytes([buf[eocd_idx + 10], buf[eocd_idx + 11]]) as usize;


    let cd_offset = u32::from_le_bytes([


        buf[eocd_idx + 16],


        buf[eocd_idx + 17],


        buf[eocd_idx + 18],


        buf[eocd_idx + 19],


    ]) as u64;





    file.seek(SeekFrom::Start(cd_offset))


        .map_err(|e| format!("Seek to central directory failed: {e}"))?;





    let mut entries = Vec::new();


    let to_read = cd_entries_count.min(max_entries);





    for _ in 0..to_read {


        let mut hdr = [0u8; 46];


        if file.read_exact(&mut hdr).is_err() {


            break;


        }





        // Central directory file header signature: 0x02014b50


        if hdr[0] != 0x50 || hdr[1] != 0x4b || hdr[2] != 0x01 || hdr[3] != 0x02 {


            break;


        }





        let uncompressed_size = u32::from_le_bytes([hdr[24], hdr[25], hdr[26], hdr[27]]) as u64;


        let fname_len = u16::from_le_bytes([hdr[28], hdr[29]]) as usize;


        let extra_len = u16::from_le_bytes([hdr[30], hdr[31]]) as i64;


        let comment_len = u16::from_le_bytes([hdr[32], hdr[33]]) as i64;





        let mut fname_buf = vec![0u8; fname_len];


        if file.read_exact(&mut fname_buf).is_err() {


            break;


        }


        let fname = String::from_utf8_lossy(&fname_buf).to_string();


        let is_dir = fname.ends_with('/') || fname.ends_with('\\');





        entries.push(ArchiveEntry {


            name: fname,


            is_dir,


            size_bytes: uncompressed_size,


            size_formatted: format_bytes(uncompressed_size),


        });





        // Skip extra field and comment


        if extra_len + comment_len > 0 {


            if file.seek(SeekFrom::Current(extra_len + comment_len)).is_err() {


                break;


            }


        }


    }





    Ok(entries)


}





#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_format_kind() {
        assert_eq!(detect_format_kind(Path::new("C:/test/file.pdf")), FileFormatKind::Pdf);
        assert_eq!(detect_format_kind(Path::new("C:/test/file.md")), FileFormatKind::Markdown);
        assert_eq!(detect_format_kind(Path::new("C:/test/file.txt")), FileFormatKind::Text);
        assert_eq!(detect_format_kind(Path::new("C:/test/file.log")), FileFormatKind::Text);
        assert_eq!(detect_format_kind(Path::new("C:/test/file.png")), FileFormatKind::Image);
        assert_eq!(detect_format_kind(Path::new("C:/test/file.jpg")), FileFormatKind::Image);
        assert_eq!(detect_format_kind(Path::new("C:/test/file.mp4")), FileFormatKind::Video);
        assert_eq!(detect_format_kind(Path::new("C:/test/file.mp3")), FileFormatKind::Audio);
        assert_eq!(detect_format_kind(Path::new("C:/test/main.rs")), FileFormatKind::Code);
        assert_eq!(detect_format_kind(Path::new("C:/test/script.ts")), FileFormatKind::Code);
        assert_eq!(detect_format_kind(Path::new("C:/test/data.json")), FileFormatKind::Code);
        assert_eq!(detect_format_kind(Path::new("C:/test/archive.zip")), FileFormatKind::Archive);
        assert_eq!(detect_format_kind(Path::new("C:/test/archive.tar.gz")), FileFormatKind::Archive);
        assert_eq!(detect_format_kind(Path::new("C:/test/doc.docx")), FileFormatKind::Document);
        assert_eq!(detect_format_kind(Path::new("C:/test/sheet.xlsx")), FileFormatKind::Document);
        assert_eq!(detect_format_kind(Path::new("C:/test/unknown.xyz")), FileFormatKind::Generic);
    }

    #[test]
    fn test_get_format_label() {
        assert_eq!(get_format_label(Path::new("a.pdf"), FileFormatKind::Pdf), "PDF Document");
        assert_eq!(get_format_label(Path::new("a.rs"), FileFormatKind::Code), "Rust Source File");
        assert_eq!(get_format_label(Path::new("a.zip"), FileFormatKind::Archive), "ZIP Archive");
    }

    #[test]
    fn test_preview_info_nonexistent_file() {
        let res = get_file_preview_info("C:/nonexistent_file_xyz_123.txt");
        assert!(res.is_ok());
        let info = res.unwrap();
        assert!(info.error.is_some());
        assert_eq!(info.format_kind, "text");
    }

    #[test]
    fn test_format_timestamp() {
        let ts = 1700000000;
        let s = format_timestamp(ts);
        assert!(s.starts_with("2023-11-14"));
    }

    #[test]
    fn test_preview_info_text_file() {
        let dir = std::env::temp_dir();
        let file_path = dir.join("lux_test_preview.txt");
        let sample = "Line 1\nLine 2\nLine 3\n";
        std::fs::write(&file_path, sample).unwrap();

        let info = get_file_preview_info(file_path.to_str().unwrap()).unwrap();
        assert_eq!(info.format_kind, "text");
        assert_eq!(info.line_count, Some(3));
        assert_eq!(info.text_content, Some(sample.to_string()));
        assert!(!info.is_truncated);

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_preview_info_large_text_truncation() {
        let dir = std::env::temp_dir();
        let file_path = dir.join("lux_test_large.txt");
        let large_content = "A".repeat(600 * 1024);
        std::fs::write(&file_path, &large_content).unwrap();

        let info = get_file_preview_info(file_path.to_str().unwrap()).unwrap();
        assert_eq!(info.format_kind, "text");
        assert!(info.is_truncated);
        assert_eq!(info.text_content.unwrap().len(), 512 * 1024);

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_detect_pdf_page_count() {
        let dir = std::env::temp_dir();
        let file_path = dir.join("lux_test_doc.pdf");
        let fake_pdf = b"%PDF-1.4\n1 0 obj\n<< /Type /Pages /Count 42 >>\nendobj\ntrailer\n<<>>\n%%EOF";
        std::fs::write(&file_path, fake_pdf).unwrap();

        let count = detect_pdf_page_count(&file_path);
        assert_eq!(count, Some(42));

        let _ = std::fs::remove_file(file_path);
    }

    #[test]
    fn test_read_zip_entries() {
        let dir = std::env::temp_dir();
        let file_path = dir.join("lux_test_archive.zip");

        let mut eocd = Vec::new();
        eocd.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
        eocd.extend_from_slice(&[0, 0]);
        eocd.extend_from_slice(&[0, 0]);
        eocd.extend_from_slice(&[0, 0]);
        eocd.extend_from_slice(&[0, 0]);
        eocd.extend_from_slice(&[0, 0, 0, 0]);
        eocd.extend_from_slice(&[0, 0, 0, 0]);
        eocd.extend_from_slice(&[0, 0]);

        std::fs::write(&file_path, eocd).unwrap();

        let entries = read_zip_entries(&file_path, 100).unwrap();
        assert_eq!(entries.len(), 0);

        let _ = std::fs::remove_file(file_path);
    }
}
