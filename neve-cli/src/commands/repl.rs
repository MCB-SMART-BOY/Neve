//! The `neve repl` command.

use crate::output;
use neve_diagnostic::emit;
use neve_eval::{AstEnv, AstEvaluator, Value, builtins};
use neve_parser::parse;
use neve_syntax::PatternKind;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use std::cell::RefCell;
use std::rc::Rc;

pub fn run() -> Result<(), String> {
    output::info(&format!("Neve REPL v{}", env!("CARGO_PKG_VERSION")));
    println!("Type :help for help, :quit to exit");
    println!();

    let mut rl = DefaultEditor::new().map_err(|e| e.to_string())?;

    // Create a persistent mutable environment for the REPL session
    // Using RefCell allows interior mutability while maintaining Rc sharing
    let env = Rc::new(RefCell::new(AstEnv::with_builtins()));

    // Buffer for multi-line input
    let mut input_buffer = String::new();
    let mut in_multiline = false;

    loop {
        let prompt = if in_multiline { "....> " } else { "neve> " };
        let readline = rl.readline(prompt);
        match readline {
            Ok(line) => {
                // Handle multi-line input
                // If line ends with backslash, continue on next line
                if line.trim_end().ends_with('\\') {
                    let trimmed = line.trim_end();
                    input_buffer.push_str(&trimmed[..trimmed.len() - 1]);
                    input_buffer.push('\n');
                    in_multiline = true;
                    continue;
                }

                // If we're in multiline mode, append this line and process
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
                if input.starts_with(':') {
                    let parts: Vec<&str> = input.split_whitespace().collect();
                    let cmd = parts.first().unwrap_or(&"");

                    match *cmd {
                        ":quit" | ":q" => break,
                        ":help" | ":h" => {
                            println!("REPL Commands:");
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
                            println!("(Type inference not yet implemented for: {})", expr_str);
                            println!("Hint: Full type checking will be available soon!");
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
                                    let current_env = env.borrow().clone();
                                    let mut evaluator =
                                        AstEvaluator::with_env(Rc::new(current_env));
                                    match evaluator.eval_file(&ast) {
                                        Ok(_) => {
                                            // Extract and store new bindings
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
                let prepared_input = prepare_repl_input(input);
                let is_expr_wrapped = prepared_input.starts_with("let __expr__ = ");

                // Parse the input
                let (ast, diagnostics) = parse(&prepared_input);

                if !diagnostics.is_empty() {
                    for diag in &diagnostics {
                        emit(input, "<repl>", diag);
                    }
                    input_buffer.clear();
                    continue;
                }

                // Evaluate with the persistent environment
                // We need to evaluate in a temporary scope to capture new bindings
                let result = {
                    // Clone the current environment for evaluation
                    let current_env = env.borrow().clone();
                    let mut evaluator = AstEvaluator::with_env(Rc::new(current_env));
                    evaluator.eval_file(&ast)
                };

                match result {
                    Ok(value) => {
                        // After successful evaluation, we need to extract new bindings
                        // from the AST and add them to our persistent environment
                        for item in &ast.items {
                            if let neve_syntax::ItemKind::Let(let_def) = &item.kind {
                                // Extract the binding name from the pattern
                                if let PatternKind::Var(ident) = &let_def.pattern.kind {
                                    // Re-evaluate just this binding in the persistent env
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
                                let current_env = env.borrow().clone();
                                let mut temp_eval = AstEvaluator::with_env(Rc::new(current_env));

                                // Create a closure value for the function
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
                        if is_expr_wrapped || !matches!(value, Value::Unit) {
                            println!("{:?}", value);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: {:?}", e);
                    }
                }

                // Clear buffer after processing
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
fn prepare_repl_input(input: &str) -> String {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return String::new();
    }

    // Check if it's already a valid item (starts with keyword)
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
        if trimmed.ends_with(';') {
            trimmed.to_string()
        } else {
            format!("{trimmed};")
        }
    } else {
        // It's an expression, wrap it as a let binding
        format!("let __expr__ = {trimmed};")
    }
}
