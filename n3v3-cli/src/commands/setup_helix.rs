//! Helix editor setup — installs grammar, queries, and config.
//! Helix 编辑器设置 — 安装语法、查询和配置。

use std::fs;
use std::path::{Path, PathBuf};

// Pre-compiled tree-sitter grammar embedded in binary.
// 预编译的 tree-sitter 语法嵌入二进制文件。
const GRAMMAR_SO: &[u8] = include_bytes!("../../tree-sitter-n3v3/n3v3.so");

// Tree-sitter query files embedded in binary.
// tree-sitter 查询文件嵌入二进制文件。
const HIGHLIGHTS: &str = include_str!("../../tree-sitter-n3v3/queries/highlights.scm");
const LOCALS: &str = include_str!("../../tree-sitter-n3v3/queries/locals.scm");
const INDENTS: &str = include_str!("../../tree-sitter-n3v3/queries/indents.scm");
const TEXTOBJECTS: &str = include_str!("../../tree-sitter-n3v3/queries/textobjects.scm");
const INJECTIONS: &str = include_str!("../../tree-sitter-n3v3/queries/injections.scm");
const FOLDS: &str = include_str!("../../tree-sitter-n3v3/queries/folds.scm");

/// Canonical languages.toml snippet — always kept in sync with editors/helix/languages.toml.
/// 权威的 languages.toml 片段 — 始终与 editors/helix/languages.toml 保持同步。
const LANGUAGES_TOML_SNIPPET: &str = r#"[[language]]
name = "n3v3"
scope = "source.n3v3"
file-types = ["n3v3"]
comment-token = "--"
indent = { tab-width = 4, unit = "    " }
language-servers = ["n3v3-lsp"]
auto-format = true

[language-server.n3v3-lsp]
command = "n3v3"
args = ["lsp"]
"#;

pub fn run() -> Result<(), String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    let home = PathBuf::from(home);
    let helix = home.join(".config").join("helix");

    // 1. Install tree-sitter grammar shared library.
    // 安装 tree-sitter 语法共享库。
    let grammar_dir = helix.join("runtime").join("grammars");
    fs::create_dir_all(&grammar_dir).map_err(|e| format!("mkdir grammar: {e}"))?;
    fs::write(grammar_dir.join("n3v3.so"), GRAMMAR_SO)
        .map_err(|e| format!("write grammar: {e}"))?;
    println!("  ok grammar");

    // 2. Install all tree-sitter query files.
    // 安装所有 tree-sitter 查询文件。
    let queries_dir = helix.join("runtime").join("queries").join("n3v3");
    fs::create_dir_all(&queries_dir).map_err(|e| format!("mkdir queries: {e}"))?;
    fs::write(queries_dir.join("highlights.scm"), HIGHLIGHTS)
        .map_err(|e| format!("write highlights: {e}"))?;
    fs::write(queries_dir.join("locals.scm"), LOCALS).map_err(|e| format!("write locals: {e}"))?;
    fs::write(queries_dir.join("indents.scm"), INDENTS)
        .map_err(|e| format!("write indents: {e}"))?;
    fs::write(queries_dir.join("textobjects.scm"), TEXTOBJECTS)
        .map_err(|e| format!("write textobjects: {e}"))?;
    fs::write(queries_dir.join("injections.scm"), INJECTIONS)
        .map_err(|e| format!("write injections: {e}"))?;
    fs::write(queries_dir.join("folds.scm"), FOLDS).map_err(|e| format!("write folds: {e}"))?;
    println!("  ok queries (highlights, locals, indents, textobjects, injections, folds)");

    // 3. Configure languages.toml — add or update the n3v3 section.
    // 配置 languages.toml — 添加或更新 n3v3 部分。
    configure_languages_toml(&helix)?;

    println!("\nDone. Open a .n3v3 file with: hx file.n3v3");
    Ok(())
}

