//! Flake support for Neve.
//! Neve 的 Flake 支持。
//!
//! Flakes provide a standard way to define reproducible Neve projects
//! with explicit dependencies and outputs.
//!
//! Flake 提供了一种标准方式来定义具有明确依赖和输出的可复现 Neve 项目。
//!
//! A flake is defined by a `flake.neve` file in the project root that exports:
//! Flake 由项目根目录中的 `flake.neve` 文件定义，该文件导出：
//!
//! - `inputs`: Dependencies on other flakes / 对其他 flake 的依赖
//! - `outputs`: A function that produces packages, configurations, etc.
//!   输出函数，生成包、配置等

use crate::ConfigError;
use neve_derive::Hash;
use neve_diagnostic::{Diagnostic, Severity};
use neve_eval::{EvaluableModuleRef, Evaluator, Value};
use neve_fetch::{
    archive as fetch_archive, git as fetch_git, url as fetch_url, verify as fetch_verify,
};
use neve_frontend::{FrontendDriver, ProgramAnalysis, analyze_source};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

/// A flake input specification.
/// Flake 输入规范。
#[derive(Debug, Clone)]
pub struct FlakeInput {
    /// Input name. / 输入名称。
    pub name: String,
    /// Input URL or path. / 输入 URL 或路径。
    pub url: String,
    /// Whether to follow another input's version. / 是否跟随另一个输入的版本。
    pub follows: Option<String>,
    /// Specific revision/commit. / 特定的修订版本/提交。
    pub rev: Option<String>,
    /// Specific branch. / 特定的分支。
    pub branch: Option<String>,
    /// Specific tag. / 特定的标签。
    pub tag: Option<String>,
}

impl FlakeInput {
    /// Create a new flake input.
    /// 创建新的 flake 输入。
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
            follows: None,
            rev: None,
            branch: None,
            tag: None,
        }
    }

    /// Set the input to follow another input.
    /// 设置输入跟随另一个输入。
    pub fn follows(mut self, other: impl Into<String>) -> Self {
        self.follows = Some(other.into());
        self
    }

    /// Set a specific revision.
    /// 设置特定的修订版本。
    pub fn rev(mut self, rev: impl Into<String>) -> Self {
        self.rev = Some(rev.into());
        self
    }

    /// Set a specific branch.
    /// 设置特定的分支。
    pub fn branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }

    /// Set a specific tag.
    /// 设置特定的标签。
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// Parse from a Value.
    /// 从 Value 解析。
    pub fn from_value(name: &str, value: &Value) -> Result<Self, ConfigError> {
        match value {
            Value::String(url) => Ok(Self::new(name, url.as_str())),
            Value::Record(fields) => {
                let url = fields
                    .get("url")
                    .and_then(|v| {
                        if let Value::String(s) = v {
                            Some(s.as_str())
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| ConfigError::Flake("input requires 'url' field".into()))?;

                let mut input = Self::new(name, url);

                if let Some(Value::String(follows)) = fields.get("follows") {
                    input.follows = Some(follows.to_string());
                }
                if let Some(Value::String(rev)) = fields.get("rev") {
                    input.rev = Some(rev.to_string());
                }
                if let Some(Value::String(branch)) = fields.get("branch") {
                    input.branch = Some(branch.to_string());
                }
                if let Some(Value::String(tag)) = fields.get("tag") {
                    input.tag = Some(tag.to_string());
                }

                Ok(input)
            }
            _ => Err(ConfigError::Flake(format!(
                "invalid input '{}': expected string or record",
                name
            ))),
        }
    }
}

/// Flake output types.
/// Flake 输出类型。
#[derive(Debug, Clone)]
pub enum FlakeOutput {
    /// A package derivation. / 包推导。
    Package(Value),
    /// A development shell. / 开发 shell。
    DevShell(Value),
    /// A NixOS/Neve system configuration. / NixOS/Neve 系统配置。
    System(Value),
    /// A home-manager configuration. / home-manager 配置。
    HomeConfig(Value),
    /// An overlay. / 覆盖层。
    Overlay(Value),
    /// A Neve module. / Neve 模块。
    Module(Value),
    /// A template. / 模板。
    Template(Value),
    /// A generic output. / 通用输出。
    Other(Value),
}

impl FlakeOutput {
    /// Extract the underlying output value.
    /// 提取底层输出值。
    fn into_value(self) -> Value {
        match self {
            Self::Package(value)
            | Self::DevShell(value)
            | Self::System(value)
            | Self::HomeConfig(value)
            | Self::Overlay(value)
            | Self::Module(value)
            | Self::Template(value)
            | Self::Other(value) => value,
        }
    }
}

/// A flake lock entry.
/// Flake 锁定条目。
#[derive(Debug, Clone)]
pub struct FlakeLockEntry {
    /// Input name. / 输入名称。
    pub name: String,
    /// Resolved URL. / 解析后的 URL。
    pub url: String,
    /// Content hash. / 内容哈希。
    pub hash: String,
    /// Last modified timestamp. / 最后修改时间戳。
    pub last_modified: u64,
    /// Revision (for git sources). / 修订版本（用于 git 源）。
    pub rev: Option<String>,
}

/// A flake lock file.
/// Flake 锁定文件。
#[derive(Debug, Clone, Default)]
pub struct FlakeLock {
    /// Version of the lock file format. / 锁定文件格式版本。
    pub version: u32,
    /// Locked inputs. / 锁定的输入。
    pub inputs: HashMap<String, FlakeLockEntry>,
}

impl FlakeLock {
    /// Create a new empty lock file.
    /// 创建新的空锁定文件。
    pub fn new() -> Self {
        Self {
            version: 1,
            inputs: HashMap::new(),
        }
    }

    /// Load a lock file from disk.
    /// 从磁盘加载锁定文件。
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Parse a lock file from JSON.
    /// 从 JSON 解析锁定文件。
    pub fn parse(content: &str) -> Result<Self, ConfigError> {
        // Simple JSON parsing for lock file
        // 简单的锁定文件 JSON 解析
        let value: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| ConfigError::Flake(format!("invalid lock file: {}", e)))?;

        let version = value.get("version").and_then(|v| v.as_u64()).unwrap_or(1) as u32;

        let mut inputs = HashMap::new();

        if let Some(nodes) = value.get("nodes").and_then(|v| v.as_object()) {
            for (name, node) in nodes {
                if name == "root" {
                    continue;
                }

                let locked = node.get("locked").and_then(|v| v.as_object());
                if let Some(locked) = locked {
                    let url = locked
                        .get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let hash = locked
                        .get("narHash")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let last_modified = locked
                        .get("lastModified")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let rev = locked
                        .get("rev")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    inputs.insert(
                        name.clone(),
                        FlakeLockEntry {
                            name: name.clone(),
                            url,
                            hash,
                            last_modified,
                            rev,
                        },
                    );
                }
            }
        }

        Ok(Self { version, inputs })
    }

    /// Save the lock file to disk.
    /// 将锁定文件保存到磁盘。
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let content = self.to_json();
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Convert to JSON string.
    /// 转换为 JSON 字符串。
    pub fn to_json(&self) -> String {
        let mut nodes = serde_json::Map::new();
        let mut names: Vec<&str> = self.inputs.keys().map(|s| s.as_str()).collect();
        names.sort_unstable();

        // Root node
        // 根节点
        let mut root_inputs = serde_json::Map::new();
        for name in &names {
            root_inputs.insert(
                (*name).to_string(),
                serde_json::Value::String((*name).to_string()),
            );
        }
        nodes.insert(
            "root".to_string(),
            serde_json::json!({
                "inputs": root_inputs
            }),
        );

        // Input nodes
        // 输入节点
        for name in names {
            let entry = &self.inputs[name];
            let mut locked = serde_json::Map::new();
            locked.insert(
                "url".to_string(),
                serde_json::Value::String(entry.url.clone()),
            );
            locked.insert(
                "narHash".to_string(),
                serde_json::Value::String(entry.hash.clone()),
            );
            locked.insert(
                "lastModified".to_string(),
                serde_json::Value::Number(entry.last_modified.into()),
            );
            if let Some(ref rev) = entry.rev {
                locked.insert("rev".to_string(), serde_json::Value::String(rev.clone()));
            }

            nodes.insert(
                name.to_string(),
                serde_json::json!({
                    "locked": locked
                }),
            );
        }

        serde_json::to_string_pretty(&serde_json::json!({
            "version": self.version,
            "nodes": nodes
        }))
        .unwrap_or_default()
    }
}

