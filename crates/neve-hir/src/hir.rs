//! HIR node definitions.
//! HIR 节点定义。

use neve_common::Span;
use std::collections::HashMap;

/// A unique identifier for a definition.
/// 定义的唯一标识符。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefId(pub u32);

/// A unique identifier for a local variable.
/// 局部变量的唯一标识符。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub u32);

/// A unique identifier for a module.
/// 模块的唯一标识符。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleId(pub u32);

/// HIR module containing all definitions.
/// 包含所有定义的 HIR 模块。
#[derive(Debug, Clone)]
pub struct Module {
    /// Module identifier. / 模块标识符。
    pub id: ModuleId,
    /// Module name (from file path or explicit module declaration). / 模块名称（来自文件路径或显式模块声明）。
    pub name: String,
    /// Top-level items in this module. / 此模块中的顶层项。
    pub items: Vec<Item>,
    /// Imports from other modules. / 从其他模块导入的内容。
    pub imports: Vec<Import>,
    /// Exported names (None = export all public items). / 导出的名称（None = 导出所有公共项）。
    pub exports: Option<Vec<String>>,
}

impl Module {
    /// Create a new empty module.
    /// 创建一个新的空模块。
    pub fn new(id: ModuleId, name: String) -> Self {
        Self {
            id,
            name,
            items: Vec::new(),
            imports: Vec::new(),
            exports: None,
        }
    }
}

/// An import declaration in HIR.
/// HIR 中的导入声明。
#[derive(Debug, Clone)]
pub struct Import {
    /// Path prefix (self, super, crate, or absolute). / 路径前缀（self、super、crate 或绝对路径）。
    pub prefix: ImportPathPrefix,
    /// The module path segments (e.g., ["list"] for `std.list`). / 模块路径段（例如 `std.list` 对应 ["list"]）。
    pub path: Vec<String>,
    /// What to import from the module. / 从模块导入的内容类型。
    pub kind: ImportKind,
    /// Optional alias for the import. / 导入的可选别名。
    pub alias: Option<String>,
    /// Whether this is a re-export (`pub import`). / 是否为重导出（`pub import`）。
    pub is_pub: bool,
    /// Source location. / 源代码位置。
    pub span: Span,
}

/// Path prefix for imports in HIR.
/// HIR 中导入的路径前缀。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImportPathPrefix {
    /// Absolute path (no prefix). / 绝对路径（无前缀）。
    #[default]
    Absolute,
    /// `self.` - relative to current module. / `self.` - 相对于当前模块。
    Self_,
    /// `super.` - relative to parent module. / `super.` - 相对于父模块。
    Super,
    /// `crate.` - relative to crate root. / `crate.` - 相对于 crate 根。
    Crate,
}

/// What kind of import this is.
/// 导入的类型。
#[derive(Debug, Clone)]
pub enum ImportKind {
    /// Import the entire module: `import std.list`. / 导入整个模块：`import std.list`。
    Module,
    /// Import specific items: `import std.list (map, filter)`. / 导入特定项：`import std.list (map, filter)`。
    Items(Vec<String>),
    /// Import all public items: `import std.list (*)`. / 导入所有公共项：`import std.list (*)`。
    All,
}

/// A module registry that tracks all loaded modules.
/// 跟踪所有已加载模块的模块注册表。
#[derive(Debug, Default)]
pub struct ModuleRegistry {
    /// All loaded modules by ID. / 按 ID 存储的所有已加载模块。
    modules: HashMap<ModuleId, Module>,
    /// Module lookup by path. / 按路径查找模块。
    path_to_id: HashMap<Vec<String>, ModuleId>,
    /// Next module ID. / 下一个模块 ID。
    next_id: u32,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new module and return its ID.
    /// 注册新模块并返回其 ID。
    pub fn register(&mut self, name: String, path: Vec<String>) -> ModuleId {
        let id = ModuleId(self.next_id);
        self.next_id += 1;

        let module = Module::new(id, name);
        self.modules.insert(id, module);
        self.path_to_id.insert(path, id);

        id
    }

    /// Get a module by ID.
    /// 按 ID 获取模块。
    pub fn get(&self, id: ModuleId) -> Option<&Module> {
        self.modules.get(&id)
    }

