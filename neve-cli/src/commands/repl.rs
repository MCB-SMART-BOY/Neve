//! The `neve repl` command.
//! `neve repl` 命令。

use crate::output;
use neve_common::Span;
use neve_diagnostic::{Severity, emit};
use neve_eval::{Evaluator, Value, builtins};
use neve_frontend::rewrite_diagnostics_with_module_names;
use neve_hir::{
    DefId, Import as HirImport, ImportKind as HirImportKind, ImportPathPrefix,
    ItemKind as HirItemKind, Module, ModuleId, ModuleLoader, ModulePath, Resolver, Ty, TyKind,
};
use neve_parser::parse;
use neve_std::stdlib;
use neve_syntax::{ImportDef, ImportItems, Item, ItemKind, PatternKind, SourceFile, Visibility};
use neve_typeck::{
    LIST_TYPE_ID, MAP_TYPE_ID, OPTION_TYPE_ID, RESULT_TYPE_ID, SET_TYPE_ID, TypeChecker,
    builtin_list, builtin_map, builtin_option, builtin_result, builtin_set,
    format_builtin_named_type,
};
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const REPL_FN_TYPE_ID: DefId = DefId(u32::MAX - 5);
const REPL_LAZY_TYPE_ID: DefId = DefId(u32::MAX - 6);

