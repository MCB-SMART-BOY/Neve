//! Language Server Protocol implementation for Neve.
//!
//! This crate provides an LSP server for Neve, enabling IDE features like:
//! - Syntax highlighting (via semantic tokens)
//! - Diagnostics (parse and type errors)
//! - Hover information
//! - Go to definition
//! - Code completion
//! - Formatting

mod backend;
mod capabilities;

pub mod document;
pub mod semantic_tokens;
pub mod symbol_index;

pub use backend::Backend;
pub use document::{Document, Diagnostic, DiagnosticSeverity};
pub use semantic_tokens::{
    generate_semantic_tokens, 
    generate_semantic_tokens_with_context,
    parameter_token_type,
    comment_token_type,
    token_types,
    token_modifiers,
};
pub use symbol_index::{SymbolIndex, Symbol, SymbolKind, SymbolRef};

use tower_lsp::{LspService, Server};

/// Run the LSP server.
pub async fn run_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    
    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}

/// Run the LSP server with custom I/O.
pub async fn run_server_with_io<I, O>(input: I, output: O)
where
    I: tokio::io::AsyncRead + Unpin,
    O: tokio::io::AsyncWrite + Unpin,
{
    let (service, socket) = LspService::new(Backend::new);
    Server::new(input, output, socket).serve(service).await;
}