/// A Neve flake.
/// Neve flake。
#[derive(Debug)]
pub struct Flake {
    /// Flake root directory. / Flake 根目录。
    pub root: PathBuf,
    /// Flake description. / Flake 描述。
    pub description: Option<String>,
    /// Flake inputs. / Flake 输入。
    pub inputs: HashMap<String, FlakeInput>,
    /// The outputs function (as a Value). / 输出函数（作为 Value）。
    pub outputs: Option<Value>,
    /// Resolved inputs (after locking). / 解析后的输入（锁定后）。
    pub resolved_inputs: HashMap<String, Value>,
    /// Lock file. / 锁定文件。
    pub lock: FlakeLock,
    source_path: Option<PathBuf>,
    source_snapshot: Option<String>,
}

impl Flake {
    /// Create a new empty flake.
    /// 创建新的空 flake。
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            description: None,
            inputs: HashMap::new(),
            outputs: None,
            resolved_inputs: HashMap::new(),
            lock: FlakeLock::new(),
            source_path: None,
            source_snapshot: None,
        }
    }

    /// Load a flake from a directory.
    /// 从目录加载 flake。
    pub fn load(root: &Path) -> Result<Self, ConfigError> {
        let flake_file = root.join("flake.neve");
        if !flake_file.exists() {
            return Err(ConfigError::Flake(format!(
                "no flake.neve found in {}",
                root.display()
            )));
        }

        let flake_path = flake_file.canonicalize().map_err(|e| {
            ConfigError::Flake(format!(
                "failed to canonicalize flake path '{}': {}",
                flake_file.display(),
                e
            ))
        })?;
        let source = std::fs::read_to_string(&flake_path)?;
        let value = eval_flake_file_via_frontend(&flake_path)?;
        let mut flake = flake_from_value(value, root.to_path_buf())?;
        flake.source_path = Some(flake_path);
        flake.source_snapshot = Some(source);

        // Try to load lock file
        // 尝试加载锁定文件
        let lock_file = root.join("flake.lock");
        if lock_file.exists() {
            flake.lock = FlakeLock::load(&lock_file)?;
        }

        Ok(flake)
    }

    /// Parse a flake from source.
    /// 从源码解析 flake。
    pub fn parse(source: &str, root: PathBuf) -> Result<Self, ConfigError> {
        let value = eval_flake_source_via_frontend(source)?;
        let mut flake = flake_from_value(value, root)?;
        flake.source_snapshot = Some(source.to_string());
        Ok(flake)
    }

    /// Lock the flake inputs.
    /// 锁定 flake 输入。
    pub fn lock_inputs(&mut self) -> Result<(), ConfigError> {
        // For each input, resolve it and add to the lock file
        // 对于每个输入，解析它并添加到锁定文件
        for (name, input) in &self.inputs {
            // Skip if already locked and not updated
            // 如果已锁定且未更新则跳过
            if self.lock.inputs.contains_key(name) {
                continue;
            }

            // Resolve the input
            // 解析输入
            let entry = self.resolve_input(input)?;
            self.lock.inputs.insert(name.clone(), entry);
        }

        Ok(())
    }

    /// Resolve a single input.
    /// 解析单个输入。
    fn resolve_input(&self, input: &FlakeInput) -> Result<FlakeLockEntry, ConfigError> {
        let url = input.url.as_str();

        if url.starts_with("github:") {
            let repo_path = url
                .strip_prefix("github:")
                .ok_or_else(|| ConfigError::Flake("invalid github URL".into()))?;
            let parts: Vec<&str> = repo_path.split('/').collect();
            if parts.len() < 2 {
                return Err(ConfigError::Flake(format!("invalid github URL: {}", url)));
            }
            let owner = parts[0];
            let repo = parts[1];
            let owner_repo = format!("{}/{}", owner, repo);
            let url_ref = if parts.len() > 2 {
                Some(parts[2..].join("/"))
            } else {
                None
            };
            let git_ref = input
                .rev
                .clone()
                .or_else(|| input.tag.clone())
                .or_else(|| input.branch.clone())
                .or(url_ref)
                .unwrap_or_else(|| "HEAD".to_string());
            let git_url = format!("https://github.com/{}.git", owner_repo);
            let (commit, hash) = resolve_git_source(&git_url, &git_ref)?;
            return Ok(FlakeLockEntry {
                name: input.name.clone(),
                url: format!(
                    "https://github.com/{}/archive/{}.tar.gz",
                    owner_repo, commit
                ),
                hash: hash_to_lock(&hash),
                last_modified: stable_last_modified_from_hash(&hash),
                rev: Some(commit),
            });
        }

        if url.starts_with("git+") || url.ends_with(".git") {
            let git_url = url.strip_prefix("git+").unwrap_or(url);
            let git_ref = input
                .rev
                .clone()
                .or_else(|| input.tag.clone())
                .or_else(|| input.branch.clone())
                .unwrap_or_else(|| "HEAD".to_string());
            let (commit, hash) = resolve_git_source(git_url, &git_ref)?;
            return Ok(FlakeLockEntry {
                name: input.name.clone(),
                url: git_url.to_string(),
                hash: hash_to_lock(&hash),
                last_modified: stable_last_modified_from_hash(&hash),
                rev: Some(commit),
            });
        }

        if url.starts_with("path:") || url.starts_with("./") || url.starts_with('/') {
            let raw = url.strip_prefix("path:").unwrap_or(url);
            let raw_path = Path::new(raw);
            let abs_path = if raw_path.is_absolute() {
                raw_path.to_path_buf()
            } else {
                self.root.join(raw_path)
            };
            let path = abs_path
                .canonicalize()
                .map_err(|e| ConfigError::Flake(format!("invalid path input '{}': {}", raw, e)))?;
            let metadata = fs::symlink_metadata(&path).map_err(|e| {
                ConfigError::Flake(format!("cannot read metadata '{}': {}", path.display(), e))
            })?;
            let hash = if metadata.file_type().is_symlink() {
                let target = fs::read_link(&path).map_err(|e| {
                    ConfigError::Flake(format!(
                        "cannot read symlink target '{}': {}",
                        path.display(),
                        e
                    ))
                })?;
                Hash::of(target.as_os_str().to_string_lossy().as_bytes())
            } else if metadata.is_file() {
                let content = fs::read(&path).map_err(|e| {
                    ConfigError::Flake(format!("cannot read file '{}': {}", path.display(), e))
                })?;
                Hash::of(&content)
            } else if metadata.is_dir() {
                fetch_verify::hash_dir(&path).map_err(|e| {
                    ConfigError::Flake(format!("cannot hash directory '{}': {}", path.display(), e))
                })?
            } else {
                return Err(ConfigError::Flake(format!(
                    "unsupported path input type: {}",
                    path.display()
                )));
            };

            return Ok(FlakeLockEntry {
                name: input.name.clone(),
                url: format!("path:{}", path.display()),
                hash: hash_to_lock(&hash),
                last_modified: stable_last_modified_from_hash(&hash),
                rev: None,
            });
        }

        if url.starts_with("http://") || url.starts_with("https://") {
            let content = fetch_url::fetch_url(url)
                .map_err(|e| ConfigError::Flake(format!("failed to fetch '{}': {}", url, e)))?;
            let hash = Hash::of(&content);
            return Ok(FlakeLockEntry {
                name: input.name.clone(),
                url: url.to_string(),
                hash: hash_to_lock(&hash),
                last_modified: stable_last_modified_from_hash(&hash),
                rev: None,
            });
        }

        // Fallback: treat as GitHub shorthand like "owner/repo" or "owner/repo/ref".
        // 回退：将其视为 GitHub 简写，如 "owner/repo" 或 "owner/repo/ref"。
        let shorthand = format!("github:{}", url);
        self.resolve_input(&FlakeInput {
            url: shorthand,
            ..input.clone()
        })
    }

    /// Save the lock file.
    /// 保存锁定文件。
    pub fn save_lock(&self) -> Result<(), ConfigError> {
        let lock_file = self.root.join("flake.lock");
        self.lock.save(&lock_file)
    }

    /// Collect materialized source roots for all flake inputs.
    /// Returns a map from input name to local source directory.
    /// 收集所有 flake 输入的物化源码根目录。
    /// 返回从输入名称到本地源码目录的映射。
    pub fn collect_input_roots(&mut self) -> Result<HashMap<String, PathBuf>, ConfigError> {
        let mut roots = HashMap::new();
        for (name, input) in &self.inputs {
            // Skip follows (they point to another input)
            if input.follows.is_some() {
                continue;
            }
            if let Some(root) =
                self.resolve_input_source_root(&input.url, input.rev.as_deref(), None)?
            {
                roots.insert(name.clone(), root);
            }
        }
        Ok(roots)
    }

    /// Update a single input in the lock file.
    /// 更新锁定文件中的单个输入。
    pub fn update_input(&mut self, name: &str) -> Result<(), ConfigError> {
        let input = self
            .inputs
            .get(name)
            .ok_or_else(|| ConfigError::Flake(format!("input '{}' not found", name)))?
            .clone();

        let entry = self.resolve_input(&input)?;
        self.lock.inputs.insert(name.to_string(), entry);

        // Save the updated lock file
        let lock_path = self.root.join("flake.lock");
        self.lock.save(&lock_path)?;

        Ok(())
    }

    /// Update all inputs that have changed since last lock.
    /// 更新自上次锁定以来已更改的所有输入。
    pub fn update_changed_inputs(&mut self) -> Result<Vec<String>, ConfigError> {
        let mut updated = Vec::new();

        for (name, input) in &self.inputs.clone() {
            // Check if this input was previously locked
            if let Some(locked) = self.lock.inputs.get(name) {
                // For git sources, check if HEAD has moved
                if input.url.starts_with("git+")
                    || input.url.ends_with(".git")
                    || input.url.starts_with("github:")
                {
                    // Re-resolve to check for updates
                    let new_entry = self.resolve_input(input)?;
                    if new_entry.rev != locked.rev {
                        self.lock.inputs.insert(name.clone(), new_entry);
                        updated.push(name.clone());
                    }
                }
            } else {
                // New input — resolve and lock
                let entry = self.resolve_input(input)?;
                self.lock.inputs.insert(name.clone(), entry);
                updated.push(name.clone());
            }
        }

        if !updated.is_empty() {
            let lock_path = self.root.join("flake.lock");
            self.lock.save(&lock_path)?;
        }

        Ok(updated)
    }

    /// Evaluate the flake outputs.
    /// 评估 flake 输出。
    pub fn eval_outputs(&mut self) -> Result<HashMap<String, FlakeOutput>, ConfigError> {
        let root = self
            .root
            .canonicalize()
            .unwrap_or_else(|_| self.root.clone());
        let mut stack = vec![root];
        self.eval_outputs_with_stack(&mut stack)
    }

    /// Evaluate outputs with recursion stack tracking for input flakes.
    /// 使用递归栈追踪输入 flake 来评估输出。
    fn eval_outputs_with_stack(
        &mut self,
        stack: &mut Vec<PathBuf>,
    ) -> Result<HashMap<String, FlakeOutput>, ConfigError> {
        let outputs_fn = self
            .outputs
            .clone()
            .ok_or_else(|| ConfigError::Flake("flake has no outputs".into()))?;

        // Resolve each declared input to a concrete value.
        // 解析每个声明的输入为具体值。
        self.resolve_all_inputs_for_outputs(stack)?;

        // Create the inputs record to pass to the outputs function
        // 创建要传递给输出函数的输入记录
        let mut inputs_record = HashMap::new();
        inputs_record.insert("self".to_string(), self.to_value());

        for name in self.inputs.keys() {
            if let Some(resolved) = self.resolved_inputs.get(name) {
                inputs_record.insert(name.clone(), resolved.clone());
            } else {
                inputs_record.insert(
                    name.clone(),
                    self.input_url_to_value(&self.inputs[name].url),
                );
            }
        }

        // Call the outputs function with inputs
        // 使用输入调用输出函数
        let result = match outputs_fn {
            Value::Closure { .. } => {
                let (mut evaluator, outputs_fn) = self.rebuild_outputs_fn_via_frontend()?;
                let inputs_value = Value::Record(Rc::new(inputs_record));
                evaluator
                    .call_value(outputs_fn, vec![inputs_value])
                    .map_err(|e| ConfigError::Eval(format!("{:?}", e)))?
            }
            Value::Record(outputs) => Value::Record(outputs),
            _ => {
                return Err(ConfigError::Flake(
                    "outputs must be a function or record".into(),
                ));
            }
        };

        // Parse the outputs
        // 解析输出
        self.parse_outputs(&result)
    }

    fn rebuild_outputs_fn_via_frontend(&self) -> Result<(Evaluator, Value), ConfigError> {
        if let Some(path) = &self.source_path {
            let (root_dir, module_path) = resolve_source_module_path(path)?;
            let analysis = FrontendDriver::new(&root_dir)
                .analyze_module_path(&module_path)
                .map_err(|err| ConfigError::Flake(format!("frontend error: {err}")))?;
            ensure_flake_program_has_no_errors(&analysis)?;
            let (evaluator, root_value) = build_program_evaluator_and_root_value(&analysis)?;
            let outputs = extract_outputs_value(&root_value)?;
            return Ok((evaluator, outputs));
        }

        if let Some(source) = &self.source_snapshot {
            let analysis = analyze_source(source);
            ensure_single_flake_has_no_errors(&analysis.diagnostics)?;
            let mut evaluator = Evaluator::new();
            let root_value = evaluator
                .eval_evaluable_module(EvaluableModuleRef::new(
                    &analysis.hir,
                    &analysis.semantics.method_resolutions,
                ))
                .map_err(|e| ConfigError::Eval(format!("{:?}", e)))?;
            let outputs = extract_outputs_value(&root_value)?;
            return Ok((evaluator, outputs));
        }

        Err(ConfigError::Flake(
            "cannot rebuild outputs function without flake source context".to_string(),
        ))
    }

    /// Resolve all declared inputs before evaluating outputs.
    /// 在评估输出前解析所有声明的输入。
    fn resolve_all_inputs_for_outputs(
        &mut self,
        stack: &mut Vec<PathBuf>,
    ) -> Result<(), ConfigError> {
        let mut resolving = HashSet::new();
        let input_names: Vec<String> = self.inputs.keys().cloned().collect();

        for name in input_names {
            let value = self.resolve_single_input_value(&name, &mut resolving, stack)?;
            self.resolved_inputs.insert(name, value);
        }

        Ok(())
    }

    /// Resolve a single input value with cycle detection.
    /// 解析单个输入值并进行循环检测。
    fn resolve_single_input_value(
        &mut self,
        name: &str,
        resolving: &mut HashSet<String>,
        stack: &mut Vec<PathBuf>,
    ) -> Result<Value, ConfigError> {
        if let Some(existing) = self.resolved_inputs.get(name) {
            return Ok(existing.clone());
        }

        let input = self
            .inputs
            .get(name)
            .cloned()
            .ok_or_else(|| ConfigError::Flake(format!("unknown flake input '{}'", name)))?;

        if !resolving.insert(name.to_string()) {
            return Err(ConfigError::Flake(format!(
                "flake input cycle detected while resolving '{}'",
                name
            )));
        }

        let resolved = (|| -> Result<Value, ConfigError> {
            if let Some(follows) = input.follows.clone() {
                if !self.inputs.contains_key(&follows) {
                    return Err(ConfigError::Flake(format!(
                        "input '{}' follows unknown input '{}'",
                        name, follows
                    )));
                }
                return self.resolve_single_input_value(&follows, resolving, stack);
            }

            if let Some(value) = self.try_resolve_input_flake_value(name, &input, stack)? {
                return Ok(value);
            }

            if let Some(entry) = self.lock.inputs.get(name) {
                return Ok(self.lock_entry_to_value(entry));
            }

            Ok(self.input_url_to_value(&input.url))
        })();

        resolving.remove(name);

        let value = resolved?;
        self.resolved_inputs.insert(name.to_string(), value.clone());
        Ok(value)
    }

    /// Try to load and evaluate an input flake recursively.
    /// 尝试递归加载并评估输入 flake。
    fn try_resolve_input_flake_value(
        &self,
        name: &str,
        input: &FlakeInput,
        stack: &mut Vec<PathBuf>,
    ) -> Result<Option<Value>, ConfigError> {
        let lock_entry = self.lock.inputs.get(name);
        let source_root = if let Some(entry) = lock_entry {
            self.resolve_input_source_root(
                entry.url.as_str(),
                entry.rev.as_deref(),
                Some(&entry.hash),
            )?
            .or_else(|| {
                self.resolve_input_source_root(input.url.as_str(), input.rev.as_deref(), None)
                    .ok()
                    .flatten()
            })
        } else {
            self.resolve_input_source_root(input.url.as_str(), input.rev.as_deref(), None)?
        };

        let Some(source_root) = source_root else {
            return Ok(None);
        };

        if !source_root.join("flake.neve").exists() {
            return Ok(None);
        }

        if stack.contains(&source_root) {
            return Err(ConfigError::Flake(format!(
                "flake input cycle detected at '{}'",
                source_root.display()
            )));
        }

        let mut input_flake = Flake::load(&source_root)?;
        stack.push(source_root.clone());
        let outputs = input_flake.eval_outputs_with_stack(stack);
        let _ = stack.pop();
        let outputs = outputs?;

        Ok(Some(self.build_input_flake_value(
            &input_flake,
            outputs,
            lock_entry,
        )))
    }

    /// Build the value exposed to `outputs` for a fully loaded input flake.
    /// 构建完整输入 flake 暴露给 `outputs` 的值。
    fn build_input_flake_value(
        &self,
        input_flake: &Flake,
        outputs: HashMap<String, FlakeOutput>,
        lock_entry: Option<&FlakeLockEntry>,
    ) -> Value {
        let mut fields = match input_flake.to_value() {
            Value::Record(record) => (*record).clone(),
            _ => HashMap::new(),
        };

        for (name, output) in outputs {
            fields.insert(name, output.into_value());
        }

        if let Some(entry) = lock_entry {
            self.apply_lock_entry_metadata(&mut fields, entry);
        }

        Value::Record(Rc::new(fields))
    }

    /// Apply lock metadata fields to an already constructed input value.
    /// 将 lock 元数据字段合并到已构建的输入值中。
    fn apply_lock_entry_metadata(
        &self,
        fields: &mut HashMap<String, Value>,
        entry: &FlakeLockEntry,
    ) {
        fields
            .entry("url".to_string())
            .or_insert_with(|| Value::String(Rc::new(entry.url.clone())));
        fields
            .entry("narHash".to_string())
            .or_insert_with(|| Value::String(Rc::new(entry.hash.clone())));
        fields
            .entry("lastModified".to_string())
            .or_insert(Value::Int(entry.last_modified.into()));

        if let Some(rev) = &entry.rev {
            fields
                .entry("rev".to_string())
                .or_insert_with(|| Value::String(Rc::new(rev.clone())));
        }
    }

    /// Resolve an input source URL to a local root directory if possible.
    /// 将输入源 URL 解析为本地根目录（若可行）。
    fn resolve_input_source_root(
        &self,
        url: &str,
        rev: Option<&str>,
        expected_lock_hash: Option<&str>,
    ) -> Result<Option<PathBuf>, ConfigError> {
        if is_path_input(url) {
            return self.materialize_path_source(url).map(Some);
        }

        if url.starts_with("git+") || url.ends_with(".git") {
            let git_url = url.strip_prefix("git+").unwrap_or(url);
            return self.materialize_git_source(git_url, rev, expected_lock_hash);
        }

        if url.starts_with("http://") || url.starts_with("https://") {
            let archive_format = archive_format_from_url(url);
            if let Some(format) = archive_format {
                return self.materialize_archive_source(url, format, expected_lock_hash);
            }
            return Ok(None);
        }

        if url.starts_with("github:") {
            let temp_input = FlakeInput {
                name: "temp-input".to_string(),
                url: url.to_string(),
                follows: None,
                rev: rev.map(|r| r.to_string()),
                branch: None,
                tag: None,
            };
            let resolved = self.resolve_input(&temp_input)?;
            return self.resolve_input_source_root(
                &resolved.url,
                resolved.rev.as_deref(),
                Some(&resolved.hash),
            );
        }

        if url.split('/').count() >= 2 {
            // Fallback: treat "owner/repo[/ref]" as GitHub shorthand.
            // 回退：将 "owner/repo[/ref]" 视为 GitHub 简写。
            let github_url = format!("github:{}", url);
            return self.resolve_input_source_root(&github_url, rev, expected_lock_hash);
        }

        Ok(None)
    }

    /// Materialize a path input and return its canonical source root.
    /// 物化路径输入并返回规范化后的源码根目录。
    fn materialize_path_source(&self, url: &str) -> Result<PathBuf, ConfigError> {
        let raw = url.strip_prefix("path:").unwrap_or(url);
        let raw_path = Path::new(raw);
        let abs_path = if raw_path.is_absolute() {
            raw_path.to_path_buf()
        } else {
            self.root.join(raw_path)
        };
        abs_path
            .canonicalize()
            .map_err(|e| ConfigError::Flake(format!("invalid path input '{}': {}", raw, e)))
    }

    /// Materialize a git input into the source cache directory.
    /// 将 git 输入物化到源码缓存目录。
    fn materialize_git_source(
        &self,
        git_url: &str,
        rev: Option<&str>,
        expected_lock_hash: Option<&str>,
    ) -> Result<Option<PathBuf>, ConfigError> {
        let git_ref = rev.unwrap_or("HEAD");
        let cache_root = flake_source_cache_dir()?;
        let cache_key = expected_lock_hash
            .and_then(lock_hash_to_raw_hex)
            .map(std::borrow::ToOwned::to_owned)
            .unwrap_or_else(|| Hash::of_str(&format!("{}@{}", git_url, git_ref)).to_hex());
        let checkout_dir = cache_root.join(format!("git-{}", cache_key));

        if !checkout_dir.exists() {
            let work_dir = temp_work_dir("neve-flake-input-git")?;
            let clone_path = work_dir.join("repo");

            let repo = fetch_git::clone_repo(git_url, &clone_path).map_err(|e| {
                ConfigError::Flake(format!("failed to clone git input '{}': {}", git_url, e))
            })?;
            fetch_git::checkout_rev(&repo, git_ref).map_err(|e| {
                ConfigError::Flake(format!(
                    "failed to checkout git input '{}' at '{}': {}",
                    git_url, git_ref, e
                ))
            })?;
            drop(repo);

            let git_dir = clone_path.join(".git");
            if git_dir.exists() {
                fs::remove_dir_all(&git_dir)
                    .map_err(|e| ConfigError::Flake(format!("failed to remove .git: {}", e)))?;
            }

            if let Some(expected_hash) = parse_lock_hash(expected_lock_hash) {
                let actual_hash = fetch_git::hash_directory(&clone_path).map_err(|e| {
                    ConfigError::Flake(format!("failed to hash git input '{}': {}", git_url, e))
                })?;
                if actual_hash != expected_hash {
                    return Err(ConfigError::Flake(format!(
                        "hash mismatch for git input '{}': expected {}, got {}",
                        git_url,
                        hash_to_lock(&expected_hash),
                        hash_to_lock(&actual_hash)
                    )));
                }
            }

            if let Some(parent) = checkout_dir.parent() {
                fs::create_dir_all(parent)?;
            }
            if let Err(err) = fs::rename(&clone_path, &checkout_dir) {
                copy_dir_recursive(&clone_path, &checkout_dir)?;
                let _ = fs::remove_dir_all(&clone_path);
                if checkout_dir.exists() {
                    // Keep copy fallback result and continue.
                    // 保留复制回退结果并继续。
                } else {
                    return Err(ConfigError::Flake(format!(
                        "failed to cache git input '{}': {}",
                        git_url, err
                    )));
                }
            }
            let _ = fs::remove_dir_all(&work_dir);
        }

        Ok(if checkout_dir.join("flake.neve").exists() {
            Some(checkout_dir.canonicalize().unwrap_or(checkout_dir))
        } else {
            None
        })
    }

    /// Materialize an archive input into the source cache directory.
    /// 将归档输入物化到源码缓存目录。
    fn materialize_archive_source(
        &self,
        url: &str,
        format: fetch_archive::ArchiveFormat,
        expected_lock_hash: Option<&str>,
    ) -> Result<Option<PathBuf>, ConfigError> {
        let cache_root = flake_source_cache_dir()?;
        let cache_key = expected_lock_hash
            .and_then(lock_hash_to_raw_hex)
            .map(std::borrow::ToOwned::to_owned)
            .unwrap_or_else(|| Hash::of_str(url).to_hex());
        let extract_dir = cache_root.join(format!("archive-{}", cache_key));

        if !extract_dir.exists() {
            let content = fetch_url::fetch_url(url)
                .map_err(|e| ConfigError::Flake(format!("failed to fetch '{}': {}", url, e)))?;

            let work_dir = temp_work_dir("neve-flake-input-archive")?;
            fetch_archive::extract_from_bytes(&content, &work_dir, format)
                .map_err(|e| ConfigError::Flake(format!("failed to extract '{}': {}", url, e)))?;

            let source_root = detected_archive_root(&work_dir);
            if let Some(expected_hash) = parse_lock_hash(expected_lock_hash) {
                let actual_hash = fetch_verify::hash_dir(&source_root).map_err(|e| {
                    ConfigError::Flake(format!(
                        "failed to hash extracted archive from '{}': {}",
                        url, e
                    ))
                })?;
                if actual_hash != expected_hash {
                    return Err(ConfigError::Flake(format!(
                        "hash mismatch for archive input '{}': expected {}, got {}",
                        url,
                        hash_to_lock(&expected_hash),
                        hash_to_lock(&actual_hash)
                    )));
                }
            }

            if let Some(parent) = extract_dir.parent() {
                fs::create_dir_all(parent)?;
            }
            if let Err(err) = fs::rename(&work_dir, &extract_dir) {
                copy_dir_recursive(&work_dir, &extract_dir)?;
                let _ = fs::remove_dir_all(&work_dir);
                if extract_dir.exists() {
                    // Keep copy fallback result and continue.
                    // 保留复制回退结果并继续。
                } else {
                    return Err(ConfigError::Flake(format!(
                        "failed to cache extracted input '{}': {}",
                        url, err
                    )));
                }
            }
        }

        let root = detected_archive_root(&extract_dir);
        Ok(if root.join("flake.neve").exists() {
            Some(root.canonicalize().unwrap_or(root))
        } else {
            None
        })
    }

    /// Create a fallback input value directly from the raw input URL.
    /// 直接从原始输入 URL 创建回退输入值。
    fn input_url_to_value(&self, url: &str) -> Value {
        let mut fields = HashMap::new();
        let out_path = if is_path_input(url) {
            let raw = url.strip_prefix("path:").unwrap_or(url);
            let raw_path = Path::new(raw);
            let abs = if raw_path.is_absolute() {
                raw_path.to_path_buf()
            } else {
                self.root.join(raw_path)
            };
            abs.canonicalize()
                .unwrap_or(abs)
                .to_string_lossy()
                .to_string()
        } else {
            url.to_string()
        };
        fields.insert("outPath".to_string(), Value::String(Rc::new(out_path)));
        fields.insert("url".to_string(), Value::String(Rc::new(url.to_string())));
        Value::Record(Rc::new(fields))
    }

    /// Convert a lock entry to an input value record exposed to outputs.
    /// 将 lock 条目转换为暴露给 outputs 的输入值记录。
    fn lock_entry_to_value(&self, entry: &FlakeLockEntry) -> Value {
        let mut fields = HashMap::new();

        let out_path = if let Some(raw) = entry.url.strip_prefix("path:") {
            let raw_path = Path::new(raw);
            let abs = if raw_path.is_absolute() {
                raw_path.to_path_buf()
            } else {
                self.root.join(raw_path)
            };
            abs.canonicalize()
                .unwrap_or(abs)
                .to_string_lossy()
                .to_string()
        } else {
            entry.url.clone()
        };

        fields.insert("outPath".to_string(), Value::String(Rc::new(out_path)));
        fields.insert("url".to_string(), Value::String(Rc::new(entry.url.clone())));
        fields.insert(
            "narHash".to_string(),
            Value::String(Rc::new(entry.hash.clone())),
        );
        fields.insert(
            "lastModified".to_string(),
            Value::Int(entry.last_modified.into()),
        );

        if let Some(rev) = &entry.rev {
            fields.insert("rev".to_string(), Value::String(Rc::new(rev.clone())));
        }

        Value::Record(Rc::new(fields))
    }

    /// Parse outputs from a value.
    /// 从值解析输出。
    fn parse_outputs(&self, value: &Value) -> Result<HashMap<String, FlakeOutput>, ConfigError> {
        let mut outputs = HashMap::new();

        if let Value::Record(fields) = value {
            for (name, val) in fields.iter() {
                let output = match name.as_str() {
                    "packages" => FlakeOutput::Package(val.clone()),
                    "devShells" | "devShell" => FlakeOutput::DevShell(val.clone()),
                    "nixosConfigurations" | "neveConfigurations" => {
                        FlakeOutput::System(val.clone())
                    }
                    "homeConfigurations" => FlakeOutput::HomeConfig(val.clone()),
                    "overlays" => FlakeOutput::Overlay(val.clone()),
                    "nixosModules" | "neveModules" => FlakeOutput::Module(val.clone()),
                    "templates" => FlakeOutput::Template(val.clone()),
                    _ => FlakeOutput::Other(val.clone()),
                };
                outputs.insert(name.clone(), output);
            }
        }

        Ok(outputs)
    }

    /// Convert flake to a Value (for self reference).
    /// 将 flake 转换为 Value（用于自引用）。
    fn to_value(&self) -> Value {
        let mut fields = HashMap::new();

        if let Some(ref desc) = self.description {
            fields.insert(
                "description".to_string(),
                Value::String(Rc::new(desc.clone())),
            );
        }

        // Add source path
        // 添加源路径
        fields.insert(
            "outPath".to_string(),
            Value::String(Rc::new(self.root.to_string_lossy().to_string())),
        );

        Value::Record(Rc::new(fields))
    }

    /// Get a package by name.
    /// 按名称获取包。
    pub fn get_package(&mut self, system: &str, name: &str) -> Result<Option<Value>, ConfigError> {
        let outputs = self.eval_outputs()?;
        let normalized_system = system.replace('-', "_");

        if let Some(FlakeOutput::Package(Value::Record(systems))) = outputs.get("packages")
            && let Some(Value::Record(pkgs)) = systems
                .get(system)
                .or_else(|| systems.get(normalized_system.as_str()))
        {
            return Ok(pkgs.get(name).cloned());
        }

        Ok(None)
    }

    /// Get the default package for a system.
    /// 获取系统的默认包。
    pub fn get_default_package(&mut self, system: &str) -> Result<Option<Value>, ConfigError> {
        self.get_package(system, "default")
    }

    /// Get a dev shell by name.
    /// 按名称获取开发 shell。
    pub fn get_dev_shell(
        &mut self,
        system: &str,
        name: &str,
    ) -> Result<Option<Value>, ConfigError> {
        let outputs = self.eval_outputs()?;
        let normalized_system = system.replace('-', "_");

        if let Some(FlakeOutput::DevShell(Value::Record(systems))) = outputs.get("devShells")
            && let Some(Value::Record(shell_map)) = systems
                .get(system)
                .or_else(|| systems.get(normalized_system.as_str()))
        {
            return Ok(shell_map.get(name).cloned());
        }

        Ok(None)
    }
}