/// Run the REPL.
/// 运行 REPL。
pub fn run() -> Result<(), String> {
    output::info(&format!("Neve REPL v{}", env!("CARGO_PKG_VERSION")));
    println!("Type :help for help, :quit to exit");
    println!();

    let mut rl = DefaultEditor::new().map_err(|e| e.to_string())?;
    let mut runtime_state = ReplHirState::new();
    let mut semantic_state = ReplSemanticState::default();

    // Buffer for multi-line input
    // 多行输入缓冲区
    let mut input_buffer = String::new();
    let mut in_multiline = false;

    loop {
        let prompt = if in_multiline { "....> " } else { "neve> " };
        let readline = rl.readline(prompt);
        match readline {
            Ok(line) => {
                // Handle multi-line input
                // 处理多行输入
                // If line ends with backslash, continue on next line
                // 如果行以反斜杠结尾，则在下一行继续
                if line.trim_end().ends_with('\\') {
                    let trimmed = line.trim_end();
                    input_buffer.push_str(&trimmed[..trimmed.len() - 1]);
                    input_buffer.push('\n');
                    in_multiline = true;
                    continue;
                }

                // If we're in multiline mode, append this line and process
                // 如果处于多行模式，追加此行并处理
                if in_multiline {
                    input_buffer.push_str(&line);
                    in_multiline = false;
                } else {
                    input_buffer = line.to_string();
                }

                let input = input_buffer.trim();

                if input.is_empty() {
                    input_buffer.clear();
                    continue;
                }

                let _ = rl.add_history_entry(input);

                // Handle REPL commands
                // 处理 REPL 命令
                if input.starts_with(':') {
                    let parts: Vec<&str> = input.split_whitespace().collect();
                    let cmd = parts.first().unwrap_or(&"");

                    match *cmd {
                        ":quit" | ":q" => break,
                        ":help" | ":h" => {
                            println!("REPL Commands:");
                            // REPL 命令：
                            println!("  :help, :h         Show this help");
                            println!("  :quit, :q         Exit the REPL");
                            println!("  :env              Show all current bindings");
                            println!("  :type <expr>      Show the type of an expression");
                            println!("  :clear            Clear all bindings (keeps builtins)");
                            println!("  :load <file>      Load and evaluate a Neve file");
                            println!();
                            println!("Tips:");
                            println!("  - Use 'let x = ...' to define variables");
                            println!("  - Use 'fn name(...) = ...' to define functions");
                            println!("  - All definitions persist across inputs");
                            println!("  - End line with \\ for multi-line input");
                            input_buffer.clear();
                            continue;
                        }
                        ":env" => {
                            let builtins_count = builtins().len();
                            let user_bindings = runtime_state.user_bindings();
                            let user_binding_count = user_bindings.len();

                            if user_bindings.is_empty() {
                                println!("(no user-defined bindings)");
                            } else {
                                println!("User-defined bindings:");
                                for (name, is_pub) in &user_bindings {
                                    let vis = if *is_pub { "pub" } else { "   " };
                                    println!("  {} {}", vis, name);
                                }
                            }
                            println!();
                            println!(
                                "({} builtins, {} user-defined)",
                                builtins_count, user_binding_count
                            );
                            input_buffer.clear();
                            continue;
                        }
                        ":type" => {
                            if parts.len() < 2 {
                                println!("Usage: :type <expression>");
                                input_buffer.clear();
                                continue;
                            }
                            let expr_str = parts[1..].join(" ");
                            match infer_repl_type(&expr_str, &runtime_state, &semantic_state) {
                                Ok(ty) => println!("{ty}"),
                                Err(TypeQueryError::Diagnostics {
                                    source,
                                    diagnostics,
                                }) => {
                                    for diag in &diagnostics {
                                        emit(&source, "<repl:type>", diag);
                                    }
                                }
                                Err(TypeQueryError::Message(message)) => {
                                    eprintln!("{message}");
                                }
                            }
                            input_buffer.clear();
                            continue;
                        }
                        ":load" => {
                            if parts.len() < 2 {
                                println!("Usage: :load <file.neve>");
                                input_buffer.clear();
                                continue;
                            }
                            let file_path = parts[1];
                            match std::fs::read_to_string(file_path) {
                                Ok(content) => {
                                    let context = match runtime_state.context_for_file(file_path) {
                                        Ok(context) => context,
                                        Err(message) => {
                                            eprintln!("{message}");
                                            input_buffer.clear();
                                            continue;
                                        }
                                    };

                                    match evaluate_repl_input(
                                        &content,
                                        true,
                                        &context,
                                        &mut runtime_state,
                                        &mut semantic_state,
                                    ) {
                                        Ok(_) => println!("Loaded: {}", file_path),
                                        Err(ReplEvalError::Diagnostics {
                                            source,
                                            diagnostics,
                                        }) => {
                                            for diag in &diagnostics {
                                                emit(&source, file_path, diag);
                                            }
                                        }
                                        Err(ReplEvalError::Message(message)) => {
                                            eprintln!("{message}");
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Cannot read file '{}': {}", file_path, e);
                                }
                            }
                            input_buffer.clear();
                            continue;
                        }
                        ":clear" => {
                            runtime_state.clear();
                            semantic_state.clear();
                            println!("Environment cleared");
                            input_buffer.clear();
                            continue;
                        }
                        _ => {
                            println!("Unknown command: {}", input);
                            println!("Type :help for available commands");
                            input_buffer.clear();
                            continue;
                        }
                    }
                }

                // Prepare input for parsing - wrap bare expressions as let bindings
                // 准备用于解析的输入 - 将裸表达式包装为 let 绑定
                let prepared_input = prepare_repl_input(input);
                let is_expr_wrapped = prepared_input.starts_with("let __expr__ = ");

                match evaluate_repl_input(
                    &prepared_input,
                    !is_expr_wrapped,
                    &ReplInputContext::repl(),
                    &mut runtime_state,
                    &mut semantic_state,
                ) {
                    Ok(value) => {
                        if is_expr_wrapped || !matches!(value, Value::Unit) {
                            println!("{:?}", value);
                        }
                    }
                    Err(ReplEvalError::Diagnostics {
                        source,
                        diagnostics,
                    }) => {
                        for diag in &diagnostics {
                            emit(&source, "<repl>", diag);
                        }
                    }
                    Err(ReplEvalError::Message(message)) => {
                        eprintln!("{message}");
                    }
                }

                // Clear buffer after processing
                // 处理后清除缓冲区
                input_buffer.clear();
            }
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }

    println!("Goodbye!");
    Ok(())
}

/// Prepare REPL input for parsing by wrapping bare expressions as let bindings.
/// 通过将裸表达式包装为 let 绑定来准备 REPL 输入用于解析。
fn prepare_repl_input(input: &str) -> String {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return String::new();
    }

    // Check if it's already a valid item (starts with keyword)
    // 检查是否已经是有效的项（以关键字开头）
    let is_item = trimmed.starts_with("let ")
        || trimmed.starts_with("fn ")
        || trimmed.starts_with("type ")
        || trimmed.starts_with("struct ")
        || trimmed.starts_with("enum ")
        || trimmed.starts_with("trait ")
        || trimmed.starts_with("impl ")
        || trimmed.starts_with("import ")
        || trimmed.starts_with("pub ");

    if is_item {
        // It's already an item, just ensure it ends with semicolon
        // 已经是一个项，只需确保以分号结尾
        if trimmed.ends_with(';') {
            trimmed.to_string()
        } else {
            format!("{trimmed};")
        }
    } else {
        // It's an expression, wrap it as a let binding
        // 是一个表达式，将其包装为 let 绑定
        format!("let __expr__ = {trimmed};")
    }
}

#[derive(Debug)]
enum ReplEvalError {
    Diagnostics {
        source: String,
        diagnostics: Vec<neve_diagnostic::Diagnostic>,
    },
    Message(String),
}

#[derive(Debug, Clone)]
struct ReplInputContext {
    root_dir: Option<PathBuf>,
    module_path: Vec<String>,
    module_name: String,
}

impl ReplInputContext {
    fn repl() -> Self {
        Self {
            root_dir: None,
            module_path: Vec::new(),
            module_name: "repl".to_string(),
        }
    }
}

#[derive(Clone)]
struct ReplHirState {
    evaluator: Evaluator,
    next_def_id: u32,
    globals: HashMap<String, DefId>,
    user_bindings: HashMap<String, bool>,
    builtin_item_imports: HashMap<String, String>,
    builtin_module_imports: HashMap<String, String>,
    imported_defs: HashMap<String, DefId>,
    imported_module_aliases: HashSet<String>,
    module_loader: ModuleLoader,
    evaluated_modules: HashSet<ModuleId>,
}

impl Default for ReplHirState {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplHirState {
    fn new() -> Self {
        let root_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::with_root_dir(root_dir)
    }

    fn with_root_dir(root_dir: PathBuf) -> Self {
        Self {
            evaluator: Evaluator::new().with_extra_builtins(std_builtin_values()),
            next_def_id: 0,
            globals: HashMap::new(),
            user_bindings: HashMap::new(),
            builtin_item_imports: HashMap::new(),
            builtin_module_imports: HashMap::new(),
            imported_defs: HashMap::new(),
            imported_module_aliases: HashSet::new(),
            module_loader: ModuleLoader::new(&root_dir),
            evaluated_modules: HashSet::new(),
        }
    }

    fn clear(&mut self) {
        let root_dir = self.module_loader.root_dir().to_path_buf();
        *self = Self::with_root_dir(root_dir);
    }

    fn root_dir(&self) -> &Path {
        self.module_loader.root_dir()
    }

    fn validate_context(&self, context: &ReplInputContext) -> Result<(), String> {
        if let Some(root_dir) = &context.root_dir
            && root_dir != self.root_dir()
        {
            return Err(format!(
                "REPL 模块图当前固定在会话根目录 '{}'；请在该项目根目录启动 REPL，或加载同一根目录下的文件",
                self.root_dir().display()
            ));
        }
        Ok(())
    }

    fn context_for_file(&self, path: impl AsRef<Path>) -> Result<ReplInputContext, String> {
        let canonical = path
            .as_ref()
            .canonicalize()
            .map_err(|e| format!("cannot resolve path '{}': {}", path.as_ref().display(), e))?;
        let relative = canonical.strip_prefix(self.root_dir()).map_err(|_| {
            format!(
                "loaded file '{}' is outside the REPL session root '{}'",
                canonical.display(),
                self.root_dir().display()
            )
        })?;

        let mut module_path: Vec<String> = relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy().to_string())
            .collect();
        if let Some(last) = module_path.last_mut()
            && last.ends_with(".neve")
        {
            *last = last.trim_end_matches(".neve").to_string();
        }
        if module_path.last().map(|segment| segment.as_str()) == Some("mod") {
            module_path.pop();
        }
        if module_path.len() == 1 && module_path[0] == "lib" {
            module_path.clear();
        }

        let module_name = module_path
            .last()
            .cloned()
            .unwrap_or_else(|| "main".to_string());
        Ok(ReplInputContext {
            root_dir: Some(self.root_dir().to_path_buf()),
            module_path,
            module_name,
        })
    }

    fn user_bindings(&self) -> Vec<(&str, bool)> {
        let mut bindings: Vec<_> = self
            .user_bindings
            .iter()
            .map(|(name, is_pub)| (name.as_str(), *is_pub))
            .collect();
        bindings.sort_by(|(left, _), (right, _)| left.cmp(right));
        bindings
    }

    fn eval_persistent(
        &mut self,
        ast: &SourceFile,
        module: &Module,
        resolver: &Resolver,
        context: &ReplInputContext,
        method_resolutions: HashMap<Span, DefId>,
    ) -> Result<Value, String> {
        self.eval_pending_loaded_modules()?;
        self.evaluator.set_method_resolutions(method_resolutions);
        let value = self
            .evaluator
            .eval_module(module)
            .map_err(|e| format!("evaluation error: {e:?}"))?;
        self.next_def_id = resolver.next_def_id().max(self.module_loader.next_def_id());
        for (name, def_id) in resolver.global_defs() {
            self.globals.insert(name.clone(), *def_id);
        }
        self.record_user_bindings(ast);
        self.record_module_imports(ast, &context.module_path)?;
        self.record_std_imports(ast);
        Ok(value)
    }

    fn eval_ephemeral(
        &mut self,
        module: &Module,
        method_resolutions: HashMap<Span, DefId>,
    ) -> Result<Value, String> {
        self.eval_pending_loaded_modules()?;
        let mut evaluator = self.evaluator.clone();
        evaluator.set_method_resolutions(method_resolutions);
        evaluator
            .eval_module(module)
            .map_err(|e| format!("evaluation error: {e:?}"))
    }

    fn build_module(
        &mut self,
        ast: &SourceFile,
        context: &ReplInputContext,
    ) -> Result<(Module, Resolver), String> {
        self.validate_context(context)?;
        self.load_import_modules(ast, &context.module_path)?;

        let mut resolver = Resolver::new();
        resolver.set_def_id_counter(self.next_def_id.max(self.module_loader.next_def_id()));
        resolver.register_existing_globals(
            self.globals
                .iter()
                .map(|(name, def_id)| (name.clone(), *def_id))
                .collect(),
        );
        resolver.register_imports(
            self.imported_defs
                .iter()
                .map(|(name, def_id)| (name.clone(), *def_id))
                .collect(),
        );
        for alias in &self.imported_module_aliases {
            resolver.register_module_import_alias(alias.clone());
        }
        for (name, builtin_name) in &self.builtin_item_imports {
            resolver.register_builtin_item_import(name.clone(), builtin_name.clone());
        }
        for (alias, module_prefix) in &self.builtin_module_imports {
            resolver.register_builtin_module_import(alias.clone(), module_prefix.clone());
        }
        resolver.set_module_loader(self.module_loader.clone());
        let module = resolver.resolve_with_path(
            ast,
            context.module_name.clone(),
            context.module_path.clone(),
        );
        Ok((module, resolver))
    }

    fn record_user_bindings(&mut self, ast: &SourceFile) {
        for (name, is_pub) in item_bindings(ast) {
            self.user_bindings.insert(name, is_pub);
        }
    }

    fn load_import_modules(
        &mut self,
        ast: &SourceFile,
        current_module_path: &[String],
    ) -> Result<(), String> {
        self.module_loader
            .set_def_id_counter(self.next_def_id.max(self.module_loader.next_def_id()));

        for item in &ast.items {
            let ItemKind::Import(import) = &item.kind else {
                continue;
            };
            if repl_std_module_prefix(import).is_some() {
                continue;
            }

            let import_path = ModulePath::from_import_def(import);
            let Some(absolute_path) = self
                .module_loader
                .resolve_module_path(&import_path, Some(current_module_path))
            else {
                return Err(format!(
                    "cannot resolve import path '{}' from current REPL context",
                    format_import_path(import)
                ));
            };

            self.module_loader
                .load_module(&absolute_path)
                .map_err(|e| format!("module load error: {e}"))?;
        }

        self.next_def_id = self.next_def_id.max(self.module_loader.next_def_id());
        Ok(())
    }

    fn eval_pending_loaded_modules(&mut self) -> Result<(), String> {
        let mut global_types = HashMap::new();
        let mut global_spans = HashMap::new();

        for module_id in self.module_loader.load_order() {
            let Some(module) = self.module_loader.hir_module(*module_id) else {
                continue;
            };
            let (types, spans) = TypeChecker::collect_signatures(module);
            global_types.extend(types);
            global_spans.extend(spans);
        }

        for module_id in self.module_loader.load_order() {
            if self.evaluated_modules.contains(module_id) {
                continue;
            }

            let Some(module) = self.module_loader.hir_module(*module_id) else {
                continue;
            };

            let mut checker =
                TypeChecker::with_global_env(global_types.clone(), global_spans.clone());
            checker.check(module);
            self.evaluator
                .set_method_resolutions(checker.method_resolutions().clone());
            self.evaluator
                .eval_module(module)
                .map_err(|e| format!("evaluation error: {e:?}"))?;
            self.evaluated_modules.insert(*module_id);
        }

        Ok(())
    }

    fn record_module_imports(
        &mut self,
        ast: &SourceFile,
        current_module_path: &[String],
    ) -> Result<(), String> {
        for item in &ast.items {
            let ItemKind::Import(import) = &item.kind else {
                continue;
            };
            if repl_std_module_prefix(import).is_some() {
                continue;
            }

            let resolved = self
                .module_loader
                .resolve_import(&hir_import_from_ast(item.span, import), current_module_path)
                .map_err(|e| format!("import resolution error: {e}"))?;

            for (name, def_id) in resolved {
                self.imported_defs.insert(name, def_id);
            }

            if matches!(import.items, ImportItems::Module)
                && let Some(alias) = import
                    .alias
                    .as_ref()
                    .map(|alias| alias.name.clone())
                    .or_else(|| import.path.last().map(|segment| segment.name.clone()))
            {
                self.imported_module_aliases.insert(alias);
            }
        }

        Ok(())
    }

    fn record_std_imports(&mut self, ast: &SourceFile) {
        for item in &ast.items {
            let ItemKind::Import(import) = &item.kind else {
                continue;
            };
            let Some(module_prefix) = repl_std_module_prefix(import) else {
                continue;
            };

            match &import.items {
                ImportItems::Module => {
                    let alias = import
                        .alias
                        .as_ref()
                        .map(|alias| alias.name.clone())
                        .or_else(|| import.path.last().map(|segment| segment.name.clone()));
                    if let Some(alias) = alias {
                        self.builtin_module_imports
                            .insert(alias, module_prefix.to_string());
                    }
                }
                ImportItems::Items(items) => {
                    for item in items {
                        self.builtin_item_imports
                            .insert(item.name.clone(), format!("{module_prefix}.{}", item.name));
                    }
                }
                ImportItems::All => {
                    for export in std_module_exports(module_prefix) {
                        self.builtin_item_imports
                            .insert(export.clone(), format!("{module_prefix}.{export}"));
                    }
                }
            }
        }
    }
}

fn evaluate_repl_input(
    current_source: &str,
    persist_defs: bool,
    context: &ReplInputContext,
    runtime_state: &mut ReplHirState,
    semantic_state: &mut ReplSemanticState,
) -> Result<Value, ReplEvalError> {
    let (ast, diagnostics) = parse(current_source);
    if !diagnostics.is_empty() {
        return Err(ReplEvalError::Diagnostics {
            source: current_source.to_string(),
            diagnostics,
        });
    }

    let (module, resolver) = runtime_state
        .build_module(&ast, context)
        .map_err(ReplEvalError::Message)?;

    let checker = match typecheck_repl_module(&module, runtime_state, semantic_state) {
        Ok(checker) => checker,
        Err(diagnostics) => {
            return Err(ReplEvalError::Diagnostics {
                source: current_source.to_string(),
                diagnostics,
            });
        }
    };
    let method_resolutions = checker.method_resolutions().clone();

    let value = if persist_defs {
        runtime_state.eval_persistent(&ast, &module, &resolver, context, method_resolutions)
    } else {
        runtime_state.eval_ephemeral(&module, method_resolutions)
    }
    .map_err(ReplEvalError::Message)?;

    if persist_defs {
        semantic_state.record_source(current_source, &ast);
        semantic_state.record_module(module);
    }

    Ok(value)
}

fn typecheck_repl_module(
    current_module: &Module,
    runtime_state: &ReplHirState,
    semantic_state: &ReplSemanticState,
) -> Result<TypeChecker, Vec<neve_diagnostic::Diagnostic>> {
    let mut checker = TypeChecker::new();

    for module_id in runtime_state.module_loader.load_order() {
        let Some(module) = runtime_state.module_loader.hir_module(*module_id) else {
            continue;
        };
        checker.check(module);
        checker.clear_diagnostics();
        checker.clear_method_resolutions();
    }

    for module in &semantic_state.modules {
        checker.check(module);
        checker.clear_diagnostics();
        checker.clear_method_resolutions();
    }

    checker.check(current_module);
    let diagnostics =
        rewrite_diagnostics_with_module_names(checker.diagnostics_ref().to_vec(), current_module);
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return Err(diagnostics);
    }

    Ok(checker)
}

fn item_bindings(ast: &SourceFile) -> Vec<(String, bool)> {
    ast.items
        .iter()
        .flat_map(|item| {
            let is_pub = item_visibility(item) != Visibility::Private;
            item_defined_names(item)
                .into_iter()
                .map(move |name| (name, is_pub))
        })
        .collect()
}

fn item_visibility(item: &Item) -> Visibility {
    match &item.kind {
        ItemKind::Let(def) => def.visibility,
        ItemKind::Fn(def) => def.visibility,
        ItemKind::TypeAlias(def) => def.visibility,
        ItemKind::Struct(def) => def.visibility,
        ItemKind::Enum(def) => def.visibility,
        ItemKind::Trait(def) => def.visibility,
        ItemKind::Import(def) => def.visibility,
        ItemKind::Impl(_) => Visibility::Private,
    }
}

fn std_builtin_values() -> impl Iterator<Item = (String, Value)> {
    stdlib()
        .into_iter()
        .map(|(name, value)| (name.to_string(), value))
}

fn repl_std_module_prefix(import: &ImportDef) -> Option<&str> {
    if import.path.len() == 2
        && import.path.first().map(|segment| segment.name.as_str()) == Some("std")
    {
        Some(import.path[1].name.as_str())
    } else {
        None
    }
}

fn std_module_exports(module_prefix: &str) -> Vec<String> {
    let prefix = format!("{module_prefix}.");
    let mut exports: Vec<_> = stdlib()
        .into_iter()
        .filter_map(|(name, _)| name.strip_prefix(&prefix).map(str::to_string))
        .filter(|name| !name.contains('.'))
        .collect();
    exports.sort();
    exports.dedup();
    exports
}

fn hir_import_from_ast(span: Span, import: &ImportDef) -> HirImport {
    let prefix = match import.prefix {
        neve_syntax::PathPrefix::Absolute => ImportPathPrefix::Absolute,
        neve_syntax::PathPrefix::Self_ => ImportPathPrefix::Self_,
        neve_syntax::PathPrefix::Super => ImportPathPrefix::Super,
        neve_syntax::PathPrefix::Crate => ImportPathPrefix::Crate,
    };

    let kind = match &import.items {
        ImportItems::Module => HirImportKind::Module,
        ImportItems::Items(items) => {
            HirImportKind::Items(items.iter().map(|item| item.name.clone()).collect())
        }
        ImportItems::All => HirImportKind::All,
    };

    HirImport {
        prefix,
        path: import
            .path
            .iter()
            .map(|segment| segment.name.clone())
            .collect(),
        kind,
        alias: import.alias.as_ref().map(|alias| alias.name.clone()),
        is_pub: import.visibility == Visibility::Public,
        span,
    }
}

fn format_import_path(import: &ImportDef) -> String {
    let prefix = match import.prefix {
        neve_syntax::PathPrefix::Absolute => "",
        neve_syntax::PathPrefix::Self_ => "self.",
        neve_syntax::PathPrefix::Super => "super.",
        neve_syntax::PathPrefix::Crate => "crate.",
    };
    format!(
        "{}{}",
        prefix,
        import
            .path
            .iter()
            .map(|segment| segment.name.as_str())
            .collect::<Vec<_>>()
            .join(".")
    )
}

#[derive(Debug)]
enum TypeQueryError {
    Diagnostics {
        source: String,
        diagnostics: Vec<neve_diagnostic::Diagnostic>,
    },
    Message(String),
}

#[derive(Debug, Clone, Default)]
struct ReplSemanticState {
    entries: Vec<ReplSemanticEntry>,
    modules: Vec<Module>,
}

#[derive(Debug, Clone)]
struct ReplSemanticEntry {
    defined_names: Vec<String>,
}

impl ReplSemanticState {
    fn record_source(&mut self, source: &str, ast: &SourceFile) {
        for entry in semantic_entries_from_ast(source, ast) {
            self.entries.push(entry);
        }
    }

    fn record_module(&mut self, module: Module) {
        self.modules.push(module);
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.modules.clear();
    }
}

fn infer_repl_type(
    expr: &str,
    runtime_state: &ReplHirState,
    state: &ReplSemanticState,
) -> Result<String, TypeQueryError> {
    let source = prepare_repl_type_input(expr);
    let (ast, diagnostics) = parse(&source);
    if !diagnostics.is_empty() {
        return Err(TypeQueryError::Diagnostics {
            source,
            diagnostics,
        });
    }

    let context = ReplInputContext::repl();
    let mut runtime = runtime_state.clone();
    let (module, _) = runtime
        .build_module(&ast, &context)
        .map_err(TypeQueryError::Message)?;
    let checker = typecheck_repl_module(&module, &runtime, state).map_err(|diagnostics| {
        TypeQueryError::Diagnostics {
            source,
            diagnostics,
        }
    })?;

    let query_def_id = find_repl_type_binding(&module).ok_or_else(|| {
        TypeQueryError::Message("internal error: missing type query binding".to_string())
    })?;

    let ty = if let Some(target_def_id) = find_repl_type_target(&module) {
        checker.global_type(target_def_id)
    } else {
        checker.global_type(query_def_id)
    }
    .ok_or_else(|| {
        TypeQueryError::Message("internal error: failed to infer queried type".to_string())
    })?;
    Ok(format_repl_semantic_type(
        &ty,
        &module,
        runtime_state,
        state,
    ))
}

fn prepare_repl_type_input(expr: &str) -> String {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("let __type__ = {trimmed};")
    }
}

