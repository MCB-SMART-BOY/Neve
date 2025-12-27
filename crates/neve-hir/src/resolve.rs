//! Name resolution and AST to HIR lowering.

use std::collections::HashMap;
use neve_syntax::{self as ast, SourceFile};
use crate::{
    Module, ModuleId, Item, ItemKind, DefId, LocalId,
    Import, ImportKind,
    FnDef, StructDef, EnumDef, TypeAlias, TraitDef, ImplDef,
    Param, GenericParam, FieldDef, VariantDef, TraitItem, ImplItem,
    Expr, ExprKind, Literal, BinOp, UnaryOp,
    Pattern, PatternKind, MatchArm,
    Stmt, StmtKind,
    Ty, TyKind,
    StringPart,
};

/// Name resolver that builds HIR from AST.
pub struct Resolver {
    /// Counter for generating unique definition IDs
    next_def_id: u32,
    /// Counter for generating unique local IDs
    next_local_id: u32,
    /// Counter for generating unique module IDs
    next_module_id: u32,
    /// Global definitions: name -> DefId
    globals: HashMap<String, DefId>,
    /// Stack of local scopes
    scopes: Vec<HashMap<String, LocalId>>,
    /// Imported names from other modules
    imported: HashMap<String, DefId>,
}

impl Resolver {
    pub fn new() -> Self {
        Self {
            next_def_id: 0,
            next_local_id: 0,
            next_module_id: 0,
            globals: HashMap::new(),
            scopes: Vec::new(),
            imported: HashMap::new(),
        }
    }

    /// Create an unknown type with the given span.
    /// Used during lowering when the type will be inferred later.
    #[inline]
    fn unknown_ty(span: neve_common::Span) -> Ty {
        Ty { kind: TyKind::Unknown, span }
    }

    /// Resolve an AST source file to HIR.
    pub fn resolve(&mut self, file: &SourceFile) -> Module {
        self.resolve_with_name(file, "main".to_string())
    }

    /// Resolve an AST source file to HIR with a specific module name.
    pub fn resolve_with_name(&mut self, file: &SourceFile, name: String) -> Module {
        let module_id = self.fresh_module_id();

        // First pass: collect imports
        let imports = self.collect_imports(file);

        // Second pass: collect all global definitions
        for item in &file.items {
            self.collect_item(item);
        }

        // Third pass: lower all items
        let items = file.items.iter()
            .filter_map(|item| self.lower_item(item))
            .collect();

        Module {
            id: module_id,
            name,
            items,
            imports,
            exports: None, // TODO: Parse explicit exports
        }
    }

    fn fresh_module_id(&mut self) -> ModuleId {
        let id = ModuleId(self.next_module_id);
        self.next_module_id += 1;
        id
    }

    /// Collect all imports from the source file.
    fn collect_imports(&mut self, file: &SourceFile) -> Vec<Import> {
        file.items.iter()
            .filter_map(|item| {
                match &item.kind {
                    ast::ItemKind::Import(import_def) => {
                        let path: Vec<String> = import_def.path.iter()
                            .map(|p| p.name.clone())
                            .collect();
                        
                        let kind = match &import_def.items {
                            ast::ImportItems::Module => ImportKind::Module,
                            ast::ImportItems::Items(items) => {
                                ImportKind::Items(items.iter().map(|i| i.name.clone()).collect())
                            }
                            ast::ImportItems::All => ImportKind::All,
                        };
                        
                        let alias = import_def.alias.as_ref().map(|a| a.name.clone());
                        
                        Some(Import {
                            path,
                            kind,
                            alias,
                            span: item.span,
                        })
                    }
                    _ => None,
                }
            })
            .collect()
    }

    fn fresh_def_id(&mut self) -> DefId {
        let id = DefId(self.next_def_id);
        self.next_def_id += 1;
        id
    }