fn flake_from_value(value: Value, root: PathBuf) -> Result<Flake, ConfigError> {
    let mut flake = Flake::new(root);

    if let Value::Record(fields) = value {
        if let Some(Value::String(desc)) = fields.get("description") {
            flake.description = Some(desc.to_string());
        }

        if let Some(Value::Record(inputs)) = fields.get("inputs") {
            for (name, input_value) in inputs.iter() {
                let input = FlakeInput::from_value(name, input_value)?;
                flake.inputs.insert(name.clone(), input);
            }
        }

        if let Some(outputs) = fields.get("outputs") {
            flake.outputs = Some(outputs.clone());
        }
    }

    Ok(flake)
}

fn eval_flake_source_via_frontend(source: &str) -> Result<Value, ConfigError> {
    let analysis = analyze_source(source);
    ensure_single_flake_has_no_errors(&analysis.diagnostics)?;
    Evaluator::new()
        .eval_evaluable_module(EvaluableModuleRef::new(
            &analysis.hir,
            &analysis.semantics.method_resolutions,
        ))
        .map_err(|e| ConfigError::Eval(format!("{:?}", e)))
}

fn eval_flake_file_via_frontend(path: &Path) -> Result<Value, ConfigError> {
    let (root_dir, module_path) = resolve_source_module_path(path)?;
    let analysis = FrontendDriver::new(&root_dir)
        .analyze_module_path(&module_path)
        .map_err(|err| ConfigError::Flake(format!("frontend error: {err}")))?;

    ensure_flake_program_has_no_errors(&analysis)?;
    let (_, root_value) = build_program_evaluator_and_root_value(&analysis)?;
    Ok(root_value)
}