    /// Get a mutable module by ID.
    /// 按 ID 获取可变模块引用。
    pub fn get_mut(&mut self, id: ModuleId) -> Option<&mut Module> {
        self.modules.get_mut(&id)
    }

    /// Look up a module by path.
    /// 按路径查找模块。
    pub fn lookup(&self, path: &[String]) -> Option<ModuleId> {
        self.path_to_id.get(path).copied()
    }

    /// Get all modules.
    /// 获取所有模块。
    pub fn all_modules(&self) -> impl Iterator<Item = &Module> {
        self.modules.values()
    }

    /// Resolve an import to get the definitions it brings into scope.
    /// 解析导入以获取其引入作用域的定义。
    pub fn resolve_import(&self, import: &Import) -> Vec<(String, DefId)> {
        let Some(module_id) = self.lookup(&import.path) else {
            return Vec::new();
        };

        let Some(module) = self.get(module_id) else {
            return Vec::new();
        };

        match &import.kind {
            ImportKind::Module => {
                // Import module as a namespace
                // 将模块作为命名空间导入
                // The alias or last path component becomes the namespace name
                // 别名或路径的最后一个组件成为命名空间名称
                vec![]
            }
            ImportKind::Items(names) => {
                // Import specific items
                // 导入特定项
                names
                    .iter()
                    .filter_map(|name| {
                        self.find_exported_def(module, name)
                            .map(|def_id| (name.clone(), def_id))
                    })
                    .collect()
            }
            ImportKind::All => {
                // Import all public items
                // 导入所有公共项
                module
                    .items
                    .iter()
                    .filter_map(|item| self.item_name(item).map(|name| (name, item.id)))
                    .collect()
            }
        }
    }

    /// Find an exported definition by name.
    /// 按名称查找导出的定义。
    fn find_exported_def(&self, module: &Module, name: &str) -> Option<DefId> {
        // Check if the module has an explicit export list
        // 检查模块是否有显式导出列表
        if let Some(exports) = &module.exports
            && !exports.contains(&name.to_string())
        {
            return None;
        }

        // Find the item with this name
        // 查找具有此名称的项
        module
            .items
            .iter()
            .find(|item| self.item_name(item).as_deref() == Some(name))
            .map(|item| item.id)
    }

    /// Get the name of an item.
    /// 获取项的名称。
    fn item_name(&self, item: &Item) -> Option<String> {
        match &item.kind {
            ItemKind::Fn(f) => Some(f.name.clone()),
            ItemKind::Struct(s) => Some(s.name.clone()),
            ItemKind::Enum(e) => Some(e.name.clone()),
            ItemKind::TypeAlias(t) => Some(t.name.clone()),
            ItemKind::Trait(t) => Some(t.name.clone()),
            ItemKind::Impl(_) => None, // Impls don't have names / Impl 没有名称
        }
    }

    /// Insert a fully resolved module into the registry.
    /// 将完全解析的模块插入注册表。
    pub fn insert(&mut self, path: Vec<String>, module: Module) {
        let id = module.id;
        self.modules.insert(id, module);
        self.path_to_id.insert(path, id);
    }

    /// Get the number of registered modules.
    /// 获取已注册模块的数量。
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// Check if the registry is empty.
    /// 检查注册表是否为空。
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// Get all module paths.
    /// 获取所有模块路径。
    pub fn all_paths(&self) -> impl Iterator<Item = &Vec<String>> {
        self.path_to_id.keys()
    }

    /// Find an item by DefId across all modules.
    /// 在所有模块中按 DefId 查找项。
    pub fn find_item(&self, def_id: DefId) -> Option<(&Module, &Item)> {
        for module in self.modules.values() {
            if let Some(item) = module.items.iter().find(|item| item.id == def_id) {
                return Some((module, item));
            }
        }
        None
    }

    /// Get the module path for a given module ID.
    /// 获取给定模块 ID 的模块路径。
    pub fn module_path(&self, id: ModuleId) -> Option<&Vec<String>> {
        self.path_to_id
            .iter()
            .find(|&(_, &mid)| mid == id)
            .map(|(path, _)| path)
    }

