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
use neve_eval::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

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

        // Root node
        // 根节点
        let mut root_inputs = serde_json::Map::new();
        for name in self.inputs.keys() {
            root_inputs.insert(name.clone(), serde_json::Value::String(name.clone()));
        }
        nodes.insert(
            "root".to_string(),
            serde_json::json!({
                "inputs": root_inputs
            }),
        );

        // Input nodes
        // 输入节点
        for (name, entry) in &self.inputs {
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
                name.clone(),
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

        // Parse the flake file
        // 解析 flake 文件
        let source = std::fs::read_to_string(&flake_file)?;
        let mut flake = Self::parse(&source, root.to_path_buf())?;

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
        use neve_eval::AstEvaluator;
        use neve_lexer::Lexer;
        use neve_parser::Parser;

        let lexer = Lexer::new(source);
        let (tokens, lex_errors) = lexer.tokenize();
        if !lex_errors.is_empty() {
            return Err(ConfigError::Flake(format!(
                "lexer errors: {:?}",
                lex_errors
            )));
        }

        let mut parser = Parser::new(tokens);
        let ast = parser.parse_file();

        let mut evaluator = AstEvaluator::new();
        evaluator = evaluator.with_base_path(root.clone());

        let value = evaluator
            .eval_file(&ast)
            .map_err(|e| ConfigError::Eval(format!("{:?}", e)))?;

        let mut flake = Self::new(root);

        // Extract flake structure from evaluated value
        // 从评估的值中提取 flake 结构
        if let Value::Record(fields) = value {
            // Description / 描述
            if let Some(Value::String(desc)) = fields.get("description") {
                flake.description = Some(desc.to_string());
            }

            // Inputs / 输入
            if let Some(Value::Record(inputs)) = fields.get("inputs") {
                for (name, input_value) in inputs.iter() {
                    let input = FlakeInput::from_value(name, input_value)?;
                    flake.inputs.insert(name.clone(), input);
                }
            }

            // Outputs / 输出
            if let Some(outputs) = fields.get("outputs") {
                flake.outputs = Some(outputs.clone());
            }
        }

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
        // Parse the URL to determine the type
        // 解析 URL 以确定类型
        let url = &input.url;

        // For now, create a placeholder entry
        // 目前，创建一个占位条目
        // In a real implementation, this would fetch the input and compute its hash
        // 在实际实现中，这会获取输入并计算其哈希
        let hash = format!("sha256-placeholder-{}", input.name);
        let last_modified = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Ok(FlakeLockEntry {
            name: input.name.clone(),
            url: url.clone(),
            hash,
            last_modified,
            rev: input.rev.clone(),
        })
    }

    /// Save the lock file.
    /// 保存锁定文件。
    pub fn save_lock(&self) -> Result<(), ConfigError> {
        let lock_file = self.root.join("flake.lock");
        self.lock.save(&lock_file)
    }

    /// Evaluate the flake outputs.
    /// 评估 flake 输出。
    pub fn eval_outputs(&mut self) -> Result<HashMap<String, FlakeOutput>, ConfigError> {
        let outputs_fn = self
            .outputs
            .clone()
            .ok_or_else(|| ConfigError::Flake("flake has no outputs".into()))?;

        // Create the inputs record to pass to the outputs function
        // 创建要传递给输出函数的输入记录
        let mut inputs_record = HashMap::new();
        inputs_record.insert("self".to_string(), self.to_value());

        for name in self.inputs.keys() {
            // For now, create placeholder values for inputs
            // 目前，为输入创建占位值
            // In a real implementation, this would load the locked input
            // 在实际实现中，这会加载锁定的输入
            if let Some(resolved) = self.resolved_inputs.get(name) {
                inputs_record.insert(name.clone(), resolved.clone());
            } else {
                inputs_record.insert(name.clone(), Value::Record(Rc::new(HashMap::new())));
            }
        }

        // Call the outputs function with inputs
        // 使用输入调用输出函数
        let result = match outputs_fn {
            Value::AstClosure(ref closure) => {
                use neve_eval::AstEvaluator;
                let mut eval = AstEvaluator::new();
                eval = eval.with_base_path(self.root.clone());

                let inputs_value = Value::Record(Rc::new(inputs_record));
                eval.call_closure(closure, vec![inputs_value])
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

        if let Some(FlakeOutput::Package(Value::Record(systems))) = outputs.get("packages")
            && let Some(Value::Record(pkgs)) = systems.get(system)
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

        if let Some(FlakeOutput::DevShell(Value::Record(systems))) = outputs.get("devShells")
            && let Some(Value::Record(shell_map)) = systems.get(system)
        {
            return Ok(shell_map.get(name).cloned());
        }

        Ok(None)
    }
}

/// Initialize a new flake in a directory.
/// 在目录中初始化新的 flake。
pub fn init_flake(root: &Path, description: Option<&str>) -> Result<Flake, ConfigError> {
    std::fs::create_dir_all(root)?;

    let flake_content = format!(
        r#"{{
    description = "{}";
    
    inputs = {{
        neve = {{
            url = "github:example/neve";
        }};
    }};
    
    outputs = \inputs -> {{
        packages = {{
            x86_64-linux = {{
                default = inputs.neve.packages.x86_64-linux.hello;
            }};
        }};
    }};
}}
"#,
        description.unwrap_or("A Neve flake")
    );

    std::fs::write(root.join("flake.neve"), flake_content)?;

    Flake::load(root)
}
