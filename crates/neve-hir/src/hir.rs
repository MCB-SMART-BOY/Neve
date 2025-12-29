//! HIR node definitions.

use neve_common::Span;
use std::collections::HashMap;

/// A unique identifier for a definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefId(pub u32);

/// A unique identifier for a local variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(pub u32);

/// A unique identifier for a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModuleId(pub u32);

/// HIR module containing all definitions.
#[derive(Debug, Clone)]
pub struct Module {
    /// Module identifier
    pub id: ModuleId,
    /// Module name (from file path or explicit module declaration)
    pub name: String,
    /// Top-level items in this module
    pub items: Vec<Item>,
    /// Imports from other modules
    pub imports: Vec<Import>,
    /// Exported names (None = export all public items)
    pub exports: Option<Vec<String>>,
}

impl Module {
    /// Create a new empty module
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
#[derive(Debug, Clone)]
pub struct Import {
    /// Path prefix (self, super, crate, or absolute)
    pub prefix: ImportPathPrefix,
    /// The module path segments (e.g., ["list"] for `std.list`)
    pub path: Vec<String>,
    /// What to import from the module
    pub kind: ImportKind,
    /// Optional alias for the import
    pub alias: Option<String>,
    /// Whether this is a re-export (`pub import`)
    pub is_pub: bool,
    /// Source location
    pub span: Span,
}

/// Path prefix for imports in HIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImportPathPrefix {
    /// Absolute path (no prefix)
    #[default]
    Absolute,
    /// `self.` - relative to current module
    Self_,
    /// `super.` - relative to parent module
    Super,
    /// `crate.` - relative to crate root
    Crate,
}

/// What kind of import this is.
#[derive(Debug, Clone)]
pub enum ImportKind {
    /// Import the entire module: `import std.list`
    Module,
    /// Import specific items: `import std.list (map, filter)`
    Items(Vec<String>),
    /// Import all public items: `import std.list (*)`
    All,
}

/// A module registry that tracks all loaded modules.
#[derive(Debug, Default)]
pub struct ModuleRegistry {
    /// All loaded modules by ID
    modules: HashMap<ModuleId, Module>,
    /// Module lookup by path
    path_to_id: HashMap<Vec<String>, ModuleId>,
    /// Next module ID
    next_id: u32,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new module and return its ID.
    pub fn register(&mut self, name: String, path: Vec<String>) -> ModuleId {
        let id = ModuleId(self.next_id);
        self.next_id += 1;

        let module = Module::new(id, name);
        self.modules.insert(id, module);
        self.path_to_id.insert(path, id);

        id
    }

    /// Get a module by ID.
    pub fn get(&self, id: ModuleId) -> Option<&Module> {
        self.modules.get(&id)
    }

    /// Get a mutable module by ID.
    pub fn get_mut(&mut self, id: ModuleId) -> Option<&mut Module> {
        self.modules.get_mut(&id)
    }

    /// Look up a module by path.
    pub fn lookup(&self, path: &[String]) -> Option<ModuleId> {
        self.path_to_id.get(path).copied()
    }

    /// Get all modules.
    pub fn all_modules(&self) -> impl Iterator<Item = &Module> {
        self.modules.values()
    }

