//! LSP backend implementation.

use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use neve_lexer::Lexer;

use crate::capabilities::server_capabilities;
use crate::document::{Document, DiagnosticSeverity as DocSeverity};
use crate::semantic_tokens::generate_semantic_tokens_with_context;
use crate::symbol_index::SymbolKind as IndexSymbolKind;

/// The LSP backend.
pub struct Backend {
    client: Client,
    documents: DashMap<String, Document>,
}

impl Backend {
    /// Create a new backend.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DashMap::new(),
        }
    }
    
    /// Publish diagnostics for a document.
    async fn publish_diagnostics(&self, uri: &Url, doc: &Document) {
        let diagnostics: Vec<Diagnostic> = doc.diagnostics
            .iter()
            .map(|d| {
                let start: usize = d.span.start.into();
                let end: usize = d.span.end.into();
                let (start_line, start_col) = doc.position_at(start);
                let (end_line, end_col) = doc.position_at(end);
                
                Diagnostic {
                    range: Range {
                        start: Position::new(start_line, start_col),
                        end: Position::new(end_line, end_col),
                    },
                    severity: Some(match d.severity {
                        DocSeverity::Error => DiagnosticSeverity::ERROR,
                        DocSeverity::Warning => DiagnosticSeverity::WARNING,
                        DocSeverity::Information => DiagnosticSeverity::INFORMATION,
                        DocSeverity::Hint => DiagnosticSeverity::HINT,
                    }),
                    code: None,
                    code_description: None,
                    source: Some("neve".to_string()),
                    message: d.message.clone(),
                    related_information: None,
                    tags: None,
                    data: None,
                }
            })
            .collect();
        
        self.client.publish_diagnostics(uri.clone(), diagnostics, None).await;
    }
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
        self.publish_diagnostics(&params.text_document.uri, &doc).await;
        self.documents.insert(uri, doc);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        
        if let Some(mut doc) = self.documents.get_mut(&uri)
            && let Some(change) = params.content_changes.into_iter().next() {
                doc.update(change.text);
                self.publish_diagnostics(&params.text_document.uri, &doc).await;
            }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        
        if let Some(text) = params.text
            && let Some(mut doc) = self.documents.get_mut(&uri) {
                doc.update(text);
                self.publish_diagnostics(&params.text_document.uri, &doc).await;
            }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        self.documents.remove(&uri);
        
        // Clear diagnostics
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri.to_string();
        let pos = params.text_document_position_params.position;
        
        if let Some(doc) = self.documents.get(&uri) {
            let offset = doc.offset_at(pos.line, pos.character);
            
            // Try to get symbol information first
            if let Some(ref index) = doc.symbol_index
                && let Some(symbol) = index.find_definition_at(offset) {
                    // Format symbol kind nicely
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
                    let full_start: usize = symbol.full_span.start.into();
                    let full_end: usize = symbol.full_span.end.into();
                    let definition_text = if full_end <= doc.content.len() {
                        // Limit to first line for display
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
                    
                    let hover_text = format!(
                        "**{}** `{}`\n\n```neve\n{}\n```",
                        kind_str, symbol.name, definition_text
                    );
                    
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
            
            // Fallback to token-based hover
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

    async fn completion(&self, _params: CompletionParams) -> Result<Option<CompletionResponse>> {
        // Basic keyword completion
        let keywords = vec![
            ("let", "Let binding", "let ${1:name} = ${2:value};"),
            ("fn", "Function definition", "fn ${1:name}(${2:params}) = ${3:body};"),
            ("if", "If expression", "if ${1:condition} then ${2:then_branch} else ${3:else_branch}"),
            ("match", "Match expression", "match ${1:expr} {\n\t${2:pattern} => ${3:body},\n}"),
            ("type", "Type alias", "type ${1:Name} = ${2:Type};"),
            ("struct", "Struct definition", "struct ${1:Name} {\n\t${2:field}: ${3:Type},\n}"),
            ("enum", "Enum definition", "enum ${1:Name} {\n\t${2:Variant},\n}"),
            ("trait", "Trait definition", "trait ${1:Name} {\n\t${2:items}\n}"),
            ("impl", "Implementation", "impl ${1:Trait} for ${2:Type} {\n\t${3:items}\n}"),
            ("import", "Import statement", "import ${1:module};"),
        ];
        
        let items: Vec<CompletionItem> = keywords
            .into_iter()
            .map(|(label, detail, snippet)| CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some(detail.to_string()),
                insert_text: Some(snippet.to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            })
            .collect();
        
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri.to_string();
        
        if let Some(doc) = self.documents.get(&uri)
            && let Ok(formatted) = neve_fmt::format(&doc.content)
                && formatted != doc.content {
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
            && let Some(ref ast) = doc.ast {
                let mut symbols = Vec::new();
                
                for item in &ast.items {
                    use neve_syntax::ItemKind;
                    
                    let (name, kind) = match &item.kind {
                        ItemKind::Let(def) => {
                            // Extract name from pattern
                            let name = format!("{:?}", def.pattern.kind);
                            (name, SymbolKind::VARIABLE)
                        }
                        ItemKind::Fn(def) => {
                            (def.name.name.clone(), SymbolKind::FUNCTION)
                        }
                        ItemKind::TypeAlias(def) => {
                            (def.name.name.clone(), SymbolKind::TYPE_PARAMETER)
                        }
                        ItemKind::Struct(def) => {
                            (def.name.name.clone(), SymbolKind::STRUCT)
                        }
                        ItemKind::Enum(def) => {
                            (def.name.name.clone(), SymbolKind::ENUM)
                        }
                        ItemKind::Trait(def) => {
                            (def.name.name.clone(), SymbolKind::INTERFACE)
                        }
                        ItemKind::Impl(_) => continue,
                        ItemKind::Import(_) => continue,
                    };
                    
                    let start: usize = item.span.start.into();
                    let end: usize = item.span.end.into();
                    let (start_line, start_col) = doc.position_at(start);
                    let (end_line, end_col) = doc.position_at(end);
                    
                    #[allow(deprecated)]
                    symbols.push(DocumentSymbol {
                        name,
                        detail: None,
                        kind,
                        tags: None,
                        deprecated: None,
                        range: Range {
                            start: Position::new(start_line, start_col),
                            end: Position::new(end_line, end_col),
                        },
                        selection_range: Range {
                            start: Position::new(start_line, start_col),
                            end: Position::new(end_line, end_col),
                        },
                        children: None,
                    });
                }
                
                return Ok(Some(DocumentSymbolResponse::Nested(symbols)));
            }
        
        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri.to_string();
        let pos = params.text_document_position_params.position;
        
        if let Some(doc) = self.documents.get(&uri)
            && let Some(ref index) = doc.symbol_index {
                let offset = doc.offset_at(pos.line, pos.character);
                
                if let Some(symbol) = index.find_definition_at(offset) {
                    let start: usize = symbol.def_span.start.into();
                    let end: usize = symbol.def_span.end.into();
                    let (start_line, start_col) = doc.position_at(start);
                    let (end_line, end_col) = doc.position_at(end);
                    
                    return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                        uri: params.text_document_position_params.text_document.uri.clone(),
                        range: Range {
                            start: Position::new(start_line, start_col),
                            end: Position::new(end_line, end_col),
                        },
                    })));
                }
            }
        
        Ok(None)
    }

    async fn references(
        &self,
        params: ReferenceParams,
    ) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let pos = params.text_document_position.position;
        let include_declaration = params.context.include_declaration;
        
        if let Some(doc) = self.documents.get(&uri)
            && let Some(ref index) = doc.symbol_index {
                let offset = doc.offset_at(pos.line, pos.character);
                let refs = index.find_references_at(offset, include_declaration);
                
                if !refs.is_empty() {
                    let locations: Vec<Location> = refs.iter()
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

    async fn rename(
        &self,
        params: RenameParams,
    ) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let pos = params.text_document_position.position;
        let new_name = params.new_name;
        
        if let Some(doc) = self.documents.get(&uri) {
            // Use the document's stored URI for consistency
            let doc_uri = &doc.uri;
            let _ = doc_uri; // Acknowledge the field is used for identification
            
            if let Some(ref index) = doc.symbol_index {
                let offset = doc.offset_at(pos.line, pos.character);
                
                // Find the symbol name at this position
                if let Some(name) = index.find_name_at(offset) {
                    // Check if this is a valid symbol that can be renamed
                    // by verifying it has a definition
                    if index.get_definitions(&name).is_none() {
                        // Symbol has no definition in this file, cannot rename
                        return Ok(None);
                    }
                    
                    // Get all references to this symbol
                    let refs = index.get_references(&name);
                    
                    if !refs.is_empty() {
                        let edits: Vec<TextEdit> = refs.iter()
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
            && let Some(ref index) = doc.symbol_index {
                let offset = doc.offset_at(pos.line, pos.character);
                
                // Find the symbol at this position
                if let Some(name) = index.find_name_at(offset) {
                    // Find the reference at this position to get its span
                    for r in &index.references {
                        let start: usize = r.span.start.into();
                        let end: usize = r.span.end.into();
                        if start <= offset && offset < end {
                            let (start_line, start_col) = doc.position_at(start);
                            let (end_line, end_col) = doc.position_at(end);
                            
                            return Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
                                range: Range {
                                    start: Position::new(start_line, start_col),
                                    end: Position::new(end_line, end_col),
                                },
                                placeholder: name,
                            }));
                        }
                    }
                }
            }
        
        Ok(None)
    }
}

// Helper function to convert symbol kind
#[allow(dead_code)]
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
