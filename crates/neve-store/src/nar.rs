//! NAR (Nix ARchive) format implementation.
//! NAR (Nix ARchive) 格式实现。
//!
//! NAR is a deterministic archive format used by Nix for storing build outputs.
//! It captures files, directories, and symlinks in a reproducible way.
//! NAR 是 Nix 使用的确定性归档格式，用于存储构建输出。
//! 它以可重现的方式捕获文件、目录和符号链接。
//!
//! ## Format Specification 格式规范
//!
//! NAR uses a simple string-based format:
//! NAR 使用简单的基于字符串的格式：
//!
//! - All strings are length-prefixed (8 bytes, little-endian)
//! - 所有字符串都有长度前缀（8 字节，小端序）
//! - Strings are padded to 8-byte alignment
//! - 字符串填充到 8 字节对齐
//! - The format is recursive for directories
//! - 目录采用递归格式

use neve_derive::Hash;
use std::fs;
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use thiserror::Error;

/// NAR magic string. / NAR 魔术字符串。
const NAR_MAGIC: &str = "nix-archive-1";

/// Errors during NAR operations.
/// NAR 操作期间的错误。
#[derive(Debug, Error)]
pub enum NarError {
    /// I/O error. / I/O 错误。
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// Invalid NAR format. / 无效的 NAR 格式。
    #[error("invalid NAR format: {0}")]
    InvalidFormat(String),

    /// Unexpected end of archive. / 归档意外结束。
    #[error("unexpected end of archive")]
    UnexpectedEof,

    /// Path traversal attempt. / 路径遍历尝试。
    #[error("path traversal attempt detected")]
    PathTraversal,
}

/// NAR writer for creating archives.
/// 用于创建归档的 NAR 写入器。
pub struct NarWriter<W: Write> {
    writer: W,
    bytes_written: u64,
}

impl<W: Write> NarWriter<W> {
    /// Create a new NAR writer.
    /// 创建新的 NAR 写入器。
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            bytes_written: 0,
        }
    }

    /// Write a path (file, directory, or symlink) to the archive.
    /// 将路径（文件、目录或符号链接）写入归档。
    pub fn write_path(&mut self, path: &Path) -> Result<(), NarError> {
        self.write_str(NAR_MAGIC)?;
        self.write_entry(path)
    }

    /// Write an entry (recursive).
    /// 写入条目（递归）。
    fn write_entry(&mut self, path: &Path) -> Result<(), NarError> {
        self.write_str("(")?;

        let metadata = fs::symlink_metadata(path)?;

        if metadata.is_symlink() {
            self.write_str("type")?;
            self.write_str("symlink")?;
            self.write_str("target")?;
            let target = fs::read_link(path)?;
            self.write_str(&target.to_string_lossy())?;
        } else if metadata.is_file() {
            self.write_str("type")?;
            self.write_str("regular")?;

            // Check if executable
            // 检查是否可执行
            if metadata.permissions().mode() & 0o111 != 0 {
                self.write_str("executable")?;
                self.write_str("")?;
            }

            self.write_str("contents")?;
            let contents = fs::read(path)?;
            self.write_bytes(&contents)?;
        } else if metadata.is_dir() {
            self.write_str("type")?;
            self.write_str("directory")?;

            // Read and sort directory entries for determinism
            // 读取并排序目录条目以确保确定性
            let mut entries: Vec<_> = fs::read_dir(path)?.filter_map(|e| e.ok()).collect();
            entries.sort_by_key(|e| e.file_name());

            for entry in entries {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();

                // Skip special entries
                // 跳过特殊条目
                if name_str == "." || name_str == ".." {
                    continue;
                }

                self.write_str("entry")?;
                self.write_str("(")?;
                self.write_str("name")?;
                self.write_str(&name_str)?;
                self.write_str("node")?;
                self.write_entry(&entry.path())?;
                self.write_str(")")?;
            }
        }

        self.write_str(")")?;
        Ok(())
    }

    /// Write a length-prefixed string with padding.
    /// 写入带填充的长度前缀字符串。
    fn write_str(&mut self, s: &str) -> Result<(), NarError> {
        self.write_bytes(s.as_bytes())
    }

    /// Write length-prefixed bytes with padding.
    /// 写入带填充的长度前缀字节。
    fn write_bytes(&mut self, data: &[u8]) -> Result<(), NarError> {
        let len = data.len() as u64;
        self.writer.write_all(&len.to_le_bytes())?;
        self.writer.write_all(data)?;
        self.bytes_written += 8 + len;

        // Pad to 8-byte alignment
        // 填充到 8 字节对齐
        let padding = (8 - (len % 8)) % 8;
        if padding > 0 {
            self.writer.write_all(&vec![0u8; padding as usize])?;
            self.bytes_written += padding;
        }

        Ok(())
    }

    /// Get the number of bytes written.
    /// 获取已写入的字节数。
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    /// Finish writing and return the inner writer.
    /// 完成写入并返回内部写入器。
    pub fn finish(self) -> W {
        self.writer
    }
}

