//! Module loading and path resolution.
//!
//! This module provides functionality for:
//! - Discovering modules from file system
//! - Resolving module paths (self, super, crate)
//! - Loading and caching modules
//! - Managing module dependencies

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use neve_diagnostic::Diagnostic;

use crate::{DefId, Import, ImportKind, ModuleId};

/// Represents a module path in the source code.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModulePath {
    /// Path segments (e.g., ["std", "list"] for `std.list`)
    pub segments: Vec<String>,
    /// Whether this is a relative path (starts with self or super)
    pub kind: ModulePathKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModulePathKind {
    /// Absolute path from crate root (e.g., `std.list`)
    Absolute,
    /// Relative to current module (e.g., `self.utils`)
    Self_,
    /// Relative to parent module (e.g., `super.common`)
    Super,
    /// Relative to crate root (e.g., `crate.config`)
    Crate,
}

impl ModulePath {
    /// Create an absolute module path.
    pub fn absolute(segments: Vec<String>) -> Self {
        Self {
            segments,
            kind: ModulePathKind::Absolute,
        }
    }

    /// Create a self-relative module path.
    pub fn self_(segments: Vec<String>) -> Self {
        Self {
            segments,
            kind: ModulePathKind::Self_,
        }
    }

    /// Create a super-relative module path.
    pub fn super_(segments: Vec<String>) -> Self {
        Self {
            segments,
            kind: ModulePathKind::Super,
        }
    }

    /// Create a crate-relative module path.
    pub fn crate_(segments: Vec<String>) -> Self {
        Self {
            segments,
            kind: ModulePathKind::Crate,
        }
    }

    /// Create a module path from an AST import definition.
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
    pub fn from_hir_import(import: &crate::Import) -> Self {
        match import.prefix {
            crate::ImportPathPrefix::Absolute => Self::absolute(import.path.clone()),
            crate::ImportPathPrefix::Self_ => Self::self_(import.path.clone()),
            crate::ImportPathPrefix::Super => Self::super_(import.path.clone()),
            crate::ImportPathPrefix::Crate => Self::crate_(import.path.clone()),
        }
    }

