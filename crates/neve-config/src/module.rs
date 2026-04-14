//! Configuration module system.
//! 配置模块系统。
//!
//! Modules are the building blocks of Neve configurations.
//! They can define options, imports, and configuration logic.
//!
//! 模块是 Neve 配置的构建块。
//! 它们可以定义选项、导入和配置逻辑。

use crate::{ConfigError, SystemConfig, UserConfig};
use neve_diagnostic::Severity;
use neve_eval::{Evaluator, Value};
use neve_frontend::{Diagnostic, FrontendDriver, ProgramAnalysis, analyze_source};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Reserved top-level keys with special module meaning.
/// 模块中具有特殊语义的保留顶层键。
const RESERVED_KEYS: [&str; 3] = ["imports", "options", "config"];

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
        let canonical = path.canonicalize().map_err(|e| {
            ConfigError::Module(format!(
                "failed to canonicalize module path '{}': {}",
                path.display(),
                e
            ))
        })?;
        let value = eval_module_file_via_frontend(&canonical)?;
        module_from_value(value, Some(canonical))
    }

    /// Load a module and its imports recursively in deterministic order.
    /// 按确定性顺序递归加载模块及其导入。
    pub fn load_with_imports(path: &Path) -> Result<Vec<Self>, ConfigError> {
        let root = path.canonicalize().map_err(|e| {
            ConfigError::Module(format!(
                "failed to canonicalize module path '{}': {}",
                path.display(),
                e
            ))
        })?;

        let mut visited = HashSet::new();
        let mut stack = Vec::new();
        let mut ordered = Vec::new();
        load_with_imports_recursive(&root, &mut visited, &mut stack, &mut ordered)?;
        Ok(ordered)
    }

    /// Load a module graph and merge it into one SystemConfig.
    /// 加载模块图并合并为一个 SystemConfig。
    pub fn load_merged(path: &Path) -> Result<SystemConfig, ConfigError> {
        let modules = Self::load_with_imports(path)?;
        let root_name = modules
            .last()
            .map(|m| m.name.clone())
            .unwrap_or_else(|| "merged".to_string());
        let mut merged = merge_modules(&modules)?;
        merged.name = root_name;
        Ok(merged)
    }

    /// Parse a module from source.
    /// 从源码解析模块。
    pub fn parse(source: &str, path: Option<PathBuf>) -> Result<Self, ConfigError> {
        let analysis = analyze_source(source);
        ensure_single_module_has_no_errors(&analysis.diagnostics, path.as_deref())?;
        let value = Evaluator::new()
            .eval_module_with_method_resolutions(
                &analysis.hir,
                &analysis.semantics.method_resolutions,
            )
            .map_err(|e| ConfigError::Eval(format!("{:?}", e)))?;

        module_from_value(value, path)
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
        let values = self.validated_config_values()?;
        let mut config = SystemConfig::new(&self.name);

        // Extract standard options
        // 提取标准选项
        if let Some(hostname) = values.get("hostname") {
            config.options.hostname = Some(expect_string(hostname, "hostname")?);
        }

        if let Some(timezone) = values.get("timezone") {
            config.options.timezone = Some(expect_string(timezone, "timezone")?);
        }

        if let Some(locale) = values.get("locale") {
            config.options.locale = Some(expect_string(locale, "locale")?);
        }

        if let Some(services) = values.get("services") {
            config.options.services = parse_string_list(services, "services")?;
        }

        if let Some(packages) = values.get("packages") {
            config.options.packages = parse_string_list(packages, "packages")?;
        }

        if let Some(users) = values.get("users") {
            config.options.users = parse_users(users)?;
        }

        if let Some(environment) = values.get("environment") {
            config.options.environment = parse_environment(environment)?;
        }

        Ok(config)
    }

    /// Validate declared options and return a config map with defaults applied.
    /// 校验已声明选项并返回已应用默认值的配置映射。
    fn validated_config_values(&self) -> Result<HashMap<String, Value>, ConfigError> {
        let mut values = self.config.clone();
        if self.options.is_empty() {
            return Ok(values);
        }

        let mut declared = HashMap::<String, &OptionDecl>::new();
        for opt in &self.options {
            if declared.insert(opt.name.clone(), opt).is_some() {
                return Err(ConfigError::Module(format!(
                    "duplicate option declaration: '{}'",
                    opt.name
                )));
            }
        }

        for (name, opt) in &declared {
            if let Some(value) = values.get(name) {
                if !value_matches_option_type(value, &opt.ty) {
                    return Err(ConfigError::Module(format!(
                        "option '{}' has invalid type: expected {}, got {:?}",
                        name,
                        option_type_name(&opt.ty),
                        value
                    )));
                }
            } else if let Some(default) = &opt.default {
                if !value_matches_option_type(default, &opt.ty) {
                    return Err(ConfigError::Module(format!(
                        "option '{}' default value has invalid type: expected {}, got {:?}",
                        name,
                        option_type_name(&opt.ty),
                        default
                    )));
                }
                values.insert(name.clone(), default.clone());
            }
        }

        for key in values.keys() {
            if !declared.contains_key(key) {
                return Err(ConfigError::Module(format!(
                    "unknown option '{}'; declare it under 'options' first",
                    key
                )));
            }
        }

        Ok(values)
    }
}

