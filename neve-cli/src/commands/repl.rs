//! The `neve repl` command.

use std::rc::Rc;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use neve_parser::parse;
use neve_diagnostic::emit;
use neve_eval::{AstEvaluator, AstEnv, Value};

pub fn run() -> Result<(), String> {
    println!("Neve REPL v{}", env!("CARGO_PKG_VERSION"));
    println!("Type :help for help, :quit to exit");
    println!();

    let mut rl = DefaultEditor::new().map_err(|e| e.to_string())?;
    
    // Create a persistent environment for the REPL session
    let env = Rc::new(AstEnv::with_builtins());

    loop {
        let readline = rl.readline("neve> ");
        match readline {
            Ok(line) => {
                let line = line.trim();

                if line.is_empty() {
                    continue;
                }

                let _ = rl.add_history_entry(line);

                // Handle REPL commands
                if line.starts_with(':') {
                    match line {
                        ":quit" | ":q" => break,
                        ":help" | ":h" => {
                            println!("Commands:");
                            println!("  :help, :h    Show this help");
                            println!("  :quit, :q    Exit the REPL");
                            println!("  :env         Show current bindings");
                            continue;
                        }
                        ":env" => {
                            println!("(environment inspection not yet implemented)");
                            continue;
                        }
                        _ => {
                            println!("Unknown command: {}", line);
                            continue;
                        }
                    }
                }

                // Parse the input
                let (ast, diagnostics) = parse(line);

                if !diagnostics.is_empty() {
                    for diag in &diagnostics {
                        emit(line, "<repl>", diag);
                    }
                    continue;
                }

                // Evaluate with the persistent environment
                let mut evaluator = AstEvaluator::with_env(env.clone());
                
                match evaluator.eval_file(&ast) {
                    Ok(value) => {
                        // Update environment with any new bindings
                        // For now, we create a new env each time (not ideal but works)
                        
                        // Print non-unit results
                        if !matches!(value, Value::Unit) {
                            println!("{:?}", value);
                        }
                    }
                    Err(e) => {
                        eprintln!("Error: {:?}", e);
                    }
                }
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
