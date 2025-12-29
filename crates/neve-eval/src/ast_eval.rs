//! Direct AST evaluation (without HIR).
//!
//! This is a simplified evaluator that works directly on the AST.
//! It's useful for quick prototyping and REPL.

use crate::EvalError;
use crate::builtin::builtins;
use crate::value::{Thunk, ThunkState, Value};
use neve_hir::{ModuleLoader, ModulePath};
use neve_syntax::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

// Re-import StringPart from syntax since we use it here
use neve_syntax::StringPart;

/// A binding with visibility information.
#[derive(Clone)]
struct Binding {
    value: Value,
    is_public: bool,
}

/// Environment for AST evaluation.
///
/// Note: For REPL usage, AstEnv can be wrapped in Rc<RefCell<AstEnv>>
/// to allow persistent state across evaluations.
#[derive(Clone, Default)]
pub struct AstEnv {
    bindings: HashMap<String, Binding>,
    parent: Option<Rc<AstEnv>>,
}

impl AstEnv {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtins() -> Self {
        let mut env = Self::new();
        // Load all builtins from the central registry - all are public
        for (name, value) in builtins() {
            env.bindings.insert(
                name.to_string(),
                Binding {
                    value,
                    is_public: true,
                },
            );
        }
        env
    }

    pub fn child(parent: Rc<AstEnv>) -> Self {
        Self {
            bindings: HashMap::new(),
            parent: Some(parent),
        }
    }

    /// Define a binding (private by default).
    pub fn define(&mut self, name: String, value: Value) {
        self.bindings.insert(
            name,
            Binding {
                value,
                is_public: false,
            },
        );
    }

    /// Define a public binding.
    pub fn define_pub(&mut self, name: String, value: Value) {
        self.bindings.insert(
            name,
            Binding {
                value,
                is_public: true,
            },
        );
    }

    /// Define a binding with explicit visibility.
    pub fn define_with_visibility(&mut self, name: String, value: Value, is_public: bool) {
        self.bindings.insert(name, Binding { value, is_public });
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(binding) = self.bindings.get(name) {
            return Some(binding.value.clone());
        }
        if let Some(parent) = &self.parent {
            return parent.get(name);
        }
        None
    }

    /// Get all bindings in this environment (not including parent).
    /// Used for module exports.
    pub fn all_bindings(&self) -> HashMap<String, Value> {
        self.bindings
            .iter()
            .map(|(k, v)| (k.clone(), v.value.clone()))
            .collect()
    }

    /// Get only public bindings in this environment.
    /// Used for module exports when respecting visibility.
    pub fn public_bindings(&self) -> HashMap<String, Value> {
        self.bindings
            .iter()
            .filter(|(_, v)| v.is_public)
            .map(|(k, v)| (k.clone(), v.value.clone()))
            .collect()
    }

    /// Check if a binding is public.
    pub fn is_public(&self, name: &str) -> bool {
        self.bindings
            .get(name)
            .map(|b| b.is_public)
            .unwrap_or(false)
    }
}

/// AST evaluator.
pub struct AstEvaluator {
    env: Rc<AstEnv>,
    /// Base path for resolving relative imports
    base_path: Option<PathBuf>,
    /// Cache of already-loaded modules
    loaded_modules: HashMap<PathBuf, Rc<AstEnv>>,
    /// Current module path (for relative imports)
    current_module_path: Vec<String>,
    /// Module loader for advanced module resolution
    module_loader: Option<ModuleLoader>,
}

impl AstEvaluator {
    pub fn new() -> Self {
        Self {
            env: Rc::new(AstEnv::with_builtins()),
            base_path: None,
            loaded_modules: HashMap::new(),
            current_module_path: Vec::new(),
            module_loader: None,
        }
    }

    pub fn with_env(env: Rc<AstEnv>) -> Self {
        Self {
            env,
            base_path: None,
            loaded_modules: HashMap::new(),
            current_module_path: Vec::new(),
            module_loader: None,
        }
    }

    pub fn with_base_path(mut self, path: PathBuf) -> Self {
        self.base_path = Some(path.clone());
        // Also initialize module loader with this path
        self.module_loader = Some(ModuleLoader::new(&path));
        self
    }

    /// Set the module loader explicitly.
    pub fn with_module_loader(mut self, loader: ModuleLoader) -> Self {
        self.module_loader = Some(loader);
        self
    }

    /// Set the current module path for relative imports.
    pub fn with_module_path(mut self, path: Vec<String>) -> Self {
        self.current_module_path = path;
        self
    }

    /// Get the module loader.
    pub fn module_loader(&self) -> Option<&ModuleLoader> {
        self.module_loader.as_ref()
    }

    /// Call an AstClosure with the given arguments.
    pub fn call_closure(
        &mut self,
        closure: &AstClosure,
        args: Vec<Value>,
    ) -> Result<Value, EvalError> {
        if args.len() != closure.params.len() {
            return Err(EvalError::WrongArity);
        }

        let mut new_env = AstEnv::child(closure.env.clone());
        for (param, arg) in closure.params.iter().zip(args) {
            let name = pattern_name(&param.pattern);
            new_env.define(name, arg);
        }

        let mut body_eval = AstEvaluator::with_env(Rc::new(new_env));
        if let Some(ref base) = self.base_path {
            body_eval.base_path = Some(base.clone());
        }
        body_eval.eval_expr(&closure.body)
    }

    /// Evaluate a source file.
    pub fn eval_file(&mut self, file: &SourceFile) -> Result<Value, EvalError> {
        let mut result = Value::Unit;

        for item in &file.items {
            result = self.eval_item(item)?;
        }

        Ok(result)
    }

    /// Evaluate a file at a given path.
    pub fn eval_file_at_path(&mut self, path: &Path) -> Result<Value, EvalError> {
        // Set base path for relative imports
        if let Some(parent) = path.parent() {
            self.base_path = Some(parent.to_path_buf());
        }

        let source = std::fs::read_to_string(path)
            .map_err(|e| EvalError::TypeError(format!("cannot read file: {}", e)))?;

        let (file, diagnostics) = neve_parser::parse(&source);

        if !diagnostics.is_empty() {
            return Err(EvalError::TypeError(format!(
                "parse error in {}",
                path.display()
            )));
        }

        self.eval_file(&file)
    }

    /// Evaluate a function definition and return its closure value.
    /// Useful for REPL to capture function definitions.
    pub fn eval_fn_def(&mut self, fn_def: &FnDef) -> Result<Value, EvalError> {
        // Create a closure that captures the current environment
        let func = AstClosure {
            params: fn_def.params.clone(),
            body: fn_def.body.clone(),
            env: self.env.clone(),
        };

        Ok(Value::AstClosure(Rc::new(func)))
    }