    /// Resolve an import to get the definitions it brings into scope.
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
                // The alias or last path component becomes the namespace name
                vec![]
            }
            ImportKind::Items(names) => {
                // Import specific items
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
                module
                    .items
                    .iter()
                    .filter_map(|item| self.item_name(item).map(|name| (name, item.id)))
                    .collect()
            }
        }
    }

    /// Find an exported definition by name.
    fn find_exported_def(&self, module: &Module, name: &str) -> Option<DefId> {
        // Check if the module has an explicit export list
        if let Some(exports) = &module.exports
            && !exports.contains(&name.to_string())
        {
            return None;
        }

        // Find the item with this name
        module
            .items
            .iter()
            .find(|item| self.item_name(item).as_deref() == Some(name))
            .map(|item| item.id)
    }

    /// Get the name of an item.
    fn item_name(&self, item: &Item) -> Option<String> {
        match &item.kind {
            ItemKind::Fn(f) => Some(f.name.clone()),
            ItemKind::Struct(s) => Some(s.name.clone()),
            ItemKind::Enum(e) => Some(e.name.clone()),
            ItemKind::TypeAlias(t) => Some(t.name.clone()),
            ItemKind::Trait(t) => Some(t.name.clone()),
            ItemKind::Impl(_) => None, // Impls don't have names
        }
    }

    /// Insert a fully resolved module into the registry.
    pub fn insert(&mut self, path: Vec<String>, module: Module) {
        let id = module.id;
        self.modules.insert(id, module);
        self.path_to_id.insert(path, id);
    }

    /// Get the number of registered modules.
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// Get all module paths.
    pub fn all_paths(&self) -> impl Iterator<Item = &Vec<String>> {
        self.path_to_id.keys()
    }

    /// Find an item by DefId across all modules.
    pub fn find_item(&self, def_id: DefId) -> Option<(&Module, &Item)> {
        for module in self.modules.values() {
            if let Some(item) = module.items.iter().find(|item| item.id == def_id) {
                return Some((module, item));
            }
        }
        None
    }

    /// Get the module path for a given module ID.
    pub fn module_path(&self, id: ModuleId) -> Option<&Vec<String>> {
        self.path_to_id
            .iter()
            .find(|&(_, &mid)| mid == id)
            .map(|(path, _)| path)
    }

    /// Resolve an import with path prefix support.
    pub fn resolve_import_with_prefix(
        &self,
        import: &Import,
        current_module_path: &[String],
    ) -> Vec<(String, DefId)> {
        // Compute the absolute path based on prefix
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
                    return Vec::new(); // Can't go above root
                }
                let mut path = current_module_path[..current_module_path.len() - 1].to_vec();
                path.extend(import.path.iter().cloned());
                path
            }
        };

        // Look up the target module
        let Some(module_id) = self.lookup(&absolute_path) else {
            return Vec::new();
        };

        let Some(module) = self.get(module_id) else {
            return Vec::new();
        };

        match &import.kind {
            ImportKind::Module => {
                // Import module as a namespace - return module binding
                let alias = import
                    .alias
                    .clone()
                    .or_else(|| absolute_path.last().cloned())
                    .unwrap_or_else(|| "module".to_string());

                // For module imports, we need special handling
                // Return empty for now - the caller should handle namespace creation
                vec![(alias, DefId(u32::MAX))] // Sentinel for module namespace
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
#[derive(Debug, Clone)]
pub struct Item {
    pub id: DefId,
    pub kind: ItemKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ItemKind {
    Fn(FnDef),
    Struct(StructDef),
    Enum(EnumDef),
    TypeAlias(TypeAlias),
    Trait(TraitDef),
    Impl(ImplDef),
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_ty: Ty,
    pub body: Expr,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub id: LocalId,
    pub name: String,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct GenericParam {
    pub name: String,
    pub bounds: Vec<Ty>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub fields: Vec<FieldDef>,
}

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub variants: Vec<VariantDef>,
}

#[derive(Debug, Clone)]
pub struct VariantDef {
    pub name: String,
    pub fields: Vec<Ty>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeAlias {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub ty: Ty,
}

#[derive(Debug, Clone)]
pub struct TraitDef {
    pub name: String,
    pub generics: Vec<GenericParam>,
    /// Method items in the trait
    pub items: Vec<TraitItem>,
    /// Associated types defined by this trait
    pub assoc_types: Vec<AssocTypeDef>,
}

/// An associated type definition in a trait.
#[derive(Debug, Clone)]
pub struct AssocTypeDef {
    /// Name of the associated type
    pub name: String,
    /// Trait bounds on the associated type (e.g., `type Item: Eq`)
    pub bounds: Vec<Ty>,
    /// Default type (if any)
    pub default: Option<Ty>,
    /// Source location
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TraitItem {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Ty>,
    pub return_ty: Ty,
    pub default: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImplDef {
    pub generics: Vec<GenericParam>,
    pub trait_ref: Option<Ty>,
    pub self_ty: Ty,
    /// Method implementations
    pub items: Vec<ImplItem>,
    /// Associated type implementations
    pub assoc_type_impls: Vec<AssocTypeImpl>,
}

/// An associated type implementation in an impl block.
#[derive(Debug, Clone)]
pub struct AssocTypeImpl {
    /// Name of the associated type being implemented
    pub name: String,
    /// The concrete type
    pub ty: Ty,
    /// Source location
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ImplItem {
    pub name: String,
    pub generics: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_ty: Ty,
    pub body: Expr,
    pub span: Span,
}

/// HIR expression.
#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub ty: Ty,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Literal(Literal),
    Var(LocalId),
    Global(DefId),
    Record(Vec<(String, Expr)>),
    List(Vec<Expr>),
    Tuple(Vec<Expr>),
    Lambda(Vec<Param>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Field(Box<Expr>, String),
    TupleIndex(Box<Expr>, u32),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Unary(UnaryOp, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Match(Box<Expr>, Vec<MatchArm>),
    Block(Vec<Stmt>, Option<Box<Expr>>),
    /// Interpolated string `` `hello {name}` ``
    Interpolated(Vec<StringPart>),
}

/// A part of an interpolated string.
#[derive(Debug, Clone)]
pub enum StringPart {
    /// Literal string part
    Literal(String),
    /// Interpolated expression
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
    Unit,
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Concat,
    Merge,
    Pipe,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum PatternKind {
    Wildcard,
    Var(LocalId, String),
    Literal(Literal),
    Tuple(Vec<Pattern>),
    List(Vec<Pattern>),
    Record(Vec<(String, Pattern)>),
    Constructor(DefId, Vec<Pattern>),
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    Let(LocalId, String, Ty, Expr),
    Expr(Expr),
}

/// HIR type representation.
#[derive(Debug, Clone)]
pub struct Ty {
    pub kind: TyKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TyKind {
    /// Primitive types
    Int,
    Float,
    Bool,
    Char,
    String,
    Unit,

    /// Type variable (for inference)
    Var(u32),

    /// Generic type parameter reference (e.g., `T` in `fn foo<T>(x: T)`)
    Param(u32, String),

    /// Named type with arguments (e.g., `List<Int>`)
    Named(DefId, Vec<Ty>),

    /// Function type
    Fn(Vec<Ty>, Box<Ty>),

    /// Tuple type
    Tuple(Vec<Ty>),

    /// Record type
    Record(Vec<(String, Ty)>),

    /// Forall type (polymorphic type, e.g., `forall a. a -> a`)
    Forall(Vec<String>, Box<Ty>),

    /// Unknown type (placeholder)
    Unknown,
}