    /// Resolve an import with path prefix support.
    /// 解析带有路径前缀支持的导入。
    pub fn resolve_import_with_prefix(
        &self,
        import: &Import,
        current_module_path: &[String],
    ) -> Vec<(String, DefId)> {
        // Compute the absolute path based on prefix
        // 根据前缀计算绝对路径
        let absolute_path = match import.prefix {
            ImportPathPrefix::Absolute => import.path.clone(),
            ImportPathPrefix::Crate => import.path.clone(),
            ImportPathPrefix::Self_ => {
                let mut path = current_module_path.to_vec();
                path.extend(import.path.iter().cloned());
                path
            }
            ImportPathPrefix::Super => {
                if current_module_path.is_empty() {
                    return Vec::new(); // Can't go above root / 无法超出根目录
                }
                let mut path = current_module_path[..current_module_path.len() - 1].to_vec();
                path.extend(import.path.iter().cloned());
                path
            }
        };

        // Look up the target module
        // 查找目标模块
        let Some(module_id) = self.lookup(&absolute_path) else {
            return Vec::new();
        };

        let Some(module) = self.get(module_id) else {
            return Vec::new();
        };

        match &import.kind {
            ImportKind::Module => {
                // Import module as a namespace - return module binding
                // 将模块作为命名空间导入 - 返回模块绑定
                let alias = import
                    .alias
                    .clone()
                    .or_else(|| absolute_path.last().cloned())
                    .unwrap_or_else(|| "module".to_string());

                // For module imports, we need special handling
                // 对于模块导入，需要特殊处理
                // Return empty for now - the caller should handle namespace creation
                // 暂时返回空 - 调用者应处理命名空间创建
                vec![(alias, DefId(u32::MAX))] // Sentinel for module namespace / 模块命名空间的哨兵值
            }
            ImportKind::Items(names) => names
                .iter()
                .filter_map(|name| {
                    self.find_exported_def(module, name)
                        .map(|def_id| (name.clone(), def_id))
                })
                .collect(),
            ImportKind::All => {
                module
                    .items
                    .iter()
                    .filter_map(|item| {
                        // Only export if in the export list (or no explicit exports)
                        // 仅在导出列表中导出（或没有显式导出）
                        let name = self.item_name(item)?;
                        if let Some(exports) = &module.exports
                            && !exports.contains(&name)
                        {
                            return None;
                        }
                        Some((name, item.id))
                    })
                    .collect()
            }
        }
    }
}

/// A top-level item in HIR.
/// HIR 中的顶层项。
#[derive(Debug, Clone)]
pub struct Item {
    /// Definition ID. / 定义 ID。
    pub id: DefId,
    /// Item kind. / 项类型。
    pub kind: ItemKind,
    /// Source location. / 源代码位置。
    pub span: Span,
}

/// Kind of top-level item.
/// 顶层项的类型。
#[derive(Debug, Clone)]
pub enum ItemKind {
    /// Function definition. / 函数定义。
    Fn(FnDef),
    /// Struct definition. / 结构体定义。
    Struct(StructDef),
    /// Enum definition. / 枚举定义。
    Enum(EnumDef),
    /// Type alias. / 类型别名。
    TypeAlias(TypeAlias),
    /// Trait definition. / Trait 定义。
    Trait(TraitDef),
    /// Implementation block. / 实现块。
    Impl(ImplDef),
}

/// Function definition.
/// 函数定义。
#[derive(Debug, Clone)]
pub struct FnDef {
    /// Function name. / 函数名称。
    pub name: String,
    /// Generic parameters. / 泛型参数。
    pub generics: Vec<GenericParam>,
    /// Function parameters. / 函数参数。
    pub params: Vec<Param>,
    /// Return type. / 返回类型。
    pub return_ty: Ty,
    /// Function body. / 函数体。
    pub body: Expr,
}

/// Function parameter.
/// 函数参数。
#[derive(Debug, Clone)]
pub struct Param {
    /// Local variable ID. / 局部变量 ID。
    pub id: LocalId,
    /// Parameter name. / 参数名称。
    pub name: String,
    /// Parameter type. / 参数类型。
    pub ty: Ty,
    /// Source location. / 源代码位置。
    pub span: Span,
}

/// Generic type parameter.
/// 泛型类型参数。
#[derive(Debug, Clone)]
pub struct GenericParam {
    /// Parameter name. / 参数名称。
    pub name: String,
    /// Trait bounds. / Trait 约束。
    pub bounds: Vec<Ty>,
    /// Source location. / 源代码位置。
    pub span: Span,
}

