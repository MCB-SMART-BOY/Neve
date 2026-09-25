//! Initialize a new n3v3 project.
//! 初始化新的 n3v3 项目。

use std::fs;
use std::path::Path;

pub fn run(dir: &str) -> Result<(), String> {
    let dir = Path::new(dir);
    fs::create_dir_all(dir).map_err(|e| format!("mkdir: {e}"))?;

    // flake.n3v3
    let flake = format!(
        r#"{{
    description = "An n3v3 project",
    name = "{}",
    version = "0.1.0",

    inputs = {{}},

    outputs = fn(inputs) {{
        let pkgs = {{}};
        let checks = {{
            default = fn() {{ true }},
        }};
        {{ packages = pkgs, checks = checks }}
    }},
}}"#,
        dir.file_name()
            .unwrap_or("my-project".as_ref())
            .to_string_lossy()
    );

    fs::write(dir.join("flake.n3v3"), flake).map_err(|e| format!("write flake.n3v3: {e}"))?;

    // main.n3v3
    let main = format!(
        r#"#!/usr/bin/env n3v3 run
-- {name} — main entry point
use std.io = io;

let (args, _) = io.args();
let name = match args {{
    [n, ..] -> n,
    [] -> "World"
}};
io.println("Hello, " ++ name ++ "!");
"#,
        name = dir
            .file_name()
            .unwrap_or("my-project".as_ref())
            .to_string_lossy()
    );

    fs::write(dir.join("main.n3v3"), main).map_err(|e| format!("write main.n3v3: {e}"))?;

    // .gitignore
    fs::write(dir.join(".gitignore"), "result\n.direnv\n")
        .map_err(|e| format!("write .gitignore: {e}"))?;

    println!("✅ Created n3v3 project in {}", dir.display());
    println!("   cd {} && n3v3 run main.n3v3", dir.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn run_writes_canonical_main_source() {
        let dir = tempfile::tempdir().expect("temporary project directory");
        run(dir.path().to_str().expect("temporary path should be UTF-8"))
            .expect("init should create the project");

        let main = std::fs::read_to_string(dir.path().join("main.n3v3"))
            .expect("generated main.n3v3 should be readable");
        assert!(main.contains("use std.io = io;"));
        assert!(main.contains("let (args, _) = io.args();"));
        assert!(!main.contains("fn main() ="));
        assert!(!main.contains("import std.io"));
        assert!(!main.contains("effect ="));

        let analysis = n3v3_frontend::analyze_source(&main);

        let has_errors = analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == n3v3_diagnostic::Severity::Error);
        assert!(
            !has_errors,
            "generated source should type-check: {:?}",
            analysis.diagnostics
        );
    }

    #[cfg(unix)]
    #[test]
    fn run_writes_loadable_flake_source() {
        let dir = tempfile::tempdir().expect("temporary project directory");
        run(dir.path().to_str().expect("temporary path should be UTF-8"))
            .expect("init should create the project");

        let flake = n3v3_config::flake::Flake::load(dir.path())
            .expect("generated flake.n3v3 should evaluate through frontend/HIR");
        assert_eq!(flake.description.as_deref(), Some("An n3v3 project"));
        assert!(
            flake.outputs.is_some(),
            "generated flake should define outputs"
        );
    }
}