fn semantic_entries_from_ast(source: &str, ast: &SourceFile) -> Vec<ReplSemanticEntry> {
    ast.items
        .iter()
        .filter_map(|item| {
            let snippet = normalize_repl_item_source(&source[item.span.range()]);
            if snippet.is_empty() {
                None
            } else {
                Some(ReplSemanticEntry {
                    defined_names: item_defined_names(item),
                })
            }
        })
        .collect()
}

fn normalize_repl_item_source(source: &str) -> String {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        String::new()
    } else if trimmed.ends_with(';') {
        trimmed.to_string()
    } else {
        format!("{trimmed};")
    }
}

fn item_defined_names(item: &Item) -> Vec<String> {
    match &item.kind {
        ItemKind::Let(let_def) => match &let_def.pattern.kind {
            PatternKind::Var(ident) if ident.name != "__expr__" && ident.name != "__type__" => {
                vec![ident.name.clone()]
            }
            _ => Vec::new(),
        },
        ItemKind::Fn(fn_def) if fn_def.name.name != "__type__" => vec![fn_def.name.name.clone()],
        ItemKind::TypeAlias(def) => vec![def.name.name.clone()],
        ItemKind::Struct(def) => vec![def.name.name.clone()],
        ItemKind::Enum(def) => vec![def.name.name.clone()],
        ItemKind::Trait(def) => vec![def.name.name.clone()],
        ItemKind::Import(import) => match &import.items {
            ImportItems::Module => import
                .alias
                .as_ref()
                .map(|alias| vec![alias.name.clone()])
                .or_else(|| import.path.last().map(|name| vec![name.name.clone()]))
                .unwrap_or_default(),
            ImportItems::Items(items) => items.iter().map(|item| item.name.clone()).collect(),
            ImportItems::All => Vec::new(),
        },
        ItemKind::Impl(_) => Vec::new(),
        ItemKind::Fn(_) => Vec::new(),
    }
}

