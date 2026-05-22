//! LSP server command.
//! LSP 服务器命令。

use neve_lsp::run_server;

/// Start the Neve Language Server.
/// 启动 Neve 语言服务器。
pub fn run() -> Result<(), String> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("failed to create tokio runtime: {e}"))?;
    rt.block_on(async {
        run_server().await;
    });
    Ok(())
}

/// Run a health check on the LSP subsystem.
/// 运行 LSP 子系统健康检查。
pub fn version() -> Result<(), String> {
    println!("Neve LSP Version Info");
    println!("====================\n");
    println!("  Binary:    neve {}", env!("CARGO_PKG_VERSION"));
    println!("  Protocol:  LSP 3.17");
    println!("  Transport: stdio (JSON-RPC 2.0)\n");
    println!("Capabilities:");
    println!("  hover              ✓");
    println!("  completion         ✓ (9 categories, type-aware)");
    println!("  completion/resolve ✓ (77 functions)");
    println!("  signatureHelp      ✓ (80 signatures)");
    println!("  definition         ✓");
    println!("  references         ✓");
    println!("  documentHighlight  ✓");
    println!("  rename             ✓");
    println!("  formatting         ✓");
    println!("  documentSymbol     ✓");
    println!("  semanticTokens     ✓ (10 types, AST-based)");
    println!("  inlayHint          ✓");
    println!("  foldingRange       ✓");
    println!("  codeAction         ✓");
    println!("  workspace/symbol   ✓");
    println!("\nEditor integration:");
    println!("  Helix:  neve setup helix");
    println!("  VS Code: neve setup vscode");
    println!("\nRun `neve lsp --check` for a full health diagnostic.");
    Ok(())
}

pub fn check() -> Result<(), String> {
    println!("Neve LSP Health Check");
    println!("====================\n");

    // 1. Check that the LSP crate can be loaded.
    // 检查 LSP 包是否可以加载。
    println!("  ✓ neve-lsp crate loaded (v{})", env!("CARGO_PKG_VERSION"));

    // 2. Test document parsing.
    // 测试文档解析。
    let doc = neve_lsp::Document::new(
        "file:///test.neve".to_string(),
        "let x = 42;\nlet y = x + 1;\n".to_string(),
    );
    let parse_ok = doc.ast.is_some();
    let hir_ok = doc.hir.is_some();
    let has_semantics = doc.semantics.is_some();
    let has_index = doc.symbol_index.is_some();

    println!(
        "  {} Document parse (AST: {}, HIR: {}, Semantics: {}, Index: {})",
        if parse_ok && hir_ok && has_semantics && has_index {
            "✓"
        } else {
            "✗"
        },
        if parse_ok { "yes" } else { "no" },
        if hir_ok { "yes" } else { "no" },
        if has_semantics { "yes" } else { "no" },
        if has_index { "yes" } else { "no" },
    );

    // 3. Test diagnostics.
    // 测试诊断。
    let error_doc = neve_lsp::Document::new(
        "file:///error.neve".to_string(),
        "let x = 42;\nlet y = x + \"hello\";\n".to_string(),
    );
    let has_diagnostics = !error_doc.diagnostics.is_empty();
    println!(
        "  {} Diagnostics ({} issues found)",
        if has_diagnostics { "✓" } else { "✗" },
        error_doc.diagnostics.len(),
    );
    for diag in &error_doc.diagnostics {
        println!(
            "    - {}: {}",
            match diag.severity {
                neve_lsp::DiagnosticSeverity::Error => "error",
                neve_lsp::DiagnosticSeverity::Warning => "warning",
                neve_lsp::DiagnosticSeverity::Note => "note",
            },
            diag.message.lines().next().unwrap_or("")
        );
    }

    // 4. Test semantic tokens.
    // 测试语义 Token。
    let tokens =
        neve_lsp::generate_semantic_tokens_from_ast("let x = 42;\nfn add(a, b) = a + b;\n");
    let has_tokens = !tokens.is_empty();
    println!(
        "  {} Semantic tokens ({} tokens generated)",
        if has_tokens { "✓" } else { "✗" },
        tokens.len(),
    );

    // 5. Test formatting.
    // 测试格式化。
    let formatted = neve_fmt::format("let x=42;");
    let fmt_ok = formatted.is_ok();
    println!(
        "  {} Formatter (result: {})",
        if fmt_ok { "✓" } else { "✗" },
        if fmt_ok { "ok" } else { "error" },
    );

    // 6. Test symbol index.
    // 测试符号索引。
    let index_doc = neve_lsp::Document::new(
        "file:///index.neve".to_string(),
        "let x = 1;\nfn hello() = \"world\";\nstruct Point { x: Int, y: Int };\n".to_string(),
    );
    if let Some(ref index) = index_doc.symbol_index {
        let def_count = index.definitions.len();
        println!("  ✓ Symbol index ({} definitions)", def_count);
        for (name, syms) in &index.definitions {
            for sym in syms {
                println!("    - {}: {:?}", name, sym.kind);
            }
        }
    } else {
        println!("  ✗ Symbol index (not built)");
    }

    // 7. Check Helix integration.
    // 检查 Helix 集成。
    println!();
    if let Ok(home) = std::env::var("HOME") {
        let hd = std::path::PathBuf::from(&home)
            .join(".config")
            .join("helix");
        let grammar = hd.join("runtime").join("grammars").join("neve.so");
        let queries_dir = hd.join("runtime").join("queries").join("neve");
        let languages_toml = hd.join("languages.toml");

        let has_grammar = grammar.exists();
        let has_queries = queries_dir.exists()
            && queries_dir.join("highlights.scm").exists()
            && queries_dir.join("locals.scm").exists()
            && queries_dir.join("indents.scm").exists();
        let has_config = languages_toml.exists()
            && std::fs::read_to_string(&languages_toml)
                .unwrap_or_default()
                .contains("name = \"neve\"");

        println!(
            "  {} Helix grammar installed",
            if has_grammar { "✓" } else { "✗" },
        );
        println!(
            "  {} Helix queries installed",
            if has_queries { "✓" } else { "✗" },
        );
        println!(
            "  {} Helix config (languages.toml)",
            if has_config { "✓" } else { "✗" },
        );

        if !has_grammar || !has_queries || !has_config {
            println!("\n  Run `neve setup helix` to install Helix integration.");
        }
    }

    // 8. Summary.
    // 总结。
    let all_ok =
        parse_ok && hir_ok && has_semantics && has_index && has_diagnostics && has_tokens && fmt_ok;
    println!(
        "\n{} LSP health check: {}",
        if all_ok { "✓" } else { "✗" },
        if all_ok {
            "ALL CHECKS PASSED"
        } else {
            "SOME CHECKS FAILED"
        },
    );

    if all_ok {
        Ok(())
    } else {
        Err("health check failed".to_string())
    }
}