fn build_program_evaluator_and_root_value(
    analysis: &ProgramAnalysis,
) -> Result<(Evaluator, Value), ConfigError> {
    let mut evaluator = Evaluator::new();
    let modules = analysis.evaluable_modules_in_order();
    let root_value = evaluator
        .eval_evaluable_modules(
            modules
                .iter()
                .map(|entry| EvaluableModuleRef::new(&entry.module, &entry.method_resolutions)),
            analysis.root_module_id(),
        )
        .map_err(|e| ConfigError::Eval(format!("{:?}", e)))?;

    Ok((evaluator, root_value))
}

fn resolve_source_module_path(path: &Path) -> Result<(PathBuf, Vec<String>), ConfigError> {
    let canonical = path.canonicalize().map_err(|e| {
        ConfigError::Flake(format!(
            "failed to canonicalize flake path '{}': {}",
            path.display(),
            e
        ))
    })?;

    let mut root_dir = canonical
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));

    let mut rel_path = canonical.clone();
    let mut saw_src = false;
    for ancestor in canonical.ancestors() {
        if ancestor.file_name().is_some_and(|name| name == "src") {
            if let Some(parent) = ancestor.parent() {
                root_dir = parent.to_path_buf();
                rel_path = canonical
                    .strip_prefix(ancestor)
                    .unwrap_or(&canonical)
                    .to_path_buf();
                saw_src = true;
            }
            break;
        }
    }

    if !saw_src {
        rel_path = canonical
            .strip_prefix(&root_dir)
            .unwrap_or(&canonical)
            .to_path_buf();
    }

    let mut segments: Vec<String> = rel_path
        .components()
        .map(|component| {
            let part = component.as_os_str().to_string_lossy().to_string();
            if part.ends_with(".neve") {
                part.trim_end_matches(".neve").to_string()
            } else {
                part
            }
        })
        .collect();

    if segments.last().map(|segment| segment.as_str()) == Some("mod") {
        segments.pop();
    }

    if segments.len() == 1 && segments[0] == "lib" {
        segments.clear();
    }

    Ok((root_dir, segments))
}

