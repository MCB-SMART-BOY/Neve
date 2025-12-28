//! Top-level AST definitions.

use neve_common::Span;
use crate::{Expr, Pattern, Type};

/// A complete source file.
#[derive(Debug, Clone)]
pub struct SourceFile {
    pub items: Vec<Item>,
    pub span: Span,
}

/// A top-level item.
#[derive(Debug, Clone)]
pub struct Item {
    pub kind: ItemKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ItemKind {
    /// `let name = expr;`
    Let(LetDef),
    /// `fn name(params) -> Type = expr;`
    Fn(FnDef),
    /// `type Name = Type;`
    TypeAlias(TypeAlias),
    /// `struct Name { fields };`
    Struct(StructDef),
    /// `enum Name { variants };`
    Enum(EnumDef),
    /// `trait Name { items };`
    Trait(TraitDef),
    /// `impl Trait for Type { items };`
    Impl(ImplDef),
    /// `import path;`
    Import(ImportDef),
}

/// A let binding at the top level.
#[derive(Debug, Clone)]
pub struct LetDef {
    pub is_pub: bool,
    pub pattern: Pattern,
    pub ty: Option<Type>,
    pub value: Expr,
}

/// A function definition.
#[derive(Debug, Clone)]
pub struct FnDef {
    pub is_pub: bool,
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Expr,
}

/// A function parameter.
#[derive(Debug, Clone)]
pub struct Param {
    pub pattern: Pattern,
    pub ty: Type,
    pub is_lazy: bool,
    pub span: Span,
}

/// A generic type parameter.
#[derive(Debug, Clone)]
pub struct GenericParam {
    pub name: Ident,
    pub bounds: Vec<Type>,
    pub span: Span,
}

/// A type alias.
#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub is_pub: bool,
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub ty: Type,
}

/// A struct definition.
#[derive(Debug, Clone)]
pub struct StructDef {
    pub is_pub: bool,
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub fields: Vec<FieldDef>,
}

/// A struct field.
#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: Ident,
    pub ty: Type,
    pub default: Option<Expr>,
    pub span: Span,
}

/// An enum definition.
#[derive(Debug, Clone)]
pub struct EnumDef {
    pub is_pub: bool,
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub variants: Vec<Variant>,
}

/// An enum variant.
#[derive(Debug, Clone)]
pub struct Variant {
    pub name: Ident,
    pub kind: VariantKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum VariantKind {
    /// `Variant`
    Unit,
    /// `Variant(T1, T2)`
    Tuple(Vec<Type>),
    /// `Variant #{ field: T }`
    Record(Vec<FieldDef>),
}

/// A trait definition.
#[derive(Debug, Clone)]
pub struct TraitDef {
    pub is_pub: bool,
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub items: Vec<TraitItem>,
}

/// A trait item (method signature).
#[derive(Debug, Clone)]
pub struct TraitItem {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub default: Option<Expr>,
    pub span: Span,
}

/// An impl block.
#[derive(Debug, Clone)]
pub struct ImplDef {
    pub generics: Vec<GenericParam>,
    pub trait_: Option<Type>,
    pub target: Type,
    pub items: Vec<ImplItem>,
}

/// An impl item (method implementation).
#[derive(Debug, Clone)]
pub struct ImplItem {
    pub name: Ident,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub body: Expr,
    pub span: Span,
}

/// An import statement.
#[derive(Debug, Clone)]
pub struct ImportDef {
    /// Path prefix (self, super, crate, or absolute)
    pub prefix: PathPrefix,
    /// The import path segments (e.g., ["utils", "helpers"])
    pub path: Vec<Ident>,
    /// What to import from the module
    pub items: ImportItems,
    /// Optional alias for the import
    pub alias: Option<Ident>,
    /// Whether this is a re-export (`pub import`)
    pub is_pub: bool,
}

/// Path prefix for imports and module paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PathPrefix {
    /// Absolute path (no prefix, starts from root)
    #[default]
    Absolute,
    /// `self::` - relative to current module
    Self_,
    /// `super::` - relative to parent module
    Super,
    /// `crate::` - relative to crate root
    Crate,
}

#[derive(Debug, Clone)]
pub enum ImportItems {
    /// Import the module itself
    Module,
    /// Import specific items: `import a.b (x, y)`
    Items(Vec<Ident>),
    /// Import all: `import a.b (*)`
    All,
}

/// Visibility level for items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    /// Private to the current module (default)
    #[default]
    Private,
    /// Public (visible everywhere)
    Public,
    /// Visible only within the crate: `pub(crate)`
    Crate,
    /// Visible to parent module: `pub(super)`
    Super,
}

/// An identifier.
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
