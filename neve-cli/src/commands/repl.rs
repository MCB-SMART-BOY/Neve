//! The `neve repl` command.
//! `neve repl` 命令。

use crate::output;
use neve_common::Span;
use neve_diagnostic::emit;
use neve_eval::{AstEnv, AstEvaluator, Value, builtins};
use neve_hir::{DefId, ItemKind as HirItemKind, Resolver, Ty, TyKind};
use neve_parser::parse;
use neve_std::std_module_overrides;
use neve_syntax::PatternKind;
use neve_typeck::TypeChecker;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const REPL_LIST_TYPE_ID: DefId = DefId(u32::MAX - 1);
const REPL_OPTION_TYPE_ID: DefId = DefId(u32::MAX - 2);
const REPL_RESULT_TYPE_ID: DefId = DefId(u32::MAX - 3);
const REPL_MAP_TYPE_ID: DefId = DefId(u32::MAX - 4);
const REPL_SET_TYPE_ID: DefId = DefId(u32::MAX - 5);
const REPL_FN_TYPE_ID: DefId = DefId(u32::MAX - 6);
const REPL_LAZY_TYPE_ID: DefId = DefId(u32::MAX - 7);

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
                            match infer_repl_type(&expr_str, &env.borrow()) {
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
                                                if let neve_syntax::ItemKind::Let(let_def) =
                                                    &item.kind
                                                {
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
                                                } else if let neve_syntax::ItemKind::Fn(fn_def) =
                                                    &item.kind
                                                {
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
                            if let neve_syntax::ItemKind::Let(let_def) = &item.kind {
                                // Extract the binding name from the pattern
                                // 从模式中提取绑定名称
                                if let PatternKind::Var(ident) = &let_def.pattern.kind {
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
                            } else if let neve_syntax::ItemKind::Fn(fn_def) = &item.kind {
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

struct ReplTypeEnv {
    global_types: HashMap<DefId, Ty>,
    global_spans: HashMap<DefId, Span>,
    global_names: HashMap<DefId, String>,
    next_def_id: u32,
}

fn infer_repl_type(expr: &str, env: &AstEnv) -> Result<String, TypeQueryError> {
    let source = prepare_repl_type_input(expr);
    let (ast, diagnostics) = parse(&source);
    if !diagnostics.is_empty() {
        return Err(TypeQueryError::Diagnostics {
            source,
            diagnostics,
        });
    }

    let mut resolver = Resolver::new();
    let repl_env = repl_type_env(env);
    for (&def_id, name) in &repl_env.global_names {
        resolver.register_import(name.clone(), def_id);
    }
    resolver.set_def_id_counter(repl_env.next_def_id);

    let hir = resolver.resolve(&ast);
    let query_def_id = find_repl_type_binding(&hir).ok_or_else(|| {
        TypeQueryError::Message("internal error: missing type query binding".to_string())
    })?;

    let mut checker = TypeChecker::with_global_env(repl_env.global_types, repl_env.global_spans);
    checker.check(&hir);
    if !checker.diagnostics_ref().is_empty() {
        return Err(TypeQueryError::Diagnostics {
            source,
            diagnostics: checker.diagnostics_ref().to_vec(),
        });
    }

    let ty = checker.global_type(query_def_id).ok_or_else(|| {
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

fn repl_type_env(env: &AstEnv) -> ReplTypeEnv {
    let bindings = env.all_bindings();
    let mut repl_env = ReplTypeEnv {
        global_types: HashMap::new(),
        global_spans: HashMap::new(),
        global_names: HashMap::new(),
        next_def_id: 1_000_000u32,
    };
    let mut next_def_id = 1_000_000u32;

    let mut entries: Vec<_> = bindings.into_iter().collect();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));

    for (name, value) in entries {
        let def_id = DefId(next_def_id);
        next_def_id += 1;
        repl_env
            .global_types
            .insert(def_id, type_from_value(&value));
        repl_env.global_spans.insert(def_id, Span::DUMMY);
        repl_env.global_names.insert(def_id, name);
    }

    repl_env.next_def_id = next_def_id;
    repl_env
}

fn find_repl_type_binding(module: &neve_hir::Module) -> Option<DefId> {
    module.items.iter().find_map(|item| match &item.kind {
        HirItemKind::Fn(def) if def.name == "__type__" => Some(item.id),
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
        Value::List(items) => {
            named_repl_ty(REPL_LIST_TYPE_ID, vec![common_runtime_type(items.iter())])
        }
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
        Value::Some(value) => named_repl_ty(REPL_OPTION_TYPE_ID, vec![type_from_value(value)]),
        Value::None => named_repl_ty(REPL_OPTION_TYPE_ID, vec![unknown_ty()]),
        Value::Ok(value) => named_repl_ty(
            REPL_RESULT_TYPE_ID,
            vec![type_from_value(value), unknown_ty()],
        ),
        Value::Err(value) => named_repl_ty(
            REPL_RESULT_TYPE_ID,
            vec![unknown_ty(), type_from_value(value)],
        ),
        Value::Map(_) => named_repl_ty(REPL_MAP_TYPE_ID, vec![unknown_ty(), unknown_ty()]),
        Value::Set(_) => named_repl_ty(REPL_SET_TYPE_ID, vec![unknown_ty()]),
        Value::Thunk(_) => named_repl_ty(REPL_LAZY_TYPE_ID, vec![unknown_ty()]),
        Value::Builtin(builtin) => fn_repl_ty(builtin.arity),
        Value::BuiltinFn(_, _) => named_repl_ty(REPL_FN_TYPE_ID, Vec::new()),
        Value::Closure { params, .. } => fn_repl_ty(params.len()),
        Value::AstClosure(closure) => fn_repl_ty(closure.params.len()),
        Value::VariantCtor { arity, .. } => fn_repl_ty(*arity),
        Value::Variant(name, payload) => match name.as_str() {
            "Some" => named_repl_ty(REPL_OPTION_TYPE_ID, vec![type_from_value(payload)]),
            "None" => named_repl_ty(REPL_OPTION_TYPE_ID, vec![unknown_ty()]),
            "Ok" => named_repl_ty(
                REPL_RESULT_TYPE_ID,
                vec![type_from_value(payload), unknown_ty()],
            ),
            "Err" => named_repl_ty(
                REPL_RESULT_TYPE_ID,
                vec![unknown_ty(), type_from_value(payload)],
            ),
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
        TyKind::Named(def_id, args) if *def_id == REPL_LIST_TYPE_ID => {
            format!("List[{}]", format_repl_type(&args[0]))
        }
        TyKind::Named(def_id, args) if *def_id == REPL_OPTION_TYPE_ID => {
            format!("Option[{}]", format_repl_type(&args[0]))
        }
        TyKind::Named(def_id, args) if *def_id == REPL_RESULT_TYPE_ID => {
            format!(
                "Result[{}, {}]",
                format_repl_type(&args[0]),
                format_repl_type(&args[1])
            )
        }
        TyKind::Named(def_id, args) if *def_id == REPL_MAP_TYPE_ID => {
            format!(
                "Map[{}, {}]",
                format_repl_type(&args[0]),
                format_repl_type(&args[1])
            )
        }
        TyKind::Named(def_id, args) if *def_id == REPL_SET_TYPE_ID => {
            format!("Set[{}]", format_repl_type(&args[0]))
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
    use super::{format_repl_type, infer_repl_type, type_from_value};
    use neve_eval::{AstEnv, Value};

    #[test]
    fn repl_type_infers_basic_expression() {
        let env = AstEnv::with_builtins();
        let ty = infer_repl_type("1 + 2", &env).expect("type inference should succeed");
        assert_eq!(ty, "Int");
    }

    #[test]
    fn repl_type_uses_persistent_bindings() {
        let mut env = AstEnv::with_builtins();
        env.define("x".to_string(), Value::Int(41.into()));
        let ty = infer_repl_type("x + 1", &env).expect("type inference should succeed");
        assert_eq!(ty, "Int");
    }

    #[test]
    fn repl_runtime_value_type_formatting_is_readable() {
        let ty = type_from_value(&Value::List(std::rc::Rc::new(vec![Value::Int(1.into())])));
        assert_eq!(format_repl_type(&ty), "List[Int]");
    }
}