fn ensure_single_flake_has_no_errors(diagnostics: &[Diagnostic]) -> Result<(), ConfigError> {
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .collect();
    if errors.is_empty() {
        return Ok(());
    }

    Err(ConfigError::Flake(format!(
        "frontend diagnostics:\n{}",
        errors
            .iter()
            .map(|diagnostic| diagnostic.message.clone())
            .collect::<Vec<_>>()
            .join("\n")
    )))
}

fn ensure_flake_program_has_no_errors(analysis: &ProgramAnalysis) -> Result<(), ConfigError> {
    if !analysis.has_blocking_diagnostics() {
        return Ok(());
    }

    let errors = analysis.blocking_diagnostic_messages();

    if errors.is_empty() {
        return Ok(());
    }

    Err(ConfigError::Flake(format!(
        "frontend diagnostics:\n{}",
        errors.join("\n")
    )))
}

fn extract_outputs_value(root_value: &Value) -> Result<Value, ConfigError> {
    let Value::Record(fields) = root_value else {
        return Err(ConfigError::Flake(
            "flake file must evaluate to a record".to_string(),
        ));
    };

    fields
        .get("outputs")
        .cloned()
        .ok_or_else(|| ConfigError::Flake("flake has no outputs".into()))
}

/// Initialize a new flake in a directory.
/// 在目录中初始化新的 flake。
pub fn init_flake(root: &Path, description: Option<&str>) -> Result<Flake, ConfigError> {
    std::fs::create_dir_all(root)?;

    let flake_content = format!(
        r#"let flake = #{{
    description = "{}",

    inputs = #{{
        neve = #{{
            url = "github:example/neve"
        }}
    }},

    outputs = fn(inputs) #{{
        packages = #{{
            x86_64_linux = #{{
                default = inputs.neve.packages.x86_64_linux.hello
            }}
        }}
    }}
}};
"#,
        description.unwrap_or("A Neve flake")
    );

    std::fs::write(root.join("flake.neve"), flake_content)?;

    Flake::load(root)
}