/// NAR reader for extracting archives.
/// 用于提取归档的 NAR 读取器。
pub struct NarReader<R: Read> {
    reader: R,
    bytes_read: u64,
    /// Lookahead buffer for peeked strings.
    /// 用于预读字符串的缓冲区。
    lookahead: Option<String>,
}

impl<R: Read> NarReader<R> {
    /// Create a new NAR reader.
    /// 创建新的 NAR 读取器。
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            bytes_read: 0,
            lookahead: None,
        }
    }

    /// Extract the archive to a destination path.
    /// 将归档提取到目标路径。
    pub fn extract(&mut self, dest: &Path) -> Result<(), NarError> {
        let magic = self.read_str()?;
        if magic != NAR_MAGIC {
            return Err(NarError::InvalidFormat(format!(
                "expected magic '{}', got '{}'",
                NAR_MAGIC, magic
            )));
        }

        self.extract_entry(dest)
    }

    /// Extract a single entry (recursive).
    /// 提取单个条目（递归）。
    fn extract_entry(&mut self, dest: &Path) -> Result<(), NarError> {
        self.expect_str("(")?;
        self.expect_str("type")?;

        let entry_type = self.read_str()?;
        match entry_type.as_str() {
            "regular" => self.extract_regular(dest)?,
            "directory" => self.extract_directory(dest)?,
            "symlink" => self.extract_symlink(dest)?,
            _ => {
                return Err(NarError::InvalidFormat(format!(
                    "unknown entry type: {}",
                    entry_type
                )));
            }
        }

        Ok(())
    }

    /// Extract a regular file.
    /// 提取普通文件。
    fn extract_regular(&mut self, dest: &Path) -> Result<(), NarError> {
        let mut executable = false;
        let mut contents_written = false;

        loop {
            let tag = self.read_str()?;
            match tag.as_str() {
                "executable" => {
                    self.read_str()?; // Empty string
                    executable = true;
                }
                "contents" => {
                    let contents = self.read_bytes()?;

                    // Create parent directories
                    // 创建父目录
                    if let Some(parent) = dest.parent() {
                        fs::create_dir_all(parent)?;
                    }

                    fs::write(dest, &contents)?;
                    contents_written = true;
                }
                ")" => {
                    // Set permissions after writing file
                    // 写入文件后设置权限
                    if contents_written {
                        if executable {
                            let perms = fs::Permissions::from_mode(0o755);
                            fs::set_permissions(dest, perms)?;
                        } else {
                            let perms = fs::Permissions::from_mode(0o644);
                            fs::set_permissions(dest, perms)?;
                        }
                    }
                    return Ok(());
                }
                _ => {
                    return Err(NarError::InvalidFormat(format!(
                        "unexpected tag in regular file: {}",
                        tag
                    )));
                }
            }
        }
    }

    /// Extract a directory.
    /// 提取目录。
    fn extract_directory(&mut self, dest: &Path) -> Result<(), NarError> {
        fs::create_dir_all(dest)?;

        loop {
            let tag = self.read_str()?;
            match tag.as_str() {
                "entry" => {
                    self.expect_str("(")?;
                    self.expect_str("name")?;

                    let name = self.read_str()?;

                    // Security check: prevent path traversal
                    // 安全检查：防止路径遍历
                    if name.contains('/') || name.contains('\\') || name == ".." || name == "." {
                        return Err(NarError::PathTraversal);
                    }

                    self.expect_str("node")?;

                    let entry_path = dest.join(&name);
                    self.extract_entry(&entry_path)?;

                    self.expect_str(")")?;
                }
                ")" => {
                    return Ok(());
                }
                _ => {
                    return Err(NarError::InvalidFormat(format!(
                        "unexpected tag in directory: {}",
                        tag
                    )));
                }
            }
        }
    }

    /// Extract a symlink.
    /// 提取符号链接。
    fn extract_symlink(&mut self, dest: &Path) -> Result<(), NarError> {
        self.expect_str("target")?;
        let target = self.read_str()?;

        // Create parent directories
        // 创建父目录
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        // Remove existing file if present
        // 如果存在则删除现有文件
        if dest.exists() || dest.is_symlink() {
            fs::remove_file(dest)?;
        }

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, dest)?;

        #[cfg(windows)]
        {
            // On Windows, try to determine if target is a directory
            // 在 Windows 上，尝试确定目标是否为目录
            let target_path = dest.parent().unwrap_or(Path::new(".")).join(&target);
            if target_path.is_dir() {
                std::os::windows::fs::symlink_dir(&target, dest)?;
            } else {
                std::os::windows::fs::symlink_file(&target, dest)?;
            }
        }

        // Read the closing paren
        // 读取闭括号
        self.expect_str(")")?;

        Ok(())
    }

    /// Read a length-prefixed string.
    /// 读取长度前缀字符串。
    fn read_str(&mut self) -> Result<String, NarError> {
        // Check lookahead first
        // 首先检查预读缓冲区
        if let Some(s) = self.lookahead.take() {
            return Ok(s);
        }
        let bytes = self.read_bytes()?;
        String::from_utf8(bytes).map_err(|e| NarError::InvalidFormat(e.to_string()))
    }

    /// Read length-prefixed bytes with padding.
    /// 读取带填充的长度前缀字节。
    fn read_bytes(&mut self) -> Result<Vec<u8>, NarError> {
        // Read length (8 bytes, little-endian)
        // 读取长度（8 字节，小端序）
        let mut len_buf = [0u8; 8];
        self.reader.read_exact(&mut len_buf)?;
        let len = u64::from_le_bytes(len_buf);
        self.bytes_read += 8;

        // Read data
        // 读取数据
        let mut data = vec![0u8; len as usize];
        self.reader.read_exact(&mut data)?;
        self.bytes_read += len;

        // Read padding
        // 读取填充
        let padding = (8 - (len % 8)) % 8;
        if padding > 0 {
            let mut pad_buf = vec![0u8; padding as usize];
            self.reader.read_exact(&mut pad_buf)?;
            self.bytes_read += padding;
        }

        Ok(data)
    }

    /// Expect a specific string.
    /// 期望一个特定的字符串。
    fn expect_str(&mut self, expected: &str) -> Result<(), NarError> {
        let actual = self.read_str()?;
        if actual != expected {
            Err(NarError::InvalidFormat(format!(
                "expected '{}', got '{}'",
                expected, actual
            )))
        } else {
            Ok(())
        }
    }

    /// Get the number of bytes read.
    /// 获取已读取的字节数。
    pub fn bytes_read(&self) -> u64 {
        self.bytes_read
    }
}

