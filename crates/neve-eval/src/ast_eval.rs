//! Direct AST evaluation (without HIR).
//!
//! This is a simplified evaluator that works directly on the AST.
//! It's useful for quick prototyping and REPL.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use neve_syntax::*;
use crate::value::Value;
use crate::builtin::builtins;
use crate::EvalError;

/// Environment for AST evaluation.
#[derive(Clone, Default)]
pub struct AstEnv {
    bindings: HashMap<String, Value>,
    parent: Option<Rc<AstEnv>>,
}

impl AstEnv {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtins() -> Self {
        let mut env = Self::new();
        // Load all builtins from the central registry
        for (name, value) in builtins() {
            env.bindings.insert(name.to_string(), value);
        }
        env
    }

    pub fn child(parent: Rc<AstEnv>) -> Self {
        Self {
            bindings: HashMap::new(),
            parent: Some(parent),
        }
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.bindings.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.bindings.get(name) {
            return Some(value.clone());
        }
        if let Some(parent) = &self.parent {
            return parent.get(name);
        }
        None
    }

    /// Get all bindings in this environment (not including parent).
    /// Used for module exports.
    pub fn all_bindings(&self) -> HashMap<String, Value> {
        self.bindings.clone()
    }
}

/// AST evaluator.
pub struct AstEvaluator {
    env: Rc<AstEnv>,
    /// Base path for resolving relative imports
    base_path: Option<PathBuf>,
    /// Cache of already-loaded modules
    loaded_modules: HashMap<PathBuf, Rc<AstEnv>>,
}

impl AstEvaluator {
    pub fn new() -> Self {
        Self {
            env: Rc::new(AstEnv::with_builtins()),
            base_path: None,
            loaded_modules: HashMap::new(),
        }
    }

    pub fn with_env(env: Rc<AstEnv>) -> Self {
        Self { 
            env,
            base_path: None,
            loaded_modules: HashMap::new(),
        }
    }

    pub fn with_base_path(mut self, path: PathBuf) -> Self {
        self.base_path = Some(path);
        self
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
            return Err(EvalError::TypeError(format!("parse error in {}", path.display())));
        }
        