/// Resolve a git source to a concrete commit and content hash.
/// 将 git 源解析为具体提交和内容哈希。
fn resolve_git_source(url: &str, git_ref: &str) -> Result<(String, Hash), ConfigError> {
    let work_dir = temp_work_dir("neve-flake-git")?;
    let clone_path = work_dir.join("repo");

    let repo = fetch_git::clone_repo(url, &clone_path)
        .map_err(|e| ConfigError::Flake(format!("failed to clone '{}': {}", url, e)))?;
    let oid = fetch_git::checkout_rev(&repo, git_ref).map_err(|e| {
        ConfigError::Flake(format!(
            "failed to checkout revision '{}' from '{}': {}",
            git_ref, url, e
        ))
    })?;
    drop(repo);

    let git_dir = clone_path.join(".git");
    if git_dir.exists() {
        fs::remove_dir_all(&git_dir)
            .map_err(|e| ConfigError::Flake(format!("failed to remove .git: {}", e)))?;
    }

    let hash = fetch_git::hash_directory(&clone_path)
        .map_err(|e| ConfigError::Flake(format!("failed to hash git source '{}': {}", url, e)))?;
    let _ = fs::remove_dir_all(&work_dir);
    Ok((oid.to_string(), hash))
}

/// Format a content hash for lock file storage.
/// 将内容哈希格式化为锁文件存储格式。
fn hash_to_lock(hash: &Hash) -> String {
    format!("blake3-{}", hash.to_hex())
}

