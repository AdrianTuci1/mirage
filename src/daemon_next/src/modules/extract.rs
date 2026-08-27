use crate::modules::manifest::{ArchiveFormat, FileEntry};
use anyhow::{Context, Result};
use std::fs;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

/// Extract an archive to `dest_dir`. Returns the list of extracted file paths.
pub fn extract_archive(
    archive_path: impl AsRef<Path>,
    dest_dir: impl AsRef<Path>,
    format: &ArchiveFormat,
) -> Result<Vec<PathBuf>> {
    let archive_path = archive_path.as_ref();
    let dest_dir = dest_dir.as_ref();
    fs::create_dir_all(dest_dir)
        .with_context(|| format!("failed to create extraction directory {}", dest_dir.display()))?;

    match format {
        ArchiveFormat::TarGz => extract_tar_gz(archive_path, dest_dir),
        ArchiveFormat::Zip => extract_zip(archive_path, dest_dir),
        ArchiveFormat::Raw => extract_raw(archive_path, dest_dir),
    }
}

fn extract_tar_gz(archive_path: &Path, dest_dir: &Path) -> Result<Vec<PathBuf>> {
    let file = fs::File::open(archive_path)
        .with_context(|| format!("failed to open tar.gz {}", archive_path.display()))?;
    let reader = BufReader::new(file);
    let gz = flate2::read::GzDecoder::new(reader);
    let mut archive = tar::Archive::new(gz);

    let mut extracted = Vec::new();
    for entry in archive.entries()? {
        let mut entry = entry?;
        let entry_path = entry.path()?;
        // Reject absolute paths and parent traversal for safety.
        let target = sanitize_path(dest_dir, &entry_path)?;
        if entry.header().entry_type().is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            entry.unpack(&target)?;
            extracted.push(target);
        }
    }
    Ok(extracted)
}

fn extract_zip(archive_path: &Path, dest_dir: &Path) -> Result<Vec<PathBuf>> {
    let file = fs::File::open(archive_path)
        .with_context(|| format!("failed to open zip {}", archive_path.display()))?;
    let mut archive = zip::ZipArchive::new(BufReader::new(file))
        .with_context(|| format!("failed to read zip {}", archive_path.display()))?;

    let mut extracted = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.is_file() {
            let target = sanitize_path(dest_dir, Path::new(file.name()))?;
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out = fs::File::create(&target)?;
            std::io::copy(&mut file, &mut out)?;
            extracted.push(target);
        }
    }
    Ok(extracted)
}

fn extract_raw(archive_path: &Path, dest_dir: &Path) -> Result<Vec<PathBuf>> {
    let name = archive_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("raw archive has no file name"))?;
    let target = dest_dir.join(name);
    fs::copy(archive_path, &target)?;
    Ok(vec![target])
}

/// Verify extracted files against the manifest and fix executable bits.
pub fn verify_extracted_files(
    dest_dir: &Path,
    files: &[FileEntry],
) -> Result<Vec<PathBuf>> {
    let mut verified = Vec::new();
    for entry in files {
        let target = dest_dir.join(&entry.relative_path);
        if !target.exists() && entry.required {
            return Err(anyhow::anyhow!(
                "required file {} is missing after extraction",
                entry.relative_path
            ));
        }
        if target.exists() {
            let actual_hash = hash_file_sha256(&target)?;
            if actual_hash != entry.sha256 {
                return Err(anyhow::anyhow!(
                    "checksum mismatch for {}: expected {}, got {}",
                    entry.relative_path,
                    entry.sha256,
                    actual_hash
                ));
            }
            if entry.executable {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&target)?.permissions();
                    perms.set_mode(perms.mode() | 0o111);
                    fs::set_permissions(&target, perms)?;
                }
            }
            verified.push(target);
        }
    }
    Ok(verified)
}

fn hash_file_sha256(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut file = fs::File::open(path)
        .with_context(|| format!("failed to open file for hashing {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

/// Prevent path traversal attacks by resolving the entry path inside dest_dir.
fn sanitize_path(dest_dir: &Path, entry_path: &Path) -> Result<PathBuf> {
    let target = dest_dir.join(entry_path);
    let canonical_dest = dest_dir.canonicalize().unwrap_or_else(|_| dest_dir.to_path_buf());
    let canonical_target = target.canonicalize().unwrap_or_else(|_| target.clone());
    if !canonical_target.starts_with(&canonical_dest) {
        return Err(anyhow::anyhow!(
            "tar/zip entry escapes extraction directory: {}",
            entry_path.display()
        ));
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::manifest::ArchiveFormat;
    use tempfile::TempDir;

    #[test]
    fn extract_raw_copies_file() {
        let src_dir = TempDir::new().unwrap();
        let dest_dir = TempDir::new().unwrap();
        let src = src_dir.path().join("model.onnx");
        fs::write(&src, b"fake model").unwrap();

        let extracted = extract_archive(&src, dest_dir.path(), &ArchiveFormat::Raw).unwrap();
        assert_eq!(extracted.len(), 1);
        assert!(extracted[0].exists());
        assert_eq!(fs::read_to_string(&extracted[0]).unwrap(), "fake model");
    }

    #[test]
    fn verify_extracted_files_checks_checksum() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("libduckdb.dylib");
        fs::write(&file_path, b"lib").unwrap();
        use sha2::{Digest, Sha256};
        let hash = hex::encode(Sha256::digest(b"lib"));

        let entries = vec![FileEntry {
            relative_path: String::from("libduckdb.dylib"),
            sha256: hash,
            executable: false,
            required: true,
        }];

        assert!(verify_extracted_files(dir.path(), &entries).is_ok());

        let bad_entries = vec![FileEntry {
            relative_path: String::from("libduckdb.dylib"),
            sha256: String::from("0000000000000000000000000000000000000000000000000000000000000000"),
            executable: false,
            required: true,
        }];
        assert!(verify_extracted_files(dir.path(), &bad_entries).is_err());
    }
}
