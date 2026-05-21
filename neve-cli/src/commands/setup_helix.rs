//! Helix editor setup command — no tree-sitter needed.
//! Helix 编辑器设置命令 — 无需 tree-sitter。

use std::fs;
use std::path::PathBuf;

pub fn run() -> Result<(), String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set")?;
    let home = PathBuf::from(home);
    let cfg = home.join(".config/helix/languages.toml");

    let config = r#"# Neve language support
[[language]]
name = "neve"
scope = "source.neve"
file-types = ["neve"]
comment-token = "--"
indent = { tab-width = 4, unit = "    " }
language-servers = ["neve-lsp"]

[language-server.neve-lsp]
command = "neve"
args = ["lsp"]
"#;

    let existing = fs::read_to_string(&cfg).unwrap_or_default();
    if existing.contains("name = \"neve\"") {
        println!("Helix already configured for Neve.");
        return Ok(());
    }

    let content = if existing.is_empty() {
        config.to_string()
    } else {
        format!("{existing}\n{config}")
    };

    fs::write(&cfg, content).map_err(|e| format!("write: {e}"))?;

    println!("✅ Helix configured for Neve.");
    println!("   LSP: diagnostics, completions, hover");
    println!("   Syntax highlighting: install tree-sitter-neve separately");
    println!("   Open: hx file.neve");
    Ok(())
}
