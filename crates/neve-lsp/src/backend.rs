//! LSP backend implementation.
//! LSP 后端实现。
//!
//! Implements the Language Server Protocol for Neve.
//! 实现 Neve 的语言服务器协议。

use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use neve_lexer::Lexer;
use neve_syntax::{ImplDef, Type, TypeKind};

use crate::Document;
use crate::capabilities::server_capabilities;
use crate::semantic_tokens::generate_semantic_tokens_with_context;
use crate::symbol_index::SymbolKind as IndexSymbolKind;
use neve_common::Span;
use neve_diagnostic::{DiagnosticKind, Severity as NeveSeverity};

/// The LSP backend.
/// LSP 后端。
pub struct Backend {
    /// The LSP client. / LSP 客户端。
    client: Client,
    /// Open documents. / 打开的文档。
    documents: DashMap<String, Document>,
}

impl Backend {
    /// Create a new backend.
    /// 创建新的后端。
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
        }
    }

    /// Publish diagnostics for a document.
    /// 发布文档的诊断信息。
    async fn publish_diagnostics(&self, uri: &Url, doc: &Document) {
        let diagnostics: Vec<Diagnostic> = doc
            .diagnostics
            .iter()
            .map(|diag| {
                let severity = match diag.severity {
                    NeveSeverity::Error => DiagnosticSeverity::ERROR,
                    NeveSeverity::Warning => DiagnosticSeverity::WARNING,
                    NeveSeverity::Note => DiagnosticSeverity::INFORMATION,
                };

                let primary_span = if diag.span.is_empty() {
                    diag.labels
                        .first()
                        .map(|label| label.span)
                        .unwrap_or(diag.span)
                } else {
                    diag.span
                };

                let mut message = diag.message.clone();
                for note in &diag.notes {
                    message.push_str("\n\nnote: ");
                    message.push_str(note);
                }
                if let Some(help) = &diag.help {
                    message.push_str("\n\nhelp: ");
                    message.push_str(help);
                }

                let related_information = if diag.labels.is_empty() {
                    None
                } else {
                    Some(
                        diag.labels
                            .iter()
                            .map(|label| DiagnosticRelatedInformation {
                                location: Location {
                                    uri: uri.clone(),
                                    range: range_for_span(doc, label.span),
                                },
                                message: label.message.clone(),
                            })
                            .collect(),
                    )
                };

                let source = match diag.kind {
                    DiagnosticKind::Lexer => "neve.lexer",
                    DiagnosticKind::Parser => "neve.parser",
                    DiagnosticKind::Type => "neve.type",
                    DiagnosticKind::Eval => "neve.eval",
                    DiagnosticKind::Module => "neve.module",
                };

                let code_description = diag.code.and_then(|code| {
                    Url::parse(&code.doc_url())
                        .ok()
                        .map(|href| CodeDescription { href })
                });

                Diagnostic {
                    range: range_for_span(doc, primary_span),
                    severity: Some(severity),
                    code: diag
                        .code
                        .map(|code| NumberOrString::String(code.as_str().to_string())),
                    code_description,
                    source: Some(source.to_string()),
                    message,
                    related_information,
                    tags: None,
                    data: None,
                }
            })
            .collect();

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }
}

fn range_for_span(doc: &Document, span: Span) -> Range {
    let content_len = doc.content.len();
    let mut start: usize = span.start.into();
    let mut end: usize = span.end.into();

    if start > content_len {
        start = content_len;
    }
    if end > content_len {
        end = content_len;
    }
    if end < start {
        end = start;
    }

    if start == end
        && let Some(next) = next_char_offset(&doc.content, start)
    {
        end = next;
    }

    let (start_line, start_col) = doc.position_at(start);
    let (end_line, end_col) = doc.position_at(end);

    Range {
        start: Position::new(start_line, start_col),
        end: Position::new(end_line, end_col),
    }
}