        self.eval_file(&file)
    }

    fn eval_item(&mut self, item: &Item) -> Result<Value, EvalError> {
        match &item.kind {
            ItemKind::Let(let_def) => {
                let value = self.eval_expr(&let_def.value)?;
                self.bind_pattern(&let_def.pattern, value.clone())?;
                Ok(value)
            }
            ItemKind::Fn(fn_def) => {
                // For recursive functions, we need to define the function first,
                // then update the closure to capture the environment that includes itself.
                let name = fn_def.name.name.clone();
                
                // Create a placeholder closure first
                let func = AstClosure {
                    params: fn_def.params.clone(),
                    body: fn_def.body.clone(),
                    env: self.env.clone(), // Will be updated below
                };
                
                // Define the function in the environment
                Rc::make_mut(&mut self.env).define(name.clone(), Value::AstClosure(Rc::new(func)));
                
                // Now update the closure to have the environment that includes itself
                let recursive_func = AstClosure {
                    params: fn_def.params.clone(),
                    body: fn_def.body.clone(),
                    env: self.env.clone(), // Now includes the function itself
                };
                Rc::make_mut(&mut self.env).define(name, Value::AstClosure(Rc::new(recursive_func)));
                
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
        let module_path = self.resolve_module_path(&import_def.path)?;
        
        // Check if module is already loaded
        if let Some(module_env) = self.loaded_modules.get(&module_path).cloned() {
            // Import from cached module
            self.import_from_env(&module_env, import_def)?;
            return Ok(());
        }
        
        // Load the module
        let source = std::fs::read_to_string(&module_path)
            .map_err(|e| EvalError::TypeError(format!("cannot load module '{}': {}", 
                import_def.path.iter().map(|i| i.name.as_str()).collect::<Vec<_>>().join("."),
                e)))?;
        
        let (file, diagnostics) = neve_parser::parse(&source);
        
        if !diagnostics.is_empty() {
            return Err(EvalError::TypeError(format!("parse error in module '{}'",
                import_def.path.iter().map(|i| i.name.as_str()).collect::<Vec<_>>().join("."))));
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

    fn resolve_module_path(&self, path: &[Ident]) -> Result<PathBuf, EvalError> {
        let module_name: String = path.iter()
            .map(|i| i.name.as_str())
            .collect::<Vec<_>>()
            .join("/");
        
        // Try various locations
        let candidates = if let Some(base) = &self.base_path {
            vec![
                base.join(format!("{}.neve", module_name)),
                base.join(&module_name).join("mod.neve"),
            ]
        } else {
            vec![
                PathBuf::from(format!("{}.neve", module_name)),
                PathBuf::from(&module_name).join("mod.neve"),
            ]
        };
        
        for candidate in &candidates {
            if candidate.exists() {
                return Ok(candidate.clone());
            }
        }
        
        Err(EvalError::TypeError(format!(
            "cannot find module '{}' (tried: {})",
            path.iter().map(|i| i.name.as_str()).collect::<Vec<_>>().join("."),
            candidates.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")
        )))
    }

    fn import_from_env(&mut self, module_env: &Rc<AstEnv>, import_def: &ImportDef) -> Result<(), EvalError> {
        match &import_def.items {
            ImportItems::Module => {
                // Import the module as a namespace
                // For now, we'll import all public bindings with a prefix
                let module_name = if let Some(alias) = &import_def.alias {
                    alias.name.clone()
                } else {
                    import_def.path.last()
                        .map(|i| i.name.clone())
                        .unwrap_or_else(|| "module".to_string())
                };
                
                // For simplicity, we create a record with all module bindings
                let bindings = module_env.all_bindings();
                let record = Value::Record(Rc::new(bindings));
                Rc::make_mut(&mut self.env).define(module_name, record);
            }
            ImportItems::Items(items) => {
                // Import specific items
                for item in items {
                    let name = &item.name;
                    if let Some(value) = module_env.get(name) {
                        // No alias support for now (Vec<Ident> doesn't have aliases)
                        Rc::make_mut(&mut self.env).define(name.clone(), value);
                    } else {
                        return Err(EvalError::TypeError(format!(
                            "module does not export '{}'", name
                        )));
                    }
                }
            }
            ImportItems::All => {
                // Import all bindings
                for (name, value) in module_env.all_bindings() {
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

            ExprKind::Var(ident) => {
                self.env.get(&ident.name)
                    .ok_or_else(|| EvalError::TypeError(format!("undefined variable: {}", ident.name)))
            }

            ExprKind::List(items) => {
                let values: Result<Vec<_>, _> = items.iter()
                    .map(|e| self.eval_expr(e))
                    .collect();
                Ok(Value::List(Rc::new(values?)))
            }

            ExprKind::Tuple(items) => {
                let values: Result<Vec<_>, _> = items.iter()
                    .map(|e| self.eval_expr(e))
                    .collect();
                Ok(Value::Tuple(Rc::new(values?)))
            }

            ExprKind::Record(fields) => {
                let mut map = HashMap::new();
                for field in fields {
                    let value = if let Some(ref v) = field.value {
                        self.eval_expr(v)?
                    } else {
                        // Shorthand: #{ x } means #{ x = x }
                        self.env.get(&field.name.name)
                            .ok_or_else(|| EvalError::TypeError(format!("undefined variable: {}", field.name.name)))?
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
                                self.env.get(&field.name.name)
                                    .ok_or_else(|| EvalError::TypeError(format!("undefined variable: {}", field.name.name)))?
                            };
                            map.insert(field.name.name.clone(), value);
                        }
                        Ok(Value::Record(Rc::new(map)))
                    }
                    _ => Err(EvalError::TypeError("record update requires a record".to_string())),
                }
            }

            ExprKind::Lambda { params, body } => {
                let closure = AstClosure {
                    params: params.iter().map(|p| Param {
                        pattern: p.pattern.clone(),
                        ty: p.ty.clone().unwrap_or(Type {
                            kind: TypeKind::Infer,
                            span: p.span,
                        }),
                        is_lazy: false,
                        span: p.span,
                    }).collect(),
                    body: (**body).clone(),
                    env: self.env.clone(),
                };
                Ok(Value::AstClosure(Rc::new(closure)))
            }

            ExprKind::Call { func, args } => {
                let func_val = self.eval_expr(func)?;
                let arg_vals: Result<Vec<_>, _> = args.iter()
                    .map(|e| self.eval_expr(e))
                    .collect();
                self.apply(func_val, arg_vals?)
            }

            ExprKind::MethodCall { receiver, method, args } => {
                let recv_val = self.eval_expr(receiver)?;
                let mut all_args = vec![recv_val];
                for arg in args {
                    all_args.push(self.eval_expr(arg)?);
                }
                
                // Look up method as a function
                if let Some(func) = self.env.get(&method.name) {
                    self.apply(func, all_args)
                } else {
                    Err(EvalError::TypeError(format!("undefined method: {}", method.name)))
                }
            }

            ExprKind::Field { base, field } => {
                let base_val = self.eval_expr(base)?;
                match base_val {
                    Value::Record(fields) => {
                        fields.get(&field.name)
                            .cloned()
                            .ok_or_else(|| EvalError::TypeError(format!("no field '{}' in record", field.name)))
                    }
                    _ => Err(EvalError::TypeError("field access requires a record".to_string())),
                }
            }

            ExprKind::TupleIndex { base, index } => {
                let base_val = self.eval_expr(base)?;
                match base_val {
                    Value::Tuple(items) => {
                        items.get(*index as usize)
                            .cloned()
                            .ok_or_else(|| EvalError::TypeError("tuple index out of bounds".to_string()))
                    }
                    _ => Err(EvalError::TypeError("tuple index requires a tuple".to_string())),
                }
            }

            ExprKind::Index { base, index } => {
                let base_val = self.eval_expr(base)?;
                let index_val = self.eval_expr(index)?;
                match (&base_val, &index_val) {
                    (Value::List(items), Value::Int(i)) => {
                        items.get(*i as usize)
                            .cloned()
                            .ok_or_else(|| EvalError::TypeError("list index out of bounds".to_string()))
                    }
                    (Value::String(s), Value::Int(i)) => {
                        s.chars().nth(*i as usize)
                            .map(Value::Char)
                            .ok_or_else(|| EvalError::TypeError("string index out of bounds".to_string()))
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

            ExprKind::If { condition, then_branch, else_branch } => {
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
                let mut value = self.env.get(&first.name)
                    .ok_or_else(|| EvalError::TypeError(format!("undefined: {}", first.name)))?;
                
                // Traverse remaining parts as field accesses
                for part in &parts[1..] {
                    match value {
                        Value::Record(ref fields) => {
                            value = fields.get(&part.name)
                                .cloned()
                                .ok_or_else(|| EvalError::TypeError(format!("no field '{}' in record", part.name)))?;
                        }
                        _ => return Err(EvalError::TypeError(format!("cannot access field '{}' on non-record", part.name))),
                    }
                }
                
                Ok(value)
            }

            _ => Err(EvalError::TypeError("unsupported expression".to_string())),
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
                if args.len() != builtin.arity {
                    return Err(EvalError::WrongArity);
                }
                (builtin.func)(&args).map_err(EvalError::TypeError)
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

    fn match_pattern( pattern: &Pattern, value: &Value) -> Option<Vec<(String, Value)>> {
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

    fn bind_pattern(&mut self, pattern: &Pattern, value: Value) -> Result<(), EvalError> {
        let bindings = Self::match_pattern(pattern, &value)
            .ok_or(EvalError::PatternMatchFailed)?;
        for (name, val) in bindings {
            Rc::make_mut(&mut self.env).define(name, val);
        }
        Ok(())
    }

    fn bind_pattern_to_env(&self, pattern: &Pattern, value: Value, env: &mut AstEnv) -> Result<(), EvalError> {
        let bindings = Self::match_pattern(pattern, &value)
            .ok_or(EvalError::PatternMatchFailed)?;
        for (name, val) in bindings {
            env.define(name, val);
        }
        Ok(())
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
        (Value::Float(x), Value::Float(y)) => {
            x.partial_cmp(y).ok_or_else(|| EvalError::TypeError("cannot compare NaN".to_string()))
        }
        (Value::String(x), Value::String(y)) => Ok(x.cmp(y)),
        (Value::Char(x), Value::Char(y)) => Ok(x.cmp(y)),
        _ => Err(EvalError::TypeError("cannot compare".to_string())),
    }
}
