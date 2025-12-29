//! Flake support for Neve.
//!
//! Flakes provide a standard way to define reproducible Neve projects
//! with explicit dependencies and outputs.
//!
//! A flake is defined by a `flake.neve` file in the project root that exports:
//! - `inputs`: Dependencies on other flakes
//! - `outputs`: A function that produces packages, configurations, etc.

use crate::ConfigError;
use neve_eval::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// A flake input specification.
#[derive(Debug, Clone)]
pub struct FlakeInput {
    /// Input name.
    pub name: String,
    /// Input URL or path.
    pub url: String,
    /// Whether to follow another input's version.
    pub follows: Option<String>,
    /// Specific revision/commit.
    pub rev: Option<String>,
    /// Specific branch.
    pub branch: Option<String>,
    /// Specific tag.
    pub tag: Option<String>,
}

impl FlakeInput {
    /// Create a new flake input.
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
    pub fn follows(mut self, other: impl Into<String>) -> Self {
        self.follows = Some(other.into());
        self
    }

    /// Set a specific revision.
    pub fn rev(mut self, rev: impl Into<String>) -> Self {
        self.rev = Some(rev.into());
        self
    }

    /// Set a specific branch.
    pub fn branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }

    /// Set a specific tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }

    /// Parse from a Value.
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
#[derive(Debug, Clone)]
pub enum FlakeOutput {
    /// A package derivation.
    Package(Value),
    /// A development shell.
    DevShell(Value),
    /// A NixOS/Neve system configuration.
    System(Value),
    /// A home-manager configuration.
    HomeConfig(Value),
    /// An overlay.
    Overlay(Value),
    /// A Neve module.
    Module(Value),
    /// A template.
    Template(Value),
    /// A generic output.
    Other(Value),
}

/// A flake lock entry.
#[derive(Debug, Clone)]
pub struct FlakeLockEntry {
    /// Input name.
    pub name: String,
    /// Resolved URL.
    pub url: String,
    /// Content hash.
    pub hash: String,
    /// Last modified timestamp.
    pub last_modified: u64,
    /// Revision (for git sources).
    pub rev: Option<String>,
}

/// A flake lock file.
#[derive(Debug, Clone, Default)]
pub struct FlakeLock {
    /// Version of the lock file format.
    pub version: u32,
    /// Locked inputs.
    pub inputs: HashMap<String, FlakeLockEntry>,
}

impl FlakeLock {
    /// Create a new empty lock file.
    pub fn new() -> Self {
        Self {
            version: 1,
            inputs: HashMap::new(),
        }
    }

    /// Load a lock file from disk.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Parse a lock file from JSON.
    pub fn parse(content: &str) -> Result<Self, ConfigError> {
        // Simple JSON parsing for lock file
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
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        let content = self.to_json();
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Convert to JSON string.
    pub fn to_json(&self) -> String {
        let mut nodes = serde_json::Map::new();

        // Root node
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
#[derive(Debug)]
pub struct Flake {
    /// Flake root directory.
    pub root: PathBuf,
    /// Flake description.
    pub description: Option<String>,
    /// Flake inputs.
    pub inputs: HashMap<String, FlakeInput>,
    /// The outputs function (as a Value).
    pub outputs: Option<Value>,
    /// Resolved inputs (after locking).
    pub resolved_inputs: HashMap<String, Value>,
    /// Lock file.
    pub lock: FlakeLock,
}

impl Flake {
    /// Create a new empty flake.
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
    pub fn load(root: &Path) -> Result<Self, ConfigError> {
        let flake_file = root.join("flake.neve");
        if !flake_file.exists() {
            return Err(ConfigError::Flake(format!(
                "no flake.neve found in {}",
                root.display()
            )));
        }

        // Parse the flake file
        let source = std::fs::read_to_string(&flake_file)?;
        let mut flake = Self::parse(&source, root.to_path_buf())?;

        // Try to load lock file
        let lock_file = root.join("flake.lock");
        if lock_file.exists() {
            flake.lock = FlakeLock::load(&lock_file)?;
        }

        Ok(flake)
    }

    /// Parse a flake from source.
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
        if let Value::Record(fields) = value {
            // Description
            if let Some(Value::String(desc)) = fields.get("description") {
                flake.description = Some(desc.to_string());
            }

            // Inputs
            if let Some(Value::Record(inputs)) = fields.get("inputs") {
                for (name, input_value) in inputs.iter() {
                    let input = FlakeInput::from_value(name, input_value)?;
                    flake.inputs.insert(name.clone(), input);
                }
            }

            // Outputs
            if let Some(outputs) = fields.get("outputs") {
                flake.outputs = Some(outputs.clone());
            }
        }

        Ok(flake)
    }

    /// Lock the flake inputs.
    pub fn lock_inputs(&mut self) -> Result<(), ConfigError> {
        // For each input, resolve it and add to the lock file
        for (name, input) in &self.inputs {
            // Skip if already locked and not updated
            if self.lock.inputs.contains_key(name) {
                continue;
            }

            // Resolve the input
            let entry = self.resolve_input(input)?;
            self.lock.inputs.insert(name.clone(), entry);
        }

        Ok(())
    }

    /// Resolve a single input.
    fn resolve_input(&self, input: &FlakeInput) -> Result<FlakeLockEntry, ConfigError> {
        // Parse the URL to determine the type
        let url = &input.url;

        // For now, create a placeholder entry
        // In a real implementation, this would fetch the input and compute its hash
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
    pub fn save_lock(&self) -> Result<(), ConfigError> {
        let lock_file = self.root.join("flake.lock");
        self.lock.save(&lock_file)
    }

    /// Evaluate the flake outputs.
    pub fn eval_outputs(&mut self) -> Result<HashMap<String, FlakeOutput>, ConfigError> {
        let outputs_fn = self
            .outputs
            .clone()
            .ok_or_else(|| ConfigError::Flake("flake has no outputs".into()))?;

        // Create the inputs record to pass to the outputs function
        let mut inputs_record = HashMap::new();
        inputs_record.insert("self".to_string(), self.to_value());

        for name in self.inputs.keys() {
            // For now, create placeholder values for inputs
            // In a real implementation, this would load the locked input
            if let Some(resolved) = self.resolved_inputs.get(name) {
                inputs_record.insert(name.clone(), resolved.clone());
            } else {
                inputs_record.insert(name.clone(), Value::Record(Rc::new(HashMap::new())));
            }
        }

        // Call the outputs function with inputs
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
        self.parse_outputs(&result)
    }

    /// Parse outputs from a value.
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
    fn to_value(&self) -> Value {
        let mut fields = HashMap::new();

        if let Some(ref desc) = self.description {
            fields.insert(
                "description".to_string(),
                Value::String(Rc::new(desc.clone())),
            );
        }

        // Add source path
        fields.insert(
            "outPath".to_string(),
            Value::String(Rc::new(self.root.to_string_lossy().to_string())),
        );

        Value::Record(Rc::new(fields))
    }

    /// Get a package by name.
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
    pub fn get_default_package(&mut self, system: &str) -> Result<Option<Value>, ConfigError> {
        self.get_package(system, "default")
    }

    /// Get a dev shell by name.
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
