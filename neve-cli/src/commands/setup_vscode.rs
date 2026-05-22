//! VS Code editor setup — copies extension files to the VS Code extensions directory.
//! VS Code 编辑器设置 — 将扩展文件复制到 VS Code 扩展目录。

use std::fs;
use std::path::{Path, PathBuf};

pub fn run() -> Result<(), String> {
    let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
    let home = PathBuf::from(home);

    // VS Code extension directory
    let vscode_ext_dir = home
        .join(".vscode")
        .join("extensions")
        .join("neve-lang.neve");

    fs::create_dir_all(&vscode_ext_dir).map_err(|e| format!("mkdir vscode extension: {e}"))?;

    // Copy extension files
    let source_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("editors")
        .join("vscode");

    let files_to_copy = ["package.json", "language-configuration.json"];

    for file in &files_to_copy {
        let src = source_dir.join(file);
        let dst = vscode_ext_dir.join(file);
        if src.exists() {
            fs::copy(&src, &dst).map_err(|e| format!("copy {file}: {e}"))?;
            println!("  ok {file}");
        }
    }

    // Copy syntax directory
    let syntax_src = source_dir.join("syntaxes");
    let syntax_dst = vscode_ext_dir.join("syntaxes");
    if syntax_src.exists() {
        copy_dir(&syntax_src, &syntax_dst)?;
        println!("  ok syntaxes/");
    }

    println!("\nVS Code extension installed to:");
    println!("  {}", vscode_ext_dir.display());
    println!("\nTo complete setup:");
    println!(
        "  1. Install dependencies: cd {} && npm install",
        vscode_ext_dir.display()
    );
    println!(
        "  2. Compile TypeScript: cd {} && npm run compile",
        vscode_ext_dir.display()
    );
    println!("  3. Restart VS Code or reload window (Ctrl+Shift+P → Reload Window)");

    Ok(())
}

fn copy_dir(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("mkdir {dst:?}: {e}"))?;
    for entry in fs::read_dir(src).map_err(|e| format!("read_dir {src:?}: {e}"))? {
        let entry = entry.map_err(|e| format!("entry: {e}"))?;
        let path = entry.path();
        let dest = dst.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_dir(&path, &dest)?;
        } else {
            fs::copy(&path, &dest).map_err(|e| format!("copy {path:?}: {e}"))?;
        }
    }
    Ok(())
}
