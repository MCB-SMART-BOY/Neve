//! Helix editor setup — installs grammar, queries, and config.
//! Helix 编辑器设置 — 安装语法、查询和配置。

use std::fs;

// Pre-compiled tree-sitter grammar embedded in binary
const GRAMMAR_SO: &[u8] = include_bytes!("../../../tree-sitter-neve/neve.so");
use std::path::PathBuf;

const HIGHLIGHTS: &str = r#"; Neve syntax highlighting
[
  "let" "fn" "if" "then" "else" "match" "import" "as"
  "type" "struct" "enum" "trait" "impl" "pub" "effect" "lazy"
] @keyword

(fn_def name: (ident) @function)
(struct_def name: (ident) @type)
(enum_def name: (ident) @type)

(number) @constant.numeric
(string) @string
(comment) @comment
("true") @constant.builtin
("false") @constant.builtin

[ "+" "-" "*" "/" "==" "!=" "<" ">" "|>" "=" "->" ] @operator
[ "(" ")" "[" "]" "{" "}" "," ";" ":" ] @punctuation.delimiter
"#;

const LANGUAGES_TOML: &str = r#"# Neve (auto-generated)
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

pub fn run() -> Result<(), String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    let home = PathBuf::from(home);
    let helix = home.join(".config").join("helix");

    // 1. Install tree-sitter grammar
    let grammar_dir = helix.join("runtime").join("grammars");
    fs::create_dir_all(&grammar_dir).map_err(|e| format!("mkdir grammar: {e}"))?;
    install_grammar_so(&grammar_dir.join("neve.so"))?;
    println!("  ok grammar");

    // 2. Install highlights
    let queries_dir = helix.join("runtime").join("queries").join("neve");
    fs::create_dir_all(&queries_dir).map_err(|e| format!("mkdir queries: {e}"))?;
    fs::write(queries_dir.join("highlights.scm"), HIGHLIGHTS).map_err(|e| format!("write: {e}"))?;
    println!("  ok highlights");

    // 3. Configure languages.toml
    let cfg = helix.join("languages.toml");
    let existing = fs::read_to_string(&cfg).unwrap_or_default();
    if !existing.contains("name = \"neve\"") {
        let content = if existing.is_empty() {
            LANGUAGES_TOML.to_string()
        } else {
            format!("{existing}\n{LANGUAGES_TOML}")
        };
        fs::write(&cfg, content).map_err(|e| format!("write config: {e}"))?;
        println!("  ok languages.toml");
    }

    println!("\nDone. Open: hx file.neve");
    Ok(())
}

fn install_grammar_so(dst: &PathBuf) -> Result<(), String> {
    fs::write(dst, GRAMMAR_SO).map_err(|e| format!("write grammar: {e}"))
}