    fn eval_item(&mut self, item: &Item) -> Result<Value, EvalError> {
        match &item.kind {
            ItemKind::Let(let_def) => {
                let value = self.eval_expr(&let_def.value)?;
                let is_pub = let_def.visibility == Visibility::Public;
                self.bind_pattern_with_visibility(&let_def.pattern, value.clone(), is_pub)?;
                Ok(value)
            }
            ItemKind::Fn(fn_def) => {
                // For recursive functions, we need to define the function first,
                // then update the closure to capture the environment that includes itself.
                let name = fn_def.name.name.clone();
                let is_pub = fn_def.visibility == Visibility::Public;

                // Create a placeholder closure first
                let func = AstClosure {
                    params: fn_def.params.clone(),
                    body: fn_def.body.clone(),
                    env: self.env.clone(), // Will be updated below
                };

                // Define the function in the environment
                Rc::make_mut(&mut self.env).define_with_visibility(
                    name.clone(),
                    Value::AstClosure(Rc::new(func)),
                    is_pub,
                );

                // Now update the closure to have the environment that includes itself
                let recursive_func = AstClosure {
                    params: fn_def.params.clone(),
                    body: fn_def.body.clone(),
                    env: self.env.clone(), // Now includes the function itself
                };
                Rc::make_mut(&mut self.env).define_with_visibility(
                    name,
                    Value::AstClosure(Rc::new(recursive_func)),
                    is_pub,
                );

                Ok(Value::Unit)
            }
            ItemKind::Import(import_def) => {
                self.eval_import(import_def)?;
                Ok(Value::Unit)
            }
            _ => Ok(Value::Unit),
        }
    }

    fn eval_import(&mut self, import_def: &ImportDef) -> Result<(), EvalError> {
        // Resolve the module path to a file path
        let module_path = self.resolve_module_path(import_def)?;

        // Check if module is already loaded
        if let Some(module_env) = self.loaded_modules.get(&module_path).cloned() {
            // Import from cached module
            self.import_from_env(&module_env, import_def)?;
            return Ok(());
        }

        // Load the module
        let source = std::fs::read_to_string(&module_path).map_err(|e| {
            EvalError::TypeError(format!(
                "cannot load module '{}': {}",
                import_def
                    .path
                    .iter()
                    .map(|i| i.name.as_str())
                    .collect::<Vec<_>>()
                    .join("."),
                e
            ))
        })?;

        let (file, diagnostics) = neve_parser::parse(&source);

        if !diagnostics.is_empty() {
            return Err(EvalError::TypeError(format!(
                "parse error in module '{}'",
                import_def
                    .path
                    .iter()
                    .map(|i| i.name.as_str())
                    .collect::<Vec<_>>()
                    .join(".")
            )));
        }

        // Create a new evaluator for the module with its own environment
        let mut module_eval = AstEvaluator::new();
        if let Some(parent) = module_path.parent() {
            module_eval.base_path = Some(parent.to_path_buf());
        }

        // Evaluate the module
        module_eval.eval_file(&file)?;

        // Cache the module environment
        let module_env = module_eval.env.clone();
        self.loaded_modules.insert(module_path, module_env.clone());

        // Import from the module
        self.import_from_env(&module_env, import_def)?;

        Ok(())
    }

    fn resolve_module_path(&self, import_def: &ImportDef) -> Result<PathBuf, EvalError> {
        let path = &import_def.path;
        let path_segments: Vec<String> = path.iter().map(|i| i.name.clone()).collect();

        // Try using ModuleLoader first if available
        if let Some(ref loader) = self.module_loader {
            let module_path = match import_def.prefix {
                PathPrefix::Absolute => ModulePath::absolute(path_segments.clone()),
                PathPrefix::Self_ => ModulePath::self_(path_segments.clone()),
                PathPrefix::Super => ModulePath::super_(path_segments.clone()),
                PathPrefix::Crate => ModulePath::crate_(path_segments.clone()),
            };

            if let Some(file_path) =
                loader.resolve_path(&module_path, Some(&self.current_module_path))
            {
                return Ok(file_path);
            }
        }

        // Fallback to manual resolution
        let module_name: String = path_segments.join("/");

        // Determine base directory based on path prefix
        let base_dir = match import_def.prefix {
            PathPrefix::Absolute => {
                // Absolute path - search from crate root or current directory
                self.base_path.clone().unwrap_or_else(|| PathBuf::from("."))
            }
            PathPrefix::Self_ => {
                // Self-relative - search from current module's directory
                self.base_path.clone().unwrap_or_else(|| PathBuf::from("."))
            }
            PathPrefix::Super => {
                // Super-relative - search from parent directory
                self.base_path
                    .as_ref()
                    .and_then(|p| p.parent())
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from(".."))
            }
            PathPrefix::Crate => {
                // Crate-relative - search from crate root
                // For now, assume crate root is the base_path or current directory
                self.base_path.clone().unwrap_or_else(|| PathBuf::from("."))
            }
        };

        // Try various locations
        let candidates = vec![
            base_dir.join(format!("{}.neve", module_name)),
            base_dir.join(&module_name).join("mod.neve"),
        ];

        for candidate in &candidates {
            if candidate.exists() {
                return Ok(candidate.clone());
            }
        }