fn next_char_offset(content: &str, offset: usize) -> Option<usize> {
    if offset >= content.len() {
        return None;
    }

    content[offset..]
        .chars()
        .next()
        .map(|ch| offset + ch.len_utf8())
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "neve-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: server_capabilities(),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Neve language server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let content = params.text_document.text;

        let doc = Document::new(uri.clone(), content);
        self.publish_diagnostics(&params.text_document.uri, &doc)
            .await;
        self.documents.insert(uri, doc);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();

        if let Some(mut doc) = self.documents.get_mut(&uri)
            && let Some(change) = params.content_changes.into_iter().next()
        {
            doc.update(change.text);
            self.publish_diagnostics(&params.text_document.uri, &doc)
                .await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri.to_string();

        if let Some(text) = params.text
            && let Some(mut doc) = self.documents.get_mut(&uri)
        {
            doc.update(text);
            self.publish_diagnostics(&params.text_document.uri, &doc)
                .await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        self.documents.remove(&uri);

        // Clear diagnostics / 清除诊断信息
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let pos = params.text_document_position_params.position;

        if let Some(doc) = self.documents.get(&uri) {
            let offset = doc.offset_at(pos.line, pos.character);

            // Try to get symbol information first / 首先尝试获取符号信息
            if let Some(ref index) = doc.symbol_index
                && let Some(symbol) = index.find_definition_site_at(offset)
            {
                // Format symbol kind nicely / 格式化符号类型
                let kind_str = match symbol.kind {
                    IndexSymbolKind::Function => "function",
                    IndexSymbolKind::Variable => "variable",
                    IndexSymbolKind::Parameter => "parameter",
                    IndexSymbolKind::TypeAlias => "type alias",
                    IndexSymbolKind::Struct => "struct",
                    IndexSymbolKind::Enum => "enum",
                    IndexSymbolKind::Variant => "variant",
                    IndexSymbolKind::Trait => "trait",
                    IndexSymbolKind::Field => "field",
                    IndexSymbolKind::Method => "method",
                };

                // Get the full definition text using full_span
                // 使用 full_span 获取完整的定义文本
                let full_start: usize = symbol.full_span.start.into();
                let full_end: usize = symbol.full_span.end.into();
                let definition_text = if full_end <= doc.content.len() {
                    // Limit to first line for display / 限制显示第一行
                    let full_text = &doc.content[full_start..full_end];
                    let first_line = full_text.lines().next().unwrap_or(full_text);
                    if first_line.len() > 80 {
                        format!("{}...", &first_line[..77])
                    } else {
                        first_line.to_string()
                    }
                } else {
                    symbol.name.clone()
                };

                let hover_text =
                    if let Some(type_info) = doc.definition_hovers.get(&symbol.def_span) {
                        format!(
                            "**{}** `{}`\n\nType: `{}`\n\n```neve\n{}\n```",
                            kind_str, symbol.name, type_info, definition_text
                        )
                    } else {
                        format!(
                            "**{}** `{}`\n\n```neve\n{}\n```",
                            kind_str, symbol.name, definition_text
                        )
                    };

                let start: usize = symbol.def_span.start.into();
                let end: usize = symbol.def_span.end.into();
                let (start_line, start_col) = doc.position_at(start);
                let (end_line, end_col) = doc.position_at(end);

                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: hover_text,
                    }),
                    range: Some(Range {
                        start: Position::new(start_line, start_col),
                        end: Position::new(end_line, end_col),
                    }),
                }));
            }

            if let Some((span, hover_text)) = doc.semantic_hover_at(offset) {
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!("`{hover_text}`"),
                    }),
                    range: Some(range_for_span(&doc, span)),
                }));
            }

            if let Some(ref index) = doc.symbol_index
                && let Some(symbol) = index.find_definition_at(offset)
                && let Some(type_info) = doc.definition_hovers.get(&symbol.def_span)
            {
                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!("`{type_info}`"),
                    }),
                    range: Some(range_for_span(&doc, symbol.def_span)),
                }));
            }

            // Fallback to token-based hover / 回退到基于 token 的悬停
            let lexer = Lexer::new(&doc.content);
            let (tokens, _) = lexer.tokenize();

            for token in tokens {
                let start: usize = token.span.start.into();
                let end: usize = token.span.end.into();
                if start <= offset && offset < end {
                    let token_text = &doc.content[start..end];
                    let hover_text = format!("Token: `{}`\nKind: `{:?}`", token_text, token.kind);

                    let (start_line, start_col) = doc.position_at(start);
                    let (end_line, end_col) = doc.position_at(end);

                    return Ok(Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: hover_text,
                        }),
                        range: Some(Range {
                            start: Position::new(start_line, start_col),
                            end: Position::new(end_line, end_col),
                        }),
                    }));
                }
            }
        }

        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let pos = params.text_document_position.position;

        let mut items = Vec::new();

        // Get context for smarter completion / 获取上下文以实现更智能的补全
        let trigger_char = params
            .context
            .as_ref()
            .and_then(|c| c.trigger_character.as_deref());

        // Check if we're after a dot (member access) / 检查是否在点后面（成员访问）
        let is_dot_completion = trigger_char == Some(".");

        if is_dot_completion {
            // Method completion based on type / 基于类型的方法补全
            items.extend(self.get_method_completions());
        } else {
            // Keywords / 关键字
            items.extend(self.get_keyword_completions());

            // Standard library functions / 标准库函数
            items.extend(self.get_stdlib_completions());

            // Types / 类型
            items.extend(self.get_type_completions());

            // Document symbols (variables, functions from current file)
            // 文档符号（当前文件中的变量、函数）
            if let Some(doc) = self.documents.get(&uri) {
                items.extend(self.get_document_completions(&doc, pos));
            }
        }

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri.to_string();

        if let Some(doc) = self.documents.get(&uri)
            && let Ok(formatted) = neve_fmt::format(&doc.content)
            && formatted != doc.content
        {
            let lines: Vec<&str> = doc.content.lines().collect();
            let end_line = lines.len().saturating_sub(1) as u32;
            let end_col = lines.last().map(|l| l.len() as u32).unwrap_or(0);

            return Ok(Some(vec![TextEdit {
                range: Range {
                    start: Position::new(0, 0),
                    end: Position::new(end_line, end_col),
                },
                new_text: formatted,
            }]));
        }

        Ok(None)
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri.to_string();

        if let Some(doc) = self.documents.get(&uri) {
            let lexer = Lexer::new(&doc.content);
            let (tokens, _) = lexer.tokenize();
            let semantic_tokens = generate_semantic_tokens_with_context(&tokens, &doc.content);

            return Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
                result_id: None,
                data: semantic_tokens,
            })));
        }

        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri.to_string();

        if let Some(doc) = self.documents.get(&uri)
            && let Some(ref ast) = doc.ast
        {
            let mut symbols = Vec::new();
            let span_range = |span: neve_common::Span| {
                let start: usize = span.start.into();
                let end: usize = span.end.into();
                let (start_line, start_col) = doc.position_at(start);
                let (end_line, end_col) = doc.position_at(end);
                Range {
                    start: Position::new(start_line, start_col),
                    end: Position::new(end_line, end_col),
                }
            };

            for item in &ast.items {
                use neve_syntax::ItemKind;

                let symbol = match &item.kind {
                    ItemKind::Let(def) => {
                        let name = format!("{:?}", def.pattern.kind);
                        #[allow(deprecated)]
                        DocumentSymbol {
                            name,
                            detail: None,
                            kind: SymbolKind::VARIABLE,
                            tags: None,
                            deprecated: None,
                            range: span_range(item.span),
                            selection_range: span_range(def.pattern.span),
                            children: None,
                        }
                    }
                    ItemKind::Fn(def) =>
                    {
                        #[allow(deprecated)]
                        DocumentSymbol {
                            name: def.name.name.clone(),
                            detail: None,
                            kind: SymbolKind::FUNCTION,
                            tags: None,
                            deprecated: None,
                            range: span_range(item.span),
                            selection_range: span_range(def.name.span),
                            children: None,
                        }
                    }
                    ItemKind::TypeAlias(def) =>
                    {
                        #[allow(deprecated)]
                        DocumentSymbol {
                            name: def.name.name.clone(),
                            detail: None,
                            kind: SymbolKind::TYPE_PARAMETER,
                            tags: None,
                            deprecated: None,
                            range: span_range(item.span),
                            selection_range: span_range(def.name.span),
                            children: None,
                        }
                    }
                    ItemKind::Struct(def) =>
                    {
                        #[allow(deprecated)]
                        DocumentSymbol {
                            name: def.name.name.clone(),
                            detail: None,
                            kind: SymbolKind::STRUCT,
                            tags: None,
                            deprecated: None,
                            range: span_range(item.span),
                            selection_range: span_range(def.name.span),
                            children: None,
                        }
                    }
                    ItemKind::Enum(def) =>
                    {
                        #[allow(deprecated)]
                        DocumentSymbol {
                            name: def.name.name.clone(),
                            detail: None,
                            kind: SymbolKind::ENUM,
                            tags: None,
                            deprecated: None,
                            range: span_range(item.span),
                            selection_range: span_range(def.name.span),
                            children: None,
                        }
                    }
                    ItemKind::Trait(def) => {
                        let mut children = Vec::new();

                        for assoc in &def.assoc_types {
                            #[allow(deprecated)]
                            children.push(DocumentSymbol {
                                name: assoc.name.name.clone(),
                                detail: None,
                                kind: SymbolKind::TYPE_PARAMETER,
                                tags: None,
                                deprecated: None,
                                range: span_range(assoc.span),
                                selection_range: span_range(assoc.name.span),
                                children: None,
                            });
                        }

                        for trait_item in &def.items {
                            #[allow(deprecated)]
                            children.push(DocumentSymbol {
                                name: trait_item.name.name.clone(),
                                detail: None,
                                kind: SymbolKind::METHOD,
                                tags: None,
                                deprecated: None,
                                range: span_range(trait_item.span),
                                selection_range: span_range(trait_item.name.span),
                                children: None,
                            });
                        }

                        #[allow(deprecated)]
                        DocumentSymbol {
                            name: def.name.name.clone(),
                            detail: None,
                            kind: SymbolKind::INTERFACE,
                            tags: None,
                            deprecated: None,
                            range: span_range(item.span),
                            selection_range: span_range(def.name.span),
                            children: Some(children),
                        }
                    }
                    ItemKind::Impl(def) => {
                        let mut children = Vec::new();

                        for assoc in &def.assoc_type_impls {
                            #[allow(deprecated)]
                            children.push(DocumentSymbol {
                                name: assoc.name.name.clone(),
                                detail: None,
                                kind: SymbolKind::TYPE_PARAMETER,
                                tags: None,
                                deprecated: None,
                                range: span_range(assoc.span),
                                selection_range: span_range(assoc.name.span),
                                children: None,
                            });
                        }

                        for impl_item in &def.items {
                            #[allow(deprecated)]
                            children.push(DocumentSymbol {
                                name: impl_item.name.name.clone(),
                                detail: None,
                                kind: SymbolKind::METHOD,
                                tags: None,
                                deprecated: None,
                                range: span_range(impl_item.span),
                                selection_range: span_range(impl_item.name.span),
                                children: None,
                            });
                        }

                        let impl_name = format_impl_name(def);
                        #[allow(deprecated)]
                        DocumentSymbol {
                            name: impl_name,
                            detail: None,
                            kind: SymbolKind::CLASS,
                            tags: None,
                            deprecated: None,
                            range: span_range(item.span),
                            selection_range: span_range(item.span),
                            children: Some(children),
                        }
                    }
                    ItemKind::Import(_) => continue,
                };

                symbols.push(symbol);
            }

            return Ok(Some(DocumentSymbolResponse::Nested(symbols)));
        }

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let pos = params.text_document_position_params.position;

        if let Some(doc) = self.documents.get(&uri)
            && let Some(ref index) = doc.symbol_index
        {
            let offset = doc.offset_at(pos.line, pos.character);

            if let Some(symbol) = index.find_definition_at(offset) {
                let start: usize = symbol.def_span.start.into();
                let end: usize = symbol.def_span.end.into();
                let (start_line, start_col) = doc.position_at(start);
                let (end_line, end_col) = doc.position_at(end);

                return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                    uri: params
                        .text_document_position_params
                        .text_document
                        .uri
                        .clone(),
                    range: Range {
                        start: Position::new(start_line, start_col),
                        end: Position::new(end_line, end_col),
                    },
                })));
            }
        }

        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let pos = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;

        if let Some(doc) = self.documents.get(&uri)
            && let Some(ref index) = doc.symbol_index
        {
            let offset = doc.offset_at(pos.line, pos.character);
            let refs = index.find_references_at(offset, include_declaration);

            if !refs.is_empty() {
                let locations: Vec<Location> = refs
                    .iter()
                    .map(|r| {
                        let start: usize = r.span.start.into();
                        let end: usize = r.span.end.into();
                        let (start_line, start_col) = doc.position_at(start);
                        let (end_line, end_col) = doc.position_at(end);

                        Location {
                            uri: params.text_document_position.text_document.uri.clone(),
                            range: Range {
                                start: Position::new(start_line, start_col),
                                end: Position::new(end_line, end_col),
                            },
                        }
                    })
                    .collect();

                return Ok(Some(locations));
            }
        }

        Ok(None)
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let pos = params.text_document_position.position;
        let new_name = params.new_name;

        if let Some(doc) = self.documents.get(&uri)
            && let Some(ref index) = doc.symbol_index
        {
            let offset = doc.offset_at(pos.line, pos.character);

            let refs = index.find_references_at(offset, true);

            if !refs.is_empty() {
                let edits: Vec<TextEdit> = refs
                    .iter()
                    .map(|r| {
                        let start: usize = r.span.start.into();
                        let end: usize = r.span.end.into();
                        let (start_line, start_col) = doc.position_at(start);
                        let (end_line, end_col) = doc.position_at(end);

                        TextEdit {
                            range: Range {
                                start: Position::new(start_line, start_col),
                                end: Position::new(end_line, end_col),
                            },
                            new_text: new_name.clone(),
                        }
                    })
                    .collect();

                let mut changes = std::collections::HashMap::new();
                changes.insert(
                    params.text_document_position.text_document.uri.clone(),
                    edits,
                );

                return Ok(Some(WorkspaceEdit {
                    changes: Some(changes),
                    document_changes: None,
                    change_annotations: None,
                }));
            }
        }

        Ok(None)
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri.to_string();
        let pos = params.position;

        if let Some(doc) = self.documents.get(&uri)
            && let Some(ref index) = doc.symbol_index
        {
            let offset = doc.offset_at(pos.line, pos.character);

            // Find the symbol at this position / 在此位置查找符号
            if let Some(reference) = index.find_reference_at(offset) {
                let start: usize = reference.span.start.into();
                let end: usize = reference.span.end.into();
                let (start_line, start_col) = doc.position_at(start);
                let (end_line, end_col) = doc.position_at(end);

                return Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
                    range: Range {
                        start: Position::new(start_line, start_col),
                        end: Position::new(end_line, end_col),
                    },
                    placeholder: reference.name.clone(),
                }));
            }
        }

        Ok(None)
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        let query = params.query.to_lowercase();
        let mut symbols = Vec::new();

        // Search across all open documents / 在所有打开的文档中搜索
        for entry in self.documents.iter() {
            let doc = entry.value();
            let uri =
                Url::parse(&doc.uri).unwrap_or_else(|_| Url::parse("file:///unknown").unwrap());

            if let Some(ref index) = doc.symbol_index {
                for (name, defs) in &index.definitions {
                    // Filter by query / 按查询过滤
                    if query.is_empty() || name.to_lowercase().contains(&query) {
                        for def in defs {
                            let start: usize = def.def_span.start.into();
                            let end: usize = def.def_span.end.into();
                            let (start_line, start_col) = doc.position_at(start);
                            let (end_line, end_col) = doc.position_at(end);

                            #[allow(deprecated)]
                            symbols.push(SymbolInformation {
                                name: name.clone(),
                                kind: convert_symbol_kind(def.kind),
                                tags: None,
                                deprecated: None,
                                location: Location {
                                    uri: uri.clone(),
                                    range: Range {
                                        start: Position::new(start_line, start_col),
                                        end: Position::new(end_line, end_col),
                                    },
                                },
                                container_name: None,
                            });
                        }
                    }
                }
            }
        }

        if symbols.is_empty() {
            Ok(None)
        } else {
            Ok(Some(symbols))
        }
    }
}

