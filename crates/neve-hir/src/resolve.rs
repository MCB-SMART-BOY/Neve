//! Name resolution and AST to HIR lowering.
//! 名称解析和 AST 到 HIR 的降级转换。

use crate::{
    AssocTypeDef, AssocTypeImpl, BinOp, DefId, EnumDef, Expr, ExprKind, FieldDef, FnDef,
    GenericParam, ImplDef, ImplItem, Import, ImportKind, ImportPathPrefix, Item, ItemKind, Literal,
    LocalId, MatchArm, Module, ModuleId, ModuleLoader, Param, Pattern, PatternKind, Stmt, StmtKind,
    StringPart, StructDef, TraitDef, TraitItem, Ty, TyKind, TypeAlias, UnaryOp, VariantDef,
};
use neve_syntax::{self as ast, SourceFile};
use std::collections::HashMap;
use std::path::Path;

/// Name resolver that builds HIR from AST.
/// 从 AST 构建 HIR 的名称解析器。
pub struct Resolver {
    /// Counter for generating unique definition IDs. / 生成唯一定义 ID 的计数器。
    next_def_id: u32,
    /// Counter for generating unique local IDs. / 生成唯一局部 ID 的计数器。
    next_local_id: u32,
    /// Counter for generating unique module IDs. / 生成唯一模块 ID 的计数器。
    next_module_id: u32,
    /// Global definitions: name -> DefId. / 全局定义：名称 -> DefId。
    globals: HashMap<String, DefId>,
    /// Stack of local scopes. / 局部作用域栈。
    scopes: Vec<HashMap<String, LocalId>>,
    /// Imported names from other modules. / 从其他模块导入的名称。
    imported: HashMap<String, DefId>,
    /// Current module path (for relative imports). / 当前模块路径（用于相对导入）。
    current_module_path: Vec<String>,
    /// Module loader for resolving imports. / 用于解析导入的模块加载器。
    module_loader: Option<ModuleLoader>,
}

impl Resolver {
    /// Create a new resolver.
    /// 创建新的解析器。
    pub fn new() -> Self {
        Self {
            next_def_id: 0,
            next_local_id: 0,
            next_module_id: 0,
            globals: HashMap::new(),
            scopes: Vec::new(),
            imported: HashMap::new(),
            current_module_path: Vec::new(),
            module_loader: None,
        }
    }

    /// Create a new resolver with a module loader for the given root directory.
    /// 为给定的根目录创建带有模块加载器的新解析器。
    pub fn with_root_dir(root_dir: impl AsRef<Path>) -> Self {
        Self {
            next_def_id: 0,
            next_local_id: 0,
            next_module_id: 0,
            globals: HashMap::new(),
            scopes: Vec::new(),
            imported: HashMap::new(),
            current_module_path: Vec::new(),
            module_loader: Some(ModuleLoader::new(root_dir)),
        }
    }

    /// Set the module loader.
    /// 设置模块加载器。
    pub fn set_module_loader(&mut self, loader: ModuleLoader) {
        self.module_loader = Some(loader);
    }

    /// Get the module loader.
    /// 获取模块加载器。
    pub fn module_loader(&self) -> Option<&ModuleLoader> {
        self.module_loader.as_ref()
    }

    /// Get mutable access to the module loader.
    /// 获取模块加载器的可变引用。
    pub fn module_loader_mut(&mut self) -> Option<&mut ModuleLoader> {
        self.module_loader.as_mut()
    }

    /// Set the current module path for relative import resolution.
    /// 设置当前模块路径以解析相对导入。
    pub fn set_current_module_path(&mut self, path: Vec<String>) {
        self.current_module_path = path;
    }

    /// Get the current module path.
    /// 获取当前模块路径。
    pub fn current_module_path(&self) -> &[String] {
        &self.current_module_path
    }

    /// Create an unknown type with the given span.
    /// Used during lowering when the type will be inferred later.
    /// 创建具有给定位置的未知类型。
    /// 在降级过程中使用，类型将在稍后推断。
    #[inline]
    fn unknown_ty(span: neve_common::Span) -> Ty {
        Ty {
            kind: TyKind::Unknown,
            span,
        }
    }