fn find_repl_type_binding(module: &neve_hir::Module) -> Option<DefId> {
    module.items.iter().find_map(|item| match &item.kind {
        HirItemKind::Fn(def) if def.name == "__type__" => Some(item.id),
        _ => None,
    })
}

fn find_repl_type_target(module: &neve_hir::Module) -> Option<DefId> {
    module.items.iter().find_map(|item| match &item.kind {
        HirItemKind::Fn(def) if def.name == "__type__" => match &def.body.kind {
            neve_hir::ExprKind::Global(def_id) => Some(*def_id),
            _ => None,
        },
        _ => None,
    })
}

fn unknown_ty() -> Ty {
    Ty {
        kind: TyKind::Unknown,
        span: Span::DUMMY,
    }
}

fn named_repl_ty(def_id: DefId, args: Vec<Ty>) -> Ty {
    Ty {
        kind: TyKind::Named(def_id, args),
        span: Span::DUMMY,
    }
}

fn fn_repl_ty(arity: usize) -> Ty {
    Ty {
        kind: TyKind::Fn(vec![unknown_ty(); arity], Box::new(unknown_ty())),
        span: Span::DUMMY,
    }
}

fn type_from_value(value: &Value) -> Ty {
    match value {
        Value::Int(_) => Ty {
            kind: TyKind::Int,
            span: Span::DUMMY,
        },
        Value::Float(_) => Ty {
            kind: TyKind::Float,
            span: Span::DUMMY,
        },
        Value::Bool(_) => Ty {
            kind: TyKind::Bool,
            span: Span::DUMMY,
        },
        Value::Char(_) => Ty {
            kind: TyKind::Char,
            span: Span::DUMMY,
        },
        Value::String(_) => Ty {
            kind: TyKind::String,
            span: Span::DUMMY,
        },
        Value::Unit => Ty {
            kind: TyKind::Unit,
            span: Span::DUMMY,
        },
        Value::List(items) => builtin_list(common_runtime_type(items.iter()), Span::DUMMY),
        Value::Tuple(items) => Ty {
            kind: TyKind::Tuple(items.iter().map(type_from_value).collect()),
            span: Span::DUMMY,
        },
        Value::Record(fields) => Ty {
            kind: TyKind::Record(
                fields
                    .iter()
                    .map(|(name, value)| (name.clone(), type_from_value(value)))
                    .collect(),
            ),
            span: Span::DUMMY,
        },
        Value::Some(value) => builtin_option(type_from_value(value), Span::DUMMY),
        Value::None => builtin_option(unknown_ty(), Span::DUMMY),
        Value::Ok(value) => builtin_result(type_from_value(value), unknown_ty(), Span::DUMMY),
        Value::Err(value) => builtin_result(unknown_ty(), type_from_value(value), Span::DUMMY),
        Value::Map(_) => builtin_map(unknown_ty(), unknown_ty(), Span::DUMMY),
        Value::Set(_) => builtin_set(unknown_ty(), Span::DUMMY),
        Value::Thunk(_) => named_repl_ty(REPL_LAZY_TYPE_ID, vec![unknown_ty()]),
        Value::Builtin(builtin) => fn_repl_ty(builtin.arity),
        Value::BuiltinFn(_, _) => named_repl_ty(REPL_FN_TYPE_ID, Vec::new()),
        Value::Closure { params, .. } => fn_repl_ty(params.len()),
        Value::AstClosure(closure) => fn_repl_ty(closure.params.len()),
        Value::VariantCtor { arity, .. } => fn_repl_ty(*arity),
        Value::Variant(name, payload) => match name.as_str() {
            "Some" => builtin_option(type_from_value(payload), Span::DUMMY),
            "None" => builtin_option(unknown_ty(), Span::DUMMY),
            "Ok" => builtin_result(type_from_value(payload), unknown_ty(), Span::DUMMY),
            "Err" => builtin_result(unknown_ty(), type_from_value(payload), Span::DUMMY),
            _ => unknown_ty(),
        },
    }
}

