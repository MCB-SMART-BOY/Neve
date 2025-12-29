//! Derivation definition.
//! 推导定义。
//!
//! A derivation is the fundamental unit of building in Neve. It describes:
//! - What sources to fetch
//! - What dependencies are needed
//! - How to build the package
//! - What outputs are produced
//!
//! 推导是 Neve 构建的基本单元。它描述：
//! - 需要获取哪些源码
//! - 需要哪些依赖
//! - 如何构建包
//! - 产生哪些输出

use crate::{Hash, Hasher, Output, StorePath};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// A derivation describes how to build a package.
/// 推导描述如何构建一个包。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Derivation {
    /// The package name. / 包名称。
    pub name: String,
    /// The package version. / 包版本。
    pub version: String,
    /// The system this derivation is for (e.g., "x86_64-linux"). / 此推导的目标系统（例如 "x86_64-linux"）。
    pub system: String,
    /// The builder executable (store path). / 构建器可执行文件（存储路径）。
    pub builder: String,
    /// Arguments to pass to the builder. / 传递给构建器的参数。
    pub args: Vec<String>,
    /// Environment variables for the build. / 构建的环境变量。
    pub env: BTreeMap<String, String>,
    /// Input derivations (dependencies). / 输入推导（依赖）。
    pub input_drvs: BTreeMap<StorePath, Vec<String>>,
    /// Input sources (already in store). / 输入源（已在存储中）。
    pub input_srcs: Vec<StorePath>,
    /// Outputs produced by this derivation. / 此推导产生的输出。
    pub outputs: BTreeMap<String, Output>,
}

impl Derivation {
    /// Create a new derivation builder.
    /// 创建新的推导构建器。
    pub fn builder(name: impl Into<String>, version: impl Into<String>) -> DerivationBuilder {
        DerivationBuilder::new(name, version)
    }

    /// Compute the hash of this derivation.
    /// 计算此推导的哈希值。
    pub fn hash(&self) -> Hash {
        let mut hasher = Hasher::new();

        // Hash all fields in a deterministic order
        // 按确定性顺序哈希所有字段
        hasher.update_str(&self.name);
        hasher.update_str(&self.version);
        hasher.update_str(&self.system);
        hasher.update_str(&self.builder);

        for arg in &self.args {
            hasher.update_str(arg);
        }

        for (key, value) in &self.env {
            hasher.update_str(key);
            hasher.update_str(value);
        }

        for (path, outputs) in &self.input_drvs {
            hasher.update(path.hash().as_bytes());
            for out in outputs {
                hasher.update_str(out);
            }
        }

        for src in &self.input_srcs {
            hasher.update(src.hash().as_bytes());
        }

        for (name, output) in &self.outputs {
            hasher.update_str(name);
            if let Some(hash) = &output.expected_hash {
                hasher.update(hash.as_bytes());
            }
        }

        hasher.finalize()
    }

    /// Get the store path for this derivation file.
    /// 获取此推导文件的存储路径。
    pub fn drv_path(&self) -> StorePath {
        StorePath::new(self.hash(), format!("{}-{}.drv", self.name, self.version))
    }

    /// Get the output path for the given output name.
    /// 获取给定输出名称的输出路径。
    pub fn output_path(&self, output: &str) -> Option<StorePath> {
        self.outputs.get(output).and_then(|o| o.path.clone())
    }

    /// Get the default output path ("out").
    /// 获取默认输出路径（"out"）。
    pub fn out_path(&self) -> Option<StorePath> {
        self.output_path("out")
    }

    /// Check if this is a fixed-output derivation.
    /// 检查是否为固定输出推导。
    pub fn is_fixed_output(&self) -> bool {
        self.outputs.values().any(|o| o.is_fixed())
    }

    /// Serialize to JSON.
    /// 序列化为 JSON。
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON.
    /// 从 JSON 反序列化。
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Builder for creating derivations.
/// 用于创建推导的构建器。
pub struct DerivationBuilder {
    name: String,
    version: String,
    system: Option<String>,
    builder: Option<String>,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    input_drvs: BTreeMap<StorePath, Vec<String>>,
    input_srcs: Vec<StorePath>,
    outputs: BTreeMap<String, Output>,
}

impl DerivationBuilder {
    /// Create a new derivation builder.
    /// 创建新的推导构建器。
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        let mut outputs = BTreeMap::new();
        outputs.insert("out".to_string(), Output::new("out"));

        Self {
            name: name.into(),
            version: version.into(),
            system: None,
            builder: None,
            args: Vec::new(),
            env: BTreeMap::new(),
            input_drvs: BTreeMap::new(),
            input_srcs: Vec::new(),
            outputs,
        }
    }

    /// Set the target system.
    /// 设置目标系统。
    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Set the builder executable.
    /// 设置构建器可执行文件。
    pub fn builder_path(mut self, builder: impl Into<String>) -> Self {
        self.builder = Some(builder.into());
        self
    }

    /// Add an argument.
    /// 添加参数。
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add multiple arguments.
    /// 添加多个参数。
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Set an environment variable.
    /// 设置环境变量。
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set multiple environment variables.
    /// 设置多个环境变量。
    pub fn envs(mut self, env: std::collections::BTreeMap<String, String>) -> Self {
        self.env.extend(env);
        self
    }

    /// Add an input derivation.
    /// 添加输入推导。
    pub fn input_drv(mut self, drv: StorePath, outputs: Vec<String>) -> Self {
        self.input_drvs.insert(drv, outputs);
        self
    }

    /// Add an input source.
    /// 添加输入源。
    pub fn input_src(mut self, src: StorePath) -> Self {
        self.input_srcs.push(src);
        self
    }

    /// Add an output.
    /// 添加输出。
    pub fn output(mut self, output: Output) -> Self {
        self.outputs.insert(output.name.clone(), output);
        self
    }

    /// Build the derivation.
    /// 构建推导。
    pub fn build(self) -> Derivation {
        Derivation {
            name: self.name,
            version: self.version,
            system: self.system.unwrap_or_else(|| current_system().to_string()),
            builder: self.builder.unwrap_or_else(|| "/bin/sh".to_string()),
            args: self.args,
            env: self.env,
            input_drvs: self.input_drvs,
            input_srcs: self.input_srcs,
            outputs: self.outputs,
        }
    }
}

/// Get the current system identifier.
/// 获取当前系统标识符。
pub fn current_system() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    const ARCH: &str = "x86_64";
    #[cfg(target_arch = "aarch64")]
    const ARCH: &str = "aarch64";
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    const ARCH: &str = "unknown";

    #[cfg(target_os = "linux")]
    const OS: &str = "linux";
    #[cfg(target_os = "macos")]
    const OS: &str = "darwin";
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    const OS: &str = "unknown";

    match (ARCH, OS) {
        ("x86_64", "linux") => "x86_64-linux",
        ("aarch64", "linux") => "aarch64-linux",
        ("x86_64", "darwin") => "x86_64-darwin",
        ("aarch64", "darwin") => "aarch64-darwin",
        _ => "unknown-unknown",
    }
}