    /// Resolve an AST source file to HIR.
    /// 将 AST 源文件解析为 HIR。
    pub fn resolve(&mut self, file: &SourceFile) -> Module {
        self.resolve_with_name(file, "main".to_string())
    }

    /// Resolve an AST source file to HIR with a specific module name.
    /// 使用特定模块名称将 AST 源文件解析为 HIR。
    pub fn resolve_with_name(&mut self, file: &SourceFile, name: String) -> Module {
        self.resolve_with_path(file, name, Vec::new())
    }

    /// Resolve an AST source file to HIR with module path for relative imports.
    /// 使用模块路径（用于相对导入）将 AST 源文件解析为 HIR。
    pub fn resolve_with_path(
        &mut self,
        file: &SourceFile,
        name: String,
        module_path: Vec<String>,
    ) -> Module {
        let module_id = self.fresh_module_id();

        // Set current module path for relative import resolution
        // 设置当前模块路径以解析相对导入
        self.current_module_path = module_path;

        // First pass: collect imports and resolve them
        // 第一遍：收集导入并解析它们
        let imports = self.collect_imports(file);

        // Process imports to bring names into scope
        // 处理导入以将名称引入作用域
        self.process_imports(&imports);

        // Second pass: collect all global definitions
        // 第二遍：收集所有全局定义
        for item in &file.items {
            self.collect_item(item);
        }

        // Third pass: lower all items
        // 第三遍：降级所有项
        let items = file
            .items
            .iter()
            .filter_map(|item| self.lower_item(item))
            .collect();

        // Collect exports based on visibility
        // 根据可见性收集导出
        let exports = self.collect_exports(file);

        Module {
            id: module_id,
            name,
            items,
            imports,
            exports,
        }
    }

    /// Process imports to bring names into scope.
    /// 处理导入以将名称引入作用域。
    fn process_imports(&mut self, imports: &[Import]) {
        for import in imports {
            if let Some(ref loader) = self.module_loader {
                // Use the module loader to resolve the import
                // 使用模块加载器解析导入
                match loader.resolve_import(import, &self.current_module_path) {
                    Ok(resolved) => {
                        for (name, def_id) in resolved {
                            self.imported.insert(name, def_id);
                        }
                    }
                    Err(_e) => {
                        // Import resolution failed - will be reported during type checking
                        // 导入解析失败 - 将在类型检查期间报告
                    }
                }
            }
        }
    }

    /// Collect exported items based on visibility.
    /// 根据可见性收集导出的项。
    fn collect_exports(&self, file: &SourceFile) -> Option<Vec<String>> {
        let mut exports = Vec::new();

        for item in &file.items {
            match &item.kind {
                ast::ItemKind::Let(def) if def.visibility == ast::Visibility::Public => {
                    if let Some(name) = self.pattern_name(&def.pattern) {
                        exports.push(name);
                    }
                }
                ast::ItemKind::Fn(def) if def.visibility == ast::Visibility::Public => {
                    exports.push(def.name.name.clone());
                }
                ast::ItemKind::Struct(def) if def.visibility == ast::Visibility::Public => {
                    exports.push(def.name.name.clone());
                }
                ast::ItemKind::Enum(def) if def.visibility == ast::Visibility::Public => {
                    exports.push(def.name.name.clone());
                    // Also export variants
                    // 同时导出变体
                    for variant in &def.variants {
                        exports.push(variant.name.name.clone());
                    }
                }
                ast::ItemKind::TypeAlias(def) if def.visibility == ast::Visibility::Public => {
                    exports.push(def.name.name.clone());
                }
                ast::ItemKind::Trait(def) if def.visibility == ast::Visibility::Public => {
                    exports.push(def.name.name.clone());
                }
                _ => {}
            }
        }

        if exports.is_empty() {
            None
        } else {
            Some(exports)
        }
    }

    /// Allocate a fresh module ID.
    /// 分配新的模块 ID。
    fn fresh_module_id(&mut self) -> ModuleId {
        let id = ModuleId(self.next_module_id);
        self.next_module_id += 1;
        id
    }

