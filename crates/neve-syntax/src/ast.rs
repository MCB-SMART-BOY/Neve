//! Top-level AST definitions.
//! 顶层 AST 定义。

use crate::{Expr, Pattern, Type};
use neve_common::Span;

/// A complete source file.
/// 完整的源文件。
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub items: Vec<Item>,
    pub span: Span,
}

/// A top-level item.
/// 顶层项。
#[derive(Debug, Clone)]
pub struct Item {
    pub kind: ItemKind,
    pub span: Span,
}

/// Kind of top-level item.
/// 顶层项的类型。
#[derive(Debug, Clone)]
pub enum ItemKind {
    /// `let name = expr;` / let 绑定
    Let(LetDef),
    /// `fn name(params) -> Type = expr;` / 函数定义
    Fn(FnDef),
    /// `type Name = Type;` / 类型别名
    TypeAlias(TypeAlias),
    /// `struct Name { fields };` / 结构体定义
    Struct(StructDef),
    /// `enum Name { variants };` / 枚举定义
    Enum(EnumDef),
    /// `trait Name { items };` / 特征定义
    Trait(TraitDef),
    /// `impl Trait for Type { items };` / 实现块
    Impl(ImplDef),
    /// `import path;` / 导入语句
    Import(ImportDef),
}

/// A let binding at the top level.
/// 顶层 let 绑定。
#[derive(Debug, Clone)]
pub struct LetDef {
    pub visibility: Visibility,
    pub pattern: Pattern,
    pub ty: Option<Type>,
    pub value: Expr,
}

/// A function definition.
/// 函数定义。
#[derive(Debug, Clone)]
pub struct FnDef {
    pub visibility: Visibility,
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Expr,
}

/// A function parameter.
/// 函数参数。
#[derive(Debug, Clone)]
pub struct Param {
    pub pattern: Pattern,
    pub ty: Type,
    pub is_lazy: bool,
    pub span: Span,
}

/// A generic type parameter.
/// 泛型类型参数。
#[derive(Debug, Clone)]
pub struct GenericParam {
    pub name: Ident,
    pub bounds: Vec<Type>,
    pub span: Span,
}

/// A type alias.
/// 类型别名。
#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub visibility: Visibility,
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub ty: Type,
}

/// A struct definition.
/// 结构体定义。
#[derive(Debug, Clone)]
pub struct StructDef {
    pub visibility: Visibility,
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub fields: Vec<FieldDef>,
}

/// A struct field.
/// 结构体字段。
#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: Ident,
    pub ty: Type,
    pub default: Option<Expr>,
    pub span: Span,
}

/// An enum definition.
/// 枚举定义。
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub visibility: Visibility,
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub variants: Vec<Variant>,
}

/// An enum variant.
/// 枚举变体。
#[derive(Debug, Clone)]
pub struct Variant {
    pub name: Ident,
    pub kind: VariantKind,
    pub span: Span,
}

/// Kind of enum variant.
/// 枚举变体的类型。
#[derive(Debug, Clone)]
pub enum VariantKind {
    /// `Variant` / 单元变体
    Unit,
    /// `Variant(T1, T2)` / 元组变体
    Tuple(Vec<Type>),
    /// `Variant #{ field: T }` / 记录变体
    Record(Vec<FieldDef>),
}

/// A trait definition.
/// 特征定义。
#[derive(Debug, Clone)]
pub struct TraitDef {
    pub visibility: Visibility,
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub items: Vec<TraitItem>,
    pub assoc_types: Vec<AssocTypeDef>,
}

/// A trait item (method signature).
/// 特征项（方法签名）。
#[derive(Debug, Clone)]
pub struct TraitItem {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub default: Option<Expr>,
    pub span: Span,
}

/// An associated type definition in a trait.
/// 特征中的关联类型定义。
#[derive(Debug, Clone)]
pub struct AssocTypeDef {
    pub name: Ident,
    pub bounds: Vec<Type>,
    pub default: Option<Type>,
    pub span: Span,
}

/// An impl block.
/// 实现块。
#[derive(Debug, Clone)]
pub struct ImplDef {
    pub generics: Vec<GenericParam>,
    pub trait_: Option<Type>,
    pub target: Type,
    pub items: Vec<ImplItem>,
    pub assoc_type_impls: Vec<AssocTypeImpl>,
}

/// An impl item (method implementation).
/// 实现项（方法实现）。
#[derive(Debug, Clone)]
pub struct ImplItem {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Expr,
    pub span: Span,
}

/// An associated type implementation in an impl block.
/// 实现块中的关联类型实现。
#[derive(Debug, Clone)]
pub struct AssocTypeImpl {
    pub name: Ident,
    pub ty: Type,
    pub span: Span,
}

/// An import statement.
/// 导入语句。
#[derive(Debug, Clone)]
pub struct ImportDef {
    /// Path prefix (self, super, crate, or absolute). / 路径前缀。
    pub prefix: PathPrefix,
    /// The import path segments (e.g., ["utils", "helpers"]). / 导入路径段。
    pub path: Vec<Ident>,
    /// What to import from the module. / 从模块导入的内容。
    pub items: ImportItems,
    /// Optional alias for the import. / 可选的导入别名。
    pub alias: Option<Ident>,
    /// Visibility for re-exports (`pub import`). / 重导出的可见性。
    pub visibility: Visibility,
}

/// Path prefix for imports and module paths.
/// 导入和模块路径的前缀。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PathPrefix {
    /// Absolute path (no prefix, starts from root). / 绝对路径。
    #[default]
    Absolute,
    /// `self::` - relative to current module. / 相对于当前模块。
    Self_,
    /// `super::` - relative to parent module. / 相对于父模块。
    Super,
    /// `crate::` - relative to crate root. / 相对于 crate 根。
    Crate,
}

/// What to import from a module.
/// 从模块导入的内容。
#[derive(Debug, Clone)]
pub enum ImportItems {
    /// Import the module itself. / 导入模块本身。
    Module,
    /// Import specific items: `import a.b (x, y)`. / 导入特定项。
    Items(Vec<Ident>),
    /// Import all: `import a.b (*)`. / 导入全部。
    All,
}

/// Visibility level for items.
/// 项的可见性级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    /// Private to the current module (default). / 私有（默认）。
    #[default]
    Private,
    /// Public (visible everywhere). / 公开（全局可见）。
    Public,
    /// Visible only within the crate: `pub(crate)`. / 仅 crate 内可见。
    Crate,
    /// Visible to parent module: `pub(super)`. / 父模块可见。
    Super,
}

/// An identifier.
/// 标识符。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident {
    pub name: String,
    pub span: Span,
}

impl Ident {
    pub fn new(name: impl Into<String>, span: Span) -> Self {
        Self {
            name: name.into(),
            span,
        }
    }
}