    /// Parse a module path from import path segments (legacy, infers prefix from first segment).
    pub fn from_import_path(segments: &[String]) -> Self {
        if segments.is_empty() {
            return Self::absolute(Vec::new());
        }

        match segments[0].as_str() {
            "self" => Self::self_(segments[1..].to_vec()),
            "super" => {
                // Handle super path - remaining segments after "super"
                Self::super_(segments[1..].to_vec())
            }
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

// Re-export Visibility from the AST
pub use neve_syntax::Visibility;

/// Information about a loaded module.
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// Module ID
    pub id: ModuleId,
    /// Module path (e.g., ["std", "list"])
    pub path: Vec<String>,
    /// File path on disk
    pub file_path: PathBuf,
    /// Parent module (None for root)
    pub parent: Option<ModuleId>,
    /// Child modules
    pub children: Vec<ModuleId>,
    /// Exported items (name -> DefId)
    pub exports: HashMap<String, DefId>,
    /// All items with visibility
    pub items: HashMap<String, (DefId, Visibility)>,
}

/// Module loader responsible for discovering and loading modules.
pub struct ModuleLoader {
    /// Root directory for source files
    root_dir: PathBuf,
    /// All loaded modules
    modules: HashMap<ModuleId, ModuleInfo>,
    /// Module lookup by path
    path_to_id: HashMap<Vec<String>, ModuleId>,
    /// Module lookup by file path
    file_to_id: HashMap<PathBuf, ModuleId>,
    /// Next module ID
    next_id: u32,
    /// Standard library path (if available)
    std_path: Option<PathBuf>,
    /// Diagnostics collected during loading
    diagnostics: Vec<Diagnostic>,
    /// Modules currently being loaded (for cycle detection)
    /// Maps module path to its loading stack for detailed error messages
    loading: HashSet<Vec<String>>,
    /// Loading stack to track the import chain
    loading_stack: Vec<Vec<String>>,
}

impl ModuleLoader {
    /// Create a new module loader with the given root directory.
    pub fn new(root_dir: impl AsRef<Path>) -> Self {
        Self {
            root_dir: root_dir.as_ref().to_path_buf(),
            modules: HashMap::new(),
            path_to_id: HashMap::new(),
            file_to_id: HashMap::new(),
            next_id: 0,
            std_path: None,
            diagnostics: Vec::new(),
            loading: HashSet::new(),
            loading_stack: Vec::new(),
        }
    }

    /// Set the standard library path.
    pub fn with_std_path(mut self, path: impl AsRef<Path>) -> Self {
        self.std_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Get the root directory.
    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    /// Get collected diagnostics.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Take collected diagnostics.
    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Allocate a new module ID.
    fn fresh_module_id(&mut self) -> ModuleId {
        let id = ModuleId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Resolve a module path to a file path.
    pub fn resolve_path(
        &self,
        path: &ModulePath,
        from_module: Option<&[String]>,
    ) -> Option<PathBuf> {
        let absolute_path = self.make_absolute(path, from_module)?;
        self.find_module_file(&absolute_path)
    }

    /// Convert a relative path to an absolute path.
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
                    return None; // Can't go above root or single-level module
                }
                // Go up two levels: remove current file and then go to parent directory
                // E.g., from ["mylib", "submod", "worker"] -> ["mylib"]
                let mut result = from[..from.len() - 2].to_vec();

                // Handle multiple super or additional path segments
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

    /// Find the file path for a module path.
    fn find_module_file(&self, module_path: &[String]) -> Option<PathBuf> {
        if module_path.is_empty() {
            return Some(self.root_dir.join("lib.neve"));
        }

        // Check if it's a standard library module
        if module_path.first().map(|s| s.as_str()) == Some("std")
            && let Some(std_path) = &self.std_path
        {
            let relative: PathBuf = module_path[1..].iter().collect();

            // Try module_name.neve
            let file_path = std_path.join(&relative).with_extension("neve");
            if file_path.exists() {
                return Some(file_path);
            }

            // Try module_name/mod.neve
            let mod_path = std_path.join(&relative).join("mod.neve");
            if mod_path.exists() {
                return Some(mod_path);
            }
        }

        // Build relative path
        let relative: PathBuf = module_path.iter().collect();

        // Try module_name.neve
        let file_path = self.root_dir.join(&relative).with_extension("neve");
        if file_path.exists() {
            return Some(file_path);
        }

        // Try module_name/mod.neve
        let mod_path = self.root_dir.join(&relative).join("mod.neve");
        if mod_path.exists() {
            return Some(mod_path);
        }

        // Try src/module_name.neve
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

    /// Load a module by path.
    pub fn load_module(&mut self, path: &[String]) -> Result<ModuleId, ModuleLoadError> {
        // Check if already loaded
        if let Some(&id) = self.path_to_id.get(path) {
            return Ok(id);
        }

        // Check for circular dependency
        if self.loading.contains(path) {
            // Build the circular dependency chain
            let mut chain = self.loading_stack.clone();
            chain.push(path.to_vec());
            return Err(ModuleLoadError::CircularDependency {
                module: path.to_vec(),
                chain,
            });
        }

        // Find the file
        let _module_path = ModulePath::absolute(path.to_vec());
        let file_path = self
            .find_module_file(path)
            .ok_or_else(|| ModuleLoadError::NotFound(path.to_vec()))?;

        // Mark as loading and add to stack
        self.loading.insert(path.to_vec());
        self.loading_stack.push(path.to_vec());

        // Read and parse the file
        let source = fs::read_to_string(&file_path)
            .map_err(|e| ModuleLoadError::IoError(file_path.clone(), e.to_string()))?;

        // Parse the source
        let (source_file, parse_errors) = neve_parser::parse(&source);

        // Collect parse errors
        for error in parse_errors {
            self.diagnostics.push(error);
        }

        // Allocate module ID
        let module_id = self.fresh_module_id();

        // Load dependencies (imports) BEFORE registering the module as loaded
        // This allows circular dependency detection to work correctly
        //
        // IMPORTANT: For `pub import` (re-exports), we need special handling to avoid
        // infinite loops when modules re-export each other's symbols.
        for item in &source_file.items {
            if let neve_syntax::ItemKind::Import(import_def) = &item.kind {
                let import_path = ModulePath::from_import_def(import_def);

                // Check if this is a re-export (pub import)
                let is_reexport = import_def.visibility != neve_syntax::Visibility::Private;

                #[allow(clippy::collapsible_if)]
                if let Some(abs_path) = self.make_absolute(&import_path, Some(path))
                    && abs_path != path
                // Only load if not a self-reference
                {
                    // For re-exports, check if the target module is already being loaded
                    // in our dependency chain. If so, we can safely skip loading it now
                    // and defer symbol resolution to later.
                    if is_reexport && self.loading.contains(&abs_path) {
                        // This is a re-export of a module that's currently being loaded.
                        // This is safe - we'll resolve the symbols later after all modules
                        // are loaded. This breaks the infinite loop.
                        continue;
                    }

                    // Propagate circular dependency errors immediately
                    if let Err(e) = self.load_module(&abs_path) {
                        match &e {
                            // Circular dependencies and module not found should fail immediately
                            ModuleLoadError::CircularDependency { .. }
                            | ModuleLoadError::NotFound(_) => {
                                // Remove from loading set and stack before returning error
                                self.loading.remove(path);
                                self.loading_stack.pop();
                                return Err(e);
                            }
                            // Other errors get logged but don't block loading
                            _ => {
                                self.diagnostics.push(Diagnostic::error(
                                    neve_diagnostic::DiagnosticKind::Module,
                                    item.span,
                                    format!(
                                        "Failed to load module '{}': {}",
                                        abs_path.join("."),
                                        e
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Create module info
        let info = ModuleInfo {
            id: module_id,
            path: path.to_vec(),
            file_path: file_path.clone(),
            parent: self.find_parent_module(path),
            children: Vec::new(),
            exports: HashMap::new(),
            items: HashMap::new(),
        };

        // Register the module as loaded (only after dependencies are loaded)
        self.modules.insert(module_id, info);
        self.path_to_id.insert(path.to_vec(), module_id);
        self.file_to_id.insert(file_path, module_id);

        // Update parent's children list
        if let Some(parent_id) = self.find_parent_module(path)
            && let Some(parent_info) = self.modules.get_mut(&parent_id)
        {
            parent_info.children.push(module_id);
        }

        // Remove from loading set and stack
        self.loading.remove(path);
        self.loading_stack.pop();

        Ok(module_id)
    }

    /// Find the parent module for a given path.
    fn find_parent_module(&self, path: &[String]) -> Option<ModuleId> {
        if path.len() <= 1 {
            return None;
        }
        self.path_to_id.get(&path[..path.len() - 1]).copied()
    }

    /// Get module info by ID.
    pub fn get_module(&self, id: ModuleId) -> Option<&ModuleInfo> {
        self.modules.get(&id)
    }

    /// Get mutable module info by ID.
    pub fn get_module_mut(&mut self, id: ModuleId) -> Option<&mut ModuleInfo> {
        self.modules.get_mut(&id)
    }

    /// Look up a module by path.
    pub fn lookup_module(&self, path: &[String]) -> Option<ModuleId> {
        self.path_to_id.get(path).copied()
    }

    /// Get all loaded modules.
    pub fn all_modules(&self) -> impl Iterator<Item = (&Vec<String>, &ModuleInfo)> {
        self.path_to_id
            .iter()
            .filter_map(|(path, &id)| self.modules.get(&id).map(|info| (path, info)))
    }

    /// Register an exported item for a module.
    pub fn register_export(
        &mut self,
        module_id: ModuleId,
        name: String,
        def_id: DefId,
        visibility: Visibility,
    ) {
        if let Some(info) = self.modules.get_mut(&module_id) {
            info.items.insert(name.clone(), (def_id, visibility));
            if visibility == Visibility::Public {
                info.exports.insert(name, def_id);
            }
        }
    }

    /// Resolve an import from one module to another.
    pub fn resolve_import(
        &self,
        import: &Import,
        from_module: &[String],
    ) -> Result<Vec<(String, DefId)>, ImportResolveError> {
        let import_path = ModulePath::from_hir_import(import);

        let target_path = self
            .make_absolute(&import_path, Some(from_module))
            .ok_or_else(|| ImportResolveError::InvalidPath(import.path.clone()))?;

        let target_id = self
            .path_to_id
            .get(&target_path)
            .ok_or_else(|| ImportResolveError::ModuleNotFound(target_path.clone()))?;

        let target_info = self
            .modules
            .get(target_id)
            .ok_or_else(|| ImportResolveError::ModuleNotFound(target_path.clone()))?;

        // Check visibility based on module relationship
        let can_access = |visibility: Visibility| -> bool {
            match visibility {
                Visibility::Public => true,
                Visibility::Crate => true, // Within same crate
                Visibility::Super => {
                    // Check if from_module is a child of target's parent
                    if let Some(parent) = &target_info.parent
                        && let Some(parent_info) = self.modules.get(parent)
                    {
                        return from_module.starts_with(&parent_info.path);
                    }
                    false
                }
                Visibility::Private => from_module == target_path.as_slice(),
            }
        };

        match &import.kind {
            ImportKind::Module => {
                // Import the module as a namespace
                let alias = import
                    .alias
                    .as_ref()
                    .or_else(|| target_path.last())
                    .cloned()
                    .ok_or_else(|| ImportResolveError::InvalidPath(import.path.clone()))?;

                // Return all accessible exports with the namespace prefix
                let exports: Vec<_> = target_info
                    .exports
                    .iter()
                    .filter(|&(name, _)| {
                        target_info
                            .items
                            .get(name)
                            .map(|(_, vis)| can_access(*vis))
                            .unwrap_or(false)
                    })
                    .map(|(name, def_id)| (format!("{}.{}", alias, name), *def_id))
                    .collect();

                Ok(exports)
            }
            ImportKind::Items(names) => {
                let mut result = Vec::new();
                for name in names {
                    if let Some(&def_id) = target_info.exports.get(name) {
                        if let Some((_, visibility)) = target_info.items.get(name) {
                            if can_access(*visibility) {
                                result.push((name.clone(), def_id));
                            } else {
                                return Err(ImportResolveError::PrivateItem(name.clone()));
                            }
                        }
                    } else {
                        return Err(ImportResolveError::ItemNotFound(name.clone()));
                    }
                }
                Ok(result)
            }
            ImportKind::All => {
                let exports: Vec<_> = target_info
                    .exports
                    .iter()
                    .filter(|(name, _)| {
                        target_info
                            .items
                            .get(*name)
                            .map(|(_, vis)| can_access(*vis))
                            .unwrap_or(false)
                    })
                    .map(|(name, &def_id)| (name.clone(), def_id))
                    .collect();
                Ok(exports)
            }
        }
    }

    /// Discover all modules in the project.
    pub fn discover_modules(&mut self) -> Result<Vec<ModuleId>, ModuleLoadError> {
        let mut discovered = Vec::new();

        // Start with lib.neve or main.neve
        let _root_file = if self.root_dir.join("lib.neve").exists() {
            self.root_dir.join("lib.neve")
        } else if self.root_dir.join("main.neve").exists() {
            self.root_dir.join("main.neve")
        } else if self.root_dir.join("src/lib.neve").exists() {
            self.root_dir.join("src/lib.neve")
        } else if self.root_dir.join("src/main.neve").exists() {
            self.root_dir.join("src/main.neve")
        } else {
            return Err(ModuleLoadError::NoRootModule);
        };

        // Load the root module
        let root_id = self.load_module(&[])?;
        discovered.push(root_id);

        // Recursively discover submodules
        self.discover_submodules(&self.root_dir.clone(), &[], &mut discovered)?;

        Ok(discovered)
    }

    /// Recursively discover submodules in a directory.
    fn discover_submodules(
        &mut self,
        dir: &Path,
        parent_path: &[String],
        discovered: &mut Vec<ModuleId>,
    ) -> Result<(), ModuleLoadError> {
        let entries = fs::read_dir(dir)
            .map_err(|e| ModuleLoadError::IoError(dir.to_path_buf(), e.to_string()))?;

        for entry in entries {
            let entry =
                entry.map_err(|e| ModuleLoadError::IoError(dir.to_path_buf(), e.to_string()))?;
            let path = entry.path();
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            if path.is_file() && file_name.ends_with(".neve") && file_name != "mod.neve" {
                let module_name = file_name.trim_end_matches(".neve");
                let mut module_path = parent_path.to_vec();
                module_path.push(module_name.to_string());

                if let Ok(id) = self.load_module(&module_path) {
                    discovered.push(id);
                }
            } else if path.is_dir() && !file_name.starts_with('.') {
                // Check for mod.neve in subdirectory
                let mod_file = path.join("mod.neve");
                if mod_file.exists() {
                    let mut module_path = parent_path.to_vec();
                    module_path.push(file_name.to_string());

                    if let Ok(id) = self.load_module(&module_path) {
                        discovered.push(id);
                    }

                    // Recurse into subdirectory
                    self.discover_submodules(&path, &module_path, discovered)?;
                }
            }
        }

        Ok(())
    }
}

/// Errors that can occur during module loading.
#[derive(Debug, Clone)]
pub enum ModuleLoadError {
    /// Module file not found
    NotFound(Vec<String>),
    /// Circular dependency detected
    CircularDependency {
        /// The module that caused the cycle
        module: Vec<String>,
        /// The full import chain showing the cycle
        chain: Vec<Vec<String>>,
    },
    /// IO error reading file
    IoError(PathBuf, String),
    /// No root module found
    NoRootModule,
    /// Parse error in module
    ParseError(Vec<String>),
}

impl std::fmt::Display for ModuleLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModuleLoadError::NotFound(path) => {
                write!(f, "module not found: {}", path.join("."))
            }
            ModuleLoadError::CircularDependency { module, chain } => {
                writeln!(
                    f,
                    "circular dependency detected when importing module: {}",
                    module.join(".")
                )?;
                writeln!(f, "\nImport chain:")?;
                for (i, m) in chain.iter().enumerate() {
                    if i == chain.len() - 1 {
                        writeln!(f, "  {} -> {} (cycle!)", m.join("."), module.join("."))?;
                    } else {
                        writeln!(f, "  {}", m.join("."))?;
                    }
                }
                Ok(())
            }
            ModuleLoadError::IoError(path, msg) => {
                write!(f, "error reading {}: {}", path.display(), msg)
            }
            ModuleLoadError::NoRootModule => {
                write!(f, "no root module found (lib.neve or main.neve)")
            }
            ModuleLoadError::ParseError(path) => {
                write!(f, "parse error in module: {}", path.join("."))
            }
        }
    }
}

/// Errors that can occur during import resolution.
#[derive(Debug, Clone)]
pub enum ImportResolveError {
    /// Invalid import path
    InvalidPath(Vec<String>),
    /// Module not found
    ModuleNotFound(Vec<String>),
    /// Item not found in module
    ItemNotFound(String),
    /// Item is private
    PrivateItem(String),
}

impl std::fmt::Display for ImportResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportResolveError::InvalidPath(path) => {
                write!(f, "invalid import path: {}", path.join("."))
            }
            ImportResolveError::ModuleNotFound(path) => {
                write!(f, "module not found: {}", path.join("."))
            }
            ImportResolveError::ItemNotFound(name) => {
                write!(f, "item not found: {}", name)
            }
            ImportResolveError::PrivateItem(name) => {
                write!(f, "item is private: {}", name)
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
    fn test_make_absolute() {
        let loader = ModuleLoader::new("/tmp");

        // Absolute path stays the same
        let path = ModulePath::absolute(vec!["std".into(), "list".into()]);
        let result = loader.make_absolute(&path, Some(&["mymod".into()]));
        assert_eq!(result, Some(vec!["std".into(), "list".into()]));

        // Self-relative path
        let path = ModulePath::self_(vec!["utils".into()]);
        let result = loader.make_absolute(&path, Some(&["mymod".into()]));
        assert_eq!(result, Some(vec!["mymod".into(), "utils".into()]));

        // Super-relative path
        let path = ModulePath::super_(vec!["common".into()]);
        let result = loader.make_absolute(&path, Some(&["parent".into(), "child".into()]));
        assert_eq!(result, Some(vec!["parent".into(), "common".into()]));
    }

    #[test]
    fn test_circular_dependency_error_message() {
        // Test that circular dependency error includes the full chain
        let error = ModuleLoadError::CircularDependency {
            module: vec!["a".into()],
            chain: vec![vec!["a".into()], vec!["b".into()], vec!["c".into()]],
        };

        let message = format!("{}", error);
        assert!(message.contains("circular dependency"));
        assert!(message.contains("Import chain"));
        assert!(message.contains("(cycle!)"));
    }

    #[test]
    fn test_loading_stack_management() {
        let mut loader = ModuleLoader::new("/tmp");

        // Initially empty
        assert!(loader.loading.is_empty());
        assert!(loader.loading_stack.is_empty());

        // Simulate loading a module
        let path = vec!["test".into()];
        loader.loading.insert(path.clone());
        loader.loading_stack.push(path.clone());

        assert!(loader.loading.contains(&path));
        assert_eq!(loader.loading_stack.len(), 1);

        // Detect cycle if trying to load the same module
        assert!(loader.loading.contains(&path));

        // Cleanup
        loader.loading.remove(&path);
        loader.loading_stack.pop();

        assert!(loader.loading.is_empty());
        assert!(loader.loading_stack.is_empty());
    }
}
