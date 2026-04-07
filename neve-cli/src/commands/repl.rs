//! The `neve repl` command.
//! `neve repl` 命令。

use crate::output;
use neve_common::Span;
use neve_diagnostic::emit;
use neve_eval::{Evaluator, Value, builtins};
use neve_frontend::{analyze_source, format_type_in_module};
use neve_hir::{DefId, ItemKind as HirItemKind, Resolver, Ty, TyKind};
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
use std::collections::HashMap;

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
                                builtins_count,
                                user_binding_count
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
                            match infer_repl_type(&expr_str, &semantic_state) {
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
                                Ok(content) => match evaluate_repl_input(
                                    &content,
                                    true,
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
                                },
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

#[derive(Clone)]
struct ReplHirState {
    evaluator: Evaluator,
    next_def_id: u32,
    globals: HashMap<String, DefId>,
    user_bindings: HashMap<String, bool>,
    builtin_item_imports: HashMap<String, String>,
    builtin_module_imports: HashMap<String, String>,
    has_trait_or_impl_items: bool,
}

impl Default for ReplHirState {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplHirState {
    fn new() -> Self {
        Self {
            evaluator: Evaluator::new().with_extra_builtins(std_builtin_values()),
            next_def_id: 0,
            globals: HashMap::new(),
            user_bindings: HashMap::new(),
            builtin_item_imports: HashMap::new(),
            builtin_module_imports: HashMap::new(),
            has_trait_or_impl_items: false,
        }
    }

    fn clear(&mut self) {
        *self = Self::new();
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
        method_resolutions: HashMap<Span, DefId>,
    ) -> Result<Value, String> {
        let (module, resolver) = self.build_module(ast);
        self.evaluator.set_method_resolutions(method_resolutions);
        let value = self
            .evaluator
            .eval_module(&module)
            .map_err(|e| format!("evaluation error: {e:?}"))?;
        self.next_def_id = resolver.next_def_id();
        for (name, def_id) in resolver.global_defs() {
            self.globals.insert(name.clone(), *def_id);
        }
        self.record_user_bindings(ast);
        self.record_std_imports(ast);
        self.has_trait_or_impl_items |= ast
            .items
            .iter()
            .any(|item| matches!(item.kind, ItemKind::Trait(_) | ItemKind::Impl(_)));
        Ok(value)
    }

    fn eval_ephemeral(
        &self,
        ast: &SourceFile,
        method_resolutions: HashMap<Span, DefId>,
    ) -> Result<Value, String> {
        let (module, _) = self.build_module(ast);
        let mut evaluator = self.evaluator.clone();
        evaluator.set_method_resolutions(method_resolutions);
        evaluator
            .eval_module(&module)
            .map_err(|e| format!("evaluation error: {e:?}"))
    }

    fn build_module(&self, ast: &SourceFile) -> (neve_hir::Module, Resolver) {
        let mut resolver = Resolver::new();
        resolver.set_def_id_counter(self.next_def_id);
        resolver.register_imports(
            self.globals
                .iter()
                .map(|(name, def_id)| (name.clone(), *def_id))
                .collect(),
        );
        for (name, builtin_name) in &self.builtin_item_imports {
            resolver.register_builtin_item_import(name.clone(), builtin_name.clone());
        }
        for (alias, module_prefix) in &self.builtin_module_imports {
            resolver.register_builtin_module_import(alias.clone(), module_prefix.clone());
        }
        let module = resolver.resolve_with_name(ast, "repl".to_string());
        (module, resolver)
    }

    fn record_user_bindings(&mut self, ast: &SourceFile) {
        for (name, is_pub) in item_bindings(ast) {
            self.user_bindings.insert(name, is_pub);
        }
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
                        self.builtin_item_imports.insert(
                            item.name.clone(),
                            format!("{module_prefix}.{}", item.name),
                        );
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

    if let Some(message) = unsupported_repl_hir_reason(&ast, runtime_state, persist_defs) {
        return Err(ReplEvalError::Message(message));
    }

    let (semantic_source, current_offset) = semantic_state.combined_source_with_offset(current_source);
    let analysis = analyze_source(&semantic_source);
    if !analysis.diagnostics.is_empty() {
        return Err(ReplEvalError::Diagnostics {
            source: semantic_source,
            diagnostics: analysis.diagnostics,
        });
    }

    let method_resolutions =
        current_method_resolutions(&analysis, current_offset, current_source.len());

    if !method_resolutions.is_empty() && runtime_state.has_trait_or_impl_items {
        return Err(ReplEvalError::Message(
            "REPL HIR backend does not yet support method dispatch across separate trait/impl inputs"
                .to_string(),
        ));
    }

    let value = if persist_defs {
        runtime_state.eval_persistent(&ast, method_resolutions)
    } else {
        runtime_state.eval_ephemeral(&ast, method_resolutions)
    }
    .map_err(ReplEvalError::Message)?;

    if persist_defs {
        semantic_state.record_source(current_source, &ast);
    }

    Ok(value)
}

fn unsupported_repl_hir_reason(
    ast: &SourceFile,
    runtime_state: &ReplHirState,
    persist_defs: bool,
) -> Option<String> {
    if let Some(message) = ast.items.iter().find_map(|item| match &item.kind {
        ItemKind::Import(import) if repl_std_module_prefix(import).is_none() => Some(
            "REPL HIR backend currently supports only `import std.<module>` imports".to_string(),
        ),
        _ => None,
    }) {
        return Some(message);
    }

    if persist_defs {
        let redefined_names: Vec<_> = item_bindings(ast)
            .into_iter()
            .map(|(name, _)| name)
            .filter(|name| runtime_state.user_bindings.contains_key(name))
            .collect();
        if !redefined_names.is_empty() {
            return Some(
                "REPL HIR backend does not yet support redefining existing top-level bindings"
                    .to_string(),
            );
        }
    }

    None
}

fn current_method_resolutions(
    analysis: &neve_frontend::AnalysisResult,
    offset: usize,
    current_len: usize,
) -> HashMap<Span, DefId> {
    let current_end = offset + current_len;
    analysis
        .method_resolutions
        .iter()
        .filter_map(|(span, def_id)| {
            let start = usize::from(span.start);
            let end = usize::from(span.end);
            if start < offset || end > current_end {
                return None;
            }
            Some((Span::from_usize(start - offset, end - offset), *def_id))
        })
        .collect()
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

#[derive(Debug, Clone, Default)]
struct ReplSemanticState {
    entries: Vec<ReplSemanticEntry>,
}

#[derive(Debug, Clone)]
struct ReplSemanticEntry {
    source: String,
    defined_names: Vec<String>,
}

impl ReplSemanticState {
    fn combined_source_with(&self, current: &str) -> String {
        self.combined_source_with_offset(current).0
    }

    fn combined_source_with_offset(&self, current: &str) -> (String, usize) {
        let mut parts: Vec<&str> = self
            .entries
            .iter()
            .map(|entry| entry.source.as_str())
            .collect();
        let offset = if parts.is_empty() {
            0
        } else {
            parts.iter().map(|part| part.len()).sum::<usize>() + parts.len()
        };
        parts.push(current);
        (parts.join("\n"), offset)
    }

    fn record_source(&mut self, source: &str, ast: &SourceFile) {
        for entry in semantic_entries_from_ast(source, ast) {
            self.entries.push(entry);
        }
    }

    fn clear(&mut self) {
        self.entries.clear();
    }
}

fn infer_repl_type(expr: &str, state: &ReplSemanticState) -> Result<String, TypeQueryError> {
    let source = state.combined_source_with(&prepare_repl_type_input(expr));
    let analysis = analyze_source(&source);
    if !analysis.diagnostics.is_empty() {
        return Err(TypeQueryError::Diagnostics {
            source,
            diagnostics: analysis.diagnostics,
        });
    }

    let query_def_id = find_repl_type_binding(&analysis.hir).ok_or_else(|| {
        TypeQueryError::Message("internal error: missing type query binding".to_string())
    })?;

    let mut checker = TypeChecker::new();
    checker.check(&analysis.hir);

    let ty = if let Some(target_def_id) = find_repl_type_target(&analysis.hir) {
        checker.global_type(target_def_id)
    } else {
        checker.global_type(query_def_id)
    }
    .ok_or_else(|| {
        TypeQueryError::Message("internal error: failed to infer queried type".to_string())
    })?;
    Ok(format_repl_semantic_type(&ty, &analysis.hir))
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
                    source: snippet,
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

fn format_repl_semantic_type(ty: &Ty, module: &neve_hir::Module) -> String {
    format_type_in_module(ty, module)
}

#[cfg(test)]
mod tests {
    use super::{
        ReplHirState, ReplSemanticState, evaluate_repl_input, format_repl_semantic_type,
        format_repl_type, infer_repl_type, prepare_repl_input, semantic_entries_from_ast,
        type_from_value,
    };
    use neve_common::Span;
    use neve_eval::Value;
    use neve_hir::{ItemKind as HirItemKind, Ty, TyKind, lower};
    use neve_parser::parse;

    #[test]
    fn repl_type_infers_basic_expression() {
        let state = ReplSemanticState::default();
        let ty = infer_repl_type("1 + 2", &state).expect("type inference should succeed");
        assert_eq!(ty, "Int");
    }

    #[test]
    fn repl_type_uses_persistent_bindings() {
        let source = "let x = 41;";
        let (ast, diagnostics) = parse(source);
        assert!(
            diagnostics.is_empty(),
            "unexpected parse diagnostics: {diagnostics:?}"
        );

        let mut state = ReplSemanticState::default();
        state.record_source(source, &ast);

        let ty = infer_repl_type("x + 1", &state).expect("type inference should succeed");
        assert_eq!(ty, "Int");
    }

    #[test]
    fn repl_type_preserves_checked_function_signature() {
        let source = "fn id<T>(x: T) -> T = x;";
        let (ast, diagnostics) = parse(source);
        assert!(
            diagnostics.is_empty(),
            "unexpected parse diagnostics: {diagnostics:?}"
        );

        let mut state = ReplSemanticState::default();
        state.record_source(source, &ast);

        let ty = infer_repl_type("id", &state).expect("type inference should succeed");
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

        assert_eq!(format_repl_semantic_type(&ty, &hir), "(User) -> User");
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

        let value = evaluate_repl_input("let x = 41;", true, &mut runtime, &mut semantic)
            .expect("definition should evaluate");
        assert_eq!(value, Value::Int(41.into()));

        let expr = prepare_repl_input("x + 1");
        let value = evaluate_repl_input(&expr, false, &mut runtime, &mut semantic)
            .expect("expression should evaluate");
        assert_eq!(value, Value::Int(42.into()));
    }

    #[test]
    fn repl_hir_runtime_rejects_cross_input_method_dispatch_for_now() {
        let mut runtime = ReplHirState::new();
        let mut semantic = ReplSemanticState::default();

        evaluate_repl_input(
            r#"
            trait Twice { fn twice(self) -> Int; };
            impl Twice for Int {
                fn twice(self) -> Int = self + self;
            };
            "#,
            true,
            &mut runtime,
            &mut semantic,
        )
        .expect("trait definition should evaluate");

        let expr = prepare_repl_input("21.twice()");
        let err = evaluate_repl_input(&expr, false, &mut runtime, &mut semantic)
            .expect_err("cross-input method call should be rejected");
        assert!(matches!(
            err,
            super::ReplEvalError::Message(message)
                if message.contains("method dispatch across separate trait/impl inputs")
        ));
    }

    #[test]
    fn repl_hir_runtime_preserves_std_imports_across_inputs() {
        let mut runtime = ReplHirState::new();
        let mut semantic = ReplSemanticState::default();

        evaluate_repl_input(
            "import std.string as string;",
            true,
            &mut runtime,
            &mut semantic,
        )
        .expect("import should evaluate");

        let expr = prepare_repl_input(r#"string.len("abcd")"#);
        let value = evaluate_repl_input(&expr, false, &mut runtime, &mut semantic)
            .expect("stdlib call should evaluate");
        assert_eq!(value, Value::Int(4.into()));
    }

    #[test]
    fn repl_hir_runtime_rejects_redefinition_for_now() {
        let mut runtime = ReplHirState::new();
        let mut semantic = ReplSemanticState::default();

        evaluate_repl_input("let x = 1;", true, &mut runtime, &mut semantic)
            .expect("first definition should evaluate");
        let err = evaluate_repl_input("let x = x + 1;", true, &mut runtime, &mut semantic)
            .expect_err("redefinition should be rejected");
        assert!(matches!(
            err,
            super::ReplEvalError::Message(message)
                if message.contains("redefining existing top-level bindings")
        ));
    }
}
