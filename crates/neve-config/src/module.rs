//! Configuration module system.
//! 配置模块系统。
//!
//! Modules are the building blocks of Neve configurations.
//! They can define options, imports, and configuration logic.
//!
//! 模块是 Neve 配置的构建块。
//! 它们可以定义选项、导入和配置逻辑。

use crate::{ConfigError, SystemConfig};
use neve_eval::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A configuration module.
/// 配置模块。
#[derive(Debug, Clone)]
pub struct Module {
    /// Module name. / 模块名称。
    pub name: String,
    /// Module file path. / 模块文件路径。
    pub path: Option<PathBuf>,
    /// Module imports. / 模块导入。
    pub imports: Vec<String>,
    /// Module options (declared). / 模块选项（已声明）。
    pub options: Vec<OptionDecl>,
    /// Module configuration (values). / 模块配置（值）。
    pub config: HashMap<String, Value>,
}

/// An option declaration.
/// 选项声明。
#[derive(Debug, Clone)]
pub struct OptionDecl {
    /// Option name. / 选项名称。
    pub name: String,
    /// Option type. / 选项类型。
    pub ty: OptionType,
    /// Default value. / 默认值。
    pub default: Option<Value>,
    /// Description. / 描述。
    pub description: Option<String>,
    /// Example value. / 示例值。
    pub example: Option<String>,
}

/// Option types.
/// 选项类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionType {
    /// Boolean. / 布尔值。
    Bool,
    /// Integer. / 整数。
    Int,
    /// String. / 字符串。
    String,
    /// Path. / 路径。
    Path,
    /// List of elements. / 元素列表。
    List(Box<OptionType>),
    /// Record with fields. / 带字段的记录。
    Record(Vec<(String, OptionType)>),
    /// Enumeration of values. / 值的枚举。
    Enum(Vec<String>),
    /// Any type. / 任意类型。
    Any,
}

impl Module {
    /// Create a new module.
    /// 创建新的模块。
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: None,
            imports: Vec::new(),
            options: Vec::new(),
            config: HashMap::new(),
        }
    }

    /// Load a module from a file.
    /// 从文件加载模块。
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content, Some(path.to_path_buf()))
    }

    /// Parse a module from source.
    /// 从源码解析模块。
    pub fn parse(source: &str, path: Option<PathBuf>) -> Result<Self, ConfigError> {
        use neve_eval::AstEvaluator;
        use neve_lexer::Lexer;
        use neve_parser::Parser;

        let lexer = Lexer::new(source);
        let (tokens, lex_errors) = lexer.tokenize();
        if !lex_errors.is_empty() {
            return Err(ConfigError::Module(format!(
                "lexer errors: {:?}",
                lex_errors
            )));
        }
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_file();

        let base_path = path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf());

        let mut evaluator = AstEvaluator::new();
        if let Some(bp) = base_path {
            evaluator = evaluator.with_base_path(bp);
        }

        let value = evaluator
            .eval_file(&ast)
            .map_err(|e| ConfigError::Eval(format!("{:?}", e)))?;

        // Extract module structure from evaluated value
        // 从评估的值中提取模块结构
        let mut module = Module::new(
            path.as_ref()
                .and_then(|p| p.file_stem())
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "anonymous".to_string()),
        );
        module.path = path;

        // If the result is a record, extract configuration
        // 如果结果是记录，提取配置
        if let Value::Record(fields) = value {
            for (key, val) in fields.iter() {
                module.config.insert(key.clone(), val.clone());
            }
        }

        Ok(module)
    }

    /// Add an import.
    /// 添加导入。
    pub fn import(mut self, module_path: impl Into<String>) -> Self {
        self.imports.push(module_path.into());
        self
    }

    /// Declare an option.
    /// 声明选项。
    pub fn option(mut self, opt: OptionDecl) -> Self {
        self.options.push(opt);
        self
    }

    /// Set a configuration value.
    /// 设置配置值。
    pub fn set(mut self, key: impl Into<String>, value: Value) -> Self {
        self.config.insert(key.into(), value);
        self
    }

    /// Get a configuration value.
    /// 获取配置值。
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.config.get(key)
    }

    /// Convert to SystemConfig.
    /// 转换为 SystemConfig。
    pub fn to_system_config(&self) -> Result<SystemConfig, ConfigError> {
        let mut config = SystemConfig::new(&self.name);

        // Extract standard options
        // 提取标准选项
        if let Some(Value::String(hostname)) = self.config.get("hostname") {
            config.options.hostname = Some(hostname.to_string());
        }

        if let Some(Value::String(timezone)) = self.config.get("timezone") {
            config.options.timezone = Some(timezone.to_string());
        }

        if let Some(Value::String(locale)) = self.config.get("locale") {
            config.options.locale = Some(locale.to_string());
        }

        if let Some(Value::List(services)) = self.config.get("services") {
            for svc in services.iter() {
                if let Value::String(s) = svc {
                    config.options.services.push(s.to_string());
                }
            }
        }

        if let Some(Value::List(packages)) = self.config.get("packages") {
            for pkg in packages.iter() {
                if let Value::String(p) = pkg {
                    config.options.packages.push(p.to_string());
                }
            }
        }

        if let Some(Value::List(env_list)) = self.config.get("environment") {
            for item in env_list.iter() {
                if let Value::Record(fields) = item
                    && let (Some(Value::String(k)), Some(Value::String(v))) =
                        (fields.get("name"), fields.get("value"))
                {
                    config
                        .options
                        .environment
                        .push((k.to_string(), v.to_string()));
                }
            }
        }

        Ok(config)
    }
}

impl OptionDecl {
    /// Create a new option declaration.
    /// 创建新的选项声明。
    pub fn new(name: impl Into<String>, ty: OptionType) -> Self {
        Self {
            name: name.into(),
            ty,
            default: None,
            description: None,
            example: None,
        }
    }

    /// Set the default value.
    /// 设置默认值。
    pub fn default(mut self, value: Value) -> Self {
        self.default = Some(value);
        self
    }

    /// Set the description.
    /// 设置描述。
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set an example.
    /// 设置示例。
    pub fn example(mut self, ex: impl Into<String>) -> Self {
        self.example = Some(ex.into());
        self
    }
}

/// Merge multiple modules into a single configuration.
/// 将多个模块合并为单个配置。
pub fn merge_modules(modules: &[Module]) -> Result<SystemConfig, ConfigError> {
    let mut merged = SystemConfig::new("merged");

    for module in modules {
        let config = module.to_system_config()?;

        // Merge options
        // 合并选项
        if config.options.hostname.is_some() {
            merged.options.hostname = config.options.hostname;
        }
        if config.options.timezone.is_some() {
            merged.options.timezone = config.options.timezone;
        }
        if config.options.locale.is_some() {
            merged.options.locale = config.options.locale;
        }

        merged.options.services.extend(config.options.services);
        merged.options.packages.extend(config.options.packages);
        merged.options.users.extend(config.options.users);
        merged
            .options
            .environment
            .extend(config.options.environment);
    }

    // Deduplicate
    // 去重
    merged.options.services.sort();
    merged.options.services.dedup();
    merged.options.packages.sort();
    merged.options.packages.dedup();

    Ok(merged)
}
