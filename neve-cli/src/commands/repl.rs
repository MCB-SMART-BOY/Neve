//! The `neve repl` command.
//! `neve repl` 命令。

use crate::output;
use neve_common::Span;
use neve_diagnostic::emit;
use neve_eval::{AstEnv, AstEvaluator, Value, builtins};
use neve_frontend::analyze_source;
use neve_hir::{DefId, ItemKind as HirItemKind, Ty, TyKind};
use neve_parser::parse;
use neve_std::std_module_overrides;
use neve_syntax::{ImportItems, Item, ItemKind, PatternKind, SourceFile};
use neve_typeck::{
    LIST_TYPE_ID, MAP_TYPE_ID, OPTION_TYPE_ID, RESULT_TYPE_ID, SET_TYPE_ID, TypeChecker,
    builtin_list, builtin_map, builtin_option, builtin_result, builtin_set,
    format_builtin_named_type,
};
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::cell::RefCell;
use std::rc::Rc;

const REPL_FN_TYPE_ID: DefId = DefId(u32::MAX - 5);
const REPL_LAZY_TYPE_ID: DefId = DefId(u32::MAX - 6);

/// Run the REPL.
/// 运行 REPL。
pub fn run() -> Result<(), String> {
    output::info(&format!("Neve REPL v{}", env!("CARGO_PKG_VERSION")));
    println!("Type :help for help, :quit to exit");
    println!();

    let mut rl = DefaultEditor::new().map_err(|e| e.to_string())?;

    // Create a persistent mutable environment for the REPL session
    // 为 REPL 会话创建持久的可变环境
    // Using RefCell allows interior mutability while maintaining Rc sharing
    // 使用 RefCell 允许内部可变性，同时保持 Rc 共享
    let env = Rc::new(RefCell::new(AstEnv::with_builtins()));
    let mut semantic_state = ReplSemanticState::default();
    let std_overrides = std_module_overrides();

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
                            let env_ref = env.borrow();
                            let bindings = env_ref.all_bindings();

                            // Separate builtins from user-defined
                            // 将内置函数与用户定义的分开
                            let builtins_count = builtins().len();
                            let user_bindings: Vec<_> = bindings
                                .keys()
                                .filter(|k| !builtins().iter().any(|(b, _)| b == *k))
                                .collect();

                            if user_bindings.is_empty() {
                                println!("(no user-defined bindings)");
                            } else {
                                println!("User-defined bindings:");
                                let mut sorted = user_bindings.clone();
                                sorted.sort();
                                for name in sorted {
                                    let is_pub = env_ref.is_public(name);
                                    let vis = if is_pub { "pub" } else { "   " };
                                    println!("  {} {}", vis, name);
                                }
                            }
                            println!();
                            println!(
                                "({} builtins, {} user-defined)",
                                builtins_count,
                                user_bindings.len()
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
                                Ok(content) => {
                                    let (ast, diagnostics) = parse(&content);
                                    if !diagnostics.is_empty() {
                                        for diag in &diagnostics {
                                            emit(&content, file_path, diag);
                                        }
                                        continue;
                                    }

                                    let semantic_source =
                                        semantic_state.combined_source_with(&content);
                                    let analysis = analyze_source(&semantic_source);
                                    if !analysis.diagnostics.is_empty() {
                                        for diag in &analysis.diagnostics {
                                            emit(&semantic_source, "<repl:load>", diag);
                                        }
                                        input_buffer.clear();
                                        continue;
                                    }

                                    // Evaluate the file in current environment
                                    // 在当前环境中求值文件
                                    let current_env = env.borrow().clone();
                                    let mut evaluator =
                                        AstEvaluator::with_env(Rc::new(current_env))
                                            .with_module_overrides(std_overrides.clone());
                                    match evaluator.eval_file(&ast) {
                                        Ok(_) => {
                                            // Extract and store new bindings
                                            // 提取并存储新绑定
                                            for item in &ast.items {
                                                if let ItemKind::Let(let_def) = &item.kind {
                                                    if let PatternKind::Var(ident) =
                                                        &let_def.pattern.kind
                                                    {
                                                        let current_env = env.borrow().clone();
                                                        let mut temp_eval = AstEvaluator::with_env(
                                                            Rc::new(current_env),
                                                        )
                                                        .with_module_overrides(
                                                            std_overrides.clone(),
                                                        );
                                                        if let Ok(val) =
                                                            temp_eval.eval_expr(&let_def.value)
                                                        {
                                                            let is_pub = let_def.visibility
                                                                != neve_syntax::Visibility::Private;
                                                            env.borrow_mut()
                                                                .define_with_visibility(
                                                                    ident.name.clone(),
                                                                    val,
                                                                    is_pub,
                                                                );
                                                        }
                                                    }
                                                } else if let ItemKind::Fn(fn_def) = &item.kind {
                                                    let current_env = env.borrow().clone();
                                                    let mut temp_eval = AstEvaluator::with_env(
                                                        Rc::new(current_env),
                                                    )
                                                    .with_module_overrides(std_overrides.clone());
                                                    if let Ok(fn_value) =
                                                        temp_eval.eval_fn_def(fn_def)
                                                    {
                                                        let is_pub = fn_def.visibility
                                                            != neve_syntax::Visibility::Private;
                                                        env.borrow_mut().define_with_visibility(
                                                            fn_def.name.name.clone(),
                                                            fn_value,
                                                            is_pub,
                                                        );
                                                    }
                                                }
                                            }
                                            semantic_state.record_source(&content, &ast);
                                            println!("Loaded: {}", file_path);
                                        }
                                        Err(e) => {
                                            eprintln!("Error loading file: {:?}", e);
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
                            *env.borrow_mut() = AstEnv::with_builtins();
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

                // Parse the input
                // 解析输入
                let (ast, diagnostics) = parse(&prepared_input);

                if !diagnostics.is_empty() {
                    for diag in &diagnostics {
                        emit(input, "<repl>", diag);
                    }
                    input_buffer.clear();
                    continue;
                }

                let semantic_source = semantic_state.combined_source_with(&prepared_input);
                let analysis = analyze_source(&semantic_source);
                if !analysis.diagnostics.is_empty() {
                    for diag in &analysis.diagnostics {
                        emit(&semantic_source, "<repl>", diag);
                    }
                    input_buffer.clear();
                    continue;
                }

                // Evaluate with the persistent environment
                // 使用持久环境进行求值
                // We need to evaluate in a temporary scope to capture new bindings
                // 我们需要在临时作用域中求值以捕获新绑定
                let result = {
                    // Clone the current environment for evaluation
                    // 克隆当前环境用于求值
                    let current_env = env.borrow().clone();
                    let mut evaluator = AstEvaluator::with_env(Rc::new(current_env))
                        .with_module_overrides(std_overrides.clone());
                    evaluator.eval_file(&ast)
                };

                match result {
                    Ok(value) => {
                        // After successful evaluation, we need to extract new bindings
                        // from the AST and add them to our persistent environment
                        for item in &ast.items {
                            if let ItemKind::Let(let_def) = &item.kind {
                                // Extract the binding name from the pattern
                                // 从模式中提取绑定名称
                                if let PatternKind::Var(ident) = &let_def.pattern.kind
                                    && ident.name != "__expr__"
                                {
                                    // Re-evaluate just this binding in the persistent env
                                    // 仅在持久环境中重新求值此绑定
                                    let current_env = env.borrow().clone();
                                    let mut temp_eval =
                                        AstEvaluator::with_env(Rc::new(current_env))
                                            .with_module_overrides(std_overrides.clone());

                                    if let Ok(val) = temp_eval.eval_expr(&let_def.value) {
                                        let is_pub =
                                            let_def.visibility != neve_syntax::Visibility::Private;
                                        env.borrow_mut().define_with_visibility(
                                            ident.name.clone(),
                                            val,
                                            is_pub,
                                        );
                                    }
                                }
                            } else if let ItemKind::Fn(fn_def) = &item.kind {
                                // Store function definitions
                                // 存储函数定义
                                let current_env = env.borrow().clone();
                                let mut temp_eval = AstEvaluator::with_env(Rc::new(current_env))
                                    .with_module_overrides(std_overrides.clone());

                                // Create a closure value for the function
                                // 为函数创建闭包值
                                if let Ok(fn_value) = temp_eval.eval_fn_def(fn_def) {
                                    let is_pub =
                                        fn_def.visibility != neve_syntax::Visibility::Private;
                                    env.borrow_mut().define_with_visibility(
                                        fn_def.name.name.clone(),
                                        fn_value,
                                        is_pub,
                                    );
                                }
                            }
                        }

                        if !is_expr_wrapped {
                            semantic_state.record_source(&prepared_input, &ast);
                        }

                        // Print non-unit results, or always print for wrapped expressions
                        // 打印非 unit 结果，或对于包装的表达式始终打印
                        if is_expr_wrapped || !matches!(value, Value::Unit) {
                            println!("{:?}", value);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: {:?}", e);
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
        let mut parts: Vec<&str> = self
            .entries
            .iter()
            .map(|entry| entry.source.as_str())
            .collect();
        parts.push(current);
        parts.join("\n")
    }

    fn record_source(&mut self, source: &str, ast: &SourceFile) {
        for entry in semantic_entries_from_ast(source, ast) {
            self.replace_conflicting_entries(&entry.defined_names);
            self.entries.push(entry);
        }
    }

    fn replace_conflicting_entries(&mut self, names: &[String]) {
        if names.is_empty() {
            return;
        }

        self.entries.retain(|entry| {
            entry
                .defined_names
                .iter()
                .all(|existing| !names.iter().any(|name| name == existing))
        });
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
    Ok(format_repl_type(&ty))
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

#[cfg(test)]
mod tests {
    use super::{
        ReplSemanticState, format_repl_type, infer_repl_type, semantic_entries_from_ast,
        type_from_value,
    };
    use neve_eval::Value;
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
}
