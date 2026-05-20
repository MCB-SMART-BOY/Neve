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