fn common_runtime_type<'a>(values: impl Iterator<Item = &'a Value>) -> Ty {
    let mut iter = values;
    let Some(first) = iter.next() else {
        return unknown_ty();
    };
    let first_ty = type_from_value(first);
    let first_fmt = format_repl_type(&first_ty);
    if iter.all(|value| format_repl_type(&type_from_value(value)) == first_fmt) {
        first_ty
    } else {
        unknown_ty()
    }
}

fn format_repl_type(ty: &Ty) -> String {
    match &ty.kind {
        TyKind::Int => "Int".to_string(),
        TyKind::Float => "Float".to_string(),
        TyKind::Bool => "Bool".to_string(),
        TyKind::Char => "Char".to_string(),
        TyKind::String => "String".to_string(),
        TyKind::Unit => "()".to_string(),
        TyKind::Var(id) => format!("?{}", id),
        TyKind::Param(_, name) => name.clone(),
        TyKind::SelfType => "Self".to_string(),
        TyKind::SelfAssoc(name) => format!("Self.{name}"),
        TyKind::Tuple(items) => {
            let parts: Vec<_> = items.iter().map(format_repl_type).collect();
            format!("({})", parts.join(", "))
        }
        TyKind::Record(fields) => {
            let parts: Vec<_> = fields
                .iter()
                .map(|(name, ty)| format!("{name}: {}", format_repl_type(ty)))
                .collect();
            format!("{{ {} }}", parts.join(", "))
        }
        TyKind::Fn(params, ret) => {
            let params: Vec<_> = params.iter().map(format_repl_type).collect();
            format!("({}) -> {}", params.join(", "), format_repl_type(ret))
        }
        TyKind::Forall(params, inner) => {
            format!("forall {}. {}", params.join(", "), format_repl_type(inner))
        }
        TyKind::Named(def_id, args)
            if [
                LIST_TYPE_ID,
                OPTION_TYPE_ID,
                RESULT_TYPE_ID,
                MAP_TYPE_ID,
                SET_TYPE_ID,
            ]
            .contains(def_id) =>
        {
            format_builtin_named_type(*def_id, args, &format_repl_type)
                .unwrap_or_else(|| neve_typeck::format_type(ty))
        }
        TyKind::Named(def_id, _) if *def_id == REPL_FN_TYPE_ID => "Fn".to_string(),
        TyKind::Named(def_id, args) if *def_id == REPL_LAZY_TYPE_ID => {
            format!("Lazy[{}]", format_repl_type(&args[0]))
        }
        TyKind::Named(_, _) => neve_typeck::format_type(ty),
        TyKind::Unknown => "_".to_string(),
    }
}