    /// Collect all imports from the source file.
    /// 从源文件收集所有导入。
    fn collect_imports(&mut self, file: &SourceFile) -> Vec<Import> {
        file.items
            .iter()
            .filter_map(|item| match &item.kind {
                ast::ItemKind::Import(import_def) => {
                    let prefix = match import_def.prefix {
                        ast::PathPrefix::Absolute => ImportPathPrefix::Absolute,
                        ast::PathPrefix::Self_ => ImportPathPrefix::Self_,
                        ast::PathPrefix::Super => ImportPathPrefix::Super,
                        ast::PathPrefix::Crate => ImportPathPrefix::Crate,
                    };

                    let path: Vec<String> =
                        import_def.path.iter().map(|p| p.name.clone()).collect();

                    let kind = match &import_def.items {
                        ast::ImportItems::Module => ImportKind::Module,
                        ast::ImportItems::Items(items) => {
                            ImportKind::Items(items.iter().map(|i| i.name.clone()).collect())
                        }
                        ast::ImportItems::All => ImportKind::All,
                    };

                    let alias = import_def.alias.as_ref().map(|a| a.name.clone());

                    Some(Import {
                        prefix,
                        path,
                        kind,
                        alias,
                        is_pub: import_def.visibility == ast::Visibility::Public,
                        span: item.span,
                    })
                }
                _ => None,
            })
            .collect()
    }

    /// Allocate a fresh definition ID.
    /// 分配新的定义 ID。
    fn fresh_def_id(&mut self) -> DefId {
        let id = DefId(self.next_def_id);
        self.next_def_id += 1;
        id
    }

    /// Allocate a fresh local ID.
    /// 分配新的局部 ID。
    fn fresh_local_id(&mut self) -> LocalId {
        let id = LocalId(self.next_local_id);
        self.next_local_id += 1;
        id
    }

