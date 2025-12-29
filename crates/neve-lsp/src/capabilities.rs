//! LSP server capabilities.
//! LSP 服务器能力。
//!
//! Defines what features the language server supports.
//! 定义语言服务器支持的功能。

use tower_lsp::lsp_types::*;

/// Get the server capabilities.
/// 获取服务器能力。
pub fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        // Text document sync / 文本文档同步
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                will_save: Some(false),
                will_save_wait_until: Some(false),
                save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                    include_text: Some(true),
                })),
            },
        )),

        // Hover / 悬停
        hover_provider: Some(HoverProviderCapability::Simple(true)),

        // Completion / 补全
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
            resolve_provider: Some(false),
            work_done_progress_options: WorkDoneProgressOptions::default(),
            all_commit_characters: None,
            completion_item: None,
        }),

        // Signature help / 签名帮助
        signature_help_provider: Some(SignatureHelpOptions {
            trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
            retrigger_characters: None,
            work_done_progress_options: WorkDoneProgressOptions::default(),
        }),

        // Go to definition / 跳转到定义
        definition_provider: Some(OneOf::Left(true)),

        // Find references / 查找引用
        references_provider: Some(OneOf::Left(true)),

        // Rename / 重命名
        rename_provider: Some(OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: WorkDoneProgressOptions::default(),
        })),

        // Document symbols / 文档符号
        document_symbol_provider: Some(OneOf::Left(true)),

        // Document formatting / 文档格式化
        document_formatting_provider: Some(OneOf::Left(true)),

        // Workspace symbol / 工作区符号
        workspace_symbol_provider: Some(OneOf::Left(true)),

        // Semantic tokens / 语义 token
        semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
            SemanticTokensOptions {
                work_done_progress_options: WorkDoneProgressOptions::default(),
                legend: SemanticTokensLegend {
                    token_types: vec![
                        SemanticTokenType::KEYWORD,
                        SemanticTokenType::VARIABLE,
                        SemanticTokenType::FUNCTION,
                        SemanticTokenType::TYPE,
                        SemanticTokenType::STRING,
                        SemanticTokenType::NUMBER,
                        SemanticTokenType::COMMENT,
                        SemanticTokenType::OPERATOR,
                        SemanticTokenType::PARAMETER,
                        SemanticTokenType::PROPERTY,
                    ],
                    token_modifiers: vec![
                        SemanticTokenModifier::DECLARATION,
                        SemanticTokenModifier::DEFINITION,
                        SemanticTokenModifier::READONLY,
                    ],
                },
                range: Some(false),
                full: Some(SemanticTokensFullOptions::Bool(true)),
            },
        )),

        ..Default::default()
    }
}
