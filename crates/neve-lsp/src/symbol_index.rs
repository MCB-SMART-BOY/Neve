//! Symbol indexing for LSP features like go-to-definition and find references.
//! 用于跳转到定义和查找引用等 LSP 功能的符号索引。
//!
//! Builds an index of all symbols and references in a document.
//! 构建文档中所有符号和引用的索引。

use neve_common::Span;
use neve_syntax::{
    Expr, ExprKind, Item, ItemKind, Pattern, PatternKind, SourceFile, Stmt, StmtKind,
};
use std::collections::HashMap;

/// A symbol in the source code.
/// 源代码中的符号。
#[derive(Debug, Clone)]
pub struct Symbol {
    /// The name of the symbol. / 符号的名称。
    pub name: String,
    /// The kind of symbol. / 符号的类型。
    pub kind: SymbolKind,
    /// The span where the symbol is defined. / 符号定义的范围。
    pub def_span: Span,
    /// The full span of the definition (e.g., entire function body).
    /// 定义的完整范围（例如，整个函数体）。
    pub full_span: Span,
}

/// The kind of a symbol.
/// 符号的类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    /// Function. / 函数。
    Function,
    /// Variable. / 变量。
    Variable,
    /// Parameter. / 参数。
    Parameter,
    /// Type alias. / 类型别名。
    TypeAlias,
    /// Struct. / 结构体。
    Struct,
    /// Enum. / 枚举。
    Enum,
    /// Enum variant. / 枚举变体。
    Variant,
    /// Trait. / Trait。
    Trait,
    /// Field. / 字段。
    Field,
    /// Method. / 方法。
    Method,
}

/// A reference to a symbol.
/// 对符号的引用。
#[derive(Debug, Clone)]
pub struct SymbolRef {
    /// The name being referenced. / 被引用的名称。
    pub name: String,
    /// The span of the reference. / 引用的范围。
    pub span: Span,
    /// Whether this is a write (definition) or read (usage).
    /// 这是写入（定义）还是读取（使用）。
    pub is_write: bool,
}

/// Index of all symbols and references in a document.
/// 文档中所有符号和引用的索引。
#[derive(Debug, Default)]
pub struct SymbolIndex {
    /// All symbol definitions, keyed by name. / 所有符号定义，按名称索引。
    pub definitions: HashMap<String, Vec<Symbol>>,
    /// All symbol references. / 所有符号引用。
    pub references: Vec<SymbolRef>,
    /// Scope-aware symbol table for local variable resolution.
    /// 用于局部变量解析的作用域感知符号表。
    scopes: Vec<HashMap<String, Symbol>>,
}

impl SymbolIndex {
    /// Create a new empty symbol index.
    /// 创建新的空符号索引。
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a symbol index from an AST.
    /// 从 AST 构建符号索引。
    pub fn from_ast(ast: &SourceFile) -> Self {
        let mut index = Self::new();
        index.index_source_file(ast);
        index
    }

    /// Find the definition of a symbol at the given offset.
    /// 在给定偏移量处查找符号的定义。
    pub fn find_definition_at(&self, offset: usize) -> Option<&Symbol> {
        // First, find what reference is at this offset
        // 首先，查找此偏移量处的引用
        let ref_at_offset = self.references.iter().find(|r| {
            let start: usize = r.span.start.into();
            let end: usize = r.span.end.into();
            start <= offset && offset < end
        })?;

        // Then find the definition for this name
        // 然后查找此名称的定义
        self.definitions.get(&ref_at_offset.name)?.first()
    }