        Err(EvalError::TypeError(format!(
            "cannot find module '{}' (tried: {})",
            path.iter()
                .map(|i| i.name.as_str())
                .collect::<Vec<_>>()
                .join("."),
            candidates
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }

    fn import_from_env(
        &mut self,
        module_env: &Rc<AstEnv>,
        import_def: &ImportDef,
    ) -> Result<(), EvalError> {
        match &import_def.items {
            ImportItems::Module => {
                // Import the module as a namespace
                // Only include public bindings
                let module_name = if let Some(alias) = &import_def.alias {
                    alias.name.clone()
                } else {
                    import_def
                        .path
                        .last()
                        .map(|i| i.name.clone())
                        .unwrap_or_else(|| "module".to_string())
                };

                // Create a record with only public module bindings
                let bindings = module_env.public_bindings();
                let record = Value::Record(Rc::new(bindings));
                Rc::make_mut(&mut self.env).define(module_name, record);
            }
            ImportItems::Items(items) => {
                // Import specific items (must be public)
                for item in items {
                    let name = &item.name;
                    // Check if the item exists and is public
                    if !module_env.is_public(name) {
                        if module_env.get(name).is_some() {
                            return Err(EvalError::TypeError(format!(
                                "'{}' is private and cannot be imported",
                                name
                            )));
                        } else {
                            return Err(EvalError::TypeError(format!(
                                "module does not export '{}'",
                                name
                            )));
                        }
                    }
                    if let Some(value) = module_env.get(name) {
                        Rc::make_mut(&mut self.env).define(name.clone(), value);
                    }
                }
            }
            ImportItems::All => {
                // Import all public bindings
                for (name, value) in module_env.public_bindings() {
                    Rc::make_mut(&mut self.env).define(name, value);
                }
            }
        }
        Ok(())
    }

    /// Evaluate an expression.
    pub fn eval_expr(&mut self, expr: &Expr) -> Result<Value, EvalError> {
        match &expr.kind {
            ExprKind::Int(n) => Ok(Value::Int(*n)),
            ExprKind::Float(f) => Ok(Value::Float(*f)),
            ExprKind::String(s) => Ok(Value::String(Rc::new(s.clone()))),
            ExprKind::Char(c) => Ok(Value::Char(*c)),
            ExprKind::Bool(b) => Ok(Value::Bool(*b)),
            ExprKind::Unit => Ok(Value::Unit),

            ExprKind::Var(ident) => self
                .env
                .get(&ident.name)
                .ok_or_else(|| EvalError::TypeError(format!("undefined variable: {}", ident.name))),

            ExprKind::List(items) => {
                let values: Result<Vec<_>, _> = items.iter().map(|e| self.eval_expr(e)).collect();
                Ok(Value::List(Rc::new(values?)))
            }

            ExprKind::Tuple(items) => {
                let values: Result<Vec<_>, _> = items.iter().map(|e| self.eval_expr(e)).collect();
                Ok(Value::Tuple(Rc::new(values?)))
            }

            ExprKind::Record(fields) => {
                let mut map = HashMap::new();
                for field in fields {
                    let value = if let Some(ref v) = field.value {
                        self.eval_expr(v)?
                    } else {
                        // Shorthand: #{ x } means #{ x = x }
                        self.env.get(&field.name.name).ok_or_else(|| {
                            EvalError::TypeError(format!("undefined variable: {}", field.name.name))
                        })?
                    };
                    map.insert(field.name.name.clone(), value);
                }
                Ok(Value::Record(Rc::new(map)))
            }

            ExprKind::RecordUpdate { base, fields } => {
                let base_val = self.eval_expr(base)?;
                match base_val {
                    Value::Record(base_map) => {
                        let mut map: HashMap<String, Value> = (*base_map).clone();
                        for field in fields {
                            let value = if let Some(ref v) = field.value {
                                self.eval_expr(v)?
                            } else {
                                self.env.get(&field.name.name).ok_or_else(|| {
                                    EvalError::TypeError(format!(
                                        "undefined variable: {}",
                                        field.name.name
                                    ))
                                })?
                            };
                            map.insert(field.name.name.clone(), value);
                        }
                        Ok(Value::Record(Rc::new(map)))
                    }
                    _ => Err(EvalError::TypeError(
                        "record update requires a record".to_string(),
                    )),
                }
            }

            ExprKind::Lambda { params, body } => {
                let closure = AstClosure {
                    params: params
                        .iter()
                        .map(|p| Param {
                            pattern: p.pattern.clone(),
                            ty: p.ty.clone().unwrap_or(Type {
                                kind: TypeKind::Infer,
                                span: p.span,
                            }),
                            is_lazy: false,
                            span: p.span,
                        })
                        .collect(),
                    body: (**body).clone(),
                    env: self.env.clone(),
                };
                Ok(Value::AstClosure(Rc::new(closure)))
            }

            ExprKind::Call { func, args } => {
                let func_val = self.eval_expr(func)?;
                let arg_vals: Result<Vec<_>, _> = args.iter().map(|e| self.eval_expr(e)).collect();
                self.apply(func_val, arg_vals?)
            }

            ExprKind::MethodCall {
                receiver,
                method,
                args,
            } => {
                let recv_val = self.eval_expr(receiver)?;
                let mut all_args = vec![recv_val];
                for arg in args {
                    all_args.push(self.eval_expr(arg)?);
                }

                // Look up method as a function
                if let Some(func) = self.env.get(&method.name) {
                    self.apply(func, all_args)
                } else {
                    Err(EvalError::TypeError(format!(
                        "undefined method: {}",
                        method.name
                    )))
                }
            }

            ExprKind::Field { base, field } => {
                let base_val = self.eval_expr(base)?;
                match base_val {
                    Value::Record(fields) => fields.get(&field.name).cloned().ok_or_else(|| {
                        EvalError::TypeError(format!("no field '{}' in record", field.name))
                    }),
                    _ => Err(EvalError::TypeError(
                        "field access requires a record".to_string(),
                    )),
                }
            }

            ExprKind::TupleIndex { base, index } => {
                let base_val = self.eval_expr(base)?;
                match base_val {
                    Value::Tuple(items) => items.get(*index as usize).cloned().ok_or_else(|| {
                        EvalError::TypeError("tuple index out of bounds".to_string())
                    }),
                    _ => Err(EvalError::TypeError(
                        "tuple index requires a tuple".to_string(),
                    )),
                }
            }

            ExprKind::Index { base, index } => {
                let base_val = self.eval_expr(base)?;
                let index_val = self.eval_expr(index)?;
                match (&base_val, &index_val) {
                    (Value::List(items), Value::Int(i)) => {
                        items.get(*i as usize).cloned().ok_or_else(|| {
                            EvalError::TypeError("list index out of bounds".to_string())
                        })
                    }
                    (Value::String(s), Value::Int(i)) => {
                        s.chars().nth(*i as usize).map(Value::Char).ok_or_else(|| {
                            EvalError::TypeError("string index out of bounds".to_string())
                        })
                    }
                    _ => Err(EvalError::TypeError("invalid index operation".to_string())),
                }
            }

            ExprKind::Binary { op, left, right } => {
                let left_val = self.eval_expr(left)?;
                let right_val = self.eval_expr(right)?;
                self.eval_binary(*op, left_val, right_val)
            }

            ExprKind::Unary { op, operand } => {
                let val = self.eval_expr(operand)?;
                self.eval_unary(*op, val)
            }

            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond = self.eval_expr(condition)?;
                if cond.is_truthy() {
                    self.eval_expr(then_branch)
                } else {
                    self.eval_expr(else_branch)
                }
            }

            ExprKind::Match { scrutinee, arms } => {
                let val = self.eval_expr(scrutinee)?;
                for arm in arms {
                    if let Some(bindings) = Self::match_pattern(&arm.pattern, &val) {
                        // Create new scope with bindings
                        let mut new_env = AstEnv::child(self.env.clone());
                        for (name, value) in bindings {
                            new_env.define(name, value);
                        }

                        // Check guard
                        if let Some(guard) = &arm.guard {
                            let mut guard_eval = AstEvaluator::with_env(Rc::new(new_env.clone()));
                            let guard_val = guard_eval.eval_expr(guard)?;
                            if !guard_val.is_truthy() {
                                continue;
                            }
                        }

                        let mut body_eval = AstEvaluator::with_env(Rc::new(new_env));
                        return body_eval.eval_expr(&arm.body);
                    }
                }
                Err(EvalError::PatternMatchFailed)
            }

            ExprKind::Block { stmts, expr } => {
                let mut new_env = AstEnv::child(self.env.clone());

                for stmt in stmts {
                    match &stmt.kind {
                        StmtKind::Let { pattern, value, .. } => {
                            let mut stmt_eval = AstEvaluator::with_env(Rc::new(new_env.clone()));
                            let val = stmt_eval.eval_expr(value)?;
                            self.bind_pattern_to_env(pattern, val, &mut new_env)?;
                        }
                        StmtKind::Expr(e) => {
                            let mut stmt_eval = AstEvaluator::with_env(Rc::new(new_env.clone()));
                            stmt_eval.eval_expr(e)?;
                        }
                    }
                }

                if let Some(e) = expr {
                    let mut final_eval = AstEvaluator::with_env(Rc::new(new_env));
                    final_eval.eval_expr(e)
                } else {
                    Ok(Value::Unit)
                }
            }

            ExprKind::Coalesce { value, default } => {
                let val = self.eval_expr(value)?;
                match val {
                    Value::None => self.eval_expr(default),
                    Value::Some(v) => Ok((*v).clone()),
                    other => Ok(other),
                }
            }

            ExprKind::Try(inner) => {
                let val = self.eval_expr(inner)?;
                match val {
                    Value::Ok(v) => Ok((*v).clone()),
                    Value::Err(e) => Err(EvalError::TypeError(format!("{:?}", e))),
                    Value::Some(v) => Ok((*v).clone()),
                    Value::None => Err(EvalError::TypeError("unwrap on None".to_string())),
                    other => Ok(other),
                }
            }

            ExprKind::Path(parts) => {
                // Look up the first part, then traverse the rest as field accesses
                if parts.is_empty() {
                    return Err(EvalError::TypeError("empty path".to_string()));
                }

                let first = &parts[0];
                let mut value = self
                    .env
                    .get(&first.name)
                    .ok_or_else(|| EvalError::TypeError(format!("undefined: {}", first.name)))?;

                // Traverse remaining parts as field accesses
                for part in &parts[1..] {
                    match value {
                        Value::Record(ref fields) => {
                            value = fields.get(&part.name).cloned().ok_or_else(|| {
                                EvalError::TypeError(format!("no field '{}' in record", part.name))
                            })?;
                        }
                        _ => {
                            return Err(EvalError::TypeError(format!(
                                "cannot access field '{}' on non-record",
                                part.name
                            )));
                        }
                    }
                }

                Ok(value)
            }

            ExprKind::Interpolated(parts) => {
                let mut result = String::new();
                for part in parts {
                    match part {
                        StringPart::Literal(s) => result.push_str(s),
                        StringPart::Expr(e) => {
                            let val = self.eval_expr(e)?;
                            result.push_str(&Self::value_to_string(&val));
                        }
                    }
                }
                Ok(Value::String(Rc::new(result)))
            }

            ExprKind::Lazy(inner) => {
                // Create a thunk that captures the expression and current environment
                let thunk = Thunk::new((**inner).clone(), self.env.clone());
                Ok(Value::Thunk(thunk))
            }

            ExprKind::ListComp { body, generators } => {
                self.eval_list_comprehension(body, generators)
            }

            ExprKind::SafeField { base, field } => {
                let base_val = self.eval_expr(base)?;
                match base_val {
                    Value::None => Ok(Value::None),
                    Value::Some(inner) => match *inner {
                        Value::Record(ref fields) => match fields.get(&field.name) {
                            Some(v) => Ok(Value::Some(Box::new(v.clone()))),
                            None => Ok(Value::None),
                        },
                        _ => Err(EvalError::TypeError(
                            "safe field access requires a record".to_string(),
                        )),
                    },
                    Value::Record(fields) => match fields.get(&field.name) {
                        Some(v) => Ok(Value::Some(Box::new(v.clone()))),
                        None => Ok(Value::None),
                    },
                    _ => Err(EvalError::TypeError(
                        "safe field access requires an option or record".to_string(),
                    )),
                }
            }

            ExprKind::PathLit(path) => Ok(Value::String(Rc::new(path.clone()))),

            ExprKind::Let {
                pattern,
                value,
                body,
                ..
            } => {
                let val = self.eval_expr(value)?;
                let mut new_env = AstEnv::child(self.env.clone());
                self.bind_pattern_to_env(pattern, val, &mut new_env)?;
                let mut body_eval = AstEvaluator::with_env(Rc::new(new_env));
                if let Some(ref base) = self.base_path {
                    body_eval.base_path = Some(base.clone());
                }
                body_eval.eval_expr(body)
            }
        }
    }

    /// Evaluate a list comprehension.
    fn eval_list_comprehension(
        &mut self,
        body: &Expr,
        generators: &[Generator],
    ) -> Result<Value, EvalError> {
        let mut results = Vec::new();
        self.eval_generators(body, generators, 0, &mut results)?;
        Ok(Value::List(Rc::new(results)))
    }

    /// Recursively evaluate generators in a list comprehension.
    fn eval_generators(
        &mut self,
        body: &Expr,
        generators: &[Generator],
        index: usize,
        results: &mut Vec<Value>,
    ) -> Result<(), EvalError> {
        if index >= generators.len() {
            // All generators exhausted, evaluate the body
            let value = self.eval_expr(body)?;
            results.push(value);
            return Ok(());
        }

        let generator = &generators[index];
        let iter_val = self.eval_expr(&generator.iter)?;

        // Get the items to iterate over
        let items = match iter_val {
            Value::List(items) => items,
            _ => {
                return Err(EvalError::TypeError(
                    "generator requires a list".to_string(),
                ));
            }
        };

        // Iterate over each item
        for item in items.iter() {
            // Create new scope with the binding
            let mut new_env = AstEnv::child(self.env.clone());
            self.bind_pattern_to_env(&generator.pattern, item.clone(), &mut new_env)?;

            // Check guard condition if present
            if let Some(ref condition) = generator.condition {
                let mut cond_eval = AstEvaluator::with_env(Rc::new(new_env.clone()));
                if let Some(ref base) = self.base_path {
                    cond_eval.base_path = Some(base.clone());
                }
                let cond_val = cond_eval.eval_expr(condition)?;
                if !cond_val.is_truthy() {
                    continue;
                }
            }

            // Recursively process remaining generators
            let mut inner_eval = AstEvaluator::with_env(Rc::new(new_env));
            if let Some(ref base) = self.base_path {
                inner_eval.base_path = Some(base.clone());
            }
            inner_eval.eval_generators(body, generators, index + 1, results)?;
        }

        Ok(())
    }

    /// Force evaluation of a thunk (used by the `force` builtin).
    pub fn force_thunk(&mut self, thunk: &Thunk) -> Result<Value, EvalError> {
        // Check current state
        {
            let state = thunk.state();
            match &*state {
                ThunkState::Evaluated(v) => return Ok(v.clone()),
                ThunkState::Evaluating => {
                    return Err(EvalError::TypeError(
                        "infinite recursion in lazy evaluation".to_string(),
                    ));
                }
                ThunkState::Unevaluated { .. } => {
                    // Will evaluate below
                }
            }
        }

        // Extract expr and env, mark as evaluating
        let (expr, env) = {
            let mut state = thunk.state_mut();
            match std::mem::replace(&mut *state, ThunkState::Evaluating) {
                ThunkState::Unevaluated { expr, env } => (expr, env),
                _ => unreachable!(),
            }
        };

        // Evaluate the expression
        let mut eval = AstEvaluator::with_env(env);
        if let Some(ref base) = self.base_path {
            eval.base_path = Some(base.clone());
        }

        let result = eval.eval_expr(&expr);

        // Store the result (or restore on error)
        match result {
            Ok(value) => {
                let mut state = thunk.state_mut();
                *state = ThunkState::Evaluated(value.clone());
                Ok(value)
            }
            Err(e) => {
                // On error, we could restore the unevaluated state or leave it as error
                // For now, store an error indicator
                let mut state = thunk.state_mut();
                *state = ThunkState::Evaluated(Value::Err(Box::new(Value::String(Rc::new(
                    e.to_string(),
                )))));
                Err(e)
            }
        }
    }

    fn eval_binary(&self, op: BinOp, left: Value, right: Value) -> Result<Value, EvalError> {
        match op {
            BinOp::Add => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
                _ => Err(EvalError::TypeError("cannot add".to_string())),
            },
            BinOp::Sub => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
                _ => Err(EvalError::TypeError("cannot subtract".to_string())),
            },
            BinOp::Mul => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
                _ => Err(EvalError::TypeError("cannot multiply".to_string())),
            },
            BinOp::Div => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => {
                    if *b == 0 {
                        Err(EvalError::DivisionByZero)
                    } else {
                        Ok(Value::Int(a / b))
                    }
                }
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
                _ => Err(EvalError::TypeError("cannot divide".to_string())),
            },
            BinOp::Mod => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => {
                    if *b == 0 {
                        Err(EvalError::DivisionByZero)
                    } else {
                        Ok(Value::Int(a % b))
                    }
                }
                _ => Err(EvalError::TypeError("cannot modulo".to_string())),
            },
            BinOp::Pow => match (&left, &right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a.pow(*b as u32))),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.powf(*b))),
                _ => Err(EvalError::TypeError("cannot power".to_string())),
            },
            BinOp::Eq => Ok(Value::Bool(values_equal(&left, &right))),
            BinOp::Ne => Ok(Value::Bool(!values_equal(&left, &right))),
            BinOp::Lt => compare(&left, &right).map(|o| Value::Bool(o.is_lt())),
            BinOp::Le => compare(&left, &right).map(|o| Value::Bool(o.is_le())),
            BinOp::Gt => compare(&left, &right).map(|o| Value::Bool(o.is_gt())),
            BinOp::Ge => compare(&left, &right).map(|o| Value::Bool(o.is_ge())),
            BinOp::And => Ok(Value::Bool(left.is_truthy() && right.is_truthy())),
            BinOp::Or => Ok(Value::Bool(left.is_truthy() || right.is_truthy())),
            BinOp::Concat => match (&left, &right) {
                (Value::List(a), Value::List(b)) => {
                    let mut result: Vec<Value> = (*a).iter().cloned().collect();
                    result.extend((*b).iter().cloned());
                    Ok(Value::List(Rc::new(result)))
                }
                (Value::String(a), Value::String(b)) => {
                    Ok(Value::String(Rc::new(format!("{}{}", a, b))))
                }
                _ => Err(EvalError::TypeError("cannot concatenate".to_string())),
            },
            BinOp::Merge => match (&left, &right) {
                (Value::Record(a), Value::Record(b)) => {
                    let mut result: HashMap<String, Value> = (**a).clone();
                    for (k, v) in b.iter() {
                        result.insert(k.clone(), v.clone());
                    }
                    Ok(Value::Record(Rc::new(result)))
                }
                _ => Err(EvalError::TypeError("cannot merge".to_string())),
            },
            BinOp::Pipe => {
                // a |> f  =>  f(a)
                self.apply_immut(right, vec![left])
            }
        }
    }

    fn eval_unary(&self, op: UnaryOp, val: Value) -> Result<Value, EvalError> {
        match op {
            UnaryOp::Neg => match val {
                Value::Int(n) => Ok(Value::Int(-n)),
                Value::Float(f) => Ok(Value::Float(-f)),
                _ => Err(EvalError::TypeError("cannot negate".to_string())),
            },
            UnaryOp::Not => Ok(Value::Bool(!val.is_truthy())),
        }
    }

    fn apply(&mut self, func: Value, args: Vec<Value>) -> Result<Value, EvalError> {
        match func {
            Value::Builtin(builtin) => {
                // Special handling for builtins that need evaluator access
                match builtin.name {
                    "force" => {
                        if args.len() != 1 {
                            return Err(EvalError::WrongArity);
                        }
                        return self.force_value(&args[0]);
                    }
                    "map" => {
                        if args.len() != 2 {
                            return Err(EvalError::WrongArity);
                        }
                        return self.builtin_map(&args[0], &args[1]);
                    }
                    "filter" => {
                        if args.len() != 2 {
                            return Err(EvalError::WrongArity);
                        }
                        return self.builtin_filter(&args[0], &args[1]);
                    }
                    "all" => {
                        if args.len() != 2 {
                            return Err(EvalError::WrongArity);
                        }
                        return self.builtin_all(&args[0], &args[1]);
                    }
                    "any" => {
                        if args.len() != 2 {
                            return Err(EvalError::WrongArity);
                        }
                        return self.builtin_any(&args[0], &args[1]);
                    }
                    "foldl" => {
                        if args.len() != 3 {
                            return Err(EvalError::WrongArity);
                        }
                        return self.builtin_foldl(&args[0], &args[1], &args[2]);
                    }
                    "foldr" => {
                        if args.len() != 3 {
                            return Err(EvalError::WrongArity);
                        }
                        return self.builtin_foldr(&args[0], &args[1], &args[2]);
                    }
                    "genList" => {
                        if args.len() != 2 {
                            return Err(EvalError::WrongArity);
                        }
                        return self.builtin_gen_list(&args[0], &args[1]);
                    }
                    "mapAttrs" => {
                        if args.len() != 2 {
                            return Err(EvalError::WrongArity);
                        }
                        return self.builtin_map_attrs(&args[0], &args[1]);
                    }
                    "filterAttrs" => {
                        if args.len() != 2 {
                            return Err(EvalError::WrongArity);
                        }
                        return self.builtin_filter_attrs(&args[0], &args[1]);
                    }
                    "concatMap" => {
                        if args.len() != 2 {
                            return Err(EvalError::WrongArity);
                        }
                        return self.builtin_concat_map(&args[0], &args[1]);
                    }
                    "partition" => {
                        if args.len() != 2 {
                            return Err(EvalError::WrongArity);
                        }
                        return self.builtin_partition(&args[0], &args[1]);
                    }
                    "groupBy" => {
                        if args.len() != 2 {
                            return Err(EvalError::WrongArity);
                        }
                        return self.builtin_group_by(&args[0], &args[1]);
                    }
                    "sort" => {
                        if args.len() != 2 {
                            return Err(EvalError::WrongArity);
                        }
                        return self.builtin_sort(&args[0], &args[1]);
                    }
                    _ => {}
                }

                if args.len() != builtin.arity {
                    return Err(EvalError::WrongArity);
                }
                (builtin.func)(&args).map_err(EvalError::TypeError)
            }
            Value::BuiltinFn(name, func) => {
                // Special handling for force
                if name == "force" {
                    if args.len() != 1 {
                        return Err(EvalError::WrongArity);
                    }
                    return self.force_value(&args[0]);
                }
                func(args).map_err(EvalError::TypeError)
            }
            Value::AstClosure(closure) => {
                if args.len() != closure.params.len() {
                    return Err(EvalError::WrongArity);
                }

                // Use the current evaluator's environment as the parent,
                // which allows recursive calls to find the function
                let mut new_env = AstEnv::child(self.env.clone());
                for (param, arg) in closure.params.iter().zip(args) {
                    let name = pattern_name(&param.pattern);
                    new_env.define(name, arg);
                }

                let mut body_eval = AstEvaluator::with_env(Rc::new(new_env));
                body_eval.eval_expr(&closure.body)
            }
            _ => Err(EvalError::NotAFunction),
        }
    }

    /// Force evaluation of a value (handles both thunks and regular values).
    fn force_value(&mut self, value: &Value) -> Result<Value, EvalError> {
        match value {
            Value::Thunk(thunk) => self.force_thunk(thunk),
            other => Ok(other.clone()), // Non-thunks are returned as-is
        }
    }

    // ========================================================================
    // Higher-order builtin implementations
    // ========================================================================

    /// map(f, list) - Apply f to each element of list
    fn builtin_map(&mut self, func: &Value, list: &Value) -> Result<Value, EvalError> {
        let items = match list {
            Value::List(items) => items,
            _ => return Err(EvalError::TypeError("map expects a list".to_string())),
        };

        let mut results = Vec::with_capacity(items.len());
        for item in items.iter() {
            let result = self.apply(func.clone(), vec![item.clone()])?;
            results.push(result);
        }
        Ok(Value::List(Rc::new(results)))
    }

    /// filter(pred, list) - Keep elements where pred(elem) is true
    fn builtin_filter(&mut self, pred: &Value, list: &Value) -> Result<Value, EvalError> {
        let items = match list {
            Value::List(items) => items,
            _ => return Err(EvalError::TypeError("filter expects a list".to_string())),
        };

        let mut results = Vec::new();
        for item in items.iter() {
            let result = self.apply(pred.clone(), vec![item.clone()])?;
            if let Value::Bool(true) = result {
                results.push(item.clone());
            }
        }
        Ok(Value::List(Rc::new(results)))
    }

    /// all(pred, list) - True if pred(elem) is true for all elements
    fn builtin_all(&mut self, pred: &Value, list: &Value) -> Result<Value, EvalError> {
        let items = match list {
            Value::List(items) => items,
            _ => return Err(EvalError::TypeError("all expects a list".to_string())),
        };

        for item in items.iter() {
            let result = self.apply(pred.clone(), vec![item.clone()])?;
            if let Value::Bool(false) = result {
                return Ok(Value::Bool(false));
            }
        }
        Ok(Value::Bool(true))
    }

    /// any(pred, list) - True if pred(elem) is true for any element
    fn builtin_any(&mut self, pred: &Value, list: &Value) -> Result<Value, EvalError> {
        let items = match list {
            Value::List(items) => items,
            _ => return Err(EvalError::TypeError("any expects a list".to_string())),
        };

        for item in items.iter() {
            let result = self.apply(pred.clone(), vec![item.clone()])?;
            if let Value::Bool(true) = result {
                return Ok(Value::Bool(true));
            }
        }
        Ok(Value::Bool(false))
    }

    /// foldl(op, init, list) - Left fold: op(op(op(init, x1), x2), x3)...
    fn builtin_foldl(
        &mut self,
        op: &Value,
        init: &Value,
        list: &Value,
    ) -> Result<Value, EvalError> {
        let items = match list {
            Value::List(items) => items,
            _ => return Err(EvalError::TypeError("foldl expects a list".to_string())),
        };

        let mut acc = init.clone();
        for item in items.iter() {
            acc = self.apply(op.clone(), vec![acc, item.clone()])?;
        }
        Ok(acc)
    }

    /// foldr(op, init, list) - Right fold: op(x1, op(x2, op(x3, init)))
    fn builtin_foldr(
        &mut self,
        op: &Value,
        init: &Value,
        list: &Value,
    ) -> Result<Value, EvalError> {
        let items = match list {
            Value::List(items) => items,
            _ => return Err(EvalError::TypeError("foldr expects a list".to_string())),
        };

        let mut acc = init.clone();
        for item in items.iter().rev() {
            acc = self.apply(op.clone(), vec![item.clone(), acc])?;
        }
        Ok(acc)
    }

    /// genList(f, n) - Generate list [f(0), f(1), ..., f(n-1)]
    fn builtin_gen_list(&mut self, func: &Value, count: &Value) -> Result<Value, EvalError> {
        let n = match count {
            Value::Int(n) => *n,
            _ => {
                return Err(EvalError::TypeError(
                    "genList expects an integer count".to_string(),
                ));
            }
        };

        if n < 0 {
            return Err(EvalError::TypeError(
                "genList count must be non-negative".to_string(),
            ));
        }

        let mut results = Vec::with_capacity(n as usize);
        for i in 0..n {
            let result = self.apply(func.clone(), vec![Value::Int(i)])?;
            results.push(result);
        }
        Ok(Value::List(Rc::new(results)))
    }

    /// mapAttrs(f, attrs) - Apply f(name, value) to each attribute
    fn builtin_map_attrs(&mut self, func: &Value, attrs: &Value) -> Result<Value, EvalError> {
        let fields = match attrs {
            Value::Record(fields) => fields,
            _ => {
                return Err(EvalError::TypeError(
                    "mapAttrs expects a record".to_string(),
                ));
            }
        };

        let mut results = HashMap::new();
        for (name, value) in fields.iter() {
            let result = self.apply(
                func.clone(),
                vec![Value::String(Rc::new(name.clone())), value.clone()],
            )?;
            results.insert(name.clone(), result);
        }
        Ok(Value::Record(Rc::new(results)))
    }

    /// filterAttrs(pred, attrs) - Keep attrs where pred(name, value) is true
    fn builtin_filter_attrs(&mut self, pred: &Value, attrs: &Value) -> Result<Value, EvalError> {
        let fields = match attrs {
            Value::Record(fields) => fields,
            _ => {
                return Err(EvalError::TypeError(
                    "filterAttrs expects a record".to_string(),
                ));
            }
        };

        let mut results = HashMap::new();
        for (name, value) in fields.iter() {
            let result = self.apply(
                pred.clone(),
                vec![Value::String(Rc::new(name.clone())), value.clone()],
            )?;
            if let Value::Bool(true) = result {
                results.insert(name.clone(), value.clone());
            }
        }
        Ok(Value::Record(Rc::new(results)))
    }

    /// concatMap(f, list) - Map and flatten: concat(map(f, list))
    fn builtin_concat_map(&mut self, func: &Value, list: &Value) -> Result<Value, EvalError> {
        let items = match list {
            Value::List(items) => items,
            _ => return Err(EvalError::TypeError("concatMap expects a list".to_string())),
        };

        let mut results = Vec::new();
        for item in items.iter() {
            let result = self.apply(func.clone(), vec![item.clone()])?;
            match result {
                Value::List(inner) => results.extend(inner.iter().cloned()),
                _ => {
                    return Err(EvalError::TypeError(
                        "concatMap function must return a list".to_string(),
                    ));
                }
            }
        }
        Ok(Value::List(Rc::new(results)))
    }

    /// partition(pred, list) - Split into { right = [...], wrong = [...] }
    fn builtin_partition(&mut self, pred: &Value, list: &Value) -> Result<Value, EvalError> {
        let items = match list {
            Value::List(items) => items,
            _ => return Err(EvalError::TypeError("partition expects a list".to_string())),
        };

        let mut right = Vec::new();
        let mut wrong = Vec::new();
        for item in items.iter() {
            let result = self.apply(pred.clone(), vec![item.clone()])?;
            if let Value::Bool(true) = result {
                right.push(item.clone());
            } else {
                wrong.push(item.clone());
            }
        }

        let mut record = HashMap::new();
        record.insert("right".to_string(), Value::List(Rc::new(right)));
        record.insert("wrong".to_string(), Value::List(Rc::new(wrong)));
        Ok(Value::Record(Rc::new(record)))
    }

    /// groupBy(f, list) - Group elements by f(elem) result
    fn builtin_group_by(&mut self, func: &Value, list: &Value) -> Result<Value, EvalError> {
        let items = match list {
            Value::List(items) => items,
            _ => return Err(EvalError::TypeError("groupBy expects a list".to_string())),
        };

        let mut groups: HashMap<String, Vec<Value>> = HashMap::new();
        for item in items.iter() {
            let key = self.apply(func.clone(), vec![item.clone()])?;
            let key_str = match key {
                Value::String(s) => (*s).clone(),
                _ => {
                    return Err(EvalError::TypeError(
                        "groupBy function must return a string".to_string(),
                    ));
                }
            };
            groups.entry(key_str).or_default().push(item.clone());
        }

        let record: HashMap<String, Value> = groups
            .into_iter()
            .map(|(k, v)| (k, Value::List(Rc::new(v))))
            .collect();
        Ok(Value::Record(Rc::new(record)))
    }

    /// sort(cmp, list) - Sort list using cmp(a, b) returning bool (true if a < b)
    fn builtin_sort(&mut self, cmp: &Value, list: &Value) -> Result<Value, EvalError> {
        let items = match list {
            Value::List(items) => items,
            _ => return Err(EvalError::TypeError("sort expects a list".to_string())),
        };

        let mut vec: Vec<Value> = items.iter().cloned().collect();

        // Use a simple insertion sort to avoid the complexity of sort_by with mutable self
        for i in 1..vec.len() {
            let mut j = i;
            while j > 0 {
                let cmp_result =
                    self.apply(cmp.clone(), vec![vec[j - 1].clone(), vec[j].clone()])?;
                match cmp_result {
                    Value::Bool(true) => break, // a < b, so order is correct
                    Value::Bool(false) => {
                        vec.swap(j - 1, j);
                        j -= 1;
                    }
                    _ => {
                        return Err(EvalError::TypeError(
                            "sort comparator must return a boolean".to_string(),
                        ));
                    }
                }
            }
        }

        Ok(Value::List(Rc::new(vec)))
    }

    fn apply_immut(&self, func: Value, args: Vec<Value>) -> Result<Value, EvalError> {
        match func {
            Value::Builtin(builtin) => {
                if args.len() != builtin.arity {
                    return Err(EvalError::WrongArity);
                }
                (builtin.func)(&args).map_err(EvalError::TypeError)
            }
            Value::AstClosure(closure) => {
                if args.len() != closure.params.len() {
                    return Err(EvalError::WrongArity);
                }

                // For immutable apply, use the closure's captured environment
                let mut new_env = AstEnv::child(closure.env.clone());
                for (param, arg) in closure.params.iter().zip(args) {
                    let name = pattern_name(&param.pattern);
                    new_env.define(name, arg);
                }

                let mut body_eval = AstEvaluator::with_env(Rc::new(new_env));
                body_eval.eval_expr(&closure.body)
            }
            _ => Err(EvalError::NotAFunction),
        }
    }

    fn match_pattern(pattern: &Pattern, value: &Value) -> Option<Vec<(String, Value)>> {
        match &pattern.kind {
            PatternKind::Wildcard => Some(Vec::new()),
            PatternKind::Var(ident) => {
                if ident.name == "_" {
                    Some(Vec::new())
                } else {
                    Some(vec![(ident.name.clone(), value.clone())])
                }
            }
            PatternKind::Literal(lit) => {
                let matches = match (lit, value) {
                    (LiteralPattern::Int(a), Value::Int(b)) => a == b,
                    (LiteralPattern::Float(a), Value::Float(b)) => a == b,
                    (LiteralPattern::String(a), Value::String(b)) => a == b.as_str(),
                    (LiteralPattern::Char(a), Value::Char(b)) => a == b,
                    (LiteralPattern::Bool(a), Value::Bool(b)) => a == b,
                    _ => false,
                };
                if matches { Some(Vec::new()) } else { None }
            }
            PatternKind::Tuple(patterns) => {
                if let Value::Tuple(values) = value {
                    if patterns.len() != values.len() {
                        return None;
                    }
                    let mut bindings = Vec::new();
                    for (p, v) in patterns.iter().zip(values.iter()) {
                        bindings.extend(Self::match_pattern(p, v)?);
                    }
                    Some(bindings)
                } else {
                    None
                }
            }
            PatternKind::List(patterns) => {
                if let Value::List(values) = value {
                    if patterns.len() != values.len() {
                        return None;
                    }
                    let mut bindings = Vec::new();
                    for (p, v) in patterns.iter().zip(values.iter()) {
                        bindings.extend(Self::match_pattern(p, v)?);
                    }
                    Some(bindings)
                } else {
                    None
                }
            }
            PatternKind::Record { fields, rest: _ } => {
                if let Value::Record(record) = value {
                    let mut bindings = Vec::new();
                    for field in fields {
                        let val = record.get(&field.name.name)?;
                        if let Some(ref pat) = field.pattern {
                            bindings.extend(Self::match_pattern(pat, val)?);
                        } else {
                            bindings.push((field.name.name.clone(), val.clone()));
                        }
                    }
                    Some(bindings)
                } else {
                    None
                }
            }
            PatternKind::Constructor { path, args } => {
                let name = path.first().map(|i| i.name.as_str()).unwrap_or("");
                match (name, value, args.as_slice()) {
                    ("Some", Value::Some(v), [p]) => Self::match_pattern(p, v),
                    ("None", Value::None, []) => Some(Vec::new()),
                    ("Ok", Value::Ok(v), [p]) => Self::match_pattern(p, v),
                    ("Err", Value::Err(v), [p]) => Self::match_pattern(p, v),
                    _ => None,
                }
            }
            PatternKind::Or(patterns) => {
                for p in patterns {
                    if let Some(bindings) = Self::match_pattern(p, value) {
                        return Some(bindings);
                    }
                }
                None
            }
            PatternKind::Binding { name, pattern } => {
                let mut bindings = Self::match_pattern(pattern, value)?;
                bindings.push((name.name.clone(), value.clone()));
                Some(bindings)
            }
            _ => None,
        }
    }

    #[allow(dead_code)]
    fn bind_pattern(&mut self, pattern: &Pattern, value: Value) -> Result<(), EvalError> {
        self.bind_pattern_with_visibility(pattern, value, false)
    }

    fn bind_pattern_with_visibility(
        &mut self,
        pattern: &Pattern,
        value: Value,
        is_public: bool,
    ) -> Result<(), EvalError> {
        let bindings = Self::match_pattern(pattern, &value).ok_or(EvalError::PatternMatchFailed)?;
        for (name, val) in bindings {
            Rc::make_mut(&mut self.env).define_with_visibility(name, val, is_public);
        }
        Ok(())
    }

    fn bind_pattern_to_env(
        &self,
        pattern: &Pattern,
        value: Value,
        env: &mut AstEnv,
    ) -> Result<(), EvalError> {
        let bindings = Self::match_pattern(pattern, &value).ok_or(EvalError::PatternMatchFailed)?;
        for (name, val) in bindings {
            env.define(name, val);
        }
        Ok(())
    }

    /// Convert a value to its string representation for interpolation.
    fn value_to_string(value: &Value) -> String {
        match value {
            Value::Int(n) => n.to_string(),
            Value::Float(f) => {
                if f.fract() == 0.0 {
                    format!("{:.1}", f)
                } else {
                    f.to_string()
                }
            }
            Value::Bool(b) => b.to_string(),
            Value::Char(c) => c.to_string(),
            Value::String(s) => s.to_string(),
            Value::Unit => "()".to_string(),
            Value::None => "None".to_string(),
            Value::Some(v) => format!("Some({})", Self::value_to_string(v)),
            Value::Ok(v) => format!("Ok({})", Self::value_to_string(v)),
            Value::Err(v) => format!("Err({})", Self::value_to_string(v)),
            Value::List(items) => {
                let strs: Vec<String> = items.iter().map(Self::value_to_string).collect();
                format!("[{}]", strs.join(", "))
            }
            Value::Tuple(items) => {
                let strs: Vec<String> = items.iter().map(Self::value_to_string).collect();
                format!("({})", strs.join(", "))
            }
            Value::Record(fields) => {
                let strs: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{} = {}", k, Self::value_to_string(v)))
                    .collect();
                format!("#{{ {} }}", strs.join(", "))
            }
            Value::Map(map) => {
                let strs: Vec<String> = map
                    .iter()
                    .map(|(k, v)| format!("{} => {}", k, Self::value_to_string(v)))
                    .collect();
                format!("Map{{ {} }}", strs.join(", "))
            }
            Value::Set(set) => {
                let strs: Vec<String> = set.iter().cloned().collect();
                format!("Set{{ {} }}", strs.join(", "))
            }
            Value::Variant(tag, payload) => {
                if matches!(**payload, Value::Unit) {
                    tag.clone()
                } else {
                    format!("{}({})", tag, Self::value_to_string(payload))
                }
            }
            Value::Builtin(b) => format!("<builtin:{}>", b.name),
            Value::BuiltinFn(name, _) => format!("<builtin:{}>", name),
            Value::AstClosure(_) => "<function>".to_string(),
            Value::Closure { .. } => "<function>".to_string(),
            Value::Thunk(thunk) => match &*thunk.state() {
                ThunkState::Evaluated(v) => Self::value_to_string(v),
                ThunkState::Evaluating => "<thunk:evaluating>".to_string(),
                ThunkState::Unevaluated { .. } => "<thunk>".to_string(),
            },
        }
    }
}