/// Struct definition.
/// 结构体定义。
#[derive(Debug, Clone)]
pub struct StructDef {
    /// Struct name. / 结构体名称。
    pub name: String,
    /// Generic parameters. / 泛型参数。
    pub generics: Vec<GenericParam>,
    /// Struct fields. / 结构体字段。
    pub fields: Vec<FieldDef>,
}

/// Struct field definition.
/// 结构体字段定义。
#[derive(Debug, Clone)]
pub struct FieldDef {
    /// Field name. / 字段名称。
    pub name: String,
    /// Field type. / 字段类型。
    pub ty: Ty,
    /// Source location. / 源代码位置。
    pub span: Span,
}

/// Enum definition.
/// 枚举定义。
#[derive(Debug, Clone)]
pub struct EnumDef {
    /// Enum name. / 枚举名称。
    pub name: String,
    /// Generic parameters. / 泛型参数。
    pub generics: Vec<GenericParam>,
    /// Enum variants. / 枚举变体。
    pub variants: Vec<VariantDef>,
}

/// Enum variant definition.
/// 枚举变体定义。
#[derive(Debug, Clone)]
pub struct VariantDef {
    /// Variant name. / 变体名称。
    pub name: String,
    /// Variant fields (for tuple variants). / 变体字段（用于元组变体）。
    pub fields: Vec<Ty>,
    /// Source location. / 源代码位置。
    pub span: Span,
}

/// Type alias definition.
/// 类型别名定义。
#[derive(Debug, Clone)]
pub struct TypeAlias {
    /// Alias name. / 别名名称。
    pub name: String,
    /// Generic parameters. / 泛型参数。
    pub generics: Vec<GenericParam>,
    /// Target type. / 目标类型。
    pub ty: Ty,
}

/// Trait definition.
/// Trait 定义。
#[derive(Debug, Clone)]
pub struct TraitDef {
    /// Trait name. / Trait 名称。
    pub name: String,
    /// Generic parameters. / 泛型参数。
    pub generics: Vec<GenericParam>,
    /// Method items in the trait. / Trait 中的方法项。
    pub items: Vec<TraitItem>,
    /// Associated types defined by this trait. / 此 Trait 定义的关联类型。
    pub assoc_types: Vec<AssocTypeDef>,
}

/// An associated type definition in a trait.
/// Trait 中的关联类型定义。
#[derive(Debug, Clone)]
pub struct AssocTypeDef {
    /// Name of the associated type. / 关联类型的名称。
    pub name: String,
    /// Trait bounds on the associated type (e.g., `type Item: Eq`). / 关联类型的 Trait 约束（例如 `type Item: Eq`）。
    pub bounds: Vec<Ty>,
    /// Default type (if any). / 默认类型（如有）。
    pub default: Option<Ty>,
    /// Source location. / 源代码位置。
    pub span: Span,
}

/// A trait item (method declaration).
/// Trait 项（方法声明）。
#[derive(Debug, Clone)]
pub struct TraitItem {
    /// Method name. / 方法名称。
    pub name: String,
    /// Generic parameters. / 泛型参数。
    pub generics: Vec<GenericParam>,
    /// Parameter types. / 参数类型。
    pub params: Vec<Ty>,
    /// Return type. / 返回类型。
    pub return_ty: Ty,
    /// Default implementation (if any). / 默认实现（如有）。
    pub default: Option<Expr>,
    /// Source location. / 源代码位置。
    pub span: Span,
}

/// Implementation block.
/// 实现块。
#[derive(Debug, Clone)]
pub struct ImplDef {
    /// Generic parameters. / 泛型参数。
    pub generics: Vec<GenericParam>,
    /// Trait being implemented (if any). / 被实现的 Trait（如有）。
    pub trait_ref: Option<Ty>,
    /// Self type. / Self 类型。
    pub self_ty: Ty,
    /// Method implementations. / 方法实现。
    pub items: Vec<ImplItem>,
    /// Associated type implementations. / 关联类型实现。
    pub assoc_type_impls: Vec<AssocTypeImpl>,
}

/// An associated type implementation in an impl block.
/// impl 块中的关联类型实现。
#[derive(Debug, Clone)]
pub struct AssocTypeImpl {
    /// Name of the associated type being implemented. / 被实现的关联类型的名称。
    pub name: String,
    /// The concrete type. / 具体类型。
    pub ty: Ty,
    /// Source location. / 源代码位置。
    pub span: Span,
}