    /// Find all references to the symbol at the given offset.
    /// 查找给定偏移量处符号的所有引用。
    pub fn find_references_at(&self, offset: usize, include_declaration: bool) -> Vec<&SymbolRef> {
        // First, find what symbol is at this offset
        // 首先，查找此偏移量处的符号
        let name = self.find_name_at(offset);

        if let Some(name) = name {
            self.references
                .iter()
                .filter(|r| r.name == name && (include_declaration || !r.is_write))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Find the name of the symbol at the given offset.
    /// 查找给定偏移量处符号的名称。
    pub fn find_name_at(&self, offset: usize) -> Option<String> {
        // Check references / 检查引用
        for r in &self.references {
            let start: usize = r.span.start.into();
            let end: usize = r.span.end.into();
            if start <= offset && offset < end {
                return Some(r.name.clone());
            }
        }

        // Check definitions / 检查定义
        for symbols in self.definitions.values() {
            for sym in symbols {
                let start: usize = sym.def_span.start.into();
                let end: usize = sym.def_span.end.into();
                if start <= offset && offset < end {
                    return Some(sym.name.clone());
                }
            }
        }

        None
    }

    /// Get all references to a symbol by name.
    /// 按名称获取符号的所有引用。
    pub fn get_references(&self, name: &str) -> Vec<&SymbolRef> {
        self.references.iter().filter(|r| r.name == name).collect()
    }

    /// Get all definitions for a name.
    /// 获取名称的所有定义。
    pub fn get_definitions(&self, name: &str) -> Option<&Vec<Symbol>> {
        self.definitions.get(name)
    }

    // === Indexing methods / 索引方法 ===

    fn index_source_file(&mut self, file: &SourceFile) {
        for item in &file.items {
            self.index_item(item);
        }
    }

    fn index_item(&mut self, item: &Item) {
        match &item.kind {
            ItemKind::Let(def) => {
                self.index_pattern(&def.pattern, true);
                self.index_expr(&def.value);
            }
            ItemKind::Fn(def) => {
                let symbol = Symbol {
                    name: def.name.name.clone(),
                    kind: SymbolKind::Function,
                    def_span: def.name.span,
                    full_span: item.span,
                };
                self.add_definition(symbol.clone());
                self.add_reference(SymbolRef {
                    name: def.name.name.clone(),
                    span: def.name.span,
                    is_write: true,
                });

                // Index parameters as Parameter kind
                // 将参数索引为 Parameter 类型
                self.push_scope();
                for param in &def.params {
                    self.index_param_pattern(&param.pattern);
                }
                self.index_expr(&def.body);
                self.pop_scope();
            }
            ItemKind::Struct(def) => {
                let symbol = Symbol {
                    name: def.name.name.clone(),
                    kind: SymbolKind::Struct,
                    def_span: def.name.span,
                    full_span: item.span,
                };
                self.add_definition(symbol);
                self.add_reference(SymbolRef {
                    name: def.name.name.clone(),
                    span: def.name.span,
                    is_write: true,
                });

                // Index fields / 索引字段
                for field in &def.fields {
                    let field_symbol = Symbol {
                        name: field.name.name.clone(),
                        kind: SymbolKind::Field,
                        def_span: field.name.span,
                        full_span: field.span,
                    };
                    self.add_definition(field_symbol);
                }
            }
            ItemKind::Enum(def) => {
                let symbol = Symbol {
                    name: def.name.name.clone(),
                    kind: SymbolKind::Enum,
                    def_span: def.name.span,
                    full_span: item.span,
                };
                self.add_definition(symbol);
                self.add_reference(SymbolRef {
                    name: def.name.name.clone(),
                    span: def.name.span,
                    is_write: true,
                });

                // Index variants / 索引变体
                for variant in &def.variants {
                    let variant_symbol = Symbol {
                        name: variant.name.name.clone(),
                        kind: SymbolKind::Variant,
                        def_span: variant.name.span,
                        full_span: variant.span,
                    };
                    self.add_definition(variant_symbol);
                    self.add_reference(SymbolRef {
                        name: variant.name.name.clone(),
                        span: variant.name.span,
                        is_write: true,
                    });
                }
            }
            ItemKind::TypeAlias(def) => {
                let symbol = Symbol {
                    name: def.name.name.clone(),
                    kind: SymbolKind::TypeAlias,
                    def_span: def.name.span,
                    full_span: item.span,
                };
                self.add_definition(symbol);
                self.add_reference(SymbolRef {
                    name: def.name.name.clone(),
                    span: def.name.span,
                    is_write: true,
                });
            }
            ItemKind::Trait(def) => {
                let symbol = Symbol {
                    name: def.name.name.clone(),
                    kind: SymbolKind::Trait,
                    def_span: def.name.span,
                    full_span: item.span,
                };
                self.add_definition(symbol);
                self.add_reference(SymbolRef {
                    name: def.name.name.clone(),
                    span: def.name.span,
                    is_write: true,
                });

                // Index trait methods / 索引 trait 方法
                for trait_item in &def.items {
                    let method_symbol = Symbol {
                        name: trait_item.name.name.clone(),
                        kind: SymbolKind::Method,
                        def_span: trait_item.name.span,
                        full_span: trait_item.span,
                    };
                    self.add_definition(method_symbol);
                }
            }
            ItemKind::Impl(def) => {
                // Index impl methods / 索引 impl 方法
                for impl_item in &def.items {
                    let method_symbol = Symbol {
                        name: impl_item.name.name.clone(),
                        kind: SymbolKind::Method,
                        def_span: impl_item.name.span,
                        full_span: impl_item.span,
                    };
                    self.add_definition(method_symbol);

                    // Index method body / 索引方法体
                    self.push_scope();
                    for param in &impl_item.params {
                        self.index_pattern(&param.pattern, true);
                    }
                    self.index_expr(&impl_item.body);
                    self.pop_scope();
                }
            }
            ItemKind::Import(_) => {
                // Imports don't define new symbols in the current module
                // 导入不会在当前模块中定义新符号
            }
        }
    }

    fn index_expr(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Var(ident) => {
                self.add_reference(SymbolRef {
                    name: ident.name.clone(),
                    span: ident.span,
                    is_write: false,
                });
            }
            ExprKind::Path(parts) => {
                for part in parts {
                    self.add_reference(SymbolRef {
                        name: part.name.clone(),
                        span: part.span,
                        is_write: false,
                    });
                }
            }
            ExprKind::Lambda { params, body } => {
                self.push_scope();
                for param in params {
                    self.index_pattern(&param.pattern, true);
                }
                self.index_expr(body);
                self.pop_scope();
            }
            ExprKind::Call { func, args } => {
                self.index_expr(func);
                for arg in args {
                    self.index_expr(arg);
                }
            }
            ExprKind::MethodCall {
                receiver,
                method,
                args,
            } => {
                self.index_expr(receiver);
                self.add_reference(SymbolRef {
                    name: method.name.clone(),
                    span: method.span,
                    is_write: false,
                });
                for arg in args {
                    self.index_expr(arg);
                }
            }
            ExprKind::Field { base, field } => {
                self.index_expr(base);
                self.add_reference(SymbolRef {
                    name: field.name.clone(),
                    span: field.span,
                    is_write: false,
                });
            }
            ExprKind::TupleIndex { base, .. } => {
                self.index_expr(base);
            }
            ExprKind::Index { base, index } => {
                self.index_expr(base);
                self.index_expr(index);
            }
            ExprKind::List(items) => {
                for item in items {
                    self.index_expr(item);
                }
            }
            ExprKind::Tuple(items) => {
                for item in items {
                    self.index_expr(item);
                }
            }
            ExprKind::Record(fields) => {
                for field in fields {
                    if let Some(value) = &field.value {
                        self.index_expr(value);
                    } else {
                        // Shorthand: #{ x } means #{ x = x }
                        // 简写：#{ x } 表示 #{ x = x }
                        self.add_reference(SymbolRef {
                            name: field.name.name.clone(),
                            span: field.name.span,
                            is_write: false,
                        });
                    }
                }
            }
            ExprKind::RecordUpdate { base, fields } => {
                self.index_expr(base);
                for field in fields {
                    if let Some(value) = &field.value {
                        self.index_expr(value);
                    }
                }
            }
            ExprKind::Binary { left, right, .. } => {
                self.index_expr(left);
                self.index_expr(right);
            }
            ExprKind::Unary { operand, .. } => {
                self.index_expr(operand);
            }
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.index_expr(condition);
                self.index_expr(then_branch);
                self.index_expr(else_branch);
            }
            ExprKind::Match { scrutinee, arms } => {
                self.index_expr(scrutinee);
                for arm in arms {
                    self.push_scope();
                    self.index_pattern(&arm.pattern, true);
                    if let Some(guard) = &arm.guard {
                        self.index_expr(guard);
                    }
                    self.index_expr(&arm.body);
                    self.pop_scope();
                }
            }
            ExprKind::Block { stmts, expr } => {
                self.push_scope();
                for stmt in stmts {
                    self.index_stmt(stmt);
                }
                if let Some(e) = expr {
                    self.index_expr(e);
                }
                self.pop_scope();
            }
            ExprKind::Coalesce { value, default } => {
                self.index_expr(value);
                self.index_expr(default);
            }
            ExprKind::Try(inner) => {
                self.index_expr(inner);
            }
            ExprKind::ListComp { body, generators } => {
                self.push_scope();
                // Index generators (they introduce bindings)
                // 索引生成器（它们引入绑定）
                for generator in generators {
                    self.index_expr(&generator.iter);
                    self.index_pattern(&generator.pattern, true);
                    if let Some(cond) = &generator.condition {
                        self.index_expr(cond);
                    }
                }
                self.index_expr(body);
                self.pop_scope();
            }
            ExprKind::SafeField { base, field } => {
                self.index_expr(base);
                self.add_reference(SymbolRef {
                    name: field.name.clone(),
                    span: field.span,
                    is_write: false,
                });
            }
            ExprKind::Let {
                pattern,
                value,
                body,
                ..
            } => {
                self.push_scope();
                self.index_expr(value);
                self.index_pattern(pattern, true);
                self.index_expr(body);
                self.pop_scope();
            }
            ExprKind::Lazy(inner) => {
                self.index_expr(inner);
            }
            ExprKind::Interpolated(parts) => {
                for part in parts {
                    if let neve_syntax::StringPart::Expr(e) = part {
                        self.index_expr(e);
                    }
                }
            }
            // Literals don't reference symbols / 字面量不引用符号
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::String(_)
            | ExprKind::Char(_)
            | ExprKind::Bool(_)
            | ExprKind::Unit
            | ExprKind::PathLit(_) => {}
        }
    }

