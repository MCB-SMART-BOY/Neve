//! Module path modeling and file resolution.
//! 模块路径建模与文件解析。

use std::path::{Path, PathBuf};

/// Represents a module path in the source code.
/// 表示源代码中的模块路径。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModulePath {
    /// Path segments (e.g., ["std", "list"] for `std.list`). / 路径段（例如 `std.list` 对应 ["std", "list"]）。
    pub segments: Vec<String>,
    /// Whether this is a relative path (starts with self or super). / 是否为相对路径（以 self 或 super 开头）。
    pub kind: ModulePathKind,
}

/// Kind of module path.
/// 模块路径的类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModulePathKind {
    /// Absolute path from crate root (e.g., `std.list`). / 从 crate 根开始的绝对路径（例如 `std.list`）。
    Absolute,
    /// Relative to current module (e.g., `self.utils`). / 相对于当前模块（例如 `self.utils`）。
    Self_,
    /// Relative to parent module (e.g., `super.common`). / 相对于父模块（例如 `super.common`）。
    Super,
    /// Relative to crate root (e.g., `crate.config`). / 相对于 crate 根（例如 `crate.config`）。
    Crate,
}

impl ModulePath {
    /// Create an absolute module path.
    /// 创建绝对模块路径。
    pub fn absolute(segments: Vec<String>) -> Self {
        Self {
            segments,
            kind: ModulePathKind::Absolute,
        }
    }

    /// Create a self-relative module path.
    /// 创建 self 相对模块路径。
    pub fn self_(segments: Vec<String>) -> Self {
        Self {
            segments,
            kind: ModulePathKind::Self_,
        }
    }

    /// Create a super-relative module path.
    /// 创建 super 相对模块路径。
    pub fn super_(segments: Vec<String>) -> Self {
        Self {
            segments,
            kind: ModulePathKind::Super,
        }
    }

    /// Create a crate-relative module path.
    /// 创建 crate 相对模块路径。
    pub fn crate_(segments: Vec<String>) -> Self {
        Self {
            segments,
            kind: ModulePathKind::Crate,
        }
    }

    /// Create a module path from an AST import definition.
    /// 从 AST 导入定义创建模块路径。
    pub fn from_import_def(import: &neve_syntax::ImportDef) -> Self {
        let segments: Vec<String> = import.path.iter().map(|i| i.name.clone()).collect();
        match import.prefix {
            neve_syntax::PathPrefix::Absolute => Self::absolute(segments),
            neve_syntax::PathPrefix::Self_ => Self::self_(segments),
            neve_syntax::PathPrefix::Super => Self::super_(segments),
            neve_syntax::PathPrefix::Crate => Self::crate_(segments),
        }
    }

    /// Create a module path from a HIR import.
    /// 从 HIR 导入创建模块路径。
    pub fn from_hir_import(import: &crate::Import) -> Self {
        match import.prefix {
            crate::ImportPathPrefix::Absolute => Self::absolute(import.path.clone()),
            crate::ImportPathPrefix::Self_ => Self::self_(import.path.clone()),
            crate::ImportPathPrefix::Super => Self::super_(import.path.clone()),
            crate::ImportPathPrefix::Crate => Self::crate_(import.path.clone()),
        }
    }

    /// Parse a module path from import path segments (legacy, infers prefix from first segment).
    /// 从导入路径段解析模块路径（遗留方式，从第一个段推断前缀）。
    pub fn from_import_path(segments: &[String]) -> Self {
        if segments.is_empty() {
            return Self::absolute(Vec::new());
        }

        match segments[0].as_str() {
            "self" => Self::self_(segments[1..].to_vec()),
            "super" => Self::super_(segments[1..].to_vec()),
            "crate" => Self::crate_(segments[1..].to_vec()),
            _ => Self::absolute(segments.to_vec()),
        }
    }
}

impl std::fmt::Display for ModulePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prefix = match self.kind {
            ModulePathKind::Absolute => "",
            ModulePathKind::Self_ => "self.",
            ModulePathKind::Super => "super.",
            ModulePathKind::Crate => "crate.",
        };
        write!(f, "{}{}", prefix, self.segments.join("."))
    }
}

/// Internal resolver for module path normalization and file lookup.
/// 模块路径归一化与文件查找的内部解析器。
#[derive(Debug, Clone)]
pub(crate) struct ModulePathResolver {
    root_dir: PathBuf,
    std_path: Option<PathBuf>,
}

impl ModulePathResolver {
    /// Create a resolver rooted at the project source root.
    /// 创建一个以项目源码根目录为基准的解析器。
    pub(crate) fn new(root_dir: impl AsRef<Path>) -> Self {
        Self {
            root_dir: root_dir.as_ref().to_path_buf(),
            std_path: None,
        }
    }