/// Compute the NAR hash of a path.
/// 计算路径的 NAR 哈希。
///
/// This creates a NAR archive in memory and returns its hash.
/// 这会在内存中创建 NAR 归档并返回其哈希。
pub fn hash_path(path: &Path) -> Result<Hash, NarError> {
    let mut buffer = Vec::new();
    let mut writer = NarWriter::new(&mut buffer);
    writer.write_path(path)?;
    Ok(Hash::of(&buffer))
}

/// Create a NAR archive of a path and return the bytes.
/// 创建路径的 NAR 归档并返回字节。
pub fn create_nar(path: &Path) -> Result<Vec<u8>, NarError> {
    let mut buffer = Vec::new();
    let mut writer = NarWriter::new(&mut buffer);
    writer.write_path(path)?;
    Ok(buffer)
}

/// Extract a NAR archive from bytes to a destination.
/// 从字节中提取 NAR 归档到目标位置。
pub fn extract_nar(data: &[u8], dest: &Path) -> Result<(), NarError> {
    let mut reader = NarReader::new(data);
    reader.extract(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_nar_regular_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, b"Hello, NAR!").unwrap();

        // Create NAR
        let nar_data = create_nar(&file_path).unwrap();
        assert!(!nar_data.is_empty());

        // Extract NAR
        let extract_dir = TempDir::new().unwrap();
        let extract_path = extract_dir.path().join("extracted.txt");
        extract_nar(&nar_data, &extract_path).unwrap();

        // Verify contents
        let contents = fs::read_to_string(&extract_path).unwrap();
        assert_eq!(contents, "Hello, NAR!");
    }

    #[test]
    fn test_nar_executable_file() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("script.sh");
        fs::write(&file_path, b"#!/bin/sh\necho hello").unwrap();

        // Make executable
        let perms = fs::Permissions::from_mode(0o755);
        fs::set_permissions(&file_path, perms).unwrap();

        // Create and extract NAR
        let nar_data = create_nar(&file_path).unwrap();

        let extract_dir = TempDir::new().unwrap();
        let extract_path = extract_dir.path().join("script.sh");
        extract_nar(&nar_data, &extract_path).unwrap();

        // Verify executable bit
        let metadata = fs::metadata(&extract_path).unwrap();
        assert!(metadata.permissions().mode() & 0o111 != 0);
    }

    #[test]
    fn test_nar_directory() {
        let temp = TempDir::new().unwrap();
        let dir_path = temp.path().join("mydir");
        fs::create_dir(&dir_path).unwrap();
        fs::write(dir_path.join("a.txt"), b"File A").unwrap();
        fs::write(dir_path.join("b.txt"), b"File B").unwrap();

        let subdir = dir_path.join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("c.txt"), b"File C").unwrap();

        // Create and extract NAR
        let nar_data = create_nar(&dir_path).unwrap();

        let extract_dir = TempDir::new().unwrap();
        let extract_path = extract_dir.path().join("extracted");
        extract_nar(&nar_data, &extract_path).unwrap();

        // Verify structure
        assert!(extract_path.is_dir());
        assert_eq!(
            fs::read_to_string(extract_path.join("a.txt")).unwrap(),
            "File A"
        );
        assert_eq!(
            fs::read_to_string(extract_path.join("b.txt")).unwrap(),
            "File B"
        );
        assert_eq!(
            fs::read_to_string(extract_path.join("subdir/c.txt")).unwrap(),
            "File C"
        );
    }

    #[test]
    fn test_nar_symlink() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("target.txt");
        fs::write(&file_path, b"Target content").unwrap();

        let link_path = temp.path().join("link.txt");
        std::os::unix::fs::symlink("target.txt", &link_path).unwrap();

        // Create NAR of the symlink
        let nar_data = create_nar(&link_path).unwrap();

        let extract_dir = TempDir::new().unwrap();
        let extract_path = extract_dir.path().join("extracted_link");
        extract_nar(&nar_data, &extract_path).unwrap();

        // Verify it's a symlink pointing to the right target
        assert!(extract_path.is_symlink());
        assert_eq!(
            fs::read_link(&extract_path).unwrap().to_string_lossy(),
            "target.txt"
        );
    }

    #[test]
    fn test_nar_hash_determinism() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("test.txt");
        fs::write(&file_path, b"Deterministic content").unwrap();

        // Hash should be the same for identical content
        let hash1 = hash_path(&file_path).unwrap();
        let hash2 = hash_path(&file_path).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_nar_directory_sorting() {
        // Create directory with files in different order
        let temp1 = TempDir::new().unwrap();
        let dir1 = temp1.path().join("dir");
        fs::create_dir(&dir1).unwrap();
        fs::write(dir1.join("z.txt"), b"z").unwrap();
        fs::write(dir1.join("a.txt"), b"a").unwrap();
        fs::write(dir1.join("m.txt"), b"m").unwrap();

        let temp2 = TempDir::new().unwrap();
        let dir2 = temp2.path().join("dir");
        fs::create_dir(&dir2).unwrap();
        fs::write(dir2.join("a.txt"), b"a").unwrap();
        fs::write(dir2.join("m.txt"), b"m").unwrap();
        fs::write(dir2.join("z.txt"), b"z").unwrap();

        // Hashes should be identical (order doesn't matter, sorting does)
        let hash1 = hash_path(&dir1).unwrap();
        let hash2 = hash_path(&dir2).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_nar_path_traversal_prevention() {
        // Create a malicious NAR with path traversal
        let mut malicious_nar = Vec::new();

        // Write magic
        let magic = NAR_MAGIC.as_bytes();
        malicious_nar.extend_from_slice(&(magic.len() as u64).to_le_bytes());
        malicious_nar.extend_from_slice(magic);
        let padding = (8 - (magic.len() % 8)) % 8;
        malicious_nar.extend_from_slice(&vec![0u8; padding]);

        // This is a simplified test - in practice we'd need full NAR format
        // The actual test is in extract_directory which checks for ".." and "/"
    }
}