impl Backend {
    /// Get keyword completions.
    /// 获取关键字补全。
    fn get_keyword_completions(&self) -> Vec<CompletionItem> {
        let keywords = vec![
            ("let", "Let binding", "let ${1:name} = ${2:value};"),
            (
                "fn",
                "Function definition",
                "fn ${1:name}(${2:params}) = ${3:body};",
            ),
            (
                "if",
                "If expression",
                "if ${1:condition} then ${2:then_branch} else ${3:else_branch}",
            ),
            (
                "match",
                "Match expression",
                "match ${1:expr} {\n\t${2:pattern} -> ${3:body},\n}",
            ),
            ("type", "Type alias", "type ${1:Name} = ${2:Type};"),
            (
                "struct",
                "Struct definition",
                "struct ${1:Name} {\n\t${2:field}: ${3:Type},\n};",
            ),
            (
                "enum",
                "Enum definition",
                "enum ${1:Name} {\n\t${2:Variant},\n};",
            ),
            (
                "trait",
                "Trait definition",
                "trait ${1:Name} {\n\t${2:items}\n};",
            ),
            (
                "impl",
                "Implementation",
                "impl ${1:Trait} for ${2:Type} {\n\t${3:items}\n};",
            ),
            ("import", "Import statement", "import ${1:module};"),
            ("pub", "Public visibility", "pub "),
            ("lazy", "Lazy evaluation", "lazy ${1:expr}"),
            ("true", "Boolean true", "true"),
            ("false", "Boolean false", "false"),
        ];

        keywords
            .into_iter()
            .map(|(label, detail, snippet)| CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some(detail.to_string()),
                insert_text: Some(snippet.to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            })
            .collect()
    }

    /// Get standard library completions.
    /// 获取标准库补全。
    fn get_stdlib_completions(&self) -> Vec<CompletionItem> {
        let mut stdlib_functions = io_completion_specs();
        stdlib_functions.extend(math_completion_specs());
        stdlib_functions.extend(vec![
            // Builtins / 内置函数
            (
                "assert",
                "Assert condition",
                "assert(${1:cond}, ${2:msg})",
                "Unit",
            ),
            ("force", "Force lazy value", "force(${1:lazy_expr})", "T"),
        ]);

        stdlib_functions.extend(fetch_map_set_completion_specs());
        stdlib_functions.extend(list_completion_specs());
        stdlib_functions.extend(string_completion_specs());
        stdlib_functions.extend(path_completion_specs());
        stdlib_functions.extend(option_result_completion_specs());

        stdlib_functions
            .into_iter()
            .map(|(label, detail, snippet, ret_type)| CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(format!("{} -> {}", detail, ret_type)),
                insert_text: Some(snippet.to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            })
            .collect()
    }