    /// Set the standard library lookup root.
    /// 设置标准库查找根路径。
    pub(crate) fn set_std_path(&mut self, path: impl AsRef<Path>) {
        self.std_path = Some(path.as_ref().to_path_buf());
    }

    /// Get the project root directory.
    /// 获取项目根目录。
    pub(crate) fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Resolve a module path into a file path on disk.
    /// 将模块路径解析为磁盘上的文件路径。
    pub(crate) fn resolve_path(
        &self,
        path: &ModulePath,
        from_module: Option<&[String]>,
    ) -> Option<PathBuf> {
        let absolute_path = self.resolve_module_path(path, from_module)?;
        self.find_module_file(&absolute_path)
    }

    /// Resolve a module path into an absolute module path.
    /// 将模块路径解析为绝对模块路径。
    pub(crate) fn resolve_module_path(
        &self,
        path: &ModulePath,
        from_module: Option<&[String]>,
    ) -> Option<Vec<String>> {
        self.make_absolute(path, from_module)
    }

    /// Find the file path for a module path.
    /// 查找模块路径对应的文件路径。
    pub(crate) fn find_module_file(&self, module_path: &[String]) -> Option<PathBuf> {
        if module_path.is_empty() {
            return Some(self.root_dir.join("lib.neve"));
        }

        if Self::is_std_path(module_path)
            && let Some(std_path) = &self.std_path
        {
            let relative: PathBuf = module_path[1..].iter().collect();
            let file_path = std_path.join(&relative).with_extension("neve");
            if file_path.exists() {
                return Some(file_path);
            }

            let mod_path = std_path.join(&relative).join("mod.neve");
            if mod_path.exists() {
                return Some(mod_path);
            }
        }

        let relative: PathBuf = module_path.iter().collect();

        let file_path = self.root_dir.join(&relative).with_extension("neve");
        if file_path.exists() {
            return Some(file_path);
        }

        let mod_path = self.root_dir.join(&relative).join("mod.neve");
        if mod_path.exists() {
            return Some(mod_path);
        }

        let src_path = self
            .root_dir
            .join("src")
            .join(&relative)
            .with_extension("neve");
        if src_path.exists() {
            return Some(src_path);
        }

        None
    }

    /// Check if a module path points into the std namespace.
    /// 检查模块路径是否指向 std 命名空间。
    pub(crate) fn is_std_path(path: &[String]) -> bool {
        path.first().map(|seg| seg == "std").unwrap_or(false)
    }

    fn make_absolute(
        &self,
        path: &ModulePath,
        from_module: Option<&[String]>,
    ) -> Option<Vec<String>> {
        match path.kind {
            ModulePathKind::Absolute => Some(path.segments.clone()),
            ModulePathKind::Crate => Some(path.segments.clone()),
            ModulePathKind::Self_ => {
                let mut result = from_module?.to_vec();
                result.extend(path.segments.iter().cloned());
                Some(result)
            }
            ModulePathKind::Super => {
                let from = from_module?;
                if from.len() < 2 {
                    return None;
                }

                let mut result = from[..from.len() - 2].to_vec();
                for seg in &path.segments {
                    if seg == "super" {
                        if result.is_empty() {
                            return None;
                        }
                        result.pop();
                    } else {
                        result.push(seg.clone());
                    }
                }
                Some(result)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_path_parsing() {
        let path = ModulePath::from_import_path(&["std".into(), "list".into()]);
        assert_eq!(path.kind, ModulePathKind::Absolute);
        assert_eq!(path.segments, vec!["std", "list"]);

        let path = ModulePath::from_import_path(&["self".into(), "utils".into()]);
        assert_eq!(path.kind, ModulePathKind::Self_);
        assert_eq!(path.segments, vec!["utils"]);

        let path = ModulePath::from_import_path(&["super".into(), "common".into()]);
        assert_eq!(path.kind, ModulePathKind::Super);
        assert_eq!(path.segments, vec!["common"]);

        let path = ModulePath::from_import_path(&["crate".into(), "config".into()]);
        assert_eq!(path.kind, ModulePathKind::Crate);
        assert_eq!(path.segments, vec!["config"]);
    }

    #[test]
    fn test_resolve_module_path() {
        let resolver = ModulePathResolver::new("/tmp");

        let path = ModulePath::absolute(vec!["std".into(), "list".into()]);
        let result = resolver.resolve_module_path(&path, Some(&["mymod".into()]));
        assert_eq!(result, Some(vec!["std".into(), "list".into()]));

        let path = ModulePath::self_(vec!["utils".into()]);
        let result = resolver.resolve_module_path(&path, Some(&["mymod".into()]));
        assert_eq!(result, Some(vec!["mymod".into(), "utils".into()]));

        let path = ModulePath::super_(vec!["common".into()]);
        let result = resolver.resolve_module_path(
            &path,
            Some(&["parent".into(), "child".into(), "file".into()]),
        );
        assert_eq!(result, Some(vec!["parent".into(), "common".into()]));
    }
}
