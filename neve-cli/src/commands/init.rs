//! Initialize a new Neve project.
//! 初始化新的 Neve 项目。

use std::fs;
use std::path::Path;

pub fn run(dir: &str) -> Result<(), String> {
    let dir = Path::new(dir);
    fs::create_dir_all(dir).map_err(|e| format!("mkdir: {e}"))?;

    // flake.neve
    let flake = format!(
        r#"{{
    description = "A Neve project";
    name = "{}";
    version = "0.1.0";

    inputs = {{}};

    outputs = fn(inputs) {{
        let pkgs = {{}};
        let checks = {{
            default = fn() {{ true }},
        }};
        {{ packages = pkgs, checks = checks }}
    }};
}}"#,
        dir.file_name()
            .unwrap_or("my-project".as_ref())
            .to_string_lossy()
    );

    fs::write(dir.join("flake.neve"), flake).map_err(|e| format!("write flake.neve: {e}"))?;

    // main.neve
    let main = format!(
        r#"#!/usr/bin/env neve run
-- {name} — main entry point
import std.io as io;

fn main() effect = {{
    let (args, _) = io.args();
    let name = match args {{
        [n, ..] -> n,
        [] -> "World"
    }};
    io.println("Hello, " ++ name ++ "!");
    0
}};
"#,
        name = dir
            .file_name()
            .unwrap_or("my-project".as_ref())
            .to_string_lossy()
    );

    fs::write(dir.join("main.neve"), main).map_err(|e| format!("write main.neve: {e}"))?;

    // .gitignore
    fs::write(dir.join(".gitignore"), "result\n.direnv\n").ok();

    println!("✅ Created Neve project in {}", dir.display());
    println!("   cd {} && neve run main.neve", dir.display());
    Ok(())
}
