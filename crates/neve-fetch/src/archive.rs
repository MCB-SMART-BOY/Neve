//! Archive extraction utilities.
//! 归档文件解压工具。
//!
//! Provides functionality for extracting various archive formats.
//! 提供解压各种归档格式的功能。

use crate::FetchError;
use flate2::read::GzDecoder;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;
use tar::Archive;

/// Supported archive formats.
/// 支持的归档格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    /// Gzip-compressed tar archive (.tar.gz, .tgz).
    /// Gzip 压缩的 tar 归档（.tar.gz, .tgz）。
    TarGz,
    /// Xz-compressed tar archive (.tar.xz, .txz).
    /// Xz 压缩的 tar 归档（.tar.xz, .txz）。
    TarXz,
    /// Plain tar archive (.tar).
    /// 普通 tar 归档（.tar）。
    Tar,
}

impl ArchiveFormat {
    /// Detect archive format from file extension.
    /// 从文件扩展名检测归档格式。
    pub fn from_path(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_str()?;
        Self::from_name(name)
    }

    /// Detect archive format from file name.
    /// 从文件名检测归档格式。
    pub fn from_name(name: &str) -> Option<Self> {
        let name = name.to_lowercase();

        if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
            Some(ArchiveFormat::TarGz)
        } else if name.ends_with(".tar.xz") || name.ends_with(".txz") {
            Some(ArchiveFormat::TarXz)
        } else if name.ends_with(".tar") {
            Some(ArchiveFormat::Tar)
        } else {
            None
        }
    }
}

/// Extract an archive to a directory.
/// 将归档解压到目录。
pub fn extract(archive_path: &Path, dest_dir: &Path) -> Result<(), FetchError> {
    let format = ArchiveFormat::from_path(archive_path).ok_or_else(|| {
        FetchError::Archive(format!(
            "unknown archive format: {}",
            archive_path.display()
        ))
    })?;

    extract_with_format(archive_path, dest_dir, format)
}

/// Extract an archive with a specific format.
/// 使用指定格式解压归档。
pub fn extract_with_format(
    archive_path: &Path,
    dest_dir: &Path,
    format: ArchiveFormat,
) -> Result<(), FetchError> {
    // Create destination directory
    // 创建目标目录
    fs::create_dir_all(dest_dir)?;

    let file = File::open(archive_path)?;

    match format {
        ArchiveFormat::TarGz => extract_tar_gz(file, dest_dir),
        ArchiveFormat::TarXz => extract_tar_xz(file, dest_dir),
        ArchiveFormat::Tar => extract_tar(file, dest_dir),
    }
}

/// Extract a .tar.gz archive.
/// 解压 .tar.gz 归档。
fn extract_tar_gz(file: File, dest_dir: &Path) -> Result<(), FetchError> {
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    extract_tar_archive(&mut archive, dest_dir)
}

/// Extract a .tar.xz archive.
/// 解压 .tar.xz 归档。
fn extract_tar_xz(file: File, dest_dir: &Path) -> Result<(), FetchError> {
    let decoder = xz2::read::XzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    extract_tar_archive(&mut archive, dest_dir)
}

/// Extract a plain .tar archive.
/// 解压普通 .tar 归档。
fn extract_tar(file: File, dest_dir: &Path) -> Result<(), FetchError> {
    let mut archive = Archive::new(file);
    extract_tar_archive(&mut archive, dest_dir)
}

/// Extract a tar archive to a directory.
/// 将 tar 归档解压到目录。
fn extract_tar_archive<R: Read>(
    archive: &mut Archive<R>,
    dest_dir: &Path,
) -> Result<(), FetchError> {
    archive
        .unpack(dest_dir)
        .map_err(|e| FetchError::Archive(format!("failed to extract tar archive: {}", e)))
}

/// Extract archive contents from memory.
/// 从内存中解压归档内容。
pub fn extract_from_bytes(
    data: &[u8],
    dest_dir: &Path,
    format: ArchiveFormat,
) -> Result<(), FetchError> {
    fs::create_dir_all(dest_dir)?;

    match format {
        ArchiveFormat::TarGz => {
            let decoder = GzDecoder::new(io::Cursor::new(data));
            let mut archive = Archive::new(decoder);
            extract_tar_archive(&mut archive, dest_dir)
        }
        ArchiveFormat::TarXz => {
            let decoder = xz2::read::XzDecoder::new(io::Cursor::new(data));
            let mut archive = Archive::new(decoder);
            extract_tar_archive(&mut archive, dest_dir)
        }
        ArchiveFormat::Tar => {
            let mut archive = Archive::new(io::Cursor::new(data));
            extract_tar_archive(&mut archive, dest_dir)
        }
    }
}

/// Strip the first N path components from extracted files.
/// 从解压的文件中去除前 N 个路径组件。
///
/// Useful for archives that have a single top-level directory.
/// 适用于只有单个顶级目录的归档。
pub fn extract_stripped(
    archive_path: &Path,
    dest_dir: &Path,
    strip_components: usize,
) -> Result<(), FetchError> {
    let format = ArchiveFormat::from_path(archive_path).ok_or_else(|| {
        FetchError::Archive(format!(
            "unknown archive format: {}",
            archive_path.display()
        ))
    })?;

    fs::create_dir_all(dest_dir)?;
    let file = File::open(archive_path)?;

    match format {
        ArchiveFormat::TarGz => {
            let decoder = GzDecoder::new(file);
            extract_tar_stripped(decoder, dest_dir, strip_components)
        }
        ArchiveFormat::TarXz => {
            let decoder = xz2::read::XzDecoder::new(file);
            extract_tar_stripped(decoder, dest_dir, strip_components)
        }
        ArchiveFormat::Tar => extract_tar_stripped(file, dest_dir, strip_components),
    }
}

/// Extract a tar archive with path stripping.
/// 带路径剥离的 tar 归档解压。
fn extract_tar_stripped<R: Read>(
    reader: R,
    dest_dir: &Path,
    strip: usize,
) -> Result<(), FetchError> {
    let mut archive = Archive::new(reader);

    for entry in archive
        .entries()
        .map_err(|e| FetchError::Archive(e.to_string()))?
    {
        let mut entry = entry.map_err(|e| FetchError::Archive(e.to_string()))?;
        let path = entry
            .path()
            .map_err(|e| FetchError::Archive(e.to_string()))?;

        // Skip entries with fewer components than we want to strip
        // 跳过组件数少于要剥离数量的条目
        let components: Vec<_> = path.components().collect();
        if components.len() <= strip {
            continue;
        }

        // Build new path with stripped components
        // 用剥离后的组件构建新路径
        let new_path: std::path::PathBuf = components[strip..].iter().collect();
        let dest_path = dest_dir.join(&new_path);

        // Create parent directories
        // 创建父目录
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Extract the entry
        // 解压条目
        if entry.header().entry_type().is_dir() {
            fs::create_dir_all(&dest_path)?;
        } else if entry.header().entry_type().is_file() {
            let mut file = File::create(&dest_path)?;
            io::copy(&mut entry, &mut file)?;

            // Preserve permissions on Unix
            // 在 Unix 上保留权限
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(mode) = entry.header().mode() {
                    fs::set_permissions(&dest_path, fs::Permissions::from_mode(mode))?;
                }
            }
        } else if entry.header().entry_type().is_symlink() {
            #[cfg(unix)]
            if let Ok(link_name) = entry.link_name()
                && let Some(target) = link_name
            {
                let _ = std::os::unix::fs::symlink(target, &dest_path);
            }
        }
    }

    Ok(())
}