fn format_repl_semantic_type(
    ty: &Ty,
    module: &Module,
    runtime_state: &ReplHirState,
    semantic_state: &ReplSemanticState,
) -> String {
    let mut names = HashMap::new();
    for module_id in runtime_state.module_loader.load_order() {
        if let Some(loaded_module) = runtime_state.module_loader.hir_module(*module_id) {
            collect_repl_type_names(loaded_module, &mut names);
        }
    }
    for previous in &semantic_state.modules {
        collect_repl_type_names(previous, &mut names);
    }
    collect_repl_type_names(module, &mut names);
    format_repl_semantic_type_with_names(ty, &names)
}

fn collect_repl_type_names(module: &Module, names: &mut HashMap<DefId, String>) {
    for item in &module.items {
        match &item.kind {
            HirItemKind::Fn(def) => {
                names.insert(item.id, def.name.clone());
            }
            HirItemKind::Struct(def) => {
                names.insert(item.id, def.name.clone());
            }
            HirItemKind::Enum(def) => {
                names.insert(item.id, def.name.clone());
                for variant in &def.variants {
                    names.insert(variant.id, variant.name.clone());
                }
            }
            HirItemKind::TypeAlias(def) => {
                names.insert(item.id, def.name.clone());
            }
            HirItemKind::Trait(def) => {
                names.insert(item.id, def.name.clone());
            }
            HirItemKind::Impl(_) => {}
        }
    }
}

