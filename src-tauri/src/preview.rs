use std::path::Path;
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
}
