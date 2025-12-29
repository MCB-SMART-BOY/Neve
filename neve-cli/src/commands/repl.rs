//! The `neve repl` command.
//! `neve repl` 命令。

use crate::output;
use neve_diagnostic::emit;
use neve_eval::{AstEnv, AstEvaluator, Value, builtins};
use neve_parser::parse;
use neve_syntax::PatternKind;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::cell::RefCell;
use std::rc::Rc;

/// Run the REPL.
/// 运行 REPL。
pub fn run() -> Result<(), String> {
    output::info(&format!("Neve REPL v{}", env!("CARGO_PKG_VERSION")));
    println!("Type :help for help, :quit to exit");
    // 输入 :help 获取帮助，:quit 退出
    println!();

    let mut rl = DefaultEditor::new().map_err(|e| e.to_string())?;

    // Create a persistent mutable environment for the REPL session
    // 为 REPL 会话创建持久的可变环境
    // Using RefCell allows interior mutability while maintaining Rc sharing
    // 使用 RefCell 允许内部可变性，同时保持 Rc 共享
    let env = Rc::new(RefCell::new(AstEnv::with_builtins()));

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
                            // 显示此帮助
                            println!("  :quit, :q         Exit the REPL");
                            // 退出 REPL
                            println!("  :env              Show all current bindings");
                            // 显示所有当前绑定
                            println!("  :type <expr>      Show the type of an expression");
                            // 显示表达式的类型
                            println!("  :clear            Clear all bindings (keeps builtins)");
                            // 清除所有绑定（保留内置函数）
                            println!("  :load <file>      Load and evaluate a Neve file");
                            // 加载并求值 Neve 文件
                            println!();
                            println!("Tips:");
                            // 提示：
                            println!("  - Use 'let x = ...' to define variables");
                            // 使用 'let x = ...' 定义变量
                            println!("  - Use 'fn name(...) = ...' to define functions");
                            // 使用 'fn name(...) = ...' 定义函数
                            println!("  - All definitions persist across inputs");
                            // 所有定义在输入之间持续存在
                            println!("  - End line with \\ for multi-line input");
                            // 以 \\ 结束行以进行多行输入
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
                                // （没有用户定义的绑定）
                            } else {
                                println!("User-defined bindings:");
                                // 用户定义的绑定：
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
                            // （{} 个内置函数，{} 个用户定义）
                            input_buffer.clear();
                            continue;
                        }
                        ":type" => {
                            if parts.len() < 2 {
                                println!("Usage: :type <expression>");
                                // 用法：:type <表达式>
                                input_buffer.clear();
                                continue;
                            }
                            let expr_str = parts[1..].join(" ");
                            println!("(Type inference not yet implemented for: {})", expr_str);
                            // （类型推断尚未实现：{}）
                            println!("Hint: Full type checking will be available soon!");
                            // 提示：完整的类型检查即将推出！
                            input_buffer.clear();
                            continue;
                        }
                        ":load" => {
                            if parts.len() < 2 {
                                println!("Usage: :load <file.neve>");
                                // 用法：:load <文件.neve>
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
                                        AstEvaluator::with_env(Rc::new(current_env));
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
                                                    );
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
                                            // 已加载：{}
                                        }
                                        Err(e) => {
                                            eprintln!("Error loading file: {:?}", e);
                                            // 加载文件错误：{:?}
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Cannot read file '{}': {}", file_path, e);
                                    // 无法读取文件 '{}'：{}
                                }
                            }
                            input_buffer.clear();
                            continue;
                        }
                        ":clear" => {
                            *env.borrow_mut() = AstEnv::with_builtins();
                            println!("Environment cleared");
                            // 环境已清除
                            input_buffer.clear();
                            continue;
                        }
                        _ => {
                            println!("Unknown command: {}", input);
                            // 未知命令：{}
                            println!("Type :help for available commands");
                            // 输入 :help 获取可用命令
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
                    let mut evaluator = AstEvaluator::with_env(Rc::new(current_env));
                    evaluator.eval_file(&ast)
                };

                match result {
                    Ok(value) => {
                        // After successful evaluation, we need to extract new bindings
                        // from the AST and add them to our persistent environment
                        // 成功求值后，我们需要从 AST 中提取新绑定
                        // 并将它们添加到持久环境中
                        for item in &ast.items {
                            if let neve_syntax::ItemKind::Let(let_def) = &item.kind {
                                // Extract the binding name from the pattern
                                // 从模式中提取绑定名称
                                if let PatternKind::Var(ident) = &let_def.pattern.kind {
                                    // Re-evaluate just this binding in the persistent env
                                    // 仅在持久环境中重新求值此绑定
                                    let current_env = env.borrow().clone();
                                    let mut temp_eval =
                                        AstEvaluator::with_env(Rc::new(current_env));

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
                                let mut temp_eval = AstEvaluator::with_env(Rc::new(current_env));

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
                        // 错误：{:?}
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
    // 再见！
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