/// An implementation item (method implementation).
/// 实现项（方法实现）。
#[derive(Debug, Clone)]
pub struct ImplItem {
    /// Method name. / 方法名称。
    pub name: String,
    /// Generic parameters. / 泛型参数。
    pub generics: Vec<GenericParam>,
    /// Method parameters. / 方法参数。
    pub params: Vec<Param>,
    /// Return type. / 返回类型。
    pub return_ty: Ty,
    /// Method body. / 方法体。
    pub body: Expr,
    /// Source location. / 源代码位置。
    pub span: Span,
}

/// HIR expression.
/// HIR 表达式。
#[derive(Debug, Clone)]
pub struct Expr {
    /// Expression kind. / 表达式类型。
    pub kind: ExprKind,
    /// Expression type. / 表达式类型。
    pub ty: Ty,
    /// Source location. / 源代码位置。
    pub span: Span,
}

/// Kind of expression.
/// 表达式的类型。
#[derive(Debug, Clone)]
pub enum ExprKind {
    /// Literal value. / 字面量值。
    Literal(Literal),
    /// Local variable reference. / 局部变量引用。
    Var(LocalId),
    /// Global definition reference. / 全局定义引用。
    Global(DefId),
    /// Record construction. / 记录构造。
    Record(Vec<(String, Expr)>),
    /// List literal. / 列表字面量。
    List(Vec<Expr>),
    /// Tuple expression. / 元组表达式。
    Tuple(Vec<Expr>),
    /// Lambda expression. / Lambda 表达式。
    Lambda(Vec<Param>, Box<Expr>),
    /// Function call. / 函数调用。
    Call(Box<Expr>, Vec<Expr>),
    /// Field access. / 字段访问。
    Field(Box<Expr>, String),
    /// Tuple index access. / 元组索引访问。
    TupleIndex(Box<Expr>, u32),
    /// Binary operation. / 二元运算。
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// Unary operation. / 一元运算。
    Unary(UnaryOp, Box<Expr>),
    /// Conditional expression. / 条件表达式。
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    /// Pattern matching. / 模式匹配。
    Match(Box<Expr>, Vec<MatchArm>),
    /// Block expression. / 块表达式。
    Block(Vec<Stmt>, Option<Box<Expr>>),
    /// Interpolated string `` `hello {name}` ``. / 插值字符串 `` `hello {name}` ``。
    Interpolated(Vec<StringPart>),
}

/// A part of an interpolated string.
/// 插值字符串的一部分。
#[derive(Debug, Clone)]
pub enum StringPart {
    /// Literal string part. / 字面量字符串部分。
    Literal(String),
    /// Interpolated expression. / 插值表达式。
    Expr(Expr),
}

/// Literal value.
/// 字面量值。
#[derive(Debug, Clone)]
pub enum Literal {
    /// Integer. / 整数。
    Int(i64),
    /// Float. / 浮点数。
    Float(f64),
    /// String. / 字符串。
    String(String),
    /// Character. / 字符。
    Char(char),
    /// Boolean. / 布尔值。
    Bool(bool),
    /// Unit value. / 单元值。
    Unit,
}

/// Binary operator.
/// 二元运算符。
#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    /// Addition (`+`). / 加法（`+`）。
    Add,
    /// Subtraction (`-`). / 减法（`-`）。
    Sub,
    /// Multiplication (`*`). / 乘法（`*`）。
    Mul,
    /// Division (`/`). / 除法（`/`）。
    Div,
    /// Modulo (`%`). / 取模（`%`）。
    Mod,
    /// Power (`**`). / 幂运算（`**`）。
    Pow,
    /// Equality (`==`). / 相等（`==`）。
    Eq,
    /// Inequality (`!=`). / 不等（`!=`）。
    Ne,
    /// Less than (`<`). / 小于（`<`）。
    Lt,
    /// Less than or equal (`<=`). / 小于等于（`<=`）。
    Le,
    /// Greater than (`>`). / 大于（`>`）。
    Gt,
    /// Greater than or equal (`>=`). / 大于等于（`>=`）。
    Ge,
    /// Logical AND (`&&`). / 逻辑与（`&&`）。
    And,
    /// Logical OR (`||`). / 逻辑或（`||`）。
    Or,
    /// List concatenation (`++`). / 列表连接（`++`）。
    Concat,
    /// Record merge (`//`). / 记录合并（`//`）。
    Merge,
    /// Pipe operator (`|>`). / 管道运算符（`|>`）。
    Pipe,
}