    /// Get type completions.
    /// 获取类型补全。
    fn get_type_completions(&self) -> Vec<CompletionItem> {
        let types = vec![
            ("Int", "Arbitrary precision integer"),
            ("Float", "64-bit floating point"),
            ("Bool", "Boolean"),
            ("Char", "Unicode character"),
            ("String", "UTF-8 string"),
            ("Path", "File system path"),
            ("Unit", "Unit type ()"),
            ("List", "List<T>"),
            ("Option", "Option<T> - Some or None"),
            ("Result", "Result<T, E> - Ok or Err"),
        ];

        types
            .into_iter()
            .map(|(label, detail)| CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                detail: Some(detail.to_string()),
                insert_text: Some(label.to_string()),
                ..Default::default()
            })
            .collect()
    }

    /// Get method completions for dot-triggered completion.
    /// 获取点触发补全的方法补全。
    fn get_method_completions(&self) -> Vec<CompletionItem> {
        let methods = vec![
            // List methods / 列表方法
            (
                "map",
                "Map function over elements",
                "map(${1:fn(x) x})",
                "List<U>",
            ),
            (
                "filter",
                "Filter elements",
                "filter(${1:fn(x) true})",
                "List<T>",
            ),
            (
                "fold",
                "Fold with accumulator",
                "fold(${1:init}, ${2:fn(acc, x) acc})",
                "U",
            ),
            ("len", "Get length", "len()", "Int"),
            ("first", "Get first element", "first()", "Option<T>"),
            ("last", "Get last element", "last()", "Option<T>"),
            (
                "get",
                "Get element at index",
                "get(${1:index})",
                "Option<T>",
            ),
            ("reverse", "Reverse elements", "reverse()", "List<T>"),
            ("sum", "Sum of elements", "sum()", "Number"),
            ("all", "Check if all match", "all(${1:fn(x) true})", "Bool"),
            (
                "any",
                "Check if any matches",
                "any(${1:fn(x) false})",
                "Bool",
            ),
            (
                "zip",
                "Zip with another list",
                "zip(${1:other})",
                "List<(T, U)>",
            ),
            ("take", "Take first n elements", "take(${1:n})", "List<T>"),
            ("drop", "Drop first n elements", "drop(${1:n})", "List<T>"),
            ("join", "Join with separator", "join(${1:sep})", "String"),
            // String methods / 字符串方法
            (
                "split",
                "Split by separator",
                "split(${1:sep})",
                "List<String>",
            ),
            ("trim", "Trim whitespace", "trim()", "String"),
            ("upper", "To uppercase", "upper()", "String"),
            ("lower", "To lowercase", "lower()", "String"),
            (
                "contains",
                "Check if contains",
                "contains(${1:sub})",
                "Bool",
            ),
            (
                "startsWith",
                "Check prefix",
                "startsWith(${1:prefix})",
                "Bool",
            ),
            ("endsWith", "Check suffix", "endsWith(${1:suffix})", "Bool"),
            (
                "replace",
                "Replace substring",
                "replace(${1:from}, ${2:to})",
                "String",
            ),
            ("chars", "Get characters", "chars()", "List<Char>"),
            // Option/Result methods / Option/Result 方法
            ("unwrap", "Unwrap value", "unwrap()", "T"),
            (
                "unwrapOr",
                "Unwrap or default",
                "unwrapOr(${1:default})",
                "T",
            ),
            ("isSome", "Check if Some", "isSome()", "Bool"),
            ("isNone", "Check if None", "isNone()", "Bool"),
            ("isOk", "Check if Ok", "isOk()", "Bool"),
            ("isErr", "Check if Err", "isErr()", "Bool"),
            // Record methods / 记录方法
            ("keys", "Get record keys", "keys()", "List<String>"),
            ("values", "Get record values", "values()", "List<T>"),
            (
                "hasField",
                "Check if has field",
                "hasField(${1:name})",
                "Bool",
            ),
        ];

        methods
            .into_iter()
            .map(|(label, detail, snippet, ret_type)| CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some(format!("{} -> {}", detail, ret_type)),
                insert_text: Some(snippet.to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            })
            .collect()
    }

    /// Get completions from document symbols.
    /// 从文档符号获取补全。
    fn get_document_completions(&self, doc: &Document, _pos: Position) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        if let Some(ref index) = doc.symbol_index {
            for (name, defs) in &index.definitions {
                if let Some(def) = defs.first() {
                    let kind = match def.kind {
                        IndexSymbolKind::Function => CompletionItemKind::FUNCTION,
                        IndexSymbolKind::Variable => CompletionItemKind::VARIABLE,
                        IndexSymbolKind::Parameter => CompletionItemKind::VARIABLE,
                        IndexSymbolKind::TypeAlias => CompletionItemKind::TYPE_PARAMETER,
                        IndexSymbolKind::Struct => CompletionItemKind::STRUCT,
                        IndexSymbolKind::Enum => CompletionItemKind::ENUM,
                        IndexSymbolKind::Variant => CompletionItemKind::ENUM_MEMBER,
                        IndexSymbolKind::Trait => CompletionItemKind::INTERFACE,
                        IndexSymbolKind::Field => CompletionItemKind::FIELD,
                        IndexSymbolKind::Method => CompletionItemKind::METHOD,
                    };

                    items.push(CompletionItem {
                        label: name.clone(),
                        kind: Some(kind),
                        detail: Some(format!("{:?}", def.kind)),
                        ..Default::default()
                    });
                }
            }
        }

        items
    }
}

fn list_completion_specs() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    vec![
        // List functions / 列表函数
        ("list.empty", "Empty list", "list.empty", "List<T>"),
        (
            "list.singleton",
            "Single-element list",
            "list.singleton(${1:x})",
            "List<T>",
        ),
        ("list.len", "List length", "list.len(${1:xs})", "Int"),
        (
            "list.isEmpty",
            "Check emptiness",
            "list.isEmpty(${1:xs})",
            "Bool",
        ),
        (
            "list.head",
            "First element",
            "list.head(${1:xs})",
            "Option<T>",
        ),
        (
            "list.tail",
            "All but first",
            "list.tail(${1:xs})",
            "List<T>",
        ),
        (
            "list.last",
            "Last element",
            "list.last(${1:xs})",
            "Option<T>",
        ),
        ("list.init", "All but last", "list.init(${1:xs})", "List<T>"),
        (
            "list.get",
            "Get element by index",
            "list.get(${1:index}, ${2:xs})",
            "Option<T>",
        ),
        (
            "list.cons",
            "Prepend element",
            "list.cons(${1:x}, ${2:xs})",
            "List<T>",
        ),
        (
            "list.take",
            "Take prefix",
            "list.take(${1:n}, ${2:xs})",
            "List<T>",
        ),
        (
            "list.drop",
            "Drop prefix",
            "list.drop(${1:n}, ${2:xs})",
            "List<T>",
        ),
        (
            "list.contains",
            "Check membership",
            "list.contains(${1:x}, ${2:xs})",
            "Bool",
        ),
        (
            "list.indexOf",
            "Find element index",
            "list.indexOf(${1:x}, ${2:xs})",
            "Option<Int>",
        ),
        (
            "list.reverse",
            "Reverse list",
            "list.reverse(${1:xs})",
            "List<T>",
        ),
        (
            "list.map",
            "Map function over list",
            "list.map(${1:f}, ${2:xs})",
            "List<U>",
        ),
        (
            "list.filter",
            "Filter list",
            "list.filter(${1:pred}, ${2:xs})",
            "List<T>",
        ),
        (
            "list.fold",
            "Fold list",
            "list.fold(${1:init}, ${2:f}, ${3:xs})",
            "U",
        ),
        (
            "list.foldRight",
            "Right fold list",
            "list.foldRight(${1:init}, ${2:f}, ${3:xs})",
            "U",
        ),
        ("list.sum", "Sum integers", "list.sum(${1:xs})", "Int"),
        (
            "list.product",
            "Multiply integers",
            "list.product(${1:xs})",
            "Int",
        ),
        ("list.sort", "Sort list", "list.sort(${1:xs})", "List<T>"),
        (
            "list.max",
            "Maximum integer element",
            "list.max(${1:xs})",
            "Option<Int>",
        ),
        (
            "list.min",
            "Minimum integer element",
            "list.min(${1:xs})",
            "Option<Int>",
        ),
        (
            "list.range",
            "Create range",
            "list.range(${1:start}, ${2:end})",
            "List<Int>",
        ),
        (
            "list.replicate",
            "Repeat value",
            "list.replicate(${1:n}, ${2:value})",
            "List<T>",
        ),
        (
            "list.zip",
            "Zip two lists",
            "list.zip(${1:xs}, ${2:ys})",
            "List<(T, U)>",
        ),
        (
            "list.unzip",
            "Unzip pairs",
            "list.unzip(${1:pairs})",
            "(List<T>, List<U>)",
        ),
    ]
}

