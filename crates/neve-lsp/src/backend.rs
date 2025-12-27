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

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let pos = params.text_document_position.position;
        
        let mut items = Vec::new();
        
        // Get context for smarter completion
        let trigger_char = params.context
            .as_ref()
            .and_then(|c| c.trigger_character.as_deref());
        
        // Check if we're after a dot (member access)
        let is_dot_completion = trigger_char == Some(".");
        
        if is_dot_completion {
            // Method completion based on type
            items.extend(self.get_method_completions());
        } else {
            // Keywords
            items.extend(self.get_keyword_completions());
            
            // Standard library functions
            items.extend(self.get_stdlib_completions());
            
            // Types
            items.extend(self.get_type_completions());
            
            // Document symbols (variables, functions from current file)
            if let Some(doc) = self.documents.get(&uri) {
                items.extend(self.get_document_completions(&doc, pos));
            }
        }
        
        Ok(Some(CompletionResponse::Array(items)))
    }
}

impl Backend {
    /// Get keyword completions.
    fn get_keyword_completions(&self) -> Vec<CompletionItem> {
        let keywords = vec![
            ("let", "Let binding", "let ${1:name} = ${2:value};"),
            ("fn", "Function definition", "fn ${1:name}(${2:params}) = ${3:body};"),
            ("if", "If expression", "if ${1:condition} then ${2:then_branch} else ${3:else_branch}"),
            ("match", "Match expression", "match ${1:expr} {\n\t${2:pattern} -> ${3:body},\n}"),
            ("type", "Type alias", "type ${1:Name} = ${2:Type};"),
            ("struct", "Struct definition", "struct ${1:Name} {\n\t${2:field}: ${3:Type},\n};"),
            ("enum", "Enum definition", "enum ${1:Name} {\n\t${2:Variant},\n};"),
            ("trait", "Trait definition", "trait ${1:Name} {\n\t${2:items}\n};"),
            ("impl", "Implementation", "impl ${1:Trait} for ${2:Type} {\n\t${3:items}\n};"),
            ("import", "Import statement", "import ${1:module};"),
            ("pub", "Public visibility", "pub "),
            ("lazy", "Lazy evaluation", "lazy ${1:expr}"),
            ("true", "Boolean true", "true"),
            ("false", "Boolean false", "false"),
        ];
        
        keywords.into_iter()
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
    fn get_stdlib_completions(&self) -> Vec<CompletionItem> {
        let stdlib_functions = vec![
            // IO functions
            ("io.readFile", "Read file contents", "io.readFile(${1:path})", "String"),
            ("io.readDir", "List directory contents", "io.readDir(${1:path})", "List<String>"),
            ("io.pathExists", "Check if path exists", "io.pathExists(${1:path})", "Bool"),
            ("io.isDir", "Check if path is directory", "io.isDir(${1:path})", "Bool"),
            ("io.isFile", "Check if path is file", "io.isFile(${1:path})", "Bool"),
            ("io.getEnv", "Get environment variable", "io.getEnv(${1:name})", "Option<String>"),
            ("io.currentDir", "Get current directory", "io.currentDir()", "String"),
            ("io.homeDir", "Get home directory", "io.homeDir()", "Option<String>"),
            ("io.hashFile", "Hash file contents", "io.hashFile(${1:path})", "String"),
            ("io.hashString", "Hash a string", "io.hashString(${1:str})", "String"),
            ("io.currentSystem", "Get current system", "io.currentSystem()", "String"),
            
            // Math functions
            ("math.abs", "Absolute value", "math.abs(${1:x})", "Number"),
            ("math.floor", "Floor of number", "math.floor(${1:x})", "Int"),
            ("math.ceil", "Ceiling of number", "math.ceil(${1:x})", "Int"),
            ("math.round", "Round number", "math.round(${1:x})", "Int"),
            ("math.sqrt", "Square root", "math.sqrt(${1:x})", "Float"),
            ("math.pow", "Power", "math.pow(${1:base}, ${2:exp})", "Number"),
            ("math.log", "Natural logarithm", "math.log(${1:x})", "Float"),
            ("math.sin", "Sine", "math.sin(${1:x})", "Float"),
            ("math.cos", "Cosine", "math.cos(${1:x})", "Float"),
            ("math.tan", "Tangent", "math.tan(${1:x})", "Float"),
            ("math.max", "Maximum of two numbers", "math.max(${1:a}, ${2:b})", "Number"),
            ("math.min", "Minimum of two numbers", "math.min(${1:a}, ${2:b})", "Number"),
            ("math.clamp", "Clamp to range", "math.clamp(${1:x}, ${2:min}, ${3:max})", "Number"),
            ("math.pi", "Pi constant", "math.pi", "Float"),
            ("math.e", "Euler's number", "math.e", "Float"),
            ("math.toInt", "Convert to integer", "math.toInt(${1:x})", "Int"),
            ("math.toFloat", "Convert to float", "math.toFloat(${1:x})", "Float"),
            
            // String functions
            ("string.len", "String length", "string.len(${1:s})", "Int"),
            ("string.concat", "Concatenate strings", "string.concat(${1:a}, ${2:b})", "String"),
            ("string.split", "Split string", "string.split(${1:s}, ${2:sep})", "List<String>"),
            ("string.join", "Join strings", "string.join(${1:list}, ${2:sep})", "String"),
            ("string.trim", "Trim whitespace", "string.trim(${1:s})", "String"),
            ("string.upper", "To uppercase", "string.upper(${1:s})", "String"),
            ("string.lower", "To lowercase", "string.lower(${1:s})", "String"),
            ("string.contains", "Check if contains", "string.contains(${1:s}, ${2:sub})", "Bool"),
            ("string.startsWith", "Check prefix", "string.startsWith(${1:s}, ${2:prefix})", "Bool"),
            ("string.endsWith", "Check suffix", "string.endsWith(${1:s}, ${2:suffix})", "Bool"),
            ("string.replace", "Replace substring", "string.replace(${1:s}, ${2:from}, ${3:to})", "String"),
            
            // List functions
            ("list.len", "List length", "list.len(${1:xs})", "Int"),
            ("list.head", "First element", "list.head(${1:xs})", "Option<T>"),
            ("list.tail", "All but first", "list.tail(${1:xs})", "List<T>"),
            ("list.last", "Last element", "list.last(${1:xs})", "Option<T>"),
            ("list.init", "All but last", "list.init(${1:xs})", "List<T>"),
            ("list.reverse", "Reverse list", "list.reverse(${1:xs})", "List<T>"),
            ("list.map", "Map function over list", "list.map(${1:f}, ${2:xs})", "List<U>"),
            ("list.filter", "Filter list", "list.filter(${1:pred}, ${2:xs})", "List<T>"),
            ("list.fold", "Fold list", "list.fold(${1:init}, ${2:f}, ${3:xs})", "U"),
            ("list.concat", "Concatenate lists", "list.concat(${1:xss})", "List<T>"),
            ("list.range", "Create range", "list.range(${1:start}, ${2:end})", "List<Int>"),
            ("list.elem", "Check membership", "list.elem(${1:x}, ${2:xs})", "Bool"),
            
            // Option functions
            ("option.some", "Wrap in Some", "option.some(${1:x})", "Option<T>"),
            ("option.none", "None value", "option.none", "Option<T>"),
            ("option.isSome", "Check if Some", "option.isSome(${1:opt})", "Bool"),
            ("option.isNone", "Check if None", "option.isNone(${1:opt})", "Bool"),
            ("option.unwrap", "Unwrap or panic", "option.unwrap(${1:opt})", "T"),
            ("option.unwrapOr", "Unwrap or default", "option.unwrapOr(${1:opt}, ${2:default})", "T"),
            ("option.map", "Map over option", "option.map(${1:f}, ${2:opt})", "Option<U>"),
            
            // Result functions
            ("result.ok", "Wrap in Ok", "result.ok(${1:x})", "Result<T, E>"),
            ("result.err", "Wrap in Err", "result.err(${1:e})", "Result<T, E>"),
            ("result.isOk", "Check if Ok", "result.isOk(${1:res})", "Bool"),
            ("result.isErr", "Check if Err", "result.isErr(${1:res})", "Bool"),
            ("result.unwrap", "Unwrap or panic", "result.unwrap(${1:res})", "T"),
            ("result.map", "Map over result", "result.map(${1:f}, ${2:res})", "Result<U, E>"),
            
            // Path functions
            ("path.join", "Join paths", "path.join(${1:base}, ${2:part})", "Path"),
            ("path.parent", "Get parent directory", "path.parent(${1:p})", "Option<Path>"),
            ("path.fileName", "Get file name", "path.fileName(${1:p})", "Option<String>"),
            ("path.extension", "Get extension", "path.extension(${1:p})", "Option<String>"),
            ("path.isAbsolute", "Check if absolute", "path.isAbsolute(${1:p})", "Bool"),
            
            // Builtins
            ("assert", "Assert condition", "assert(${1:cond}, ${2:msg})", "Unit"),
            ("force", "Force lazy value", "force(${1:lazy_expr})", "T"),
        ];
        
        stdlib_functions.into_iter()
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
        
        types.into_iter()
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
    fn get_method_completions(&self) -> Vec<CompletionItem> {
        let methods = vec![
            // List methods
            ("map", "Map function over elements", "map(${1:fn(x) x})", "List<U>"),
            ("filter", "Filter elements", "filter(${1:fn(x) true})", "List<T>"),
            ("fold", "Fold with accumulator", "fold(${1:init}, ${2:fn(acc, x) acc})", "U"),
            ("len", "Get length", "len()", "Int"),
            ("first", "Get first element", "first()", "Option<T>"),
            ("last", "Get last element", "last()", "Option<T>"),
            ("get", "Get element at index", "get(${1:index})", "Option<T>"),
            ("reverse", "Reverse elements", "reverse()", "List<T>"),
            ("sum", "Sum of elements", "sum()", "Number"),
            ("all", "Check if all match", "all(${1:fn(x) true})", "Bool"),
            ("any", "Check if any matches", "any(${1:fn(x) false})", "Bool"),
            ("zip", "Zip with another list", "zip(${1:other})", "List<(T, U)>"),
            ("take", "Take first n elements", "take(${1:n})", "List<T>"),
            ("drop", "Drop first n elements", "drop(${1:n})", "List<T>"),
            ("join", "Join with separator", "join(${1:sep})", "String"),
            
            // String methods
            ("split", "Split by separator", "split(${1:sep})", "List<String>"),
            ("trim", "Trim whitespace", "trim()", "String"),
            ("upper", "To uppercase", "upper()", "String"),
            ("lower", "To lowercase", "lower()", "String"),
            ("contains", "Check if contains", "contains(${1:sub})", "Bool"),
            ("startsWith", "Check prefix", "startsWith(${1:prefix})", "Bool"),
            ("endsWith", "Check suffix", "endsWith(${1:suffix})", "Bool"),
            ("replace", "Replace substring", "replace(${1:from}, ${2:to})", "String"),
            ("chars", "Get characters", "chars()", "List<Char>"),
            
            // Option/Result methods
            ("unwrap", "Unwrap value", "unwrap()", "T"),
            ("unwrapOr", "Unwrap or default", "unwrapOr(${1:default})", "T"),
            ("isSome", "Check if Some", "isSome()", "Bool"),
            ("isNone", "Check if None", "isNone()", "Bool"),
            ("isOk", "Check if Ok", "isOk()", "Bool"),
            ("isErr", "Check if Err", "isErr()", "Bool"),
            
            // Record methods  
            ("keys", "Get record keys", "keys()", "List<String>"),
            ("values", "Get record values", "values()", "List<T>"),
            ("hasField", "Check if has field", "hasField(${1:name})", "Bool"),
        ];
        
        methods.into_iter()
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        let query = params.query.to_lowercase();
        let mut symbols = Vec::new();
        
        // Search across all open documents
        for entry in self.documents.iter() {
            let doc = entry.value();
            let uri = Url::parse(&doc.uri).unwrap_or_else(|_| {
                Url::parse("file:///unknown").unwrap()
            });
            
            if let Some(ref index) = doc.symbol_index {
                for (name, defs) in &index.definitions {
                    // Filter by query
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