impl Default for AstEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

/// Closure for AST evaluation.
#[derive(Clone)]
pub struct AstClosure {
    pub params: Vec<Param>,
    pub body: Expr,
    pub env: Rc<AstEnv>,
}

fn pattern_name(pattern: &Pattern) -> String {
    match &pattern.kind {
        PatternKind::Var(ident) => ident.name.clone(),
        _ => "_".to_string(),
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Char(x), Value::Char(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Unit, Value::Unit) => true,
        (Value::None, Value::None) => true,
        (Value::List(x), Value::List(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(a, b)| values_equal(a, b))
        }
        (Value::Tuple(x), Value::Tuple(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(a, b)| values_equal(a, b))
        }
        _ => false,
    }
}

fn compare(a: &Value, b: &Value) -> Result<std::cmp::Ordering, EvalError> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Ok(x.cmp(y)),
        (Value::Float(x), Value::Float(y)) => x
            .partial_cmp(y)
            .ok_or_else(|| EvalError::TypeError("cannot compare NaN".to_string())),
        (Value::String(x), Value::String(y)) => Ok(x.cmp(y)),
        (Value::Char(x), Value::Char(y)) => Ok(x.cmp(y)),
        _ => Err(EvalError::TypeError("cannot compare".to_string())),
    }
}
