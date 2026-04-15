//! The `neve repl` command.
//! `neve repl` 命令。

use crate::commands::module_graph;
use crate::output;
use neve_common::Span;
use neve_diagnostic::{Severity, emit};
use neve_eval::{Evaluator, Value, builtins};
use neve_frontend::{
    FrontendSession, ModuleAnalysis, SessionBuildInputs, SessionBuildResult,
    format_type_with_names_map,
};
use neve_hir::{DefId, ItemKind as HirItemKind, Module, ModuleId, Ty, TyKind};
use neve_parser::parse;
use neve_std::stdlib;
use neve_syntax::{ImportDef, ImportItems, Item, ItemKind, PatternKind, SourceFile, Visibility};
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

const REPL_FN_TYPE_ID: DefId = DefId(u32::MAX - 100);
const REPL_LAZY_TYPE_ID: DefId = DefId(u32::MAX - 101);

/// Run the REPL.
/// 运行 REPL。
pub fn run() -> Result<(), String> {
    output::info(&format!("Neve REPL v{}", env!("CARGO_PKG_VERSION")));
    println!("Type :help for help, :quit to exit");
    println!();

    let mut rl = DefaultEditor::new().map_err(|e| e.to_string())?;
    let root_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut runtime_state = ReplHirState::new();
    let mut semantic_state = ReplSemanticState::with_root_dir(root_dir);

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
                            println!("Project root: {}", semantic_state.root_dir().display());
                            println!();

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
                                    let context = match semantic_state
                                        .context_for_file(file_path, runtime_state.is_pristine())
                                    {
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
                                        Err(ReplEvalError::ModuleDiagnostics(entries)) => {
                                            for entry in &entries {
                                                for diag in &entry.diagnostics {
                                                    emit(
                                                        &entry.source,
                                                        &entry.path.display().to_string(),
                                                        diag,
                                                    );
                                                }
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
                    Err(ReplEvalError::ModuleDiagnostics(entries)) => {
                        for entry in &entries {
                            for diag in &entry.diagnostics {
                                emit(&entry.source, &entry.path.display().to_string(), diag);
                            }
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
    ModuleDiagnostics(Vec<LoadedModuleDiagnostics>),
    Message(String),
}

#[derive(Debug)]
struct LoadedModuleDiagnostics {
    path: PathBuf,
    source: String,
    diagnostics: Vec<neve_diagnostic::Diagnostic>,
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
    evaluated_modules: HashSet<ModuleId>,
}

impl Default for ReplHirState {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplHirState {
    fn new() -> Self {
        Self::with_root_dir(PathBuf::from("."))
    }

    fn with_root_dir(_root_dir: PathBuf) -> Self {
        Self {
            evaluator: Evaluator::new().with_extra_builtins(std_builtin_values()),
            next_def_id: 0,
            globals: HashMap::new(),
            user_bindings: HashMap::new(),
            builtin_item_imports: HashMap::new(),
            builtin_module_imports: HashMap::new(),
            imported_defs: HashMap::new(),
            imported_module_aliases: HashSet::new(),
            evaluated_modules: HashSet::new(),
        }
    }

    fn clear(&mut self) {
        *self = Self::new();
    }

    fn is_pristine(&self) -> bool {
        self.globals.is_empty()
            && self.user_bindings.is_empty()
            && self.builtin_item_imports.is_empty()
            && self.builtin_module_imports.is_empty()
            && self.imported_defs.is_empty()
            && self.imported_module_aliases.is_empty()
            && self.evaluated_modules.is_empty()
    }

    fn user_bindings(&self) -> Vec<(&str, bool)> {
        let mut bindings: Vec<_> = self
            .user_bindings
            .iter()
            .map(|(name, is_pub)| (name.as_str(), *is_pub))
            .collect();
        bindings.sort_by_key(|(name, _)| *name);
        bindings
    }

    fn eval_persistent(&mut self, input: PersistentEvalInput<'_>) -> Result<Value, String> {
        self.eval_pending_loaded_modules(input.semantic_state)?;
        self.evaluator
            .set_method_resolutions(input.method_resolutions);
        let value = self
            .evaluator
            .eval_module(input.module)
            .map_err(|e| format!("evaluation error: {e:?}"))?;
        self.next_def_id = input.next_def_id.max(input.semantic_state.next_def_id());
        for (name, def_id) in input.global_defs {
            self.globals.insert(name.clone(), *def_id);
        }
        self.record_user_bindings(input.ast);
        self.record_module_imports(input.ast, &input.context.module_path, input.semantic_state)?;
        self.record_std_imports(input.ast);
        Ok(value)
    }

    fn eval_ephemeral(
        &mut self,
        module: &Module,
        semantic_state: &ReplSemanticState,
        method_resolutions: HashMap<Span, DefId>,
    ) -> Result<Value, String> {
        self.eval_pending_loaded_modules(semantic_state)?;
        let mut evaluator = self.evaluator.clone();
        evaluator
            .eval_module_with_method_resolutions(module, &method_resolutions)
            .map_err(|e| format!("evaluation error: {e:?}"))
    }

    fn build_module(
        &mut self,
        ast: &SourceFile,
        context: &ReplInputContext,
        semantic_state: &mut ReplSemanticState,
    ) -> Result<SessionBuildResult, String> {
        semantic_state.validate_context(context)?;
        semantic_state.build_module_from_ast(
            ast,
            context.module_name.clone(),
            context.module_path.clone(),
            SessionBuildInputs {
                next_def_id: self.next_def_id.max(semantic_state.next_def_id()),
                existing_globals: self
                    .globals
                    .iter()
                    .map(|(name, def_id)| (name.clone(), *def_id))
                    .collect(),
                imported_defs: self
                    .imported_defs
                    .iter()
                    .map(|(name, def_id)| (name.clone(), *def_id))
                    .collect(),
                imported_module_aliases: self.imported_module_aliases.iter().cloned().collect(),
                builtin_item_imports: self
                    .builtin_item_imports
                    .iter()
                    .map(|(name, builtin)| (name.clone(), builtin.clone()))
                    .collect(),
                builtin_module_imports: self
                    .builtin_module_imports
                    .iter()
                    .map(|(alias, prefix)| (alias.clone(), prefix.clone()))
                    .collect(),
            },
        )
    }

    fn record_user_bindings(&mut self, ast: &SourceFile) {
        for (name, is_pub) in item_bindings(ast) {
            self.user_bindings.insert(name, is_pub);
        }
    }

    fn eval_pending_loaded_modules(
        &mut self,
        semantic_state: &ReplSemanticState,
    ) -> Result<(), String> {
        let analyses = semantic_state.analyze_loaded_modules();

        for module_id in semantic_state.load_order() {
            if self.evaluated_modules.contains(module_id) {
                continue;
            }

            let Some(module) = semantic_state.hir_module(*module_id) else {
                continue;
            };
            let Some(analysis) = analyses.get(module_id) else {
                continue;
            };

            self.evaluator
                .eval_module_with_method_resolutions(module, &analysis.semantics.method_resolutions)
                .map_err(|e| format!("evaluation error: {e:?}"))?;
            self.evaluated_modules.insert(*module_id);
        }

        Ok(())
    }

    fn record_module_imports(
        &mut self,
        ast: &SourceFile,
        current_module_path: &[String],
        semantic_state: &ReplSemanticState,
    ) -> Result<(), String> {
        let resolved = semantic_state.resolve_ast_imports(ast, current_module_path)?;

        for (name, def_id) in resolved.bindings {
            self.imported_defs.insert(name, def_id);
        }
        for alias in resolved.module_aliases {
            self.imported_module_aliases.insert(alias);
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

struct PersistentEvalInput<'a> {
    ast: &'a SourceFile,
    module: &'a Module,
    global_defs: &'a HashMap<String, DefId>,
    next_def_id: u32,
    context: &'a ReplInputContext,
    semantic_state: &'a ReplSemanticState,
    method_resolutions: HashMap<Span, DefId>,
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

    let build = runtime_state
        .build_module(&ast, context, semantic_state)
        .map_err(ReplEvalError::Message)?;
    if !build.newly_loaded.is_empty() {
        report_loaded_module_diagnostics(semantic_state, &build.newly_loaded)?;
    }

    let analysis = match analyze_repl_module(&build.module, semantic_state) {
        Ok(analysis) => analysis,
        Err(diagnostics) => {
            return Err(ReplEvalError::Diagnostics {
                source: current_source.to_string(),
                diagnostics,
            });
        }
    };
    let method_resolutions = analysis.semantics.method_resolutions.clone();

    let value = if persist_defs {
        runtime_state.eval_persistent(PersistentEvalInput {
            ast: &ast,
            module: &build.module,
            global_defs: &build.global_defs,
            next_def_id: build.next_def_id,
            context,
            semantic_state,
            method_resolutions,
        })
    } else {
        runtime_state.eval_ephemeral(&build.module, semantic_state, method_resolutions)
    }
    .map_err(ReplEvalError::Message)?;

    if persist_defs {
        semantic_state.record_module(build.module);
    }

    Ok(value)
}

fn analyze_repl_module(
    current_module: &Module,
    semantic_state: &ReplSemanticState,
) -> Result<ModuleAnalysis, Vec<neve_diagnostic::Diagnostic>> {
    let analysis = semantic_state.analyze_module(current_module);
    if analysis
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return Err(analysis.diagnostics);
    }

    Ok(analysis)
}

fn report_loaded_module_diagnostics(
    semantic_state: &ReplSemanticState,
    newly_loaded: &[ModuleId],
) -> Result<(), ReplEvalError> {
    let entries: Vec<_> = semantic_state
        .loaded_module_diagnostics(newly_loaded)
        .into_iter()
        .map(|entry| LoadedModuleDiagnostics {
            path: entry.file_path.clone(),
            source: std::fs::read_to_string(&entry.file_path).unwrap_or_default(),
            diagnostics: entry.diagnostics,
        })
        .collect();

    if entries.is_empty() {
        Ok(())
    } else {
        Err(ReplEvalError::ModuleDiagnostics(entries))
    }
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

#[derive(Debug)]
enum TypeQueryError {
    Diagnostics {
        source: String,
        diagnostics: Vec<neve_diagnostic::Diagnostic>,
    },
    Message(String),
}

#[derive(Debug, Clone)]
struct ReplSemanticState {
    session: FrontendSession,
}

impl Default for ReplSemanticState {
    fn default() -> Self {
        let root_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self::with_root_dir(root_dir)
    }
}

impl ReplSemanticState {
    fn with_root_dir(root_dir: PathBuf) -> Self {
        Self {
            session: FrontendSession::new(root_dir),
        }
    }

    fn root_dir(&self) -> &Path {
        self.session.root_dir()
    }

    fn clear(&mut self) {
        self.session.clear();
    }

    fn is_pristine(&self) -> bool {
        self.session.is_pristine()
    }

    fn next_def_id(&self) -> u32 {
        self.session.next_def_id()
    }

    fn hir_module(&self, module_id: ModuleId) -> Option<&Module> {
        self.session.hir_module(module_id)
    }

    fn load_order(&self) -> &[ModuleId] {
        self.session.load_order()
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

    fn context_for_file(
        &mut self,
        path: impl AsRef<Path>,
        allow_root_switch: bool,
    ) -> Result<ReplInputContext, String> {
        let canonical = path
            .as_ref()
            .canonicalize()
            .map_err(|e| format!("cannot resolve path '{}': {}", path.as_ref().display(), e))?;
        let relative = match canonical.strip_prefix(self.root_dir()) {
            Ok(relative) => relative,
            Err(_) => {
                let (root_dir, _) = module_graph::resolve_module_path(&canonical)?;
                if !(allow_root_switch && self.is_pristine()) {
                    return Err(format!(
                        "loaded file '{}' is outside the current REPL session root '{}'; run :clear before switching to another project root",
                        canonical.display(),
                        self.root_dir().display()
                    ));
                }
                self.session.rebase_root(root_dir.clone());
                canonical.strip_prefix(self.root_dir()).map_err(|_| {
                    format!(
                        "loaded file '{}' is outside the REPL session root '{}'",
                        canonical.display(),
                        self.root_dir().display()
                    )
                })?
            }
        };

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

    fn record_module(&mut self, module: Module) {
        self.session.record_module(module);
    }

    fn build_module_from_ast(
        &mut self,
        ast: &SourceFile,
        module_name: String,
        module_path: Vec<String>,
        inputs: SessionBuildInputs,
    ) -> Result<SessionBuildResult, String> {
        self.session
            .build_module_from_ast(ast, module_name, module_path, &inputs)
            .map_err(|e| e.to_string())
    }

    fn analyze_module(&self, current_module: &Module) -> ModuleAnalysis {
        self.session.analyze_module(current_module)
    }

    fn analyze_loaded_modules(&self) -> HashMap<ModuleId, ModuleAnalysis> {
        self.session.analyze_loaded_modules()
    }

    fn loaded_module_diagnostics(
        &self,
        newly_loaded: &[ModuleId],
    ) -> Vec<neve_frontend::SessionLoadedDiagnostics> {
        self.session.loaded_module_diagnostics(newly_loaded)
    }

    fn type_names_with_current(&self, current_module: &Module) -> HashMap<DefId, String> {
        self.session.type_names_with_current(current_module)
    }

    fn resolve_ast_imports(
        &self,
        ast: &SourceFile,
        current_module_path: &[String],
    ) -> Result<neve_frontend::SessionResolvedImports, String> {
        self.session
            .resolve_ast_imports(ast, current_module_path)
            .map_err(|e| e.to_string())
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
    let mut semantic = state.clone();
    let build = runtime
        .build_module(&ast, &context, &mut semantic)
        .map_err(TypeQueryError::Message)?;
    let analysis = analyze_repl_module(&build.module, &semantic).map_err(|diagnostics| {
        TypeQueryError::Diagnostics {
            source,
            diagnostics,
        }
    })?;

    let query_def_id = find_repl_type_binding(&build.module).ok_or_else(|| {
        TypeQueryError::Message("internal error: missing type query binding".to_string())
    })?;

    let ty = if let Some(target_def_id) = find_repl_type_target(&build.module) {
        analysis.semantics.global_type(target_def_id)
    } else {
        analysis.semantics.global_type(query_def_id)
    }
    .ok_or_else(|| {
        TypeQueryError::Message("internal error: failed to infer queried type".to_string())
    })?;
    Ok(format_repl_semantic_type(ty, &build.module, &semantic))
}

fn prepare_repl_type_input(expr: &str) -> String {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("let __type__ = {trimmed};")
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

fn format_repl_semantic_type(
    ty: &Ty,
    module: &Module,
    semantic_state: &ReplSemanticState,
) -> String {
    let names = collect_repl_type_names(semantic_state, module);
    format_repl_semantic_type_with_names(ty, &names)
}

fn collect_repl_type_names(
    semantic_state: &ReplSemanticState,
    module: &Module,
) -> HashMap<DefId, String> {
    semantic_state.type_names_with_current(module)
}

fn format_repl_semantic_type_with_names(ty: &Ty, names: &HashMap<DefId, String>) -> String {
    match &ty.kind {
        TyKind::Named(def_id, _) if *def_id == REPL_FN_TYPE_ID => "Fn".to_string(),
        TyKind::Named(def_id, args) if *def_id == REPL_LAZY_TYPE_ID => {
            format!(
                "Lazy[{}]",
                format_repl_semantic_type_with_names(&args[0], names)
            )
        }
        _ => format_type_with_names_map(ty, names),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        REPL_FN_TYPE_ID, REPL_LAZY_TYPE_ID, ReplEvalError, ReplHirState, ReplInputContext,
        ReplSemanticState, TypeQueryError, evaluate_repl_input, format_repl_semantic_type,
        infer_repl_type, prepare_repl_input,
    };
    use neve_common::Span;
    use neve_diagnostic::{ErrorCode, Severity};
    use neve_eval::{
        Value,
        value::{
            CommandValue, PipelineValue, ProcessResultValue, RedirectValue, TaskOutputKind,
            TaskValue,
        },
    };
    use neve_hir::{DefId, ItemKind as HirItemKind, Ty, TyKind, lower};
    use neve_parser::parse;

    fn normalize_inference_vars(input: &str) -> String {
        let mut output = String::new();
        let mut chars = input.chars().peekable();

        while let Some(ch) = chars.next() {
            output.push(ch);
            if ch == '?' {
                while matches!(chars.peek(), Some(next) if next.is_ascii_digit()) {
                    chars.next();
                }
            }
        }

        output
    }

    fn assert_repl_type_diagnostic(
        expr: &str,
        runtime: &ReplHirState,
        state: &ReplSemanticState,
        expected_code: ErrorCode,
        message_fragment: &str,
    ) {
        let err = infer_repl_type(expr, runtime, state)
            .expect_err("expected REPL type inference to fail with diagnostics");
        let TypeQueryError::Diagnostics { diagnostics, .. } = err else {
            panic!("expected diagnostic-bearing REPL type error, got {:?}", err);
        };
        let diag = diagnostics
            .iter()
            .find(|diag| {
                diag.code == Some(expected_code) && diag.message.contains(message_fragment)
            })
            .unwrap_or_else(|| {
                panic!(
                    "expected REPL diagnostic code {:?} containing {:?}, got {:?}",
                    expected_code, message_fragment, diagnostics
                )
            });
        assert_eq!(diag.severity, Severity::Error);
    }

    use neve_syntax::SourceFile;
    use neve_typeck::{
        BYTES_TYPE_ID, COMMAND_TYPE_ID, LIST_TYPE_ID, MAP_TYPE_ID, OPTION_TYPE_ID, PATH_TYPE_ID,
        PIPELINE_TYPE_ID, PROCESS_RESULT_TYPE_ID, REDIRECT_TYPE_ID, RESULT_TYPE_ID, SET_TYPE_ID,
        TASK_TYPE_ID, builtin_bytes, builtin_command, builtin_list, builtin_map, builtin_option,
        builtin_path, builtin_pipeline, builtin_process_result, builtin_redirect, builtin_result,
        builtin_set, builtin_task, format_builtin_named_type,
    };
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[derive(Debug, Clone)]
    struct ReplSemanticEntry {
        defined_names: Vec<String>,
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
                        defined_names: super::item_defined_names(item),
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
            Value::Path(_) => builtin_path(Span::DUMMY),
            Value::Bytes(_) => builtin_bytes(Span::DUMMY),
            Value::Command(_) => builtin_command(Span::DUMMY),
            Value::Pipeline(_) => builtin_pipeline(Span::DUMMY),
            Value::Redirect(_) => builtin_redirect(Span::DUMMY),
            Value::Task(task) => builtin_task(task_output_type(task), Span::DUMMY),
            Value::ProcessResult(_) => builtin_process_result(Span::DUMMY),
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

    fn task_output_type(task: &TaskValue) -> Ty {
        match task.output() {
            TaskOutputKind::ProcessResult => builtin_process_result(Span::DUMMY),
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
            TyKind::DynamicRecord(fields) => {
                let parts: Vec<_> = fields
                    .iter()
                    .map(|(name, ty)| format!("{name}: {}", format_repl_type(ty)))
                    .collect();
                if parts.is_empty() {
                    "{ .. }".to_string()
                } else {
                    format!("{{ {}, .. }}", parts.join(", "))
                }
            }
            TyKind::SafeRecordBase(fields) => {
                let parts: Vec<_> = fields
                    .iter()
                    .map(|(name, ty)| format!("{name}: {}", format_repl_type(ty)))
                    .collect();
                if parts.is_empty() {
                    "RecordOrOption[{ .. }]".to_string()
                } else {
                    format!("RecordOrOption[{{ {}, .. }}]", parts.join(", "))
                }
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
    fn repl_type_uses_canonical_assoc_return_for_trait_method_calls() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            r#"
            trait Iterator {
                type Item;
                fn first(self) -> Self.Item;
            };
            impl Iterator for Int {
                type Item = String;
                fn first(self) -> Self.Item = toString(self);
            };
            "#,
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("trait definition should evaluate");

        let ty =
            infer_repl_type("1.first()", &runtime, &state).expect("type inference should succeed");
        assert_eq!(ty, "String");
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

        let mut state = ReplSemanticState::default();
        state.record_module(hir.clone());
        assert_eq!(
            format_repl_semantic_type(&ty, &hir, &state),
            "(User) -> User"
        );
    }

    #[test]
    fn repl_type_formats_dynamic_record_shape_readably() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "let outputs = fn(inputs) inputs.dep.packages.default;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("definition should evaluate");

        let ty =
            infer_repl_type("outputs", &runtime, &state).expect("type inference should succeed");
        assert_eq!(
            normalize_inference_vars(&ty),
            "({ dep: { packages: { default: ?, .. }, .. }, .. }) -> ?"
        );
    }

    #[test]
    fn repl_type_uses_optional_flow_result_for_try_expr() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.option as option;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type("option.some(41)? + 1", &runtime, &state)
            .expect("type inference should succeed");
        assert_eq!(ty, "Int");
    }

    #[test]
    fn repl_type_uses_path_runtime_bridge_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(r#"path.fromString("/tmp/file.txt")"#, &runtime, &state)
            .expect("type inference should succeed");
        assert_eq!(ty, "Path");
    }

    #[test]
    fn repl_type_uses_typed_path_adapter_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"path.extensionPath(path.joinPath(path.fromString("/tmp"), "neve.txt"))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Option[String]");
    }

    #[test]
    fn repl_type_uses_std_list_sort_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.list as list;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"list.sort(io.readDirEntryPaths(path.fromString("/tmp")))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "List[Path]");
    }

    #[test]
    fn repl_type_uses_std_list_max_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.list as list;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(r#"list.max([1, 3, 2])"#, &runtime, &state)
            .expect("type inference should succeed");
        assert_eq!(ty, "Option[Int]");
    }

    #[test]
    fn repl_type_uses_std_list_head_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.list as list;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"list.head(io.readDirEntryPaths(path.fromString("/tmp")))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Option[Path]");
    }

    #[test]
    fn repl_type_uses_std_list_reverse_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.list as list;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"list.reverse(io.readDirEntryPaths(path.fromString("/tmp")))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "List[Path]");
    }

    #[test]
    fn repl_type_uses_std_list_get_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.list as list;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"list.get(0, io.readDirEntryPaths(path.fromString("/tmp")))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Option[Path]");
    }

    #[test]
    fn repl_type_uses_std_list_cons_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.list as list;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"list.cons(path.fromString("/"), io.readDirEntryPaths(path.fromString("/tmp")))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "List[Path]");
    }

    #[test]
    fn repl_type_uses_std_list_take_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.list as list;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"list.take(2, io.readDirEntryPaths(path.fromString("/tmp")))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "List[Path]");
    }

    #[test]
    fn repl_type_uses_std_list_drop_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.list as list;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"list.drop(1, io.readDirEntryPaths(path.fromString("/tmp")))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "List[Path]");
    }

    #[test]
    fn repl_type_uses_std_list_contains_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.list as list;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"list.contains(path.fromString("/"), io.readDirEntryPaths(path.fromString("/tmp")))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Bool");
    }

    #[test]
    fn repl_type_uses_std_list_index_of_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.list as list;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"list.indexOf(path.fromString("/"), io.readDirEntryPaths(path.fromString("/tmp")))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Option[Int]");
    }

    #[test]
    fn repl_type_uses_std_list_sum_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.list as list;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(r#"list.sum([1, 2, 3])"#, &runtime, &state)
            .expect("type inference should succeed");
        assert_eq!(ty, "Int");
    }

    #[test]
    fn repl_type_uses_std_list_product_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.list as list;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(r#"list.product([2, 3, 4])"#, &runtime, &state)
            .expect("type inference should succeed");
        assert_eq!(ty, "Int");
    }

    #[test]
    fn repl_type_uses_std_list_replicate_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.list as list;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"list.replicate(2, path.fromString("/tmp"))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "List[Path]");
    }

    #[test]
    fn repl_type_uses_std_list_zip_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.list as list;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"list.zip(io.readDirEntryPaths(path.fromString("/tmp")), [1, 2])"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "List[(Path, Int)]");
    }

    #[test]
    fn repl_type_uses_std_math_constant_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.math as math;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty =
            infer_repl_type("math.inf", &runtime, &state).expect("type inference should succeed");
        assert_eq!(ty, "Float");
    }

    #[test]
    fn repl_type_uses_std_fetch_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.fetch as fetch;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(r#"fetch.path("Cargo.toml").cached"#, &runtime, &state)
            .expect("type inference should succeed");
        assert_eq!(ty, "Bool");
    }

    #[test]
    fn repl_type_uses_std_map_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input("import std.Map;", true, &context, &mut runtime, &mut state)
            .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"Map.getWithDefault("a", 0, Map.insert("a", 1, Map.empty))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Int");
    }

    #[test]
    fn repl_type_uses_std_set_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input("import std.Set;", true, &context, &mut runtime, &mut state)
            .expect("import should evaluate");

        let ty = infer_repl_type(
            "Set.isDisjoint(Set.fromList([1]), Set.fromList([2]))",
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Bool");
    }

    #[test]
    fn repl_type_uses_std_list_unzip_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.list as list;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"list.unzip([
                (path.fromString("/tmp"), 1),
                (path.fromString("/var"), 2),
            ])"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "(List[Path], List[Int])");
    }

    #[test]
    fn repl_type_uses_std_list_fold_right_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.list as list;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "fn step(x, acc) = x + acc;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("function should evaluate");

        let ty = infer_repl_type(r#"list.foldRight(0, step, [1, 2, 3])"#, &runtime, &state)
            .expect("type inference should succeed");
        assert_eq!(ty, "Int");
    }

    #[test]
    fn repl_type_uses_io_read_file_path_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.readFilePath(path.fromString("/tmp/file.txt"))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "String");
    }

    #[test]
    fn repl_type_uses_io_read_file_bytes_path_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.readFileBytesPath(path.fromString("/tmp/file.bin"))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Bytes");
    }

    #[test]
    fn repl_type_uses_io_read_dir_path_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.readDirPath(path.fromString("/tmp"))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "List[String]");
    }

    #[test]
    fn repl_type_uses_io_read_dir_entry_paths_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.readDirEntryPaths(path.fromString("/tmp"))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "List[Path]");
    }

    #[test]
    fn repl_type_uses_io_write_file_bytes_path_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.writeFileBytesPath(path.fromString("/tmp/file.out"), io.readFileBytesPath(path.fromString("/tmp/file.bin")))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "()");
    }

    #[test]
    fn repl_type_uses_io_write_file_path_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.writeFilePath(path.fromString("/tmp/file.out"), "hello")"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "()");
    }

    #[test]
    fn repl_type_uses_io_append_file_path_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.appendFilePath(path.fromString("/tmp/file.out"), "hello")"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "()");
    }

    #[test]
    fn repl_type_uses_io_append_file_bytes_path_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.appendFileBytesPath(path.fromString("/tmp/file.out"), io.readFileBytesPath(path.fromString("/tmp/file.bin")))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "()");
    }

    #[test]
    fn repl_type_uses_io_current_dir_path_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(r#"io.currentDirPath()"#, &runtime, &state)
            .expect("type inference should succeed");
        assert_eq!(ty, "Path");
    }

    #[test]
    fn repl_type_uses_io_home_dir_path_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(r#"io.homeDirPath()"#, &runtime, &state)
            .expect("type inference should succeed");
        assert_eq!(ty, "Option[Path]");
    }

    #[test]
    fn repl_type_uses_io_create_dir_all_path_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io; import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.createDirAllPath(path.fromString("/tmp/neve-dir"))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "()");
    }

    #[test]
    fn repl_type_uses_io_remove_dir_all_path_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io; import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.removeDirAllPath(path.fromString("/tmp/neve-dir"))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "()");
    }

    #[test]
    fn repl_type_uses_io_hash_file_path_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io; import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.hashFilePath(path.fromString("/tmp/file.txt"))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "String");
    }

    #[test]
    fn repl_type_uses_io_command_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(r#"io.command("printf", ["neve"])"#, &runtime, &state)
            .expect("type inference should succeed");
        assert_eq!(ty, "Command");
    }

    #[test]
    fn repl_type_uses_io_command_with_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.commandWith(#{ program = "printf", args = ["neve"], cwd = "/tmp" })"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Command");
    }

    #[test]
    fn repl_type_uses_io_exec_command_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.execCommand(io.command("rustc", ["--version"]))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "ProcessResult");
    }

    #[test]
    fn repl_type_uses_io_pipeline_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.pipeline([io.command("printf", ["neve"]), io.command("cat", [])])"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Pipeline");
    }

    #[test]
    fn repl_type_uses_io_pipeline_with_redirects_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;\nimport std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.pipelineWithRedirects(io.pipeline([io.command("printf", ["neve"]), io.command("cat", [])]), [io.redirectStdoutPath(path.fromString("/tmp/neve.out"))])"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Pipeline");
    }

    #[test]
    fn repl_type_uses_io_exec_pipeline_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.execPipeline(io.pipeline([io.command("printf", ["neve"]), io.command("cat", [])]))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "ProcessResult");
    }

    #[test]
    fn repl_type_uses_io_exec_pipeline_with_redirect_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.execPipeline(
                io.pipelineWithRedirects(
                    io.pipeline([io.command("printf", ["neve"]), io.command("cat", [])]),
                    [io.redirectStdoutPath(path.fromString("/tmp/neve.out"))]
                )
            )"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "ProcessResult");
    }

    #[test]
    fn repl_type_uses_io_exec_pipeline_with_redirects_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.execPipeline(
                io.pipelineWithRedirects(
                    io.pipeline([io.command("printf", ["neve"]), io.command("cat", [])]),
                    [
                        io.redirectStdoutPath(path.fromString("/tmp/neve.out")),
                        io.redirectStderrPath(path.fromString("/tmp/neve.err"))
                    ]
                )
            )"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "ProcessResult");
    }

    #[test]
    fn repl_type_uses_io_command_with_redirects_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.commandWithRedirects(
                io.command("printf", ["neve"]),
                [io.redirectStdoutPath(path.fromString("/tmp/neve.out"))]
            )"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Command");
    }

    #[test]
    fn repl_type_uses_io_redirect_stdout_path_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.redirectStdoutPath(path.fromString("/tmp/neve.out"))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Redirect");
    }

    #[test]
    fn repl_type_uses_io_redirect_stderr_path_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.redirectStderrPath(path.fromString("/tmp/neve.err"))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Redirect");
    }

    #[test]
    fn repl_type_uses_io_redirect_stdin_path_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.redirectStdinPath(path.fromString("/tmp/neve.in"))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Redirect");
    }

    #[test]
    fn repl_type_uses_io_exec_command_with_redirect_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.execCommand(
                io.commandWithRedirects(
                    io.command("printf", ["neve"]),
                    [io.redirectStdoutPath(path.fromString("/tmp/neve.out"))]
                )
            )"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "ProcessResult");
    }

    #[test]
    fn repl_type_uses_io_exec_command_with_redirects_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.execCommand(
                io.commandWithRedirects(
                    io.command("printf", ["neve"]),
                    [
                        io.redirectStdoutPath(path.fromString("/tmp/neve.out")),
                        io.redirectStderrPath(path.fromString("/tmp/neve.err"))
                    ]
                )
            )"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "ProcessResult");
    }

    #[test]
    fn repl_type_uses_io_task_command_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.taskCommand(io.command("printf", ["neve"]))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Task[ProcessResult]");
    }

    #[test]
    fn repl_type_uses_io_task_pipeline_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.taskPipeline(io.pipeline([io.command("printf", ["neve"]), io.command("cat", [])]))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Task[ProcessResult]");
    }

    #[test]
    fn repl_type_uses_io_await_task_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.awaitTask(io.taskCommand(io.command("rustc", ["--version"])))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "ProcessResult");
    }

    #[test]
    fn repl_type_uses_io_await_pipeline_task_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.awaitTask(io.taskPipeline(io.pipeline([io.command("printf", ["neve"]), io.command("cat", [])])))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "ProcessResult");
    }

    #[test]
    fn repl_type_uses_io_await_tasks_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.awaitTasks([io.taskCommand(io.command("printf", ["neve"])), io.taskPipeline(io.pipeline([io.command("printf", ["lang"]), io.command("cat", [])]))])"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "List[ProcessResult]");
    }

    #[test]
    fn repl_type_uses_explicit_shell_exec_command_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.execCommand(io.command("sh", ["-c", "rustc --version"]))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "ProcessResult");
    }

    #[test]
    fn repl_type_uses_io_exec_with_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.execCommand(io.commandWith(#{ program = "rustc", args = ["--version"] }))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "ProcessResult");
    }

    #[test]
    fn repl_type_uses_io_process_success_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.processSuccess(io.execCommand(io.command("rustc", ["--version"])))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Bool");
    }

    #[test]
    fn repl_type_uses_io_process_stdout_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.processStdout(io.execCommand(io.command("rustc", ["--version"])))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "String");
    }

    #[test]
    fn repl_type_uses_io_process_code_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.processCode(io.execCommand(io.command("rustc", ["--version"])))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Int");
    }

    #[test]
    fn repl_type_uses_io_process_stderr_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.processStderr(io.execCommand(io.command("rustc", ["--version"])))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "String");
    }

    #[test]
    fn repl_type_uses_io_path_exists_path_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.pathExistsPath(path.fromString("/tmp/file.txt"))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Bool");
    }

    #[test]
    fn repl_type_uses_io_is_dir_path_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(r#"io.isDirPath(path.fromString("/tmp"))"#, &runtime, &state)
            .expect("type inference should succeed");
        assert_eq!(ty, "Bool");
    }

    #[test]
    fn repl_type_uses_io_is_file_path_result() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");
        evaluate_repl_input(
            "import std.path as path;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            r#"io.isFilePath(path.fromString("/tmp/file.txt"))"#,
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "Bool");
    }

    #[test]
    fn repl_type_uses_optional_flow_result_for_coalesce_expr() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.option as option;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type("option.none ?? 5", &runtime, &state)
            .expect("type inference should succeed");
        assert_eq!(ty, "Int");
    }

    #[test]
    fn repl_type_uses_optional_flow_result_for_safe_field_coalesce_expr() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.option as option;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        let ty = infer_repl_type(
            "option.some(#{ name = \"test\" })?.name ?? \"default\"",
            &runtime,
            &state,
        )
        .expect("type inference should succeed");
        assert_eq!(ty, "String");
    }

    #[test]
    fn repl_type_reports_invalid_try_optional_flow_diagnostic() {
        let runtime = ReplHirState::new();
        let state = ReplSemanticState::default();
        assert_repl_type_diagnostic(
            "41?",
            &runtime,
            &state,
            ErrorCode::TypeMismatch,
            "`?` expects Option-like or Result-like value",
        );
    }

    #[test]
    fn repl_type_reports_invalid_coalesce_optional_flow_diagnostic() {
        let runtime = ReplHirState::new();
        let state = ReplSemanticState::default();
        assert_repl_type_diagnostic(
            "41 ?? 0",
            &runtime,
            &state,
            ErrorCode::TypeMismatch,
            "`??` expects Option-like value",
        );
    }

    #[test]
    fn repl_type_reports_invalid_safe_field_boundary_diagnostic() {
        let runtime = ReplHirState::new();
        let state = ReplSemanticState::default();
        assert_repl_type_diagnostic(
            r#"42?.name ?? "default""#,
            &runtime,
            &state,
            ErrorCode::TypeMismatch,
            "safe field access requires a record or Option[Record]",
        );
    }

    #[test]
    fn repl_type_reports_invalid_io_read_file_path_diagnostic() {
        let mut runtime = ReplHirState::new();
        let mut state = ReplSemanticState::default();
        let context = ReplInputContext::repl();

        evaluate_repl_input(
            "import std.io as io;",
            true,
            &context,
            &mut runtime,
            &mut state,
        )
        .expect("import should evaluate");

        assert_repl_type_diagnostic(
            r#"io.readFilePath("/tmp/file.txt")"#,
            &runtime,
            &state,
            ErrorCode::TypeMismatch,
            "type mismatch",
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
    fn repl_runtime_value_type_formatting_uses_path_builtin_name() {
        let ty = type_from_value(&Value::Path(std::rc::Rc::new(PathBuf::from("/tmp/neve"))));
        assert!(matches!(ty.kind, TyKind::Named(def_id, _) if def_id == PATH_TYPE_ID));
        assert_eq!(format_repl_type(&ty), "Path");
    }

    #[test]
    fn repl_runtime_value_type_formatting_uses_bytes_builtin_name() {
        let ty = type_from_value(&Value::Bytes(std::rc::Rc::new(vec![
            0xde, 0xad, 0xbe, 0xef,
        ])));
        assert!(matches!(ty.kind, TyKind::Named(def_id, _) if def_id == BYTES_TYPE_ID));
        assert_eq!(format_repl_type(&ty), "Bytes");
    }

    #[test]
    fn repl_runtime_value_type_formatting_uses_command_builtin_name() {
        let ty = type_from_value(&Value::Command(std::rc::Rc::new(CommandValue::new(
            "printf",
            vec!["neve".to_string()],
        ))));
        assert!(matches!(ty.kind, TyKind::Named(def_id, _) if def_id == COMMAND_TYPE_ID));
        assert_eq!(format_repl_type(&ty), "Command");
    }

    #[test]
    fn repl_runtime_value_type_formatting_uses_pipeline_builtin_name() {
        let ty = type_from_value(&Value::Pipeline(std::rc::Rc::new(PipelineValue::new(
            vec![
                std::rc::Rc::new(CommandValue::new("printf", vec!["neve".to_string()])),
                std::rc::Rc::new(CommandValue::new("cat", Vec::new())),
            ],
        ))));
        assert!(matches!(ty.kind, TyKind::Named(def_id, _) if def_id == PIPELINE_TYPE_ID));
        assert_eq!(format_repl_type(&ty), "Pipeline");
    }

    #[test]
    fn repl_runtime_value_type_formatting_uses_redirect_builtin_name() {
        let ty = type_from_value(&Value::Redirect(std::rc::Rc::new(
            RedirectValue::stdout_path("/tmp/neve.out"),
        )));
        assert!(matches!(ty.kind, TyKind::Named(def_id, _) if def_id == REDIRECT_TYPE_ID));
        assert_eq!(format_repl_type(&ty), "Redirect");
    }

    #[test]
    fn repl_runtime_value_type_formatting_uses_process_result_builtin_name() {
        let ty = type_from_value(&Value::ProcessResult(std::rc::Rc::new(
            ProcessResultValue::new(0, true, "stdout", "stderr"),
        )));
        assert!(matches!(ty.kind, TyKind::Named(def_id, _) if def_id == PROCESS_RESULT_TYPE_ID));
        assert_eq!(format_repl_type(&ty), "ProcessResult");
    }

    #[test]
    fn repl_runtime_value_type_formatting_uses_task_builtin_name() {
        let ty = type_from_value(&Value::Task(std::rc::Rc::new(
            neve_eval::value::TaskValue::command_process_result(std::rc::Rc::new(
                CommandValue::new("printf", vec!["neve".to_string()]),
            )),
        )));
        assert!(matches!(ty.kind, TyKind::Named(def_id, _) if def_id == TASK_TYPE_ID));
        assert_eq!(format_repl_type(&ty), "Task[ProcessResult]");
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
        let mut semantic = ReplSemanticState::with_root_dir(temp_dir.path().to_path_buf());
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
        let mut semantic = ReplSemanticState::with_root_dir(temp_dir.path().to_path_buf());
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
        let mut semantic = ReplSemanticState::with_root_dir(temp_dir.path().to_path_buf());
        let context = semantic
            .context_for_file(
                temp_dir.path().join("app").join("mod.neve"),
                runtime.is_pristine(),
            )
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

    #[test]
    fn repl_reports_type_errors_from_newly_imported_modules() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("broken.neve"),
            "pub fn bad() = 1 + true;",
        )
        .unwrap();

        let mut runtime = ReplHirState::with_root_dir(temp_dir.path().to_path_buf());
        let mut semantic = ReplSemanticState::with_root_dir(temp_dir.path().to_path_buf());
        let context = ReplInputContext::repl();

        let error = evaluate_repl_input(
            "import broken (bad);",
            true,
            &context,
            &mut runtime,
            &mut semantic,
        )
        .expect_err("importing a broken module should surface diagnostics");

        match error {
            ReplEvalError::ModuleDiagnostics(entries) => {
                assert_eq!(entries.len(), 1);
                assert!(entries[0].path.ends_with("broken.neve"));
                assert!(
                    entries[0]
                        .diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.severity == Severity::Error)
                );
            }
            other => panic!("expected module diagnostics, got {other:?}"),
        }
    }

    #[test]
    fn repl_can_switch_project_root_after_clear_for_load() {
        let first = TempDir::new().unwrap();
        let second = TempDir::new().unwrap();
        fs::create_dir_all(second.path().join("app")).unwrap();
        fs::write(
            second.path().join("app").join("helper.neve"),
            "pub fn inc(x) = x + 1;",
        )
        .unwrap();
        fs::write(
            second.path().join("app").join("mod.neve"),
            "import self.helper (inc); let answer = inc(41);",
        )
        .unwrap();

        let mut runtime = ReplHirState::with_root_dir(first.path().to_path_buf());
        runtime.clear();
        let mut semantic = ReplSemanticState::with_root_dir(first.path().to_path_buf());

        let context = semantic
            .context_for_file(
                second.path().join("app").join("mod.neve"),
                runtime.is_pristine(),
            )
            .expect("context should rebase to the new root");
        assert_eq!(context.root_dir.as_deref(), Some(semantic.root_dir()));

        let source = fs::read_to_string(second.path().join("app").join("mod.neve")).unwrap();
        let value = evaluate_repl_input(&source, true, &context, &mut runtime, &mut semantic)
            .expect("loaded binding should evaluate after root switch");
        assert_eq!(value, Value::Int(42.into()));
    }

    #[test]
    fn repl_rejects_project_root_switch_when_session_is_not_clear() {
        let first = TempDir::new().unwrap();
        let second = TempDir::new().unwrap();
        fs::write(second.path().join("main.neve"), "let answer = 42;").unwrap();

        let mut runtime = ReplHirState::with_root_dir(first.path().to_path_buf());
        let mut semantic = ReplSemanticState::with_root_dir(first.path().to_path_buf());
        let context = ReplInputContext::repl();
        evaluate_repl_input("let x = 1;", true, &context, &mut runtime, &mut semantic)
            .expect("definition should evaluate");

        let error = semantic
            .context_for_file(second.path().join("main.neve"), runtime.is_pristine())
            .expect_err("non-empty sessions should not silently mix project roots");
        assert!(
            error.contains(":clear"),
            "expected guidance to clear the session, got {error}"
        );
    }
}