fn math_completion_specs() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    vec![
        ("math.abs", "Absolute value", "math.abs(${1:x})", "Number"),
        ("math.floor", "Floor of number", "math.floor(${1:x})", "Int"),
        ("math.ceil", "Ceiling of number", "math.ceil(${1:x})", "Int"),
        ("math.round", "Round number", "math.round(${1:x})", "Int"),
        ("math.sqrt", "Square root", "math.sqrt(${1:x})", "Float"),
        (
            "math.pow",
            "Power",
            "math.pow(${1:base}, ${2:exp})",
            "Number",
        ),
        ("math.log", "Natural logarithm", "math.log(${1:x})", "Float"),
        ("math.sin", "Sine", "math.sin(${1:x})", "Float"),
        ("math.cos", "Cosine", "math.cos(${1:x})", "Float"),
        ("math.tan", "Tangent", "math.tan(${1:x})", "Float"),
        (
            "math.max",
            "Maximum of two numbers",
            "math.max(${1:a}, ${2:b})",
            "Number",
        ),
        (
            "math.min",
            "Minimum of two numbers",
            "math.min(${1:a}, ${2:b})",
            "Number",
        ),
        (
            "math.clamp",
            "Clamp to range",
            "math.clamp(${1:x}, ${2:min}, ${3:max})",
            "Number",
        ),
        ("math.pi", "Pi constant", "math.pi", "Float"),
        ("math.e", "Euler's number", "math.e", "Float"),
        ("math.inf", "Infinity constant", "math.inf", "Float"),
        ("math.nan", "NaN constant", "math.nan", "Float"),
        (
            "math.toInt",
            "Convert to integer",
            "math.toInt(${1:x})",
            "Int",
        ),
        (
            "math.toFloat",
            "Convert to float",
            "math.toFloat(${1:x})",
            "Float",
        ),
    ]
}

fn fetch_map_set_completion_specs() -> Vec<(&'static str, &'static str, &'static str, &'static str)>
{
    vec![
        (
            "fetch.path",
            "Fetch local path",
            "fetch.path(${1:path})",
            "#{ path: String, hash: String, cached: Bool }",
        ),
        (
            "fetch.pathWithHash",
            "Fetch local path with hash",
            "fetch.pathWithHash(${1:path}, ${2:hash})",
            "#{ path: String, hash: String, cached: Bool }",
        ),
        (
            "fetch.url",
            "Fetch URL",
            "fetch.url(${1:url})",
            "#{ path: String, hash: String, cached: Bool }",
        ),
        (
            "fetch.urlWithHash",
            "Fetch URL with hash",
            "fetch.urlWithHash(${1:url}, ${2:hash})",
            "#{ path: String, hash: String, cached: Bool }",
        ),
        (
            "fetch.git",
            "Fetch git revision",
            "fetch.git(${1:url}, ${2:rev})",
            "#{ path: String, hash: String, cached: Bool }",
        ),
        (
            "fetch.gitWithHash",
            "Fetch git revision with hash",
            "fetch.gitWithHash(${1:url}, ${2:rev}, ${3:hash})",
            "#{ path: String, hash: String, cached: Bool }",
        ),
        ("Map.empty", "Empty map", "Map.empty", "Map[K, V]"),
        (
            "Map.singleton",
            "Singleton map",
            "Map.singleton(${1:key}, ${2:value})",
            "Map[K, V]",
        ),
        (
            "Map.fromList",
            "Map from pairs",
            "Map.fromList(${1:items})",
            "Map[K, V]",
        ),
        (
            "Map.get",
            "Lookup map value",
            "Map.get(${1:key}, ${2:map})",
            "Option[V]",
        ),
        (
            "Map.getWithDefault",
            "Lookup map value with default",
            "Map.getWithDefault(${1:key}, ${2:default}, ${3:map})",
            "V",
        ),
        (
            "Map.contains",
            "Check map key presence",
            "Map.contains(${1:key}, ${2:map})",
            "Bool",
        ),
        ("Map.size", "Map size", "Map.size(${1:map})", "Int"),
        (
            "Map.isEmpty",
            "Check if map is empty",
            "Map.isEmpty(${1:map})",
            "Bool",
        ),
        (
            "Map.insert",
            "Insert map entry",
            "Map.insert(${1:key}, ${2:value}, ${3:map})",
            "Map[K, V]",
        ),
        (
            "Map.remove",
            "Remove map entry",
            "Map.remove(${1:key}, ${2:map})",
            "Map[K, V]",
        ),
        (
            "Map.union",
            "Union of maps",
            "Map.union(${1:left}, ${2:right})",
            "Map[K, V]",
        ),
        (
            "Map.intersection",
            "Intersection of maps",
            "Map.intersection(${1:left}, ${2:right})",
            "Map[K, V]",
        ),
        (
            "Map.difference",
            "Difference of maps",
            "Map.difference(${1:left}, ${2:right})",
            "Map[K, V]",
        ),
        ("Set.empty", "Empty set", "Set.empty", "Set[A]"),
        (
            "Set.singleton",
            "Singleton set",
            "Set.singleton(${1:value})",
            "Set[A]",
        ),
        (
            "Set.fromList",
            "Set from list",
            "Set.fromList(${1:items})",
            "Set[A]",
        ),
        (
            "Set.contains",
            "Check set membership",
            "Set.contains(${1:value}, ${2:set})",
            "Bool",
        ),
        ("Set.size", "Set size", "Set.size(${1:set})", "Int"),
        (
            "Set.isEmpty",
            "Check if set is empty",
            "Set.isEmpty(${1:set})",
            "Bool",
        ),
        (
            "Set.insert",
            "Insert into set",
            "Set.insert(${1:value}, ${2:set})",
            "Set[A]",
        ),
        (
            "Set.remove",
            "Remove from set",
            "Set.remove(${1:value}, ${2:set})",
            "Set[A]",
        ),
        (
            "Set.union",
            "Union of sets",
            "Set.union(${1:left}, ${2:right})",
            "Set[A]",
        ),
        (
            "Set.intersection",
            "Intersection of sets",
            "Set.intersection(${1:left}, ${2:right})",
            "Set[A]",
        ),
        (
            "Set.difference",
            "Difference of sets",
            "Set.difference(${1:left}, ${2:right})",
            "Set[A]",
        ),
        (
            "Set.symmetricDifference",
            "Symmetric difference of sets",
            "Set.symmetricDifference(${1:left}, ${2:right})",
            "Set[A]",
        ),
        (
            "Set.isSubset",
            "Check subset relation",
            "Set.isSubset(${1:left}, ${2:right})",
            "Bool",
        ),
        (
            "Set.isSuperset",
            "Check superset relation",
            "Set.isSuperset(${1:left}, ${2:right})",
            "Bool",
        ),
        (
            "Set.isDisjoint",
            "Check disjoint sets",
            "Set.isDisjoint(${1:left}, ${2:right})",
            "Bool",
        ),
    ]
}