/// Produce a deterministic lastModified value from a content hash.
/// 基于内容哈希生成确定性的 lastModified 值。
fn stable_last_modified_from_hash(hash: &Hash) -> u64 {
    let hex = hash.to_hex();
    let prefix = hex.get(..16).unwrap_or(hex.as_str());
    u64::from_str_radix(prefix, 16).unwrap_or(0)
}

/// Parse a lock hash string like `blake3-<hex>` into a `Hash`.
/// 将 `blake3-<hex>` 形式的锁哈希解析为 `Hash`。
fn parse_lock_hash(lock_hash: Option<&str>) -> Option<Hash> {
    let lock_hash = lock_hash?;
    let raw = lock_hash_to_raw_hex(lock_hash)?;
    Hash::from_hex(raw).ok()
}

/// Strip lock hash prefix and return raw hex content.
/// 去除锁哈希前缀并返回原始十六进制内容。
fn lock_hash_to_raw_hex(lock_hash: &str) -> Option<&str> {
    let raw = lock_hash.strip_prefix("blake3-").unwrap_or(lock_hash);
    if raw.len() == 64 && raw.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(raw)
    } else {
        None
    }
}

/// Return whether a URL is a local path input.
/// 判断 URL 是否为本地路径输入。
fn is_path_input(url: &str) -> bool {
    url.starts_with("path:")
        || url.starts_with("./")
        || url.starts_with("../")
        || url.starts_with('/')
}

/// Detect archive format from a URL path.
/// 从 URL 路径检测归档格式。
fn archive_format_from_url(url: &str) -> Option<fetch_archive::ArchiveFormat> {
    let without_query = url.split('?').next().unwrap_or(url);
    let file_name = Path::new(without_query).file_name()?.to_str()?;
    fetch_archive::ArchiveFormat::from_name(file_name)
}

/// Determine archive extraction root and skip single top-level wrapper directory when present.
/// 确定归档解压根目录，并在存在单一顶层包装目录时跳过它。
fn detected_archive_root(root: &Path) -> PathBuf {
    if root.join("flake.neve").exists() {
        return root.to_path_buf();
    }

    let Ok(entries) = fs::read_dir(root) else {
        return root.to_path_buf();
    };

    let dirs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            entry
                .file_type()
                .ok()
                .filter(|t| t.is_dir())
                .map(|_| entry.path())
        })
        .collect();

    if dirs.len() == 1 {
        return dirs[0].clone();
    }

    root.to_path_buf()
}