/// Unary operator.
/// 一元运算符。
#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    /// Negation (`-`). / 取负（`-`）。
    Neg,
    /// Logical NOT (`!`). / 逻辑非（`!`）。
    Not,
}

/// A match arm with pattern, optional guard, and body.
/// 带有模式、可选守卫和主体的匹配分支。
#[derive(Debug, Clone)]
pub struct MatchArm {
    /// Pattern to match. / 要匹配的模式。
    pub pattern: Pattern,
    /// Optional guard expression. / 可选的守卫表达式。
    pub guard: Option<Expr>,
    /// Arm body. / 分支主体。
    pub body: Expr,
    /// Source location. / 源代码位置。
    pub span: Span,
}

/// HIR pattern.
/// HIR 模式。
#[derive(Debug, Clone)]
pub struct Pattern {
    /// Pattern kind. / 模式类型。
    pub kind: PatternKind,
    /// Source location. / 源代码位置。
    pub span: Span,
}

/// Kind of pattern.
/// 模式的类型。
#[derive(Debug, Clone)]
pub enum PatternKind {
    /// Wildcard pattern (`_`). / 通配符模式（`_`）。
    Wildcard,
    /// Variable binding. / 变量绑定。
    Var(LocalId, String),
    /// Literal pattern. / 字面量模式。
    Literal(Literal),
    /// Tuple pattern. / 元组模式。
    Tuple(Vec<Pattern>),
    /// List pattern. / 列表模式。
    List(Vec<Pattern>),
    /// Record pattern. / 记录模式。
    Record(Vec<(String, Pattern)>),
    /// Constructor pattern. / 构造器模式。
    Constructor(DefId, Vec<Pattern>),
}

/// HIR statement.
/// HIR 语句。
#[derive(Debug, Clone)]
pub struct Stmt {
    /// Statement kind. / 语句类型。
    pub kind: StmtKind,
    /// Source location. / 源代码位置。
    pub span: Span,
}

/// Kind of statement.
/// 语句的类型。
#[derive(Debug, Clone)]
pub enum StmtKind {
    /// Let binding. / let 绑定。
    Let(LocalId, String, Ty, Expr),
    /// Expression statement. / 表达式语句。
    Expr(Expr),
}

/// HIR type representation.
/// HIR 类型表示。
#[derive(Debug, Clone)]
pub struct Ty {
    /// Type kind. / 类型种类。
    pub kind: TyKind,
    /// Source location. / 源代码位置。
    pub span: Span,
}

/// Kind of type.
/// 类型的种类。
#[derive(Debug, Clone)]
pub enum TyKind {
    // Primitive types / 原始类型
    /// Integer type. / 整数类型。
    Int,
    /// Float type. / 浮点数类型。
    Float,
    /// Boolean type. / 布尔类型。
    Bool,
    /// Character type. / 字符类型。
    Char,
    /// String type. / 字符串类型。
    String,
    /// Unit type. / 单元类型。
    Unit,

    /// Type variable (for inference). / 类型变量（用于推断）。
    Var(u32),

    /// Generic type parameter reference (e.g., `T` in `fn foo<T>(x: T)`). / 泛型类型参数引用（例如 `fn foo<T>(x: T)` 中的 `T`）。
    Param(u32, String),

    /// Named type with arguments (e.g., `List<Int>`). / 带参数的命名类型（例如 `List<Int>`）。
    Named(DefId, Vec<Ty>),

    /// Function type. / 函数类型。
    Fn(Vec<Ty>, Box<Ty>),

    /// Tuple type. / 元组类型。
    Tuple(Vec<Ty>),

    /// Record type. / 记录类型。
    Record(Vec<(String, Ty)>),

    /// Forall type (polymorphic type, e.g., `forall a. a -> a`). / Forall 类型（多态类型，例如 `forall a. a -> a`）。
    Forall(Vec<String>, Box<Ty>),

    /// Unknown type (placeholder). / 未知类型（占位符）。
    Unknown,
}