fn string_completion_specs() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    vec![
        ("string.len", "String length", "string.len(${1:s})", "Int"),
        (
            "string.chars",
            "Characters of string",
            "string.chars(${1:s})",
            "List[Char]",
        ),
        (
            "string.split",
            "Split string",
            "string.split(${1:s}, ${2:sep})",
            "List[String]",
        ),
        (
            "string.join",
            "Join strings",
            "string.join(${1:xs}, ${2:sep})",
            "String",
        ),
        (
            "string.trim",
            "Trim whitespace",
            "string.trim(${1:s})",
            "String",
        ),
        (
            "string.upper",
            "To uppercase",
            "string.upper(${1:s})",
            "String",
        ),
        (
            "string.lower",
            "To lowercase",
            "string.lower(${1:s})",
            "String",
        ),
        (
            "string.contains",
            "Check if contains",
            "string.contains(${1:s}, ${2:needle})",
            "Bool",
        ),
        (
            "string.startsWith",
            "Check prefix",
            "string.startsWith(${1:s}, ${2:prefix})",
            "Bool",
        ),
        (
            "string.endsWith",
            "Check suffix",
            "string.endsWith(${1:s}, ${2:suffix})",
            "Bool",
        ),
        (
            "string.replace",
            "Replace substring",
            "string.replace(${1:s}, ${2:from}, ${3:to})",
            "String",
        ),
        (
            "string.substring",
            "Substring by range",
            "string.substring(${1:s}, ${2:start}, ${3:end})",
            "String",
        ),
        (
            "string.isEmpty",
            "Check if string is empty",
            "string.isEmpty(${1:s})",
            "Bool",
        ),
        (
            "string.repeat",
            "Repeat string",
            "string.repeat(${1:s}, ${2:n})",
            "String",
        ),
        (
            "string.lines",
            "Split string into lines",
            "string.lines(${1:s})",
            "List[String]",
        ),
    ]
}

fn io_completion_specs() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    vec![
        // IO functions / IO 函数
        (
            "io.readFile",
            "Read file contents",
            "io.readFile(${1:path})",
            "String",
        ),
        (
            "io.readFilePath",
            "Read file contents from a typed path",
            "io.readFilePath(${1:path})",
            "String",
        ),
        (
            "io.readFileBytesPath",
            "Read file bytes from a typed path",
            "io.readFileBytesPath(${1:path})",
            "Bytes",
        ),
        (
            "io.readDir",
            "List directory contents",
            "io.readDir(${1:path})",
            "List<String>",
        ),
        (
            "io.readDirPath",
            "List directory contents from a typed path",
            "io.readDirPath(${1:path})",
            "List<String>",
        ),
        (
            "io.readDirEntryPaths",
            "List directory entry paths from a typed path",
            "io.readDirEntryPaths(${1:path})",
            "List<Path>",
        ),
        (
            "io.writeFile",
            "Write file contents",
            "io.writeFile(${1:path}, ${2:content})",
            "Unit",
        ),
        (
            "io.writeFilePath",
            "Write file contents to a typed path",
            "io.writeFilePath(${1:path}, ${2:content})",
            "Unit",
        ),
        (
            "io.appendFile",
            "Append file contents",
            "io.appendFile(${1:path}, ${2:content})",
            "Unit",
        ),
        (
            "io.appendFilePath",
            "Append file contents to a typed path",
            "io.appendFilePath(${1:path}, ${2:content})",
            "Unit",
        ),
        (
            "io.writeFileBytesPath",
            "Write bytes to a typed path",
            "io.writeFileBytesPath(${1:path}, ${2:bytes})",
            "Unit",
        ),
        (
            "io.appendFileBytesPath",
            "Append bytes to a typed path",
            "io.appendFileBytesPath(${1:path}, ${2:bytes})",
            "Unit",
        ),
        (
            "io.createDirAll",
            "Create directories recursively",
            "io.createDirAll(${1:path})",
            "Unit",
        ),
        (
            "io.createDirAllPath",
            "Create directories recursively from a typed path",
            "io.createDirAllPath(${1:path})",
            "Unit",
        ),
        (
            "io.removeDirAll",
            "Remove directories recursively",
            "io.removeDirAll(${1:path})",
            "Unit",
        ),
        (
            "io.removeDirAllPath",
            "Remove directories recursively from a typed path",
            "io.removeDirAllPath(${1:path})",
            "Unit",
        ),
        (
            "io.pathExists",
            "Check if path exists",
            "io.pathExists(${1:path})",
            "Bool",
        ),
        (
            "io.pathExistsPath",
            "Check if typed path exists",
            "io.pathExistsPath(${1:path})",
            "Bool",
        ),
        (
            "io.isDir",
            "Check if path is directory",
            "io.isDir(${1:path})",
            "Bool",
        ),
        (
            "io.isDirPath",
            "Check if typed path is directory",
            "io.isDirPath(${1:path})",
            "Bool",
        ),
        (
            "io.isFile",
            "Check if path is file",
            "io.isFile(${1:path})",
            "Bool",
        ),
        (
            "io.isFilePath",
            "Check if typed path is file",
            "io.isFilePath(${1:path})",
            "Bool",
        ),
        (
            "io.getEnv",
            "Get environment variable",
            "io.getEnv(${1:name})",
            "Option<String>",
        ),
        (
            "io.currentDir",
            "Get current directory",
            "io.currentDir()",
            "String",
        ),
        (
            "io.currentDirPath",
            "Get current directory as typed path",
            "io.currentDirPath()",
            "Path",
        ),
        (
            "io.homeDir",
            "Get home directory",
            "io.homeDir()",
            "Option<String>",
        ),
        (
            "io.homeDirPath",
            "Get home directory as typed path",
            "io.homeDirPath()",
            "Option<Path>",
        ),
        (
            "io.command",
            "Construct command",
            "io.command(${1:program}, ${2:args})",
            "Command",
        ),
        (
            "io.commandWith",
            "Construct configured command",
            "io.commandWith(${1:opts})",
            "Command",
        ),
        (
            "io.commandWithRedirects",
            "Attach redirects to command",
            "io.commandWithRedirects(${1:command}, ${2:redirects})",
            "Command",
        ),
        (
            "io.execCommand",
            "Execute command",
            "io.execCommand(${1:command})",
            "ProcessResult",
        ),
        (
            "io.pipeline",
            "Construct pipeline",
            "io.pipeline(${1:commands})",
            "Pipeline",
        ),
        (
            "io.pipelineWithRedirects",
            "Attach redirects to pipeline",
            "io.pipelineWithRedirects(${1:pipeline}, ${2:redirects})",
            "Pipeline",
        ),
        (
            "io.execPipeline",
            "Execute pipeline",
            "io.execPipeline(${1:pipeline})",
            "ProcessResult",
        ),
        (
            "io.redirectStdoutPath",
            "Redirect stdout to typed path",
            "io.redirectStdoutPath(${1:path})",
            "Redirect",
        ),
        (
            "io.redirectStderrPath",
            "Redirect stderr to typed path",
            "io.redirectStderrPath(${1:path})",
            "Redirect",
        ),
        (
            "io.redirectStdinPath",
            "Redirect stdin from typed path",
            "io.redirectStdinPath(${1:path})",
            "Redirect",
        ),
        (
            "io.taskCommand",
            "Create command task",
            "io.taskCommand(${1:command})",
            "Task<ProcessResult>",
        ),
        (
            "io.taskPipeline",
            "Create pipeline task",
            "io.taskPipeline(${1:pipeline})",
            "Task<ProcessResult>",
        ),
        (
            "io.awaitTask",
            "Await task result",
            "io.awaitTask(${1:task})",
            "ProcessResult",
        ),
        (
            "io.awaitTasks",
            "Await task list",
            "io.awaitTasks(${1:tasks})",
            "List<ProcessResult>",
        ),
        (
            "io.processSuccess",
            "Check process success",
            "io.processSuccess(${1:result})",
            "Bool",
        ),
        (
            "io.processStdout",
            "Get process stdout",
            "io.processStdout(${1:result})",
            "String",
        ),
        (
            "io.processCode",
            "Get process exit code",
            "io.processCode(${1:result})",
            "Int",
        ),
        (
            "io.processStderr",
            "Get process stderr",
            "io.processStderr(${1:result})",
            "String",
        ),
        (
            "io.hashFile",
            "Hash file contents",
            "io.hashFile(${1:path})",
            "String",
        ),
        (
            "io.hashFilePath",
            "Hash file contents from a typed path",
            "io.hashFilePath(${1:path})",
            "String",
        ),
        (
            "io.hashString",
            "Hash a string",
            "io.hashString(${1:str})",
            "String",
        ),
        (
            "io.currentSystem",
            "Get current system",
            "io.currentSystem()",
            "String",
        ),
    ]
}

