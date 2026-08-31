//! Content sniffing and text extraction for the semantic index.
//!
//! The semantic index stores one vector per file. What gets embedded depends on
//! the file type: images go through the vision encoder, text-like files through
//! the text encoder using an excerpt of their contents, and everything else is
//! embedded from its name and path only.

use std::path::Path;

/// Maximum number of bytes read from a text-like file.
pub const MAX_TEXT_BYTES: u64 = 256 * 1024;
/// Maximum number of characters kept from a text-like file.
pub const MAX_TEXT_CHARS: usize = 2_000;
/// Images larger than this are indexed by name only, never decoded.
pub const MAX_IMAGE_BYTES: u64 = 32 * 1024 * 1024;

/// How a file can be turned into an embedding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    /// Decodable raster image: embedded through the vision encoder.
    Image,
    /// Readable text: embedded through the text encoder using an excerpt.
    Text,
    /// Anything else: embedded from name and path only.
    Metadata,
}

impl MediaKind {
    /// Value stored in the `modality` column of the semantic index.
    pub fn as_str(&self) -> &'static str {
        match self {
            MediaKind::Image => "image",
            MediaKind::Text => "text",
            MediaKind::Metadata => "name",
        }
    }
}

pub fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default()
}

pub fn is_image_extension(ext: &str) -> bool {
    matches!(
        ext,
        "png" | "jpg" | "jpeg" | "jfif" | "webp" | "gif" | "bmp" | "dib" | "ico"
    )
}

pub fn is_text_extension(ext: &str) -> bool {
    matches!(
        ext,
        "txt"
            | "md"
            | "markdown"
            | "rst"
            | "adoc"
            | "log"
            | "csv"
            | "tsv"
            | "json"
            | "jsonc"
            | "json5"
            | "yaml"
            | "yml"
            | "toml"
            | "ini"
            | "env"
            | "xml"
            | "html"
            | "htm"
            | "css"
            | "scss"
            | "sql"
            | "rs"
            | "go"
            | "py"
            | "rb"
            | "php"
            | "swift"
            | "kt"
            | "kts"
            | "java"
            | "scala"
            | "dart"
            | "c"
            | "h"
            | "cc"
            | "cpp"
            | "hpp"
            | "cs"
            | "m"
            | "mm"
            | "js"
            | "mjs"
            | "cjs"
            | "jsx"
            | "ts"
            | "tsx"
            | "vue"
            | "svelte"
            | "sh"
            | "bash"
            | "zsh"
            | "fish"
            | "ps1"
            | "lua"
            | "r"
            | "jl"
            | "gradle"
            | "cmake"
            | "make"
            | "mk"
            | "diff"
            | "patch"
            | "conf"
            | "cfg"
    )
}

/// Classify a file by extension. Size limits are applied by the caller.
pub fn media_kind(path: &Path) -> MediaKind {
    let ext = extension(path);
    if is_image_extension(&ext) {
        MediaKind::Image
    } else if is_text_extension(&ext) {
        MediaKind::Text
    } else {
        MediaKind::Metadata
    }
}

/// Read a bounded excerpt of a text-like file.
///
/// Returns `None` when the file is too large, unreadable, or does not look like
/// text (a NUL byte in the first chunk means binary).
pub fn text_excerpt(path: &Path) -> Option<String> {
    let bytes = read_prefix(path, MAX_TEXT_BYTES as usize)?;
    if bytes.iter().take(8_192).any(|b| *b == 0) {
        return None;
    }
    let text = String::from_utf8_lossy(&bytes);
    Some(truncate_chars(text.as_ref(), MAX_TEXT_CHARS))
}

fn read_prefix(path: &Path, limit: usize) -> Option<Vec<u8>> {
    use std::io::Read;
    let file = std::fs::File::open(path).ok()?;
    let mut buf = Vec::with_capacity(limit.min(64 * 1024));
    file.take(limit as u64).read_to_end(&mut buf).ok()?;
    Some(buf)
}

pub fn truncate_chars(text: &str, max_chars: usize) -> String {
    match text.char_indices().nth(max_chars) {
        Some((idx, _)) => text[..idx].to_string(),
        None => text.to_string(),
    }
}

/// Name and parent directory of a file, used as the strongest textual signal.
pub fn name_and_parent(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let parent = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{name} {parent}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn classifies_by_extension() {
        assert_eq!(media_kind(Path::new("/a/photo.JPG")), MediaKind::Image);
        assert_eq!(media_kind(Path::new("/a/notes.md")), MediaKind::Text);
        assert_eq!(media_kind(Path::new("/a/archive.zip")), MediaKind::Metadata);
    }

    #[test]
    fn extracts_text_excerpt() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("notes.txt");
        std::fs::write(&file, b"quarterly revenue grew by twelve percent").unwrap();
        let excerpt = text_excerpt(&file).unwrap();
        assert!(excerpt.contains("quarterly revenue"));
    }

    #[test]
    fn rejects_binary_payloads() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("blob.txt");
        let mut f = std::fs::File::create(&file).unwrap();
        f.write_all(b"abc\0def").unwrap();
        assert!(text_excerpt(&file).is_none());
    }

    #[test]
    fn excerpt_is_bounded() {
        let long = "é".repeat(5_000);
        let out = truncate_chars(&long, MAX_TEXT_CHARS);
        assert_eq!(out.chars().count(), MAX_TEXT_CHARS);
    }

    #[test]
    fn name_and_parent_drops_directories() {
        assert_eq!(
            name_and_parent(Path::new("/x/reports/q3.pdf")),
            "q3.pdf reports"
        );
    }
}