    /// Push a new local scope onto the stack.
    /// 将新的局部作用域压入栈。
    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Pop the top local scope from the stack.
    /// 从栈中弹出顶部的局部作用域。
    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    /// Define a new local variable in the current scope.
    /// 在当前作用域中定义新的局部变量。
    fn define_local(&mut self, name: String) -> LocalId {
        let id = self.fresh_local_id();
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, id);
        }
        id
    }

    /// Look up a local variable by name.
    /// 按名称查找局部变量。
    fn lookup_local(&self, name: &str) -> Option<LocalId> {
        for scope in self.scopes.iter().rev() {
            if let Some(&id) = scope.get(name) {
                return Some(id);
            }
        }
        None
    }

    /// Look up a global definition by name.
    /// 按名称查找全局定义。
    fn lookup_global(&self, name: &str) -> Option<DefId> {
        // First check local globals, then imported names
        // 首先检查本地全局变量，然后检查导入的名称
        self.globals
            .get(name)
            .or_else(|| self.imported.get(name))
            .copied()
    }

    /// Register an imported name for resolution.
    /// 注册导入的名称以供解析。
    pub fn register_import(&mut self, name: String, def_id: DefId) {
        self.imported.insert(name, def_id);
    }

    /// Register multiple imported names from a module registry resolution.
    /// 从模块注册表解析中注册多个导入的名称。
    pub fn register_imports(&mut self, imports: Vec<(String, DefId)>) {
        for (name, def_id) in imports {
            self.imported.insert(name, def_id);
        }
    }

    // === First pass: collect definitions ===
    // === 第一遍：收集定义 ===

    /// Collect a top-level item definition.
    /// 收集顶层项定义。
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
                // 同时注册变体
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
                // Impl 不引入名称
            }
            ast::ItemKind::Import(_) => {
                // Imports are handled separately
                // 导入单独处理
            }
        }
    }

    /// Extract the name from a pattern (if it's a simple variable pattern).
    /// 从模式中提取名称（如果是简单的变量模式）。
    fn pattern_name(&self, pattern: &ast::Pattern) -> Option<String> {
        match &pattern.kind {
            ast::PatternKind::Var(ident) => Some(ident.name.clone()),
            _ => None,
        }
    }

    // === Second pass: lower items ===
    // === 第二遍：降级项 ===

    /// Lower an AST item to HIR.
    /// 将 AST 项降级为 HIR。
    fn lower_item(&mut self, item: &ast::Item) -> Option<Item> {
        match &item.kind {
            ast::ItemKind::Let(def) => {
                // Top-level let becomes a function with no parameters
                // 顶层 let 变成没有参数的函数
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
                let params: Vec<Param> = def.params.iter().map(|p| self.lower_param(p)).collect();

                let return_ty = def
                    .return_type
                    .as_ref()
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
                let fields = def
                    .fields
                    .iter()
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
                let variants = def
                    .variants
                    .iter()
                    .map(|v| {
                        let fields = match &v.kind {
                            ast::VariantKind::Unit => Vec::new(),
                            ast::VariantKind::Tuple(types) => {
                                types.iter().map(|t| self.lower_type(t)).collect()
                            }
                            ast::VariantKind::Record(field_defs) => {
                                field_defs.iter().map(|f| self.lower_type(&f.ty)).collect()
                            }
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

                let items = def
                    .items
                    .iter()
                    .filter_map(|ti| self.lower_trait_item(ti))
                    .collect();

                let assoc_types = def
                    .assoc_types
                    .iter()
                    .map(|at| self.lower_assoc_type_def(at))
                    .collect();

                Some(Item {
                    id,
                    kind: ItemKind::Trait(TraitDef {
                        name: def.name.name.clone(),
                        generics,
                        items,
                        assoc_types,
                    }),
                    span: item.span,
                })
            }
            ast::ItemKind::Impl(def) => {
                let id = self.fresh_def_id();
                let generics = self.lower_generics(&def.generics);

                let trait_ref = def.trait_.as_ref().map(|t| self.lower_type(t));

                let self_ty = self.lower_type(&def.target);

                let items = def
                    .items
                    .iter()
                    .filter_map(|ii| self.lower_impl_item(ii))
                    .collect();

                let assoc_type_impls = def
                    .assoc_type_impls
                    .iter()
                    .map(|ati| self.lower_assoc_type_impl(ati))
                    .collect();

                Some(Item {
                    id,
                    kind: ItemKind::Impl(ImplDef {
                        generics,
                        trait_ref,
                        self_ty,
                        items,
                        assoc_type_impls,
                    }),
                    span: item.span,
                })
            }
            ast::ItemKind::Import(_) => None,
        }
    }

    /// Lower generic parameters.
    /// 降级泛型参数。
    fn lower_generics(&self, generics: &[ast::GenericParam]) -> Vec<GenericParam> {
        generics
            .iter()
            .map(|p| GenericParam {
                name: p.name.name.clone(),
                bounds: p.bounds.iter().map(|b| self.lower_type(b)).collect(),
                span: p.span,
            })
            .collect()
    }

    /// Lower a function parameter.
    /// 降级函数参数。
    fn lower_param(&mut self, param: &ast::Param) -> Param {
        let name = self
            .pattern_name(&param.pattern)
            .unwrap_or_else(|| "_".to_string());
        let id = self.define_local(name.clone());
        let ty = self.lower_type(&param.ty);

        Param {
            id,
            name,
            ty,
            span: param.span,
        }
    }

    /// Lower a trait item (method declaration).
    /// 降级 trait 项（方法声明）。
    fn lower_trait_item(&mut self, item: &ast::TraitItem) -> Option<TraitItem> {
        self.push_scope();

        let generics = self.lower_generics(&item.generics);
        let params = item.params.iter().map(|p| self.lower_type(&p.ty)).collect();
        let return_ty = item
            .return_type
            .as_ref()
            .map(|t| self.lower_type(t))
            .unwrap_or(Ty {
                kind: TyKind::Unit,
                span: item.span,
            });
        let default = item.default.as_ref().map(|e| self.lower_expr(e));

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

    /// Lower an impl item (method implementation).
    /// 降级 impl 项（方法实现）。
    fn lower_impl_item(&mut self, item: &ast::ImplItem) -> Option<ImplItem> {
        self.push_scope();

        let generics = self.lower_generics(&item.generics);
        let params: Vec<Param> = item.params.iter().map(|p| self.lower_param(p)).collect();
        let return_ty = item
            .return_type
            .as_ref()
            .map(|t| self.lower_type(t))
            .unwrap_or(Ty {
                kind: TyKind::Unit,
                span: item.span,
            });
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

    /// Lower an associated type definition.
    /// 降级关联类型定义。
    fn lower_assoc_type_def(&self, assoc_type: &ast::AssocTypeDef) -> AssocTypeDef {
        AssocTypeDef {
            name: assoc_type.name.name.clone(),
            bounds: assoc_type
                .bounds
                .iter()
                .map(|b| self.lower_type(b))
                .collect(),
            default: assoc_type.default.as_ref().map(|t| self.lower_type(t)),
            span: assoc_type.span,
        }
    }

    /// Lower an associated type implementation.
    /// 降级关联类型实现。
    fn lower_assoc_type_impl(&self, assoc_type_impl: &ast::AssocTypeImpl) -> AssocTypeImpl {
        AssocTypeImpl {
            name: assoc_type_impl.name.name.clone(),
            ty: self.lower_type(&assoc_type_impl.ty),
            span: assoc_type_impl.span,
        }
    }

    // === Lower expressions ===
    // === 降级表达式 ===

    /// Lower an AST expression to HIR.
    /// 将 AST 表达式降级为 HIR。
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
                    // 未知变量 - 将在类型检查期间捕获
                    ExprKind::Global(DefId(u32::MAX))
                }
            }

            ast::ExprKind::Path(parts) => {
                // Handle path like `r.a.b` as nested field access
                // 将类似 `r.a.b` 的路径处理为嵌套字段访问
                if parts.is_empty() {
                    ExprKind::Literal(Literal::Unit)
                } else {
                    // Start with the first part as base
                    // 以第一部分作为基础
                    let first = &parts[0];
                    let mut result_kind = if let Some(local_id) = self.lookup_local(&first.name) {
                        ExprKind::Var(local_id)
                    } else if let Some(def_id) = self.lookup_global(&first.name) {
                        ExprKind::Global(def_id)
                    } else {
                        ExprKind::Global(DefId(u32::MAX))
                    };

                    // Chain field accesses for remaining parts
                    // 为剩余部分链接字段访问
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
                let fields = fields
                    .iter()
                    .map(|f| {
                        let value =
                            f.value
                                .as_ref()
                                .map(|e| self.lower_expr(e))
                                .unwrap_or_else(|| {
                                    // Shorthand: #{ x } means #{ x = x }
                                    // 简写：#{ x } 表示 #{ x = x }
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
                // 将 #{ base | field = value } 解糖为记录字面量
                // This is a simplification; real implementation would merge
                // 这是简化；实际实现会合并
                let base_expr = self.lower_expr(base);
                let update_fields: Vec<(String, Expr)> = fields
                    .iter()
                    .map(|f| {
                        let value = f
                            .value
                            .as_ref()
                            .map(|e| self.lower_expr(e))
                            .unwrap_or_else(|| base_expr.clone());
                        (f.name.name.clone(), value)
                    })
                    .collect();
                ExprKind::Record(update_fields)
            }

            ast::ExprKind::Lambda { params, body } => {
                self.push_scope();
                let params: Vec<Param> = params
                    .iter()
                    .map(|p| {
                        let name = self
                            .pattern_name(&p.pattern)
                            .unwrap_or_else(|| "_".to_string());
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

            ast::ExprKind::MethodCall {
                receiver,
                method,
                args,
            } => {
                // Desugar method call to function call: receiver.method(args) -> method(receiver, args)
                // 将方法调用解糖为函数调用：receiver.method(args) -> method(receiver, args)
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
                // 将索引解糖为函数调用：base[index] -> index(base, index)
                let base = self.lower_expr(base);
                let index = self.lower_expr(index);
                // Use sentinel DefId for builtin index operation, resolved at eval time
                // 使用哨兵 DefId 表示内置索引操作，在求值时解析
                let index_fn = Expr {
                    kind: ExprKind::Global(DefId(u32::MAX)),
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

            ast::ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond = self.lower_expr(condition);
                let then_br = self.lower_expr(then_branch);
                let else_br = self.lower_expr(else_branch);
                ExprKind::If(Box::new(cond), Box::new(then_br), Box::new(else_br))
            }

            ast::ExprKind::Match { scrutinee, arms } => {
                let scrutinee = self.lower_expr(scrutinee);
                let arms = arms.iter().map(|arm| self.lower_match_arm(arm)).collect();
                ExprKind::Match(Box::new(scrutinee), arms)
            }

            ast::ExprKind::Block { stmts, expr } => {
                self.push_scope();
                let stmts = stmts.iter().map(|s| self.lower_stmt(s)).collect();
                let expr = expr.as_ref().map(|e| Box::new(self.lower_expr(e)));
                self.pop_scope();
                ExprKind::Block(stmts, expr)
            }

            ast::ExprKind::Coalesce { value, default } => {
                // Desugar value ?? default to match
                // 将 value ?? default 解糖为 match
                let value = self.lower_expr(value);
                let default = self.lower_expr(default);

                // match value { Some(x) => x, None => default }
                let x_id = self.fresh_local_id();
                let arms = vec![
                    MatchArm {
                        pattern: Pattern {
                            kind: PatternKind::Constructor(
                                DefId(u32::MAX),
                                vec![Pattern {
                                    kind: PatternKind::Var(x_id, "x".to_string()),
                                    span,
                                }],
                            ),
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
                // 将 expr? 解糖为 match expr { Ok(x) => x, Err(e) => return Err(e) }
                let inner = self.lower_expr(inner);
                let x_id = self.fresh_local_id();

                let arms = vec![MatchArm {
                    pattern: Pattern {
                        kind: PatternKind::Constructor(
                            DefId(u32::MAX),
                            vec![Pattern {
                                kind: PatternKind::Var(x_id, "x".to_string()),
                                span,
                            }],
                        ),
                        span,
                    },
                    guard: None,
                    body: Expr {
                        kind: ExprKind::Var(x_id),
                        ty: Self::unknown_ty(span),
                        span,
                    },
                    span,
                }];

                ExprKind::Match(Box::new(inner), arms)
            }

            ast::ExprKind::Interpolated(parts) => {
                let parts = parts
                    .iter()
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

    /// Lower a statement to HIR.
    /// 将语句降级为 HIR。
    fn lower_stmt(&mut self, stmt: &ast::Stmt) -> Stmt {
        let span = stmt.span;
        let kind = match &stmt.kind {
            ast::StmtKind::Let { pattern, ty, value } => {
                let name = self
                    .pattern_name(pattern)
                    .unwrap_or_else(|| "_".to_string());
                let id = self.define_local(name.clone());
                let ty = ty
                    .as_ref()
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

    /// Lower a match arm to HIR.
    /// 将匹配分支降级为 HIR。
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

    /// Lower a pattern to HIR.
    /// 将模式降级为 HIR。
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
                let patterns = patterns.iter().map(|p| self.lower_pattern(p)).collect();
                PatternKind::Tuple(patterns)
            }

            ast::PatternKind::List(patterns) => {
                let patterns = patterns.iter().map(|p| self.lower_pattern(p)).collect();
                PatternKind::List(patterns)
            }

            ast::PatternKind::Record { fields, .. } => {
                let fields = fields
                    .iter()
                    .map(|f| {
                        let pattern = f
                            .pattern
                            .as_ref()
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
                let def_id = path
                    .first()
                    .and_then(|p| self.lookup_global(&p.name))
                    .unwrap_or(DefId(u32::MAX));
                let args = args.iter().map(|p| self.lower_pattern(p)).collect();
                PatternKind::Constructor(def_id, args)
            }

            ast::PatternKind::Or(patterns) => {
                // For now, just use the first alternative
                // 目前，只使用第一个选项
                if let Some(first) = patterns.first() {
                    return self.lower_pattern(first);
                }
                PatternKind::Wildcard
            }

            ast::PatternKind::Binding { name, pattern } => {
                let id = self.define_local(name.name.clone());
                let _inner = self.lower_pattern(pattern);
                // For @ patterns, we bind the whole value
                // 对于 @ 模式，我们绑定整个值
                PatternKind::Var(id, name.name.clone())
            }

            _ => PatternKind::Wildcard,
        };

        Pattern { kind, span }
    }

    // === Lower types ===
    // === 降级类型 ===

    /// Lower an AST type to HIR.
    /// 将 AST 类型降级为 HIR。
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
                let fields = fields
                    .iter()
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
    // === 降级运算符 ===

    /// Lower a binary operator.
    /// 降级二元运算符。
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

    /// Lower a unary operator.
    /// 降级一元运算符。
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