/// Get the shared cache directory for materialized flake input sources.
/// 获取用于物化 flake 输入源码的共享缓存目录。
fn flake_source_cache_dir() -> Result<PathBuf, ConfigError> {
    let path = std::env::temp_dir().join("neve-flake-source-cache");
    fs::create_dir_all(&path)
        .map_err(|e| ConfigError::Flake(format!("failed to create source cache dir: {}", e)))?;
    Ok(path)
}

/// Recursively copy a directory.
/// 递归复制目录。
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), ConfigError> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Create a temporary working directory.
/// 创建临时工作目录。
fn temp_work_dir(prefix: &str) -> Result<PathBuf, ConfigError> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let path = std::env::temp_dir().join(format!("{}-{}-{}", prefix, std::process::id(), nonce));
    fs::create_dir_all(&path).map_err(|e| {
        ConfigError::Flake(format!(
            "failed to create temporary directory '{}': {}",
            prefix, e
        ))
    })?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_eval_outputs_recursively_loads_path_inputs() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempdir()?;
        let root = temp.path();

        let core_dir = root.join("core");
        fs::create_dir_all(&core_dir)?;
        fs::write(
            core_dir.join("flake.neve"),
            r#"let flake = #{
    outputs = fn(inputs) #{
        packages = #{
            x86_64_linux = #{
                default = #{
                    name = "core",
                    version = "1.0.0"
                }
            }
        }
    }
};
"#,
        )?;

        let dep_dir = root.join("dep");
        fs::create_dir_all(&dep_dir)?;
        fs::write(
            dep_dir.join("flake.neve"),
            r#"let flake = #{
    inputs = #{
        core = #{ url = "../core" }
    },

    outputs = fn(inputs) #{
        packages = #{
            x86_64_linux = #{
                default = inputs.core.packages.x86_64_linux.default
            }
        }
    }
};
"#,
        )?;

        fs::write(
            root.join("flake.neve"),
            r#"let flake = #{
    inputs = #{
        dep = #{ url = "./dep" }
    },

    outputs = fn(inputs) #{
        packages = #{
            x86_64_linux = #{
                default = inputs.dep.packages.x86_64_linux.default
            }
        }
    }
};
"#,
        )?;

        let mut flake = Flake::load(root)?;
        let default_pkg = flake
            .get_default_package("x86_64-linux")?
            .ok_or("missing default package")?;

        match default_pkg {
            Value::Record(fields) => {
                let Some(Value::String(name)) = fields.get("name") else {
                    return Err("missing package name".into());
                };
                assert_eq!(name.as_str(), "core");
            }
            _ => return Err("expected package record".into()),
        }

        Ok(())
    }

    #[test]
    fn test_eval_outputs_supports_follows_alias() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempdir()?;
        let root = temp.path();

        let dep_dir = root.join("dep");
        fs::create_dir_all(&dep_dir)?;
        fs::write(
            dep_dir.join("flake.neve"),
            r#"let flake = #{
    outputs = fn(inputs) #{
        packages = #{
            x86_64_linux = #{
                default = #{
                    name = "dep",
                    version = "3.1.0"
                }
            }
        }
    }
};
"#,
        )?;

        fs::write(
            root.join("flake.neve"),
            r#"let flake = #{
    inputs = #{
        base = #{ url = "./dep" },
        alias = #{ url = "./dep", follows = "base" }
    },

    outputs = fn(inputs) #{
        packages = #{
            x86_64_linux = #{
                default = inputs.alias.packages.x86_64_linux.default
            }
        }
    }
};
"#,
        )?;

        let mut flake = Flake::load(root)?;
        let default_pkg = flake
            .get_default_package("x86_64-linux")?
            .ok_or("missing default package")?;

        match default_pkg {
            Value::Record(fields) => {
                let Some(Value::String(name)) = fields.get("name") else {
                    return Err("missing package name".into());
                };
                assert_eq!(name.as_str(), "dep");
            }
            _ => return Err("expected package record".into()),
        }

        Ok(())
    }

    #[test]
    fn test_flake_load_supports_language_imports_via_frontend_hir()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempdir()?;
        let root = temp.path();

        fs::write(
            root.join("helpers.neve"),
            r#"fn package(name) = #{
    name = name,
    version = "3.1.0"
};"#,
        )?;

        fs::write(
            root.join("flake.neve"),
            r#"use helpers (package);

let flake = #{
    outputs = fn(inputs) #{
        packages = #{
            x86_64_linux = #{
                default = package("frontend")
            }
        }
    }
};
"#,
        )?;

        let mut flake = Flake::load(root)?;
        let default_pkg = flake
            .get_default_package("x86_64-linux")?
            .ok_or("missing default package")?;

        match default_pkg {
            Value::Record(fields) => {
                let Some(Value::String(name)) = fields.get("name") else {
                    return Err("missing package name".into());
                };
                assert_eq!(name.as_str(), "frontend");
            }
            _ => return Err("expected package record".into()),
        }

        Ok(())
    }

    #[test]
    fn test_flake_load_reports_frontend_type_errors() -> Result<(), Box<dyn std::error::Error>> {
        let temp = tempdir()?;
        let root = temp.path();

        fs::write(root.join("flake.neve"), "let flake = 1 + true;")?;

        let err = Flake::load(root).expect_err("type error should be surfaced");
        let message = err.to_string();
        assert!(message.contains("frontend diagnostics"));
        assert!(message.contains("flake.neve"));

        Ok(())
    }

    #[test]
    fn test_lock_to_json_is_deterministic_for_input_order() {
        let mut lock_a = FlakeLock::new();
        lock_a.inputs.insert(
            "b".to_string(),
            FlakeLockEntry {
                name: "b".to_string(),
                url: "https://example.com/b.tar.gz".to_string(),
                hash: "blake3-2222".to_string(),
                last_modified: 2,
                rev: None,
            },
        );
        lock_a.inputs.insert(
            "a".to_string(),
            FlakeLockEntry {
                name: "a".to_string(),
                url: "https://example.com/a.tar.gz".to_string(),
                hash: "blake3-1111".to_string(),
                last_modified: 1,
                rev: Some("rev-a".to_string()),
            },
        );

        let mut lock_b = FlakeLock::new();
        lock_b.inputs.insert(
            "a".to_string(),
            FlakeLockEntry {
                name: "a".to_string(),
                url: "https://example.com/a.tar.gz".to_string(),
                hash: "blake3-1111".to_string(),
                last_modified: 1,
                rev: Some("rev-a".to_string()),
            },
        );
        lock_b.inputs.insert(
            "b".to_string(),
            FlakeLockEntry {
                name: "b".to_string(),
                url: "https://example.com/b.tar.gz".to_string(),
                hash: "blake3-2222".to_string(),
                last_modified: 2,
                rev: None,
            },
        );

        assert_eq!(lock_a.to_json(), lock_b.to_json());
    }

    #[test]
    fn test_stable_last_modified_from_hash_is_deterministic() {
        let hash = Hash::of(b"same-content");
        let ts1 = stable_last_modified_from_hash(&hash);
        let ts2 = stable_last_modified_from_hash(&hash);
        assert_eq!(ts1, ts2);
    }
}