    fn fresh_local_id(&mut self) -> LocalId {
        let id = LocalId(self.next_local_id);
        self.next_local_id += 1;
        id
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define_local(&mut self, name: String) -> LocalId {
        let id = self.fresh_local_id();
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, id);
        }
        id
    }

    fn lookup_local(&self, name: &str) -> Option<LocalId> {
        for scope in self.scopes.iter().rev() {
            if let Some(&id) = scope.get(name) {
                return Some(id);
            }
        }
        None
    }

    fn lookup_global(&self, name: &str) -> Option<DefId> {
        // First check local globals, then imported names
        self.globals.get(name)
            .or_else(|| self.imported.get(name))
            .copied()
    }
    
    /// Register an imported name for resolution.
    pub fn register_import(&mut self, name: String, def_id: DefId) {
        self.imported.insert(name, def_id);
    }
    
    /// Register multiple imported names from a module registry resolution.
    pub fn register_imports(&mut self, imports: Vec<(String, DefId)>) {
        for (name, def_id) in imports {
            self.imported.insert(name, def_id);
        }
    }

    // === First pass: collect definitions ===

    fn collect_item(&mut self, item: &ast::Item) {
        match &item.kind {
            ast::ItemKind::Let(def) => {
                if let Some(name) = self.pattern_name(&def.pattern) {
                    let id = self.fresh_def_id();
                    self.globals.insert(name, id);
                }
            }
            ast::ItemKind::Fn(def) => {
                let id = self.fresh_def_id();
                self.globals.insert(def.name.name.clone(), id);
            }
            ast::ItemKind::Struct(def) => {
                let id = self.fresh_def_id();
                self.globals.insert(def.name.name.clone(), id);
            }
            ast::ItemKind::Enum(def) => {
                let id = self.fresh_def_id();
                self.globals.insert(def.name.name.clone(), id);
                // Also register variants
                for variant in &def.variants {
                    let vid = self.fresh_def_id();
                    self.globals.insert(variant.name.name.clone(), vid);
                }
            }
            ast::ItemKind::TypeAlias(def) => {
                let id = self.fresh_def_id();
                self.globals.insert(def.name.name.clone(), id);
            }
            ast::ItemKind::Trait(def) => {
                let id = self.fresh_def_id();
                self.globals.insert(def.name.name.clone(), id);
            }
            ast::ItemKind::Impl(_) => {
                // Impls don't introduce names
            }
            ast::ItemKind::Import(_) => {
                // TODO: Handle imports
            }
        }
    }

    fn pattern_name(&self, pattern: &ast::Pattern) -> Option<String> {
        match &pattern.kind {
            ast::PatternKind::Var(ident) => Some(ident.name.clone()),
            _ => None,
        }
    }

    // === Second pass: lower items ===

    fn lower_item(&mut self, item: &ast::Item) -> Option<Item> {
        

        match &item.kind {
            ast::ItemKind::Let(def) => {
                // Top-level let becomes a function with no parameters
                let name = self.pattern_name(&def.pattern)?;
                let id = self.lookup_global(&name)?;
                
                self.push_scope();
                let body = self.lower_expr(&def.value);
                self.pop_scope();

                Some(Item {
                    id,
                    kind: ItemKind::Fn(FnDef {
                        name,
                        generics: Vec::new(),
                        params: Vec::new(),
                        return_ty: Self::unknown_ty(item.span),
                        body,
                    }),
                    span: item.span,
                })
            }
            ast::ItemKind::Fn(def) => {
                let id = self.lookup_global(&def.name.name)?;
                
                self.push_scope();
                
                let generics = self.lower_generics(&def.generics);
                let params: Vec<Param> = def.params.iter()
                    .map(|p| self.lower_param(p))
                    .collect();
                
                let return_ty = def.return_type.as_ref()
                    .map(|t| self.lower_type(t))
                    .unwrap_or_else(|| Self::unknown_ty(def.name.span));
                
                let body = self.lower_expr(&def.body);
                
                self.pop_scope();

                Some(Item {
                    id,
                    kind: ItemKind::Fn(FnDef {
                        name: def.name.name.clone(),
                        generics,
                        params,
                        return_ty,
                        body,
                    }),
                    span: item.span,
                })
            }
            ast::ItemKind::Struct(def) => {
                let id = self.lookup_global(&def.name.name)?;
                let generics = self.lower_generics(&def.generics);
                let fields = def.fields.iter()
                    .map(|f| FieldDef {
                        name: f.name.name.clone(),
                        ty: self.lower_type(&f.ty),
                        span: f.span,
                    })
                    .collect();

                Some(Item {
                    id,
                    kind: ItemKind::Struct(StructDef {
                        name: def.name.name.clone(),
                        generics,
                        fields,
                    }),
                    span: item.span,
                })
            }
            ast::ItemKind::Enum(def) => {
                let id = self.lookup_global(&def.name.name)?;
                let generics = self.lower_generics(&def.generics);
                let variants = def.variants.iter()
                    .map(|v| {
                        let fields = match &v.kind {
                            ast::VariantKind::Unit => Vec::new(),
                            ast::VariantKind::Tuple(types) => types.iter()
                                .map(|t| self.lower_type(t))
                                .collect(),
                            ast::VariantKind::Record(field_defs) => field_defs.iter()
                                .map(|f| self.lower_type(&f.ty))
                                .collect(),
                        };
                        VariantDef {
                            name: v.name.name.clone(),
                            fields,
                            span: v.span,
                        }
                    })
                    .collect();

                Some(Item {
                    id,
                    kind: ItemKind::Enum(EnumDef {
                        name: def.name.name.clone(),
                        generics,
                        variants,
                    }),
                    span: item.span,
                })
            }
            ast::ItemKind::TypeAlias(def) => {
                let id = self.lookup_global(&def.name.name)?;
                let generics = self.lower_generics(&def.generics);
                let ty = self.lower_type(&def.ty);

                Some(Item {
                    id,
                    kind: ItemKind::TypeAlias(TypeAlias {
                        name: def.name.name.clone(),
                        generics,
                        ty,
                    }),
                    span: item.span,
                })
            }
            ast::ItemKind::Trait(def) => {
                let id = self.lookup_global(&def.name.name)?;
                let generics = self.lower_generics(&def.generics);
                
                let items = def.items.iter()
                    .filter_map(|ti| self.lower_trait_item(ti))
                    .collect();

                Some(Item {
                    id,
                    kind: ItemKind::Trait(TraitDef {
                        name: def.name.name.clone(),
                        generics,
                        items,
                        assoc_types: Vec::new(), // TODO: parse associated types from AST
                    }),
                    span: item.span,
                })
            }
            ast::ItemKind::Impl(def) => {
                let id = self.fresh_def_id();
                let generics = self.lower_generics(&def.generics);
                
                let trait_ref = def.trait_.as_ref()
                    .map(|t| self.lower_type(t));
                
                let self_ty = self.lower_type(&def.target);
                
                let items = def.items.iter()
                    .filter_map(|ii| self.lower_impl_item(ii))
                    .collect();

                Some(Item {
                    id,
                    kind: ItemKind::Impl(ImplDef {
                        generics,
                        trait_ref,
                        self_ty,
                        items,
                        assoc_type_impls: Vec::new(), // TODO: parse associated type impls from AST
                    }),
                    span: item.span,
                })
            }
            ast::ItemKind::Import(_) => None,
        }
    }

    fn lower_generics(&self, generics: &[ast::GenericParam]) -> Vec<GenericParam> {
        generics.iter()
            .map(|p| GenericParam {
                name: p.name.name.clone(),
                bounds: p.bounds.iter().map(|b| self.lower_type(b)).collect(),
                span: p.span,
            })
            .collect()
    }

    fn lower_param(&mut self, param: &ast::Param) -> Param {
        let name = self.pattern_name(&param.pattern).unwrap_or_else(|| "_".to_string());
        let id = self.define_local(name.clone());
        let ty = self.lower_type(&param.ty);

        Param {
            id,
            name,
            ty,
            span: param.span,
        }
    }

    fn lower_trait_item(&mut self, item: &ast::TraitItem) -> Option<TraitItem> {
        self.push_scope();
        
        let generics = self.lower_generics(&item.generics);
        let params = item.params.iter()
            .map(|p| self.lower_type(&p.ty))
            .collect();
        let return_ty = item.return_type.as_ref()
            .map(|t| self.lower_type(t))
            .unwrap_or(Ty { kind: TyKind::Unit, span: item.span });
        let default = item.default.as_ref()
            .map(|e| self.lower_expr(e));

        self.pop_scope();

        Some(TraitItem {
            name: item.name.name.clone(),
            generics,
            params,
            return_ty,
            default,
            span: item.span,
        })
    }

    fn lower_impl_item(&mut self, item: &ast::ImplItem) -> Option<ImplItem> {
        self.push_scope();

        let generics = self.lower_generics(&item.generics);
        let params: Vec<Param> = item.params.iter()
            .map(|p| self.lower_param(p))
            .collect();
        let return_ty = item.return_type.as_ref()
            .map(|t| self.lower_type(t))
            .unwrap_or(Ty { kind: TyKind::Unit, span: item.span });
        let body = self.lower_expr(&item.body);

        self.pop_scope();

        Some(ImplItem {
            name: item.name.name.clone(),
            generics,
            params,
            return_ty,
            body,
            span: item.span,
        })
    }

    // === Lower expressions ===

    fn lower_expr(&mut self, expr: &ast::Expr) -> Expr {
        let span = expr.span;
        let kind = match &expr.kind {
            ast::ExprKind::Int(n) => ExprKind::Literal(Literal::Int(*n)),
            ast::ExprKind::Float(f) => ExprKind::Literal(Literal::Float(*f)),
            ast::ExprKind::String(s) => ExprKind::Literal(Literal::String(s.clone())),
            ast::ExprKind::Char(c) => ExprKind::Literal(Literal::Char(*c)),
            ast::ExprKind::Bool(b) => ExprKind::Literal(Literal::Bool(*b)),
            ast::ExprKind::Unit => ExprKind::Literal(Literal::Unit),

            ast::ExprKind::Var(ident) => {
                if let Some(local_id) = self.lookup_local(&ident.name) {
                    ExprKind::Var(local_id)
                } else if let Some(def_id) = self.lookup_global(&ident.name) {
                    ExprKind::Global(def_id)
                } else {
                    // Unknown variable - will be caught during type checking
                    ExprKind::Global(DefId(u32::MAX))
                }
            }

            ast::ExprKind::Path(parts) => {
                // Handle path like `r.a.b` as nested field access
                if parts.is_empty() {
                    ExprKind::Literal(Literal::Unit)
                } else {
                    // Start with the first part as base
                    let first = &parts[0];
                    let mut result_kind = if let Some(local_id) = self.lookup_local(&first.name) {
                        ExprKind::Var(local_id)
                    } else if let Some(def_id) = self.lookup_global(&first.name) {
                        ExprKind::Global(def_id)
                    } else {
                        ExprKind::Global(DefId(u32::MAX))
                    };
                    
                    // Chain field accesses for remaining parts
                    for part in &parts[1..] {
                        let base_expr = Expr {
                            kind: result_kind,
                            ty: Self::unknown_ty(span),
                            span,
                        };
                        result_kind = ExprKind::Field(Box::new(base_expr), part.name.clone());
                    }
                    
                    result_kind
                }
            }

            ast::ExprKind::List(items) => {
                let items = items.iter().map(|e| self.lower_expr(e)).collect();
                ExprKind::List(items)
            }

            ast::ExprKind::Tuple(items) => {
                let items = items.iter().map(|e| self.lower_expr(e)).collect();
                ExprKind::Tuple(items)
            }

            ast::ExprKind::Record(fields) => {
                let fields = fields.iter()
                    .map(|f| {
                        let value = f.value.as_ref()
                            .map(|e| self.lower_expr(e))
                            .unwrap_or_else(|| {
                                // Shorthand: #{ x } means #{ x = x }
                                let name = &f.name.name;
                                if let Some(local_id) = self.lookup_local(name) {
                                    Expr {
                                        kind: ExprKind::Var(local_id),
                                        ty: Self::unknown_ty(span),
                                        span,
                                    }
                                } else if let Some(def_id) = self.lookup_global(name) {
                                    Expr {
                                        kind: ExprKind::Global(def_id),
                                        ty: Self::unknown_ty(span),
                                        span,
                                    }
                                } else {
                                    Expr {
                                        kind: ExprKind::Global(DefId(u32::MAX)),
                                        ty: Self::unknown_ty(span),
                                        span,
                                    }
                                }
                            });
                        (f.name.name.clone(), value)
                    })
                    .collect();
                ExprKind::Record(fields)
            }

            ast::ExprKind::RecordUpdate { base, fields } => {
                // Desugar #{ base | field = value } to a record literal
                // This is a simplification; real implementation would merge
                let base_expr = self.lower_expr(base);
                let update_fields: Vec<(String, Expr)> = fields.iter()
                    .map(|f| {
                        let value = f.value.as_ref()
                            .map(|e| self.lower_expr(e))
                            .unwrap_or_else(|| base_expr.clone());
                        (f.name.name.clone(), value)
                    })
                    .collect();
                ExprKind::Record(update_fields)
            }

            ast::ExprKind::Lambda { params, body } => {
                self.push_scope();
                let params: Vec<Param> = params.iter()
                    .map(|p| {
                        let name = self.pattern_name(&p.pattern).unwrap_or_else(|| "_".to_string());
                        let id = self.define_local(name.clone());
                        Param {
                            id,
                            name,
                            ty: Self::unknown_ty(p.span),
                            span: p.span,
                        }
                    })
                    .collect();
                let body = self.lower_expr(body);
                self.pop_scope();
                ExprKind::Lambda(params, Box::new(body))
            }

            ast::ExprKind::Call { func, args } => {
                let func = self.lower_expr(func);
                let args = args.iter().map(|e| self.lower_expr(e)).collect();
                ExprKind::Call(Box::new(func), args)
            }

            ast::ExprKind::MethodCall { receiver, method, args } => {
                // Desugar method call to function call: receiver.method(args) -> method(receiver, args)
                let recv = self.lower_expr(receiver);
                let mut all_args = vec![recv];
                all_args.extend(args.iter().map(|e| self.lower_expr(e)));
                
                let func = if let Some(def_id) = self.lookup_global(&method.name) {
                    Expr {
                        kind: ExprKind::Global(def_id),
                        ty: Self::unknown_ty(span),
                        span,
                    }
                } else {
                    Expr {
                        kind: ExprKind::Global(DefId(u32::MAX)),
                        ty: Self::unknown_ty(span),
                        span,
                    }
                };
                
                ExprKind::Call(Box::new(func), all_args)
            }

            ast::ExprKind::Field { base, field } => {
                let base = self.lower_expr(base);
                ExprKind::Field(Box::new(base), field.name.clone())
            }

            ast::ExprKind::TupleIndex { base, index } => {
                let base = self.lower_expr(base);
                ExprKind::TupleIndex(Box::new(base), *index)
            }

            ast::ExprKind::Index { base, index } => {
                // Desugar index to a function call: base[index] -> index(base, index)
                let base = self.lower_expr(base);
                let index = self.lower_expr(index);
                let index_fn = Expr {
                    kind: ExprKind::Global(DefId(u32::MAX)), // TODO: resolve index function
                    ty: Self::unknown_ty(span),
                    span,
                };
                ExprKind::Call(Box::new(index_fn), vec![base, index])
            }

            ast::ExprKind::Binary { op, left, right } => {
                let left = self.lower_expr(left);
                let right = self.lower_expr(right);
                let op = self.lower_binop(*op);
                ExprKind::Binary(op, Box::new(left), Box::new(right))
            }

            ast::ExprKind::Unary { op, operand } => {
                let operand = self.lower_expr(operand);
                let op = self.lower_unaryop(*op);
                ExprKind::Unary(op, Box::new(operand))
            }

            ast::ExprKind::If { condition, then_branch, else_branch } => {
                let cond = self.lower_expr(condition);
                let then_br = self.lower_expr(then_branch);
                let else_br = self.lower_expr(else_branch);
                ExprKind::If(Box::new(cond), Box::new(then_br), Box::new(else_br))
            }

            ast::ExprKind::Match { scrutinee, arms } => {
                let scrutinee = self.lower_expr(scrutinee);
                let arms = arms.iter()
                    .map(|arm| self.lower_match_arm(arm))
                    .collect();
                ExprKind::Match(Box::new(scrutinee), arms)
            }

            ast::ExprKind::Block { stmts, expr } => {
                self.push_scope();
                let stmts = stmts.iter()
                    .map(|s| self.lower_stmt(s))
                    .collect();
                let expr = expr.as_ref().map(|e| Box::new(self.lower_expr(e)));
                self.pop_scope();
                ExprKind::Block(stmts, expr)
            }

            ast::ExprKind::Coalesce { value, default } => {
                // Desugar value ?? default to match
                let value = self.lower_expr(value);
                let default = self.lower_expr(default);
                
                // match value { Some(x) => x, None => default }
                let x_id = self.fresh_local_id();
                let arms = vec![
                    MatchArm {
                        pattern: Pattern {
                            kind: PatternKind::Constructor(DefId(u32::MAX), vec![
                                Pattern {
                                    kind: PatternKind::Var(x_id, "x".to_string()),
                                    span,
                                }
                            ]),
                            span,
                        },
                        guard: None,
                        body: Expr {
                            kind: ExprKind::Var(x_id),
                            ty: Self::unknown_ty(span),
                            span,
                        },
                        span,
                    },
                    MatchArm {
                        pattern: Pattern {
                            kind: PatternKind::Wildcard,
                            span,
                        },
                        guard: None,
                        body: default,
                        span,
                    },
                ];
                
                ExprKind::Match(Box::new(value), arms)
            }

            ast::ExprKind::Try(inner) => {
                // Desugar expr? to match expr { Ok(x) => x, Err(e) => return Err(e) }
                let inner = self.lower_expr(inner);
                let x_id = self.fresh_local_id();
                
                let arms = vec![
                    MatchArm {
                        pattern: Pattern {
                            kind: PatternKind::Constructor(DefId(u32::MAX), vec![
                                Pattern {
                                    kind: PatternKind::Var(x_id, "x".to_string()),
                                    span,
                                }
                            ]),
                            span,
                        },
                        guard: None,
                        body: Expr {
                            kind: ExprKind::Var(x_id),
                            ty: Self::unknown_ty(span),
                            span,
                        },
                        span,
                    },
                ];
                
                ExprKind::Match(Box::new(inner), arms)
            }

            ast::ExprKind::Interpolated(parts) => {
                let parts = parts.iter()
                    .map(|part| match part {
                        ast::StringPart::Literal(s) => StringPart::Literal(s.clone()),
                        ast::StringPart::Expr(e) => StringPart::Expr(self.lower_expr(e)),
                    })
                    .collect();
                ExprKind::Interpolated(parts)
            }

            _ => ExprKind::Literal(Literal::Unit),
        };

        Expr {
            kind,
            ty: Self::unknown_ty(span),
            span,
        }
    }

    fn lower_stmt(&mut self, stmt: &ast::Stmt) -> Stmt {
        let span = stmt.span;
        let kind = match &stmt.kind {
            ast::StmtKind::Let { pattern, ty, value } => {
                let name = self.pattern_name(pattern).unwrap_or_else(|| "_".to_string());
                let id = self.define_local(name.clone());
                let ty = ty.as_ref()
                    .map(|t| self.lower_type(t))
                    .unwrap_or_else(|| Self::unknown_ty(span));
                let value = self.lower_expr(value);
                StmtKind::Let(id, name, ty, value)
            }
            ast::StmtKind::Expr(e) => {
                let expr = self.lower_expr(e);
                StmtKind::Expr(expr)
            }
        };

        Stmt { kind, span }
    }

    fn lower_match_arm(&mut self, arm: &ast::MatchArm) -> MatchArm {
        self.push_scope();
        let pattern = self.lower_pattern(&arm.pattern);
        let guard = arm.guard.as_ref().map(|e| self.lower_expr(e));
        let body = self.lower_expr(&arm.body);
        self.pop_scope();

        MatchArm {
            pattern,
            guard,
            body,
            span: arm.span,
        }
    }

    fn lower_pattern(&mut self, pattern: &ast::Pattern) -> Pattern {
        let span = pattern.span;
        let kind = match &pattern.kind {
            ast::PatternKind::Wildcard => PatternKind::Wildcard,
            
            ast::PatternKind::Var(ident) => {
                if ident.name == "_" {
                    PatternKind::Wildcard
                } else {
                    let id = self.define_local(ident.name.clone());
                    PatternKind::Var(id, ident.name.clone())
                }
            }

            ast::PatternKind::Literal(lit) => {
                let literal = match lit {
                    ast::LiteralPattern::Int(n) => Literal::Int(*n),
                    ast::LiteralPattern::Float(f) => Literal::Float(*f),
                    ast::LiteralPattern::String(s) => Literal::String(s.clone()),
                    ast::LiteralPattern::Char(c) => Literal::Char(*c),
                    ast::LiteralPattern::Bool(b) => Literal::Bool(*b),
                };
                PatternKind::Literal(literal)
            }

            ast::PatternKind::Tuple(patterns) => {
                let patterns = patterns.iter()
                    .map(|p| self.lower_pattern(p))
                    .collect();
                PatternKind::Tuple(patterns)
            }

            ast::PatternKind::List(patterns) => {
                let patterns = patterns.iter()
                    .map(|p| self.lower_pattern(p))
                    .collect();
                PatternKind::List(patterns)
            }

            ast::PatternKind::Record { fields, .. } => {
                let fields = fields.iter()
                    .map(|f| {
                        let pattern = f.pattern.as_ref()
                            .map(|p| self.lower_pattern(p))
                            .unwrap_or_else(|| {
                                let id = self.define_local(f.name.name.clone());
                                Pattern {
                                    kind: PatternKind::Var(id, f.name.name.clone()),
                                    span,
                                }
                            });
                        (f.name.name.clone(), pattern)
                    })
                    .collect();
                PatternKind::Record(fields)
            }

            ast::PatternKind::Constructor { path, args } => {
                let def_id = path.first()
                    .and_then(|p| self.lookup_global(&p.name))
                    .unwrap_or(DefId(u32::MAX));
                let args = args.iter()
                    .map(|p| self.lower_pattern(p))
                    .collect();
                PatternKind::Constructor(def_id, args)
            }

            ast::PatternKind::Or(patterns) => {
                // For now, just use the first alternative
                if let Some(first) = patterns.first() {
                    return self.lower_pattern(first);
                }
                PatternKind::Wildcard
            }

            ast::PatternKind::Binding { name, pattern } => {
                let id = self.define_local(name.name.clone());
                let _inner = self.lower_pattern(pattern);
                // For @ patterns, we bind the whole value
                PatternKind::Var(id, name.name.clone())
            }

            _ => PatternKind::Wildcard,
        };

        Pattern { kind, span }
    }

    // === Lower types ===

    fn lower_type(&self, ty: &ast::Type) -> Ty {
        let span = ty.span;
        let kind = match &ty.kind {
            ast::TypeKind::Named { path, args } => {
                if path.len() == 1 && args.is_empty() {
                    let name = &path[0].name;
                    match name.as_str() {
                        "Int" => TyKind::Int,
                        "Float" => TyKind::Float,
                        "Bool" => TyKind::Bool,
                        "Char" => TyKind::Char,
                        "String" => TyKind::String,
                        "Unit" => TyKind::Unit,
                        _ => {
                            if let Some(def_id) = self.lookup_global(name) {
                                TyKind::Named(def_id, Vec::new())
                            } else {
                                TyKind::Unknown
                            }
                        }
                    }
                } else if let Some(first) = path.first() {
                    if let Some(def_id) = self.lookup_global(&first.name) {
                        let lowered_args = args.iter().map(|t| self.lower_type(t)).collect();
                        TyKind::Named(def_id, lowered_args)
                    } else {
                        TyKind::Unknown
                    }
                } else {
                    TyKind::Unknown
                }
            }

            ast::TypeKind::Function { params, result } => {
                let params = params.iter().map(|t| self.lower_type(t)).collect();
                let ret = self.lower_type(result);
                TyKind::Fn(params, Box::new(ret))
            }

            ast::TypeKind::Tuple(types) => {
                let types = types.iter().map(|t| self.lower_type(t)).collect();
                TyKind::Tuple(types)
            }

            ast::TypeKind::Record(fields) => {
                let fields = fields.iter()
                    .map(|f| (f.name.name.clone(), self.lower_type(&f.ty)))
                    .collect();
                TyKind::Record(fields)
            }

            ast::TypeKind::Unit => TyKind::Unit,

            ast::TypeKind::Infer => TyKind::Unknown,
        };

        Ty { kind, span }
    }

    // === Lower operators ===

    fn lower_binop(&self, op: ast::BinOp) -> BinOp {
        match op {
            ast::BinOp::Add => BinOp::Add,
            ast::BinOp::Sub => BinOp::Sub,
            ast::BinOp::Mul => BinOp::Mul,
            ast::BinOp::Div => BinOp::Div,
            ast::BinOp::Mod => BinOp::Mod,
            ast::BinOp::Pow => BinOp::Pow,
            ast::BinOp::Eq => BinOp::Eq,
            ast::BinOp::Ne => BinOp::Ne,
            ast::BinOp::Lt => BinOp::Lt,
            ast::BinOp::Le => BinOp::Le,
            ast::BinOp::Gt => BinOp::Gt,
            ast::BinOp::Ge => BinOp::Ge,
            ast::BinOp::And => BinOp::And,
            ast::BinOp::Or => BinOp::Or,
            ast::BinOp::Concat => BinOp::Concat,
            ast::BinOp::Merge => BinOp::Merge,
            ast::BinOp::Pipe => BinOp::Pipe,
        }
    }

    fn lower_unaryop(&self, op: ast::UnaryOp) -> UnaryOp {
        match op {
            ast::UnaryOp::Neg => UnaryOp::Neg,
            ast::UnaryOp::Not => UnaryOp::Not,
        }
    }
}

impl Default for Resolver {
    fn default() -> Self {
        Self::new()
    }
}