/// Add or update the n3v3 section in the user's languages.toml.
/// 在用户 languages.toml 中添加或更新 n3v3 部分。
///
/// Strategy:
/// 1. If no config file exists, create it with the n3v3 snippet.
/// 2. If config exists but no n3v3 section, append the snippet.
/// 3. If a n3v3 section already exists, replace it with the canonical version.
fn configure_languages_toml(helix_dir: &Path) -> Result<(), String> {
    let cfg = helix_dir.join("languages.toml");
    let existing = fs::read_to_string(&cfg).unwrap_or_default();

    let has_n3v3 = existing.contains("name = \"n3v3\"");

    let content = if existing.is_empty() {
        // Brand-new config. / 全新配置。
        println!("  ok languages.toml (new)");
        LANGUAGES_TOML_SNIPPET.to_string()
    } else if !has_n3v3 {
        // Append to existing config. / 追加到现有配置。
        println!("  ok languages.toml (added)");
        format!("{existing}\n{LANGUAGES_TOML_SNIPPET}")
    } else {
        // Replace the existing n3v3 section. / 替换现有的 n3v3 部分。
        println!("  ok languages.toml (updated)");
        replace_n3v3_block(&existing)
    };

    fs::write(&cfg, content).map_err(|e| format!("write languages.toml: {e}"))?;
    Ok(())
}

/// Replace the n3v3 TOML block in the existing content with the canonical snippet.
/// 将现有内容中的 n3v3 TOML 块替换为权威片段。
///
/// Scans for the `[[language]]` block that contains `name = "n3v3"` (plus its
/// `[language-server.n3v3-lsp]` subsection) and replaces the entire block.
/// 扫描包含 `name = "n3v3"` 的 `[[language]]` 块（及其
/// `[language-server.n3v3-lsp]` 子节），并替换整个块。
fn replace_n3v3_block(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut output = Vec::<String>::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Is this the start of the n3v3 [[language]] block?
        // 这是 n3v3 [[language]] 块的开始吗？
        if trimmed == "[[language]]" && is_n3v3_block(&lines, i) {
            // Emit the canonical snippet once.
            // 一次性输出权威片段。
            output.push(LANGUAGES_TOML_SNIPPET.to_string());
            // Skip the [[language]] block and its [language-server] subsection.
            // 跳过 [[language]] 块及其 [language-server] 子节。
            i += 1;
            while i < lines.len() {
                let next = lines[i].trim();
                if next.starts_with("[[") && !next.contains("n3v3") {
                    // Next [[section]] — back to normal. / 下一个 [[section]] — 恢复正常。
                    break;
                }
                if next.starts_with('[') && !next.starts_with("[[") && !next.contains("n3v3-lsp") {
                    // A non-n3v3 subsection — back to normal. / 非 n3v3 子节 — 恢复正常。
                    // But first check if this line is still part of n3v3's lsp config.
                    if next == "[language-server.n3v3-lsp]" {
                        i += 1;
                        continue;
                    }
                    break;
                }
                i += 1;
            }
        } else {
            output.push(line.to_string());
            i += 1;
        }
    }

    output.join("\n")
}

/// Check whether the [[language]] block starting at `start_idx` is the n3v3 block.
/// 检查从 `start_idx` 开始的 [[language]] 块是否是 n3v3 块。
fn is_n3v3_block(lines: &[&str], start_idx: usize) -> bool {
    // Look ahead up to 20 lines for `name = "n3v3"`.
    // 在接下来的 20 行中查找 `name = "n3v3"`。
    let end = (start_idx + 20).min(lines.len());
    for line in lines.iter().take(end).skip(start_idx + 1) {
        let line = line.trim();
        if line.starts_with("[[") || (line.starts_with('[') && !line.starts_with("[[")) {
            // Next section started without finding n3v3. / 下一个节开始了，没有找到 n3v3。
            return false;
        }
        if line.contains("name = \"n3v3\"") || line == "name = \"n3v3\"" {
            return true;
        }
    }
    false
}
