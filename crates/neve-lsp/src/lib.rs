//! Language Server Protocol implementation for Neve.
//! Neve 的语言服务器协议实现。
//!
//! This crate provides an LSP server for Neve, enabling IDE features like:
//! 本 crate 提供 Neve 的 LSP 服务器，支持以下 IDE 功能：
//!
//! - Syntax highlighting (via semantic tokens) / 语法高亮（通过语义 token）
//! - Diagnostics (parse and type errors) / 诊断（解析和类型错误）
//! - Hover information / 悬停信息
//! - Go to definition / 跳转到定义
//! - Code completion / 代码补全
//! - Formatting / 格式化

mod backend;
mod capabilities;

pub mod document;
pub mod semantic_tokens;
pub mod symbol_index;

pub use backend::Backend;
pub use document::{Diagnostic, DiagnosticSeverity, Document};
pub use semantic_tokens::{
    comment_token_type, generate_semantic_tokens, generate_semantic_tokens_with_context,
    parameter_token_type, token_modifiers, token_types,
};
pub use symbol_index::{Symbol, SymbolIndex, SymbolKind, SymbolRef};

use tower_lsp::{LspService, Server};

/// Run the LSP server.
/// 运行 LSP 服务器。
pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

/// Run the LSP server with custom I/O.
/// 使用自定义 I/O 运行 LSP 服务器。
pub async fn run_server_with_io<I, O>(input: I, output: O)
where
    I: tokio::io::AsyncRead + Unpin,
    O: tokio::io::AsyncWrite + Unpin,
{
    let (service, socket) = LspService::new(Backend::new);
    Server::new(input, output, socket).serve(service).await;
}