fn eval_module_file_via_frontend(path: &Path) -> Result<Value, ConfigError> {
    let (root_dir, module_path) = resolve_source_module_path(path)?;
    let analysis = FrontendDriver::new(&root_dir)
        .analyze_module_path(&module_path)
        .map_err(|err| ConfigError::Module(format!("frontend error: {err}")))?;

    ensure_program_has_no_errors(&analysis)?;
    eval_program_root_value(&analysis)
}

fn resolve_source_module_path(path: &Path) -> Result<(PathBuf, Vec<String>), ConfigError> {
    let canonical = path.canonicalize().map_err(|e| {
        ConfigError::Module(format!(
            "failed to canonicalize module path '{}': {}",
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

fn ensure_single_module_has_no_errors(
    diagnostics: &[Diagnostic],
    path: Option<&Path>,
) -> Result<(), ConfigError> {
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .collect();
    if errors.is_empty() {
        return Ok(());
    }

    let prefix = path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<memory>".to_string());
    Err(ConfigError::Module(format!(
        "frontend diagnostics:\n{}",
        errors
            .iter()
            .map(|diagnostic| format!("{prefix}: {}", diagnostic.message))
            .collect::<Vec<_>>()
            .join("\n")
    )))
}

fn ensure_program_has_no_errors(analysis: &ProgramAnalysis) -> Result<(), ConfigError> {
    let mut errors = Vec::new();

    for module_id in analysis.load_order() {
        let Some(info) = analysis.module_info(*module_id) else {
            continue;
        };

        for diagnostic in analysis.diagnostics(*module_id).unwrap_or(&[]) {
            if diagnostic.severity == Severity::Error {
                errors.push(format!(
                    "{}: {}",
                    info.file_path.display(),
                    diagnostic.message
                ));
            }
        }
    }

    if errors.is_empty() {
        return Ok(());
    }

    Err(ConfigError::Module(format!(
        "frontend diagnostics:\n{}",
        errors.join("\n")
    )))
}

fn eval_program_root_value(analysis: &ProgramAnalysis) -> Result<Value, ConfigError> {
    let mut evaluator = Evaluator::new();
    let mut root_value = Value::Unit;

    for module_id in analysis.load_order() {
        let Some(module) = analysis.hir_module(*module_id) else {
            continue;
        };
        let method_resolutions = analysis
            .semantics(*module_id)
            .map(|semantics| &semantics.method_resolutions)
            .cloned()
            .unwrap_or_default();

        let value = evaluator
            .eval_module_with_method_resolutions(module, &method_resolutions)
            .map_err(|e| ConfigError::Eval(format!("{:?}", e)))?;

        if *module_id == analysis.root_module_id() {
            root_value = value;
        }
    }

    Ok(root_value)
}

fn module_from_value(value: Value, path: Option<PathBuf>) -> Result<Module, ConfigError> {
    let mut module = Module::new(
        path.as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "anonymous".to_string()),
    );
    module.path = path;

    let Value::Record(fields) = value else {
        return Err(ConfigError::Module(
            "module file must evaluate to a record".to_string(),
        ));
    };

    if let Some(imports_value) = fields.get("imports") {
        module.imports = parse_imports(imports_value)?;
    }
    if let Some(options_value) = fields.get("options") {
        module.options = parse_options(options_value)?;
    }

    if let Some(config_value) = fields.get("config") {
        module.config = extract_record(config_value, "config")?;
    } else {
        for (key, val) in fields.iter() {
            if !RESERVED_KEYS.contains(&key.as_str()) {
                module.config.insert(key.clone(), val.clone());
            }
        }
    }

    Ok(module)
}

/// Recursively load module imports with cycle detection.
/// 递归加载模块导入并检测循环。
fn load_with_imports_recursive(
    path: &Path,
    visited: &mut HashSet<PathBuf>,
    stack: &mut Vec<PathBuf>,
    ordered: &mut Vec<Module>,
) -> Result<(), ConfigError> {
    if visited.contains(path) {
        return Ok(());
    }

    if let Some(pos) = stack.iter().position(|p| p == path) {
        let mut cycle = stack[pos..]
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>();
        cycle.push(path.display().to_string());
        return Err(ConfigError::Module(format!(
            "module import cycle detected: {}",
            cycle.join(" -> ")
        )));
    }

    stack.push(path.to_path_buf());
    let module = Module::load(path)?;
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    for import in &module.imports {
        let import_path = resolve_import_path(base, import)?;
        load_with_imports_recursive(&import_path, visited, stack, ordered)?;
    }
    let _ = stack.pop();

    visited.insert(path.to_path_buf());
    ordered.push(module);
    Ok(())
}

/// Resolve an import path relative to a module file.
/// 解析相对于模块文件的导入路径。
fn resolve_import_path(base: &Path, import: &str) -> Result<PathBuf, ConfigError> {
    let raw = Path::new(import);
    let candidate = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        base.join(raw)
    };
    candidate.canonicalize().map_err(|e| {
        ConfigError::Module(format!(
            "failed to resolve import '{}': {}",
            candidate.display(),
            e
        ))
    })
}

/// Parse `imports` field from evaluated module value.
/// 从模块求值结果解析 `imports` 字段。
fn parse_imports(value: &Value) -> Result<Vec<String>, ConfigError> {
    let Value::List(items) = value else {
        return Err(ConfigError::Module(
            "'imports' must be a list of strings".to_string(),
        ));
    };

    let mut imports = Vec::new();
    for item in items.iter() {
        let Value::String(s) = item else {
            return Err(ConfigError::Module(
                "'imports' must contain only strings".to_string(),
            ));
        };
        imports.push(s.to_string());
    }
    Ok(imports)
}

/// Parse `options` declarations map.
/// 解析 `options` 声明映射。
fn parse_options(value: &Value) -> Result<Vec<OptionDecl>, ConfigError> {
    let Value::Record(options) = value else {
        return Err(ConfigError::Module(
            "'options' must be a record".to_string(),
        ));
    };

    let mut decls = Vec::new();
    for (name, raw_decl) in options.iter() {
        decls.push(parse_option_decl(name, raw_decl)?);
    }
    Ok(decls)
}

/// Parse one option declaration entry.
/// 解析单个选项声明条目。
fn parse_option_decl(name: &str, value: &Value) -> Result<OptionDecl, ConfigError> {
    let Value::Record(fields) = value else {
        return Err(ConfigError::Module(format!(
            "option '{}' must be a record",
            name
        )));
    };

    let ty_value = fields
        .get("type")
        .ok_or_else(|| ConfigError::Module(format!("option '{}' is missing 'type'", name)))?;
    let ty = parse_option_type(ty_value)?;
    let mut decl = OptionDecl::new(name, ty);

    if let Some(default) = fields.get("default") {
        if !value_matches_option_type(default, &decl.ty) {
            return Err(ConfigError::Module(format!(
                "option '{}' default type mismatch: expected {}, got {:?}",
                name,
                option_type_name(&decl.ty),
                default
            )));
        }
        decl.default = Some(default.clone());
    }

    if let Some(description) = fields.get("description") {
        let Value::String(s) = description else {
            return Err(ConfigError::Module(format!(
                "option '{}' field 'description' must be a string",
                name
            )));
        };
        decl.description = Some(s.to_string());
    }

    if let Some(example) = fields.get("example") {
        let Value::String(s) = example else {
            return Err(ConfigError::Module(format!(
                "option '{}' field 'example' must be a string",
                name
            )));
        };
        decl.example = Some(s.to_string());
    }

    Ok(decl)
}

/// Parse a type descriptor used in option declarations.
/// 解析选项声明中的类型描述。
fn parse_option_type(value: &Value) -> Result<OptionType, ConfigError> {
    match value {
        Value::String(s) => match s.as_str() {
            "bool" | "Bool" => Ok(OptionType::Bool),
            "int" | "Int" => Ok(OptionType::Int),
            "string" | "String" => Ok(OptionType::String),
            "path" | "Path" => Ok(OptionType::Path),
            "any" | "Any" => Ok(OptionType::Any),
            other => Err(ConfigError::Module(format!(
                "unknown option type '{}'",
                other
            ))),
        },
        Value::Record(fields) => {
            if let Some(list_of) = fields.get("listOf").or_else(|| fields.get("list_of")) {
                return Ok(OptionType::List(Box::new(parse_option_type(list_of)?)));
            }
            if let Some(enum_values) = fields.get("enum") {
                return Ok(OptionType::Enum(parse_string_list(enum_values, "enum")?));
            }
            if let Some(record_fields) = fields.get("fields") {
                let Value::Record(record_map) = record_fields else {
                    return Err(ConfigError::Module(
                        "record option type 'fields' must be a record".to_string(),
                    ));
                };
                let mut parsed = Vec::new();
                for (field_name, field_ty) in record_map.iter() {
                    parsed.push((field_name.clone(), parse_option_type(field_ty)?));
                }
                return Ok(OptionType::Record(parsed));
            }

            Err(ConfigError::Module(
                "unsupported composite option type descriptor".to_string(),
            ))
        }
        _ => Err(ConfigError::Module(
            "option type descriptor must be a string or record".to_string(),
        )),
    }
}

/// Convert a typed option to a human-friendly type description.
/// 将选项类型转换为易读字符串。
fn option_type_name(ty: &OptionType) -> String {
    match ty {
        OptionType::Bool => "Bool".to_string(),
        OptionType::Int => "Int".to_string(),
        OptionType::String => "String".to_string(),
        OptionType::Path => "Path".to_string(),
        OptionType::List(inner) => format!("List<{}>", option_type_name(inner)),
        OptionType::Record(_) => "Record".to_string(),
        OptionType::Enum(_) => "Enum".to_string(),
        OptionType::Any => "Any".to_string(),
    }
}

/// Check whether a runtime value matches an option type declaration.
/// 检查运行时值是否匹配选项类型声明。
fn value_matches_option_type(value: &Value, ty: &OptionType) -> bool {
    match ty {
        OptionType::Bool => matches!(value, Value::Bool(_)),
        OptionType::Int => matches!(value, Value::Int(_)),
        OptionType::String => matches!(value, Value::String(_)),
        OptionType::Path => matches!(value, Value::String(_)),
        OptionType::List(inner) => match value {
            Value::List(items) => items.iter().all(|v| value_matches_option_type(v, inner)),
            _ => false,
        },
        OptionType::Record(fields) => match value {
            Value::Record(record) => fields.iter().all(|(name, field_ty)| {
                record
                    .get(name)
                    .map(|v| value_matches_option_type(v, field_ty))
                    .unwrap_or(false)
            }),
            _ => false,
        },
        OptionType::Enum(values) => match value {
            Value::String(s) => values.iter().any(|v| v == s.as_str()),
            _ => false,
        },
        OptionType::Any => true,
    }
}

/// Extract a record value into an owned HashMap clone.
/// 将记录值提取为拥有所有权的 HashMap 克隆。
fn extract_record(value: &Value, field: &str) -> Result<HashMap<String, Value>, ConfigError> {
    let Value::Record(record) = value else {
        return Err(ConfigError::Module(format!("'{}' must be a record", field)));
    };
    Ok(record.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
}

/// Parse a required string value.
/// 解析必需字符串值。
fn expect_string(value: &Value, field: &str) -> Result<String, ConfigError> {
    let Value::String(s) = value else {
        return Err(ConfigError::Module(format!("'{}' must be a string", field)));
    };
    Ok(s.to_string())
}

/// Parse list of strings from a Value.
/// 从 Value 解析字符串列表。
fn parse_string_list(value: &Value, field: &str) -> Result<Vec<String>, ConfigError> {
    let Value::List(items) = value else {
        return Err(ConfigError::Module(format!("'{}' must be a list", field)));
    };

    let mut parsed = Vec::new();
    for item in items.iter() {
        let Value::String(s) = item else {
            return Err(ConfigError::Module(format!(
                "'{}' must contain only strings",
                field
            )));
        };
        parsed.push(s.to_string());
    }
    Ok(parsed)
}

/// Parse `environment = [{name, value}, ...]`.
/// 解析 `environment = [{name, value}, ...]`。
fn parse_environment(value: &Value) -> Result<Vec<(String, String)>, ConfigError> {
    let Value::List(items) = value else {
        return Err(ConfigError::Module(
            "'environment' must be a list".to_string(),
        ));
    };

    let mut parsed = Vec::new();
    for item in items.iter() {
        let Value::Record(fields) = item else {
            return Err(ConfigError::Module(
                "'environment' entries must be records".to_string(),
            ));
        };
        let key = fields
            .get("name")
            .ok_or_else(|| ConfigError::Module("'environment' entry missing 'name'".to_string()))
            .and_then(|v| expect_string(v, "environment.name"))?;
        let value = fields
            .get("value")
            .ok_or_else(|| ConfigError::Module("'environment' entry missing 'value'".to_string()))
            .and_then(|v| expect_string(v, "environment.value"))?;
        parsed.push((key, value));
    }
    Ok(parsed)
}

/// Parse `users = [{...}, ...]`.
/// 解析 `users = [{...}, ...]`。
fn parse_users(value: &Value) -> Result<Vec<UserConfig>, ConfigError> {
    let Value::List(items) = value else {
        return Err(ConfigError::Module("'users' must be a list".to_string()));
    };

    let mut users = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let Value::Record(fields) = item else {
            return Err(ConfigError::Module(format!(
                "'users[{}]' must be a record",
                index
            )));
        };

        let name = fields
            .get("name")
            .ok_or_else(|| ConfigError::Module(format!("'users[{}]' missing 'name'", index)))
            .and_then(|v| expect_string(v, &format!("users[{}].name", index)))?;

        let mut user = UserConfig::new(name);

        if let Some(home) = fields.get("home") {
            user.home = PathBuf::from(expect_string(home, &format!("users[{}].home", index))?);
        }
        if let Some(shell) = fields.get("shell") {
            user.shell = Some(expect_string(shell, &format!("users[{}].shell", index))?);
        }
        if let Some(groups) = fields.get("groups") {
            user.groups = parse_string_list(groups, &format!("users[{}].groups", index))?;
        }
        if let Some(packages) = fields.get("packages") {
            user.packages = parse_string_list(packages, &format!("users[{}].packages", index))?;
        }
        if let Some(password_hash) = fields
            .get("passwordHash")
            .or_else(|| fields.get("password_hash"))
        {
            user.password_hash = Some(expect_string(
                password_hash,
                &format!("users[{}].passwordHash", index),
            )?);
        }

        users.push(user);
    }

    Ok(users)
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
