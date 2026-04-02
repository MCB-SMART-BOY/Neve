//! Name resolution and AST to HIR lowering.
//! 名称解析和 AST 到 HIR 的降级转换。

use crate::{
    AssocTypeDef, AssocTypeImpl, BinOp, DefId, EnumDef, Expr, ExprKind, FieldDef, FnDef, Generator,
    GenericParam, ImplDef, ImplItem, Import, ImportKind, ImportPathPrefix, Item, ItemKind, Literal,
    LocalId, MatchArm, Module, ModuleId, ModuleLoader, Param, Pattern, PatternKind, Stmt, StmtKind,
    StringPart, StructDef, TraitDef, TraitItem, Ty, TyKind, TypeAlias, UnaryOp, VariantDef,
    builtin_constructor_id,
};
use neve_common::Span;
use neve_syntax::{self as ast, SourceFile};
use std::collections::{HashMap, HashSet};
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
    /// Stack of in-scope generic parameters (name -> param index).
    /// 当前可见泛型参数栈（名称 -> 参数索引）。
    generic_scopes: Vec<HashMap<String, u32>>,
    /// Monotonic index source for outer generic scopes that should not be
    /// captured by inner `forall` binders.
    /// 为外层泛型作用域分配不会被内层 `forall` 误捕获的参数索引。
    next_outer_generic_idx: u32,
    /// Imported names from other modules. / 从其他模块导入的名称。
    imported: HashMap<String, DefId>,
    /// Imported std builtin items keyed by in-scope name.
    /// 作用域内名称到 std builtin 全名的映射。
    imported_builtin_items: HashMap<String, String>,
    /// Imported std builtin modules keyed by namespace alias.
    /// 命名空间别名到 std builtin 模块前缀的映射。
    imported_builtin_modules: HashMap<String, String>,
    /// Imported module aliases (namespace roots). / 导入的模块别名（命名空间根）。
    imported_modules: HashSet<String>,
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
            generic_scopes: Vec::new(),
            next_outer_generic_idx: 1_000_000,
            imported: HashMap::new(),
            imported_builtin_items: HashMap::new(),
            imported_builtin_modules: HashMap::new(),
            imported_modules: HashSet::new(),
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
            generic_scopes: Vec::new(),
            next_outer_generic_idx: 1_000_000,
            imported: HashMap::new(),
            imported_builtin_items: HashMap::new(),
            imported_builtin_modules: HashMap::new(),
            imported_modules: HashSet::new(),
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

    /// Set the global definition ID counter.
    /// 设置全局定义 ID 计数器。
    pub fn set_def_id_counter(&mut self, next: u32) {
        self.next_def_id = next;
    }

    /// Get the next global definition ID.
    /// 获取下一个全局定义 ID。
    pub fn next_def_id(&self) -> u32 {
        self.next_def_id
    }

    /// Get resolved global definitions for this module.
    /// 获取当前模块解析到的全局定义。
    pub fn global_defs(&self) -> &HashMap<String, DefId> {
        &self.globals
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
        self.resolve_with_path_and_id(file, name, module_path, module_id)
    }

    /// Resolve an AST source file to HIR with an explicit module ID.
    /// 使用显式模块 ID 将 AST 源文件解析为 HIR。
    pub fn resolve_with_path_and_id(
        &mut self,
        file: &SourceFile,
        name: String,
        module_path: Vec<String>,
        module_id: ModuleId,
    ) -> Module {
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
            self.record_import_alias(import);
            if self.try_register_std_import(import) {
                continue;
            }
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
            self.register_import_item_placeholders(import);
        }
    }

    /// Record module import aliases for namespace path resolution.
    /// 记录模块导入别名，用于命名空间路径解析。
    fn record_import_alias(&mut self, import: &Import) {
        if let ImportKind::Module = import.kind {
            let alias = import.alias.clone().or_else(|| import.path.last().cloned());
            if let Some(alias) = alias {
                self.imported_modules.insert(alias);
            }
        }
    }

    /// Register placeholder defs for explicit item imports when unresolved.
    /// 为未解析的显式项导入注册占位定义。
    fn register_import_item_placeholders(&mut self, import: &Import) {
        if let ImportKind::Items(names) = &import.kind {
            for name in names {
                if !self.imported.contains_key(name) {
                    let def_id = self.fresh_def_id();
                    self.imported.insert(name.clone(), def_id);
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

    /// Push a generic scope whose parameters are bound by the current item.
    /// 压入由当前项绑定的泛型作用域。
    fn push_bound_generic_scope(&mut self, generics: &[ast::GenericParam]) {
        let mut scope = HashMap::new();
        for (idx, param) in generics.iter().enumerate() {
            scope.insert(param.name.name.clone(), idx as u32);
        }
        self.generic_scopes.push(scope);
    }

    /// Push a generic scope inherited from an outer item.
    /// 压入来自外层项的泛型作用域。
    fn push_outer_generic_scope(&mut self, generics: &[ast::GenericParam]) {
        let mut scope = HashMap::new();
        for param in generics {
            let idx = self.next_outer_generic_idx;
            self.next_outer_generic_idx += 1;
            scope.insert(param.name.name.clone(), idx);
        }
        self.generic_scopes.push(scope);
    }

    /// Pop the top generic scope from the stack.
    /// 从栈中弹出顶部的泛型作用域。
    fn pop_generic_scope(&mut self) {
        self.generic_scopes.pop();
    }

    /// Look up an in-scope generic parameter by name.
    /// 按名称查找当前作用域中的泛型参数。
    fn lookup_generic(&self, name: &str) -> Option<u32> {
        for scope in self.generic_scopes.iter().rev() {
            if let Some(&idx) = scope.get(name) {
                return Some(idx);
            }
        }
        None
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

    fn std_builtin_module_prefix(path: &[String]) -> Option<String> {
        if path.len() == 2 && path.first().map(|segment| segment.as_str()) == Some("std") {
            Some(path[1].clone())
        } else {
            None
        }
    }

    fn std_builtin_exports(module_prefix: &str) -> Option<&'static [&'static str]> {
        const FETCH: &[&str] = &[
            "git",
            "gitWithHash",
            "path",
            "pathWithHash",
            "url",
            "urlWithHash",
        ];
        const IO: &[&str] = &[
            "appendFile",
            "createDirAll",
            "currentDir",
            "currentSystem",
            "exec",
            "execShell",
            "execWith",
            "getEnv",
            "hashFile",
            "hashString",
            "homeDir",
            "isDir",
            "isFile",
            "pathExists",
            "readDir",
            "readFile",
            "removeDirAll",
            "writeFile",
        ];
        const LIST: &[&str] = &[
            "append",
            "cons",
            "contains",
            "drop",
            "empty",
            "filter",
            "fold",
            "foldRight",
            "get",
            "head",
            "indexOf",
            "init",
            "isEmpty",
            "last",
            "len",
            "map",
            "max",
            "min",
            "product",
            "range",
            "replicate",
            "reverse",
            "singleton",
            "sort",
            "sum",
            "tail",
            "take",
            "unzip",
            "zip",
        ];
        const MAP: &[&str] = &[
            "contains",
            "difference",
            "empty",
            "filter",
            "filterWithKey",
            "fold",
            "foldWithKey",
            "fromList",
            "get",
            "getWithDefault",
            "insert",
            "intersection",
            "isEmpty",
            "keys",
            "map",
            "mapWithKey",
            "remove",
            "singleton",
            "size",
            "toList",
            "union",
            "update",
            "values",
        ];
        const MATH: &[&str] = &[
            "abs", "ceil", "clamp", "cos", "e", "exp", "floor", "inf", "isInf", "isNan", "log",
            "log10", "max", "min", "nan", "pi", "pow", "round", "sin", "sqrt", "tan", "toFloat",
            "toInt",
        ];
        const OPTION: &[&str] = &["is_none", "is_some", "none", "some", "unwrap", "unwrap_or"];
        const PATH: &[&str] = &["extension", "filename", "is_absolute", "join", "parent"];
        const RESULT: &[&str] = &["err", "is_err", "is_ok", "ok", "unwrap", "unwrap_err"];
        const SET: &[&str] = &[
            "contains",
            "difference",
            "empty",
            "filter",
            "fold",
            "fromList",
            "insert",
            "intersection",
            "isDisjoint",
            "isEmpty",
            "isSubset",
            "isSuperset",
            "map",
            "partition",
            "remove",
            "singleton",
            "size",
            "symmetricDifference",
            "toList",
            "union",
        ];
        const STRING: &[&str] = &[
            "chars",
            "contains",
            "endsWith",
            "isEmpty",
            "join",
            "len",
            "lines",
            "lower",
            "repeat",
            "replace",
            "split",
            "startsWith",
            "substring",
            "trim",
            "upper",
        ];

        match module_prefix {
            "fetch" => Some(FETCH),
            "io" => Some(IO),
            "list" => Some(LIST),
            "Map" => Some(MAP),
            "math" => Some(MATH),
            "option" => Some(OPTION),
            "path" => Some(PATH),
            "result" => Some(RESULT),
            "Set" => Some(SET),
            "string" => Some(STRING),
            _ => None,
        }
    }

    fn try_register_std_import(&mut self, import: &Import) -> bool {
        let Some(module_prefix) = Self::std_builtin_module_prefix(&import.path) else {
            return false;
        };

        match &import.kind {
            ImportKind::Items(names) => {
                for name in names {
                    self.imported_builtin_items
                        .insert(name.clone(), format!("{module_prefix}.{name}"));
                }
                true
            }
            ImportKind::Module => {
                let alias = import.alias.clone().or_else(|| import.path.last().cloned());
                if let Some(alias) = alias {
                    self.imported_builtin_modules.insert(alias, module_prefix);
                    true
                } else {
                    false
                }
            }
            ImportKind::All => {
                let Some(names) = Self::std_builtin_exports(&module_prefix) else {
                    return false;
                };
                for name in names {
                    self.imported_builtin_items
                        .insert((*name).to_string(), format!("{module_prefix}.{name}"));
                }
                true
            }
        }
    }

    fn is_supported_builtin(name: &str) -> bool {
        matches!(name, "force" | "isLazy" | "isEvaluated" | "toString")
    }

    fn lower_name_kind(&self, name: &str) -> ExprKind {
        if let Some(local_id) = self.lookup_local(name) {
            ExprKind::Var(local_id)
        } else if let Some(def_id) = self.lookup_global(name) {
            ExprKind::Global(def_id)
        } else if let Some(builtin_name) = self.imported_builtin_items.get(name) {
            ExprKind::Builtin(builtin_name.clone())
        } else if Self::is_supported_builtin(name) {
            ExprKind::Builtin(name.to_string())
        } else {
            ExprKind::Global(DefId(u32::MAX))
        }
    }

    fn lower_name_expr(&self, name: &str, span: neve_common::Span) -> Expr {
        Expr {
            kind: self.lower_name_kind(name),
            ty: Self::unknown_ty(span),
            span,
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
                self.push_bound_generic_scope(&def.generics);

                let generics = self.lower_generics(&def.generics);
                let params: Vec<Param> = def.params.iter().map(|p| self.lower_param(p)).collect();

                let return_ty = def
                    .return_type
                    .as_ref()
                    .map(|t| self.lower_type(t))
                    .unwrap_or_else(|| Self::unknown_ty(def.name.span));

                let body = self.lower_expr(&def.body);

                self.pop_generic_scope();
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
                self.push_bound_generic_scope(&def.generics);
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
                self.pop_generic_scope();

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
                self.push_bound_generic_scope(&def.generics);
                let generics = self.lower_generics(&def.generics);
                let variants = def
                    .variants
                    .iter()
                    .map(|v| {
                        let variant_id =
                            self.lookup_global(&v.name.name).unwrap_or(DefId(u32::MAX));
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
                            id: variant_id,
                            name: v.name.name.clone(),
                            fields,
                            span: v.span,
                        }
                    })
                    .collect();
                self.pop_generic_scope();

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
                self.push_bound_generic_scope(&def.generics);
                let generics = self.lower_generics(&def.generics);
                let ty = self.lower_type(&def.ty);
                self.pop_generic_scope();

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
                self.push_outer_generic_scope(&def.generics);
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
                self.pop_generic_scope();

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
                self.push_outer_generic_scope(&def.generics);
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
                self.pop_generic_scope();

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
        self.lower_param_parts(&param.pattern, Some(&param.ty), param.span)
    }

    fn lower_param_parts(
        &mut self,
        pattern: &ast::Pattern,
        ty: Option<&ast::Type>,
        span: Span,
    ) -> Param {
        let name = self
            .pattern_name(pattern)
            .unwrap_or_else(|| "_".to_string());
        let id = self.define_local(name.clone());
        let ty = ty
            .map(|ty| self.lower_type(ty))
            .unwrap_or_else(|| Self::unknown_ty(span));

        Param {
            id,
            name,
            ty,
            span,
        }
    }

    /// Lower a trait item (method declaration).
    /// 降级 trait 项（方法声明）。
    fn lower_trait_item(&mut self, item: &ast::TraitItem) -> Option<TraitItem> {
        self.push_scope();
        self.push_bound_generic_scope(&item.generics);

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

        self.pop_generic_scope();
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
        self.push_bound_generic_scope(&item.generics);
        let id = self.fresh_def_id();

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

        self.pop_generic_scope();
        self.pop_scope();

        Some(ImplItem {
            id,
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
            ast::ExprKind::Int(n) => ExprKind::Literal(Literal::Int(n.clone())),
            ast::ExprKind::Float(f) => ExprKind::Literal(Literal::Float(*f)),
            ast::ExprKind::String(s) => ExprKind::Literal(Literal::String(s.clone())),
            ast::ExprKind::Char(c) => ExprKind::Literal(Literal::Char(*c)),
            ast::ExprKind::Bool(b) => ExprKind::Literal(Literal::Bool(*b)),
            ast::ExprKind::Unit => ExprKind::Literal(Literal::Unit),

            ast::ExprKind::Var(ident) => self.lower_name_kind(&ident.name),

            ast::ExprKind::Path(parts) => {
                // Handle path like `r.a.b` as nested field access
                // 将类似 `r.a.b` 的路径处理为嵌套字段访问
                if parts.is_empty() {
                    ExprKind::Literal(Literal::Unit)
                } else {
                    let first = &parts[0];

                    if parts.len() > 1
                        && self.lookup_local(&first.name).is_none()
                        && self.lookup_global(&first.name).is_none()
                        && let Some(module_prefix) = self.imported_builtin_modules.get(&first.name)
                    {
                        let suffix = parts[1..]
                            .iter()
                            .map(|part| part.name.as_str())
                            .collect::<Vec<_>>()
                            .join(".");
                        ExprKind::Builtin(format!("{module_prefix}.{suffix}"))
                    } else {
                        let base_kind = self
                            .lookup_local(&first.name)
                            .map(ExprKind::Var)
                            .or_else(|| self.lookup_global(&first.name).map(ExprKind::Global))
                            .or_else(|| {
                                Self::is_supported_builtin(&first.name)
                                    .then(|| ExprKind::Builtin(first.name.clone()))
                            });

                        if let Some(mut result_kind) = base_kind {
                            // Chain field accesses for remaining parts
                            // 为剩余部分链接字段访问
                            for part in &parts[1..] {
                                let base_expr = Expr {
                                    kind: result_kind,
                                    ty: Self::unknown_ty(span),
                                    span,
                                };
                                result_kind =
                                    ExprKind::Field(Box::new(base_expr), part.name.clone());
                            }
                            result_kind
                        } else {
                            let full_name = parts
                                .iter()
                                .map(|part| part.name.as_str())
                                .collect::<Vec<_>>()
                                .join(".");

                            if let Some(def_id) = self.lookup_global(&full_name) {
                                ExprKind::Global(def_id)
                            } else if self.imported_modules.contains(&first.name) {
                                let def_id = match self.imported.get(&full_name).copied() {
                                    Some(def_id) => def_id,
                                    None => {
                                        let def_id = self.fresh_def_id();
                                        self.imported.insert(full_name, def_id);
                                        def_id
                                    }
                                };
                                ExprKind::Global(def_id)
                            } else {
                                let mut result_kind = ExprKind::Global(DefId(u32::MAX));
                                for part in &parts[1..] {
                                    let base_expr = Expr {
                                        kind: result_kind,
                                        ty: Self::unknown_ty(span),
                                        span,
                                    };
                                    result_kind =
                                        ExprKind::Field(Box::new(base_expr), part.name.clone());
                                }
                                result_kind
                            }
                        }
                    }
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
                                    self.lower_name_expr(name, span)
                                });
                        (f.name.name.clone(), value)
                    })
                    .collect();
                ExprKind::Record(fields)
            }

            ast::ExprKind::RecordUpdate { base, fields } => {
                // Desugar #{ base | field = value } to base // #{ field = value }
                // 将 #{ base | field = value } 解糖为 base // #{ field = value }
                let base_expr = self.lower_expr(base);
                let update_fields: Vec<(String, Expr)> = fields
                    .iter()
                    .map(|f| {
                        let value =
                            f.value
                                .as_ref()
                                .map(|e| self.lower_expr(e))
                                .unwrap_or_else(|| {
                                    // Shorthand: #{ base | x } means use variable x
                                    // 简写：#{ base | x } 表示使用变量 x
                                    let name = &f.name.name;
                                    self.lower_name_expr(name, span)
                                });
                        (f.name.name.clone(), value)
                    })
                    .collect();
                let update_expr = Expr {
                    kind: ExprKind::Record(update_fields),
                    ty: Self::unknown_ty(span),
                    span,
                };
                ExprKind::Binary(BinOp::Merge, Box::new(base_expr), Box::new(update_expr))
            }

            ast::ExprKind::Lambda { params, body } => {
                self.push_scope();
                let params: Vec<Param> = params
                    .iter()
                    .map(|p| self.lower_param_parts(&p.pattern, p.ty.as_ref(), p.span))
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
            } => ExprKind::MethodCall {
                receiver: Box::new(self.lower_expr(receiver)),
                method: method.name.clone(),
                target: Box::new(self.lower_name_expr(&method.name, span)),
                args: args.iter().map(|e| self.lower_expr(e)).collect(),
            },

            ast::ExprKind::Field { base, field } => {
                let base = self.lower_expr(base);
                ExprKind::Field(Box::new(base), field.name.clone())
            }

            ast::ExprKind::TupleIndex { base, index } => {
                let base = self.lower_expr(base);
                ExprKind::TupleIndex(Box::new(base), *index)
            }

            ast::ExprKind::SafeField { base, field } => {
                let base = self.lower_expr(base);
                ExprKind::SafeField {
                    base: Box::new(base),
                    field: field.name.clone(),
                }
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

            ast::ExprKind::Let {
                pattern,
                ty,
                value,
                body,
            } => {
                let value = Box::new(self.lower_expr(value));
                self.push_scope();
                let pattern = self.lower_pattern(pattern);
                let ty = ty.as_ref().map(|t| self.lower_type(t));
                let body = Box::new(self.lower_expr(body));
                self.pop_scope();
                ExprKind::Let {
                    pattern,
                    ty,
                    value,
                    body,
                }
            }

            ast::ExprKind::Block { stmts, expr } => {
                self.push_scope();
                let stmts = stmts.iter().map(|s| self.lower_stmt(s)).collect();
                let expr = expr.as_ref().map(|e| Box::new(self.lower_expr(e)));
                self.pop_scope();
                ExprKind::Block(stmts, expr)
            }

            ast::ExprKind::Lazy(inner) => ExprKind::Lazy(Box::new(self.lower_expr(inner))),

            ast::ExprKind::ListComp { body, generators } => {
                let mut lowered_generators = Vec::with_capacity(generators.len());
                let mut pushed_scopes = 0usize;

                for generator in generators {
                    let iter = self.lower_expr(&generator.iter);
                    self.push_scope();
                    pushed_scopes += 1;
                    let pattern = self.lower_pattern(&generator.pattern);
                    let condition = generator.condition.as_ref().map(|e| self.lower_expr(e));
                    lowered_generators.push(Generator {
                        pattern,
                        iter,
                        condition,
                        span: generator.span,
                    });
                }

                let body = Box::new(self.lower_expr(body));

                for _ in 0..pushed_scopes {
                    self.pop_scope();
                }

                ExprKind::ListComp {
                    body,
                    generators: lowered_generators,
                }
            }

            ast::ExprKind::Coalesce { value, default } => ExprKind::Coalesce {
                value: Box::new(self.lower_expr(value)),
                default: Box::new(self.lower_expr(default)),
            },

            ast::ExprKind::Try(inner) => ExprKind::Try(Box::new(self.lower_expr(inner))),

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

            ast::ExprKind::PathLit(path) => ExprKind::Literal(Literal::String(path.clone())),
        };

        Expr {
            kind,
            ty: Self::unknown_ty(span),
            span,
        }
    }

    // List comprehension generators are lowered inline to preserve scope ordering.

    /// Lower a statement to HIR.
    /// 将语句降级为 HIR。
    fn lower_stmt(&mut self, stmt: &ast::Stmt) -> Stmt {
        let span = stmt.span;
        let kind = match &stmt.kind {
            ast::StmtKind::Let { pattern, ty, value } => {
                let value = self.lower_expr(value);
                let pattern = self.lower_pattern(pattern);
                let ty = ty.as_ref().map(|t| self.lower_type(t));
                StmtKind::Let { pattern, ty, value }
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
        self.lower_pattern_with_bindings(pattern, None, true)
    }

    fn lower_pattern_binding(
        &mut self,
        name: &str,
        shared_bindings: Option<&HashMap<String, LocalId>>,
        expose_new_bindings: bool,
    ) -> LocalId {
        if let Some(shared) = shared_bindings
            && let Some(id) = shared.get(name)
        {
            return *id;
        }

        if expose_new_bindings {
            self.define_local(name.to_string())
        } else {
            self.fresh_local_id()
        }
    }

    fn collect_pattern_bindings(pattern: &Pattern, bindings: &mut HashMap<String, LocalId>) {
        match &pattern.kind {
            PatternKind::Wildcard | PatternKind::Literal(_) => {}
            PatternKind::Var(id, name) | PatternKind::Binding(id, name, _) => {
                bindings.insert(name.clone(), *id);
                if let PatternKind::Binding(_, _, inner) = &pattern.kind {
                    Self::collect_pattern_bindings(inner, bindings);
                }
            }
            PatternKind::Tuple(patterns)
            | PatternKind::List(patterns)
            | PatternKind::Constructor(_, patterns)
            | PatternKind::Or(patterns) => {
                for pattern in patterns {
                    Self::collect_pattern_bindings(pattern, bindings);
                }
            }
            PatternKind::ListRest { init, rest, tail } => {
                for pattern in init {
                    Self::collect_pattern_bindings(pattern, bindings);
                }
                if let Some(pattern) = rest {
                    Self::collect_pattern_bindings(pattern, bindings);
                }
                for pattern in tail {
                    Self::collect_pattern_bindings(pattern, bindings);
                }
            }
            PatternKind::Record(fields) => {
                for (_, pattern) in fields {
                    Self::collect_pattern_bindings(pattern, bindings);
                }
            }
        }
    }

    fn lower_pattern_with_bindings(
        &mut self,
        pattern: &ast::Pattern,
        shared_bindings: Option<&HashMap<String, LocalId>>,
        expose_new_bindings: bool,
    ) -> Pattern {
        let span = pattern.span;
        let kind = match &pattern.kind {
            ast::PatternKind::Wildcard => PatternKind::Wildcard,

            ast::PatternKind::Var(ident) => {
                if ident.name == "_" {
                    PatternKind::Wildcard
                } else {
                    let id = self.lower_pattern_binding(
                        &ident.name,
                        shared_bindings,
                        expose_new_bindings,
                    );
                    PatternKind::Var(id, ident.name.clone())
                }
            }

            ast::PatternKind::Literal(lit) => {
                let literal = match lit {
                    ast::LiteralPattern::Int(n) => Literal::Int(n.clone()),
                    ast::LiteralPattern::Float(f) => Literal::Float(*f),
                    ast::LiteralPattern::String(s) => Literal::String(s.clone()),
                    ast::LiteralPattern::Char(c) => Literal::Char(*c),
                    ast::LiteralPattern::Bool(b) => Literal::Bool(*b),
                };
                PatternKind::Literal(literal)
            }

            ast::PatternKind::Tuple(patterns) => {
                let patterns = patterns
                    .iter()
                    .map(|p| {
                        self.lower_pattern_with_bindings(p, shared_bindings, expose_new_bindings)
                    })
                    .collect();
                PatternKind::Tuple(patterns)
            }

            ast::PatternKind::List(patterns) => {
                let patterns = patterns
                    .iter()
                    .map(|p| {
                        self.lower_pattern_with_bindings(p, shared_bindings, expose_new_bindings)
                    })
                    .collect();
                PatternKind::List(patterns)
            }

            ast::PatternKind::ListRest { init, rest, tail } => {
                let init = init
                    .iter()
                    .map(|p| {
                        self.lower_pattern_with_bindings(p, shared_bindings, expose_new_bindings)
                    })
                    .collect();
                let rest = rest.as_ref().map(|pattern| {
                    Box::new(self.lower_pattern_with_bindings(
                        pattern,
                        shared_bindings,
                        expose_new_bindings,
                    ))
                });
                let tail = tail
                    .iter()
                    .map(|p| {
                        self.lower_pattern_with_bindings(p, shared_bindings, expose_new_bindings)
                    })
                    .collect();
                PatternKind::ListRest { init, rest, tail }
            }

            ast::PatternKind::Record { fields, .. } => {
                let fields = fields
                    .iter()
                    .map(|f| {
                        let pattern = f
                            .pattern
                            .as_ref()
                            .map(|p| {
                                self.lower_pattern_with_bindings(
                                    p,
                                    shared_bindings,
                                    expose_new_bindings,
                                )
                            })
                            .unwrap_or_else(|| {
                                let id = self.lower_pattern_binding(
                                    &f.name.name,
                                    shared_bindings,
                                    expose_new_bindings,
                                );
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
                    .or_else(|| {
                        if path.len() == 1 {
                            builtin_constructor_id(&path[0].name)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(DefId(u32::MAX));
                let args = args
                    .iter()
                    .map(|p| {
                        self.lower_pattern_with_bindings(p, shared_bindings, expose_new_bindings)
                    })
                    .collect();
                PatternKind::Constructor(def_id, args)
            }

            ast::PatternKind::Or(patterns) => {
                if let Some((first, rest)) = patterns.split_first() {
                    let first = self.lower_pattern_with_bindings(
                        first,
                        shared_bindings,
                        expose_new_bindings,
                    );
                    let mut bindings = HashMap::new();
                    Self::collect_pattern_bindings(&first, &mut bindings);

                    let mut lowered = Vec::with_capacity(patterns.len());
                    lowered.push(first);
                    lowered.extend(rest.iter().map(|pattern| {
                        self.lower_pattern_with_bindings(pattern, Some(&bindings), false)
                    }));
                    PatternKind::Or(lowered)
                } else {
                    PatternKind::Wildcard
                }
            }

            ast::PatternKind::Binding { name, pattern } => {
                let id =
                    self.lower_pattern_binding(&name.name, shared_bindings, expose_new_bindings);
                let inner =
                    self.lower_pattern_with_bindings(pattern, shared_bindings, expose_new_bindings);
                PatternKind::Binding(id, name.name.clone(), Box::new(inner))
            }
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
                        "Self" => TyKind::SelfType,
                        _ => {
                            if let Some(param_idx) = self.lookup_generic(name) {
                                TyKind::Param(param_idx, name.clone())
                            } else if let Some(def_id) = self.lookup_global(name) {
                                TyKind::Named(def_id, Vec::new())
                            } else {
                                TyKind::Unknown
                            }
                        }
                    }
                } else if path.len() == 2 && args.is_empty() && path[0].name == "Self" {
                    TyKind::SelfAssoc(path[1].name.clone())
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