fn format_repl_semantic_type_with_names(ty: &Ty, names: &HashMap<DefId, String>) -> String {
    match &ty.kind {
        TyKind::Int => "Int".to_string(),
        TyKind::Float => "Float".to_string(),
        TyKind::Bool => "Bool".to_string(),
        TyKind::Char => "Char".to_string(),
        TyKind::String => "String".to_string(),
        TyKind::Unit => "()".to_string(),
        TyKind::Var(id) => format!("?{id}"),
        TyKind::Param(_, name) => name.clone(),
        TyKind::SelfType => "Self".to_string(),
        TyKind::SelfAssoc(name) => format!("Self.{name}"),
        TyKind::Tuple(items) => {
            let parts: Vec<_> = items
                .iter()
                .map(|item| format_repl_semantic_type_with_names(item, names))
                .collect();
            format!("({})", parts.join(", "))
        }
        TyKind::Record(fields) => {
            let parts: Vec<_> = fields
                .iter()
                .map(|(name, ty)| {
                    format!(
                        "{name}: {}",
                        format_repl_semantic_type_with_names(ty, names)
                    )
                })
                .collect();
            format!("{{ {} }}", parts.join(", "))
        }
        TyKind::Fn(params, ret) => {
            let params: Vec<_> = params
                .iter()
                .map(|param| format_repl_semantic_type_with_names(param, names))
                .collect();
            format!(
                "({}) -> {}",
                params.join(", "),
                format_repl_semantic_type_with_names(ret, names)
            )
        }
        TyKind::Forall(params, inner) => format!(
            "forall {}. {}",
            params.join(", "),
            format_repl_semantic_type_with_names(inner, names)
        ),
        TyKind::Named(def_id, args)
            if [
                LIST_TYPE_ID,
                OPTION_TYPE_ID,
                RESULT_TYPE_ID,
                MAP_TYPE_ID,
                SET_TYPE_ID,
            ]
            .contains(def_id) =>
        {
            format_builtin_named_type(*def_id, args, &|arg| {
                format_repl_semantic_type_with_names(arg, names)
            })
            .unwrap_or_else(|| neve_typeck::format_type(ty))
        }
        TyKind::Named(def_id, _) if *def_id == REPL_FN_TYPE_ID => "Fn".to_string(),
        TyKind::Named(def_id, args) if *def_id == REPL_LAZY_TYPE_ID => {
            format!(
                "Lazy[{}]",
                format_repl_semantic_type_with_names(&args[0], names)
            )
        }
        TyKind::Named(def_id, args) => {
            if let Some(name) = names.get(def_id) {
                if args.is_empty() {
                    name.clone()
                } else {
                    let args: Vec<_> = args
                        .iter()
                        .map(|arg| format_repl_semantic_type_with_names(arg, names))
                        .collect();
                    format!("{name}[{}]", args.join(", "))
                }
            } else {
                neve_typeck::format_type(ty)
            }
        }
        TyKind::Unknown => "_".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ReplHirState, ReplInputContext, ReplSemanticState, evaluate_repl_input,
        format_repl_semantic_type, format_repl_type, infer_repl_type, prepare_repl_input,
        semantic_entries_from_ast, type_from_value,
    };
    use neve_common::Span;
    use neve_eval::Value;
    use neve_hir::{ItemKind as HirItemKind, Ty, TyKind, lower};
    use neve_parser::parse;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn repl_type_infers_basic_expression() {
        let runtime = ReplHirState::new();
        let state = ReplSemanticState::default();
        let ty = infer_repl_type("1 + 2", &runtime, &state).expect("type inference should succeed");
        assert_eq!(ty, "Int");
    }

    #[test]
    fn repl_type_uses_persistent_bindings() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input("let x = 41;", true, &context, &mut runtime, &mut state)
            .expect("definition should evaluate");

        let ty = infer_repl_type("x + 1", &runtime, &state).expect("type inference should succeed");
        assert_eq!(ty, "Int");
    }

    #[test]
    fn repl_type_preserves_checked_function_signature() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "fn id<T>(x: T) -> T = x;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("definition should evaluate");

        let ty = infer_repl_type("id", &runtime, &state).expect("type inference should succeed");
        assert_eq!(ty, "forall T. (T) -> T");
    }

    #[test]
    fn repl_type_formats_local_named_types_readably() {
        let source = "struct User {}; fn id(x: User) -> User = x;";
        let (ast, diagnostics) = parse(source);
        assert!(
            diagnostics.is_empty(),
            "unexpected parse diagnostics: {diagnostics:?}"
        );

        let hir = lower(&ast);
        let ty = match &hir.items[1].kind {
            HirItemKind::Fn(fn_def) => Ty {
                kind: TyKind::Fn(
                    fn_def.params.iter().map(|param| param.ty.clone()).collect(),
                    Box::new(fn_def.return_ty.clone()),
                ),
                span: Span::DUMMY,
            },
            other => panic!("expected function item, got {other:?}"),
        };

        let runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        state.record_module(hir.clone());
        assert_eq!(
            format_repl_semantic_type(&ty, &hir, &runtime, &state),
            "(User) -> User"
        );
    }

    #[test]
    fn semantic_state_skips_hidden_expr_bindings() {
        let source = "let __expr__ = 1 + 2;";
        let (ast, diagnostics) = parse(source);
        assert!(
            diagnostics.is_empty(),
            "unexpected parse diagnostics: {diagnostics:?}"
        );

        let entries = semantic_entries_from_ast(source, &ast);
        assert!(entries.iter().all(|entry| entry.defined_names.is_empty()));
    }

    #[test]
    fn repl_runtime_value_type_formatting_is_readable() {
        let ty = type_from_value(&Value::List(std::rc::Rc::new(vec![Value::Int(1.into())])));
        assert_eq!(format_repl_type(&ty), "List[Int]");
    }

    #[test]
    fn repl_hir_runtime_persists_bindings_across_inputs() {
        let mut runtime = ReplHirState::new();
        let mut semantic = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        let value = evaluate_repl_input("let x = 41;", true, &context, &mut runtime, &mut semantic)
            .expect("definition should evaluate");
        assert_eq!(value, Value::Int(41.into()));

        let expr = prepare_repl_input("x + 1");
        let value = evaluate_repl_input(&expr, false, &context, &mut runtime, &mut semantic)
            .expect("expression should evaluate");
        assert_eq!(value, Value::Int(42.into()));
    }

    #[test]
    fn repl_hir_runtime_preserves_method_calls_across_inputs() {
        let mut runtime = ReplHirState::new();
        let mut semantic = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            r#"
            trait Twice { fn twice(self) -> Int; };
            impl Twice for Int {
                fn twice(self) -> Int = self + self;
            };
            "#,
            true,
            &context,
            &mut runtime,
            &mut semantic,
        )
        .expect("trait definition should evaluate");

        let expr = prepare_repl_input("21.twice()");
        let value = evaluate_repl_input(&expr, false, &context, &mut runtime, &mut semantic)
            .expect("cross-input method call should evaluate");
        assert_eq!(value, Value::Int(42.into()));
    }

    #[test]
    fn repl_hir_runtime_preserves_std_imports_across_inputs() {
        let mut runtime = ReplHirState::new();
        let mut semantic = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.string as string;",
            true,
            &context,
            &mut runtime,
            &mut semantic,
        )
        .expect("import should evaluate");

        let expr = prepare_repl_input(r#"string.len("abcd")"#);
        let value = evaluate_repl_input(&expr, false, &context, &mut runtime, &mut semantic)
            .expect("stdlib call should evaluate");
        assert_eq!(value, Value::Int(4.into()));
    }

    #[test]
    fn repl_hir_runtime_preserves_redefinition_across_inputs() {
        let mut runtime = ReplHirState::new();
        let mut semantic = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input("let x = 1;", true, &context, &mut runtime, &mut semantic)
            .expect("first definition should evaluate");
        let value = evaluate_repl_input(
            "let x = x + 1;",
            true,
            &context,
            &mut runtime,
            &mut semantic,
        )
        .expect("redefinition should evaluate");
        assert_eq!(value, Value::Int(2.into()));

        let expr = prepare_repl_input("x");
        let value = evaluate_repl_input(&expr, false, &context, &mut runtime, &mut semantic)
            .expect("latest binding should evaluate");
        assert_eq!(value, Value::Int(2.into()));

        let ty = infer_repl_type("x", &runtime, &semantic).expect("type query should succeed");
        assert_eq!(ty, "Int");
    }

    #[test]
    fn repl_hir_runtime_preserves_project_module_item_imports() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("math.neve"),
            "pub fn add(x, y) = x + y;",
        )
        .unwrap();

        let mut runtime = ReplHirState::with_root_dir(temp_dir.path().to_path_buf());
        let mut semantic = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import math (add);",
            true,
            &context,
            &mut runtime,
            &mut semantic,
        )
        .expect("module import should evaluate");

        let expr = prepare_repl_input("add(1, 2)");
        let value = evaluate_repl_input(&expr, false, &context, &mut runtime, &mut semantic)
            .expect("imported function should evaluate");
        assert_eq!(value, Value::Int(3.into()));
    }

    #[test]
    fn repl_hir_runtime_preserves_project_module_namespace_imports() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("math.neve"),
            "pub fn add(x, y) = x + y;",
        )
        .unwrap();

        let mut runtime = ReplHirState::with_root_dir(temp_dir.path().to_path_buf());
        let mut semantic = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input("import math;", true, &context, &mut runtime, &mut semantic)
            .expect("module namespace import should evaluate");

        let expr = prepare_repl_input("math.add(20, 22)");
        let value = evaluate_repl_input(&expr, false, &context, &mut runtime, &mut semantic)
            .expect("namespace import should evaluate");
        assert_eq!(value, Value::Int(42.into()));
    }

    #[test]
    fn repl_hir_runtime_loads_file_with_relative_module_imports() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join("app")).unwrap();
        fs::write(
            temp_dir.path().join("app").join("helper.neve"),
            "pub fn inc(x) = x + 1;",
        )
        .unwrap();
        fs::write(
            temp_dir.path().join("app").join("mod.neve"),
            "import self.helper (inc); let answer = inc(41);",
        )
        .unwrap();

        let mut runtime = ReplHirState::with_root_dir(temp_dir.path().to_path_buf());
        let mut semantic = ReplSemanticState::default();
        let context = runtime
            .context_for_file(temp_dir.path().join("app").join("mod.neve"))
            .expect("file context should resolve");

        let source = fs::read_to_string(temp_dir.path().join("app").join("mod.neve")).unwrap();
        let value = evaluate_repl_input(&source, true, &context, &mut runtime, &mut semantic)
            .expect("loaded file should evaluate");
        assert_eq!(value, Value::Int(42.into()));

        let expr = prepare_repl_input("answer");
        let value = evaluate_repl_input(
            &expr,
            false,
            &ReplInputContext::repl(),
            &mut runtime,
            &mut semantic,
        )
        .expect("loaded binding should persist");
        assert_eq!(value, Value::Int(42.into()));
    }
}