fn path_completion_specs() -> Vec<(&'static str, &'static str, &'static str, &'static str)> {
    vec![
        // Path functions / 路径函数
        (
            "path.fromString",
            "Construct typed path",
            "path.fromString(${1:path})",
            "Path",
        ),
        (
            "path.joinPath",
            "Join typed path with child",
            "path.joinPath(${1:base}, ${2:child})",
            "Path",
        ),
        (
            "path.parentPath",
            "Get parent typed path",
            "path.parentPath(${1:path})",
            "Option<Path>",
        ),
        (
            "path.filenamePath",
            "Get typed path file name",
            "path.filenamePath(${1:path})",
            "Option<String>",
        ),
        (
            "path.extensionPath",
            "Get typed path extension",
            "path.extensionPath(${1:path})",
            "Option<String>",
        ),
        (
            "path.isAbsolutePath",
            "Check if typed path is absolute",
            "path.isAbsolutePath(${1:path})",
            "Bool",
        ),
        (
            "path.join",
            "Join string paths",
            "path.join(${1:base}, ${2:part})",
            "String",
        ),
        (
            "path.parent",
            "Get parent directory string",
            "path.parent(${1:path})",
            "Option<String>",
        ),
        (
            "path.filename",
            "Get file name string",
            "path.filename(${1:path})",
            "Option<String>",
        ),
        (
            "path.extension",
            "Get extension string",
            "path.extension(${1:path})",
            "Option<String>",
        ),
        (
            "path.is_absolute",
            "Check if string path is absolute",
            "path.is_absolute(${1:path})",
            "Bool",
        ),
    ]
}

fn option_result_completion_specs() -> Vec<(&'static str, &'static str, &'static str, &'static str)>
{
    vec![
        // Option functions / Option 函数
        (
            "option.some",
            "Wrap in Some",
            "option.some(${1:x})",
            "Option<T>",
        ),
        ("option.none", "None value", "option.none", "Option<T>"),
        (
            "option.is_some",
            "Check if Some",
            "option.is_some(${1:opt})",
            "Bool",
        ),
        (
            "option.is_none",
            "Check if None",
            "option.is_none(${1:opt})",
            "Bool",
        ),
        (
            "option.unwrap",
            "Unwrap option",
            "option.unwrap(${1:opt})",
            "T",
        ),
        (
            "option.unwrap_or",
            "Unwrap or default",
            "option.unwrap_or(${1:opt}, ${2:default})",
            "T",
        ),
        // Result functions / Result 函数
        (
            "result.ok",
            "Wrap in Ok",
            "result.ok(${1:x})",
            "Result<T, E>",
        ),
        (
            "result.err",
            "Wrap in Err",
            "result.err(${1:e})",
            "Result<T, E>",
        ),
        (
            "result.is_ok",
            "Check if Ok",
            "result.is_ok(${1:res})",
            "Bool",
        ),
        (
            "result.is_err",
            "Check if Err",
            "result.is_err(${1:res})",
            "Bool",
        ),
        (
            "result.unwrap",
            "Unwrap result",
            "result.unwrap(${1:res})",
            "T",
        ),
        (
            "result.unwrap_err",
            "Unwrap error",
            "result.unwrap_err(${1:res})",
            "E",
        ),
    ]
}

/// Helper function to convert symbol kind.
/// 转换符号类型的辅助函数。
fn convert_symbol_kind(kind: IndexSymbolKind) -> SymbolKind {
    match kind {
        IndexSymbolKind::Function => SymbolKind::FUNCTION,
        IndexSymbolKind::Variable => SymbolKind::VARIABLE,
        IndexSymbolKind::Parameter => SymbolKind::VARIABLE,
        IndexSymbolKind::TypeAlias => SymbolKind::TYPE_PARAMETER,
        IndexSymbolKind::Struct => SymbolKind::STRUCT,
        IndexSymbolKind::Enum => SymbolKind::ENUM,
        IndexSymbolKind::Variant => SymbolKind::ENUM_MEMBER,
        IndexSymbolKind::Trait => SymbolKind::INTERFACE,
        IndexSymbolKind::Field => SymbolKind::FIELD,
        IndexSymbolKind::Method => SymbolKind::METHOD,
    }
}

/// Format an impl header for symbol views.
/// 为符号视图格式化 impl 标题。
fn format_impl_name(def: &ImplDef) -> String {
    let target = format_type_name(&def.target);
    match &def.trait_ {
        Some(trait_ty) => format!("impl {} for {}", format_type_name(trait_ty), target),
        None => format!("impl {}", target),
    }
}