    fn index_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Let { pattern, value, .. } => {
                self.index_expr(value);
                self.index_pattern(pattern, true);
            }
            StmtKind::Expr(e) => {
                self.index_expr(e);
            }
        }
    }

    /// Index a pattern that is a function parameter.
    /// 索引作为函数参数的模式。
    fn index_param_pattern(&mut self, pattern: &Pattern) {
        match &pattern.kind {
            PatternKind::Var(ident) => {
                if ident.name != "_" {
                    let symbol = Symbol {
                        name: ident.name.clone(),
                        kind: SymbolKind::Parameter,
                        def_span: ident.span,
                        full_span: pattern.span,
                    };
                    self.add_definition(symbol.clone());
                    self.add_to_scope(symbol);
                    self.add_reference(SymbolRef {
                        name: ident.name.clone(),
                        span: ident.span,
                        is_write: true,
                    });
                }
            }
            PatternKind::Tuple(patterns) => {
                for p in patterns {
                    self.index_param_pattern(p);
                }
            }
            PatternKind::Record { fields, .. } => {
                for field in fields {
                    if let Some(p) = &field.pattern {
                        self.index_param_pattern(p);
                    } else {
                        let symbol = Symbol {
                            name: field.name.name.clone(),
                            kind: SymbolKind::Parameter,
                            def_span: field.name.span,
                            full_span: field.span,
                        };
                        self.add_definition(symbol.clone());
                        self.add_to_scope(symbol);
                        self.add_reference(SymbolRef {
                            name: field.name.name.clone(),
                            span: field.name.span,
                            is_write: true,
                        });
                    }
                }
            }
            _ => {
                // For other patterns, fall back to regular indexing
                // 对于其他模式，回退到常规索引
                self.index_pattern(pattern, true);
            }
        }
    }

    fn index_pattern(&mut self, pattern: &Pattern, is_definition: bool) {
        match &pattern.kind {
            PatternKind::Var(ident) => {
                if ident.name != "_" {
                    if is_definition {
                        let symbol = Symbol {
                            name: ident.name.clone(),
                            kind: SymbolKind::Variable,
                            def_span: ident.span,
                            full_span: pattern.span,
                        };
                        self.add_definition(symbol.clone());
                        self.add_to_scope(symbol);
                    }
                    self.add_reference(SymbolRef {
                        name: ident.name.clone(),
                        span: ident.span,
                        is_write: is_definition,
                    });
                }
            }
            PatternKind::Tuple(patterns) => {
                for p in patterns {
                    self.index_pattern(p, is_definition);
                }
            }
            PatternKind::List(patterns) => {
                for p in patterns {
                    self.index_pattern(p, is_definition);
                }
            }
            PatternKind::Record { fields, .. } => {
                for field in fields {
                    if let Some(p) = &field.pattern {
                        self.index_pattern(p, is_definition);
                    } else {
                        // Shorthand: #{ x } means #{ x = x }
                        // 简写：#{ x } 表示 #{ x = x }
                        if is_definition {
                            let symbol = Symbol {
                                name: field.name.name.clone(),
                                kind: SymbolKind::Variable,
                                def_span: field.name.span,
                                full_span: field.span,
                            };
                            self.add_definition(symbol.clone());
                            self.add_to_scope(symbol);
                        }
                        self.add_reference(SymbolRef {
                            name: field.name.name.clone(),
                            span: field.name.span,
                            is_write: is_definition,
                        });
                    }
                }
            }
            PatternKind::Constructor { path, args } => {
                // Constructor name is a reference
                // 构造函数名是一个引用
                for part in path {
                    self.add_reference(SymbolRef {
                        name: part.name.clone(),
                        span: part.span,
                        is_write: false,
                    });
                }
                // Pattern arguments may introduce bindings
                // 模式参数可能引入绑定
                for arg in args {
                    self.index_pattern(arg, is_definition);
                }
            }
            PatternKind::Or(patterns) => {
                for p in patterns {
                    self.index_pattern(p, is_definition);
                }
            }
            PatternKind::Binding { name, pattern } => {
                if is_definition {
                    let symbol = Symbol {
                        name: name.name.clone(),
                        kind: SymbolKind::Variable,
                        def_span: name.span,
                        full_span: pattern.span,
                    };
                    self.add_definition(symbol.clone());
                    self.add_to_scope(symbol);
                }
                self.add_reference(SymbolRef {
                    name: name.name.clone(),
                    span: name.span,
                    is_write: is_definition,
                });
                self.index_pattern(pattern, is_definition);
            }
            PatternKind::ListRest { init, rest, tail } => {
                for p in init {
                    self.index_pattern(p, is_definition);
                }
                if let Some(r) = rest {
                    self.index_pattern(r, is_definition);
                }
                for p in tail {
                    self.index_pattern(p, is_definition);
                }
            }
            // These don't introduce or reference symbols
            // 这些不引入或引用符号
            PatternKind::Wildcard | PatternKind::Literal(_) => {}
        }
    }

    // === Scope management / 作用域管理 ===

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn add_to_scope(&mut self, symbol: Symbol) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(symbol.name.clone(), symbol);
        }
    }

    fn add_definition(&mut self, symbol: Symbol) {
        self.definitions
            .entry(symbol.name.clone())
            .or_default()
            .push(symbol);
    }

    fn add_reference(&mut self, reference: SymbolRef) {
        self.references.push(reference);
    }
}