/// Render a type into a compact name for display.
/// 将类型渲染为紧凑的显示名称。
fn format_type_name(ty: &Type) -> String {
    match &ty.kind {
        TypeKind::Named { path, args } => {
            let name = path
                .iter()
                .map(|part| part.name.as_str())
                .collect::<Vec<_>>()
                .join(".");
            if args.is_empty() {
                name
            } else {
                let args_str = args
                    .iter()
                    .map(format_type_name)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}<{}>", name, args_str)
            }
        }
        TypeKind::Function { params, result } => {
            let params_str = params
                .iter()
                .map(format_type_name)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({}) -> {}", params_str, format_type_name(result))
        }
        TypeKind::Tuple(elems) => {
            let elems_str = elems
                .iter()
                .map(format_type_name)
                .collect::<Vec<_>>()
                .join(", ");
            format!("({})", elems_str)
        }
        TypeKind::Record(fields) => {
            let fields_str = fields
                .iter()
                .map(|field| format!("{}: {}", field.name.name, format_type_name(&field.ty)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("#{{ {} }}", fields_str)
        }
        TypeKind::Unit => "()".to_string(),
        TypeKind::Infer => "_".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        fetch_map_set_completion_specs, io_completion_specs, list_completion_specs,
        math_completion_specs, option_result_completion_specs, path_completion_specs,
        string_completion_specs,
    };

    #[test]
    fn test_list_stdlib_completions_match_real_surface() {
        let labels: Vec<_> = list_completion_specs()
            .into_iter()
            .map(|(label, _, _, _)| label)
            .collect();

        assert!(labels.contains(&"list.empty"));
        assert!(labels.contains(&"list.singleton"));
        assert!(labels.contains(&"list.isEmpty"));
        assert!(labels.contains(&"list.foldRight"));
        assert!(labels.contains(&"list.zip"));
    }

    #[test]
    fn test_list_stdlib_completions_omit_stale_surface_entries() {
        let labels: Vec<_> = list_completion_specs()
            .into_iter()
            .map(|(label, _, _, _)| label)
            .collect();

        assert!(!labels.contains(&"list.concat"));
        assert!(!labels.contains(&"list.elem"));
    }

    #[test]
    fn test_io_stdlib_completions_match_real_surface() {
        let labels: Vec<_> = io_completion_specs()
            .into_iter()
            .map(|(label, _, _, _)| label)
            .collect();

        assert!(labels.contains(&"io.readFilePath"));
        assert!(labels.contains(&"io.pathExistsPath"));
        assert!(labels.contains(&"io.currentDirPath"));
        assert!(labels.contains(&"io.homeDirPath"));
        assert!(labels.contains(&"io.command"));
        assert!(labels.contains(&"io.commandWith"));
        assert!(labels.contains(&"io.commandWithRedirects"));
        assert!(labels.contains(&"io.execCommand"));
        assert!(labels.contains(&"io.pipeline"));
        assert!(labels.contains(&"io.pipelineWithRedirects"));
        assert!(labels.contains(&"io.execPipeline"));
        assert!(labels.contains(&"io.redirectStdoutPath"));
        assert!(labels.contains(&"io.redirectStderrPath"));
        assert!(labels.contains(&"io.redirectStdinPath"));
        assert!(labels.contains(&"io.taskCommand"));
        assert!(labels.contains(&"io.taskPipeline"));
        assert!(labels.contains(&"io.awaitTask"));
        assert!(labels.contains(&"io.awaitTasks"));
        assert!(labels.contains(&"io.processSuccess"));
        assert!(labels.contains(&"io.processStdout"));
        assert!(labels.contains(&"io.processCode"));
        assert!(labels.contains(&"io.processStderr"));
    }

    #[test]
    fn test_io_stdlib_completions_omit_removed_compat_wrappers() {
        let labels: Vec<_> = io_completion_specs()
            .into_iter()
            .map(|(label, _, _, _)| label)
            .collect();

        assert!(!labels.contains(&"io.exec"));
        assert!(!labels.contains(&"io.execWith"));
        assert!(!labels.contains(&"io.execShell"));
        assert!(!labels.contains(&"io.execResult"));
        assert!(!labels.contains(&"io.execShellResult"));
        assert!(!labels.contains(&"io.execWithResult"));
        assert!(!labels.contains(&"io.execCommandWithRedirects"));
        assert!(!labels.contains(&"io.execPipelineWithRedirects"));
    }

    #[test]
    fn test_option_result_stdlib_completions_match_real_surface() {
        let labels: Vec<_> = option_result_completion_specs()
            .into_iter()
            .map(|(label, _, _, _)| label)
            .collect();

        assert!(labels.contains(&"option.some"));
        assert!(labels.contains(&"option.none"));
        assert!(labels.contains(&"option.is_some"));
        assert!(labels.contains(&"option.is_none"));
        assert!(labels.contains(&"option.unwrap"));
        assert!(labels.contains(&"option.unwrap_or"));
        assert!(labels.contains(&"result.ok"));
        assert!(labels.contains(&"result.err"));
        assert!(labels.contains(&"result.is_ok"));
        assert!(labels.contains(&"result.is_err"));
        assert!(labels.contains(&"result.unwrap"));
        assert!(labels.contains(&"result.unwrap_err"));
    }

    #[test]
    fn test_option_result_stdlib_completions_omit_stale_surface_entries() {
        let labels: Vec<_> = option_result_completion_specs()
            .into_iter()
            .map(|(label, _, _, _)| label)
            .collect();

        assert!(!labels.contains(&"option.isSome"));
        assert!(!labels.contains(&"option.isNone"));
        assert!(!labels.contains(&"option.unwrapOr"));
        assert!(!labels.contains(&"option.map"));
        assert!(!labels.contains(&"result.isOk"));
        assert!(!labels.contains(&"result.isErr"));
        assert!(!labels.contains(&"result.map"));
    }

    #[test]
    fn test_string_stdlib_completions_match_real_surface() {
        let labels: Vec<_> = string_completion_specs()
            .into_iter()
            .map(|(label, _, _, _)| label)
            .collect();

        assert!(labels.contains(&"string.len"));
        assert!(labels.contains(&"string.chars"));
        assert!(labels.contains(&"string.split"));
        assert!(labels.contains(&"string.join"));
        assert!(labels.contains(&"string.trim"));
        assert!(labels.contains(&"string.upper"));
        assert!(labels.contains(&"string.lower"));
        assert!(labels.contains(&"string.contains"));
        assert!(labels.contains(&"string.startsWith"));
        assert!(labels.contains(&"string.endsWith"));
        assert!(labels.contains(&"string.replace"));
        assert!(labels.contains(&"string.substring"));
        assert!(labels.contains(&"string.isEmpty"));
        assert!(labels.contains(&"string.repeat"));
        assert!(labels.contains(&"string.lines"));
    }

    #[test]
    fn test_string_stdlib_completions_omit_stale_surface_entries() {
        let labels: Vec<_> = string_completion_specs()
            .into_iter()
            .map(|(label, _, _, _)| label)
            .collect();

        assert!(!labels.contains(&"string.concat"));
    }

    #[test]
    fn test_fetch_map_set_stdlib_completions_match_real_surface() {
        let labels: Vec<_> = fetch_map_set_completion_specs()
            .into_iter()
            .map(|(label, _, _, _)| label)
            .collect();

        assert!(labels.contains(&"fetch.path"));
        assert!(labels.contains(&"fetch.pathWithHash"));
        assert!(labels.contains(&"fetch.url"));
        assert!(labels.contains(&"fetch.urlWithHash"));
        assert!(labels.contains(&"fetch.git"));
        assert!(labels.contains(&"fetch.gitWithHash"));
        assert!(labels.contains(&"Map.empty"));
        assert!(labels.contains(&"Map.getWithDefault"));
        assert!(labels.contains(&"Map.size"));
        assert!(labels.contains(&"Map.isEmpty"));
        assert!(labels.contains(&"Map.insert"));
        assert!(labels.contains(&"Map.remove"));
        assert!(labels.contains(&"Map.union"));
        assert!(labels.contains(&"Map.intersection"));
        assert!(labels.contains(&"Map.difference"));
        assert!(labels.contains(&"Set.empty"));
        assert!(labels.contains(&"Set.size"));
        assert!(labels.contains(&"Set.isEmpty"));
        assert!(labels.contains(&"Set.insert"));
        assert!(labels.contains(&"Set.remove"));
        assert!(labels.contains(&"Set.union"));
        assert!(labels.contains(&"Set.intersection"));
        assert!(labels.contains(&"Set.difference"));
        assert!(labels.contains(&"Set.symmetricDifference"));
        assert!(labels.contains(&"Set.isSubset"));
        assert!(labels.contains(&"Set.isSuperset"));
        assert!(labels.contains(&"Set.isDisjoint"));
    }

    #[test]
    fn test_fetch_map_set_stdlib_completions_omit_runtime_only_surface_entries() {
        let labels: Vec<_> = fetch_map_set_completion_specs()
            .into_iter()
            .map(|(label, _, _, _)| label)
            .collect();

        assert!(!labels.contains(&"Map.keys"));
        assert!(!labels.contains(&"Map.values"));
        assert!(!labels.contains(&"Map.toList"));
        assert!(!labels.contains(&"Map.update"));
        assert!(!labels.contains(&"Map.map"));
        assert!(!labels.contains(&"Map.mapWithKey"));
        assert!(!labels.contains(&"Map.filter"));
        assert!(!labels.contains(&"Map.filterWithKey"));
        assert!(!labels.contains(&"Map.fold"));
        assert!(!labels.contains(&"Map.foldWithKey"));
        assert!(!labels.contains(&"Set.toList"));
        assert!(!labels.contains(&"Set.map"));
        assert!(!labels.contains(&"Set.filter"));
        assert!(!labels.contains(&"Set.fold"));
        assert!(!labels.contains(&"Set.partition"));
    }

    #[test]
    fn test_math_stdlib_completions_include_explicit_constant_surface() {
        let labels: Vec<_> = math_completion_specs()
            .into_iter()
            .map(|(label, _, _, _)| label)
            .collect();

        assert!(labels.contains(&"math.pi"));
        assert!(labels.contains(&"math.e"));
        assert!(labels.contains(&"math.inf"));
        assert!(labels.contains(&"math.nan"));
    }

    #[test]
    fn test_path_stdlib_completions_match_real_surface() {
        let labels: Vec<_> = path_completion_specs()
            .into_iter()
            .map(|(label, _, _, _)| label)
            .collect();

        assert!(labels.contains(&"path.fromString"));
        assert!(labels.contains(&"path.joinPath"));
        assert!(labels.contains(&"path.filenamePath"));
        assert!(labels.contains(&"path.isAbsolutePath"));
        assert!(labels.contains(&"path.filename"));
        assert!(labels.contains(&"path.is_absolute"));
    }

    #[test]
    fn test_path_stdlib_completions_omit_stale_surface_entries() {
        let labels: Vec<_> = path_completion_specs()
            .into_iter()
            .map(|(label, _, _, _)| label)
            .collect();

        assert!(!labels.contains(&"path.fileName"));
        assert!(!labels.contains(&"path.isAbsolute"));
    }
}
