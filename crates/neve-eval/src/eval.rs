//! HIR expression evaluation.

use crate::{Environment, Value};
use neve_hir::{
    BinOp, DefId, Expr, ExprKind, FnDef, Item, ItemKind, Literal, LocalId, Module, UnaryOp,
};
use std::collections::HashMap;
use std::rc::Rc;
use thiserror::Error;

/// Evaluation errors.
#[derive(Debug, Error)]
pub enum EvalError {
    #[error("unbound variable")]
    UnboundVariable,

    #[error("type error: {0}")]
    TypeError(String),

    #[error("division by zero")]
    DivisionByZero,

    #[error("assertion failed: {0}")]
    AssertionFailed(String),

    #[error("pattern match failed")]
    PatternMatchFailed,

    #[error("not a function")]
    NotAFunction,

    #[error("wrong number of arguments")]
    WrongArity,
}

/// Result of evaluating an expression with tail call detection.
enum TcoResult {
    /// Normal value result
    Value(Value),
    /// Tail call detected: (function, arguments)
    TailCall(Value, Vec<Value>),
}

/// The HIR evaluator.
pub struct Evaluator {
    /// Local variable environment
    env: Environment,
    /// Global definitions (functions, etc.)
    globals: HashMap<DefId, GlobalDef>,
}

/// A global definition.
#[derive(Clone)]
enum GlobalDef {
    Function(FnDef),
    Value(Value),
}

impl Evaluator {
    /// Create a new evaluator.
    pub fn new() -> Self {
        Self {
            env: Environment::new(),
            globals: HashMap::new(),
        }
    }

    /// Create an evaluator with built-in functions.
    pub fn with_builtins() -> Self {
        let mut eval = Self::new();
        eval.define_builtins();
        eval
    }

    fn define_builtins(&mut self) {
        // We'll store builtins as special values that can be called
        // For now, they're handled specially in apply()
    }

    /// Evaluate a module and return the last value.
    pub fn eval_module(&mut self, module: &Module) -> Result<Value, EvalError> {
        // First pass: collect all global definitions
        for item in &module.items {
            self.collect_item(item);
        }

        // Second pass: evaluate definitions (for values) and return last result
        let mut result = Value::Unit;
        for item in &module.items {
            result = self.eval_item(item)?;
        }

        Ok(result)
    }

    fn collect_item(&mut self, item: &Item) {
        if let ItemKind::Fn(fn_def) = &item.kind {
            self.globals
                .insert(item.id, GlobalDef::Function(fn_def.clone()));
        }
    }

    fn eval_item(&mut self, item: &Item) -> Result<Value, EvalError> {
        match &item.kind {
            ItemKind::Fn(fn_def) => {
                // For top-level let (converted to zero-param function), evaluate immediately
                if fn_def.params.is_empty() {
                    let value = self.eval(&fn_def.body)?;
                    self.globals
                        .insert(item.id, GlobalDef::Value(value.clone()));
                    Ok(value)
                } else {
                    // For real functions, they're already collected
                    Ok(Value::Unit)
                }
            }
            _ => Ok(Value::Unit),
        }
    }

    /// Evaluate an expression.
    pub fn eval(&mut self, expr: &Expr) -> Result<Value, EvalError> {
        match &expr.kind {
            ExprKind::Literal(lit) => Ok(self.eval_literal(lit)),

            ExprKind::Var(id) => self.env.get(*id).ok_or(EvalError::UnboundVariable),

            ExprKind::Global(def_id) => {
                match self.globals.get(def_id).cloned() {
                    Some(GlobalDef::Value(v)) => Ok(v),
                    Some(GlobalDef::Function(fn_def)) => {
                        // Return a closure value
                        Ok(Value::Closure {
                            params: fn_def.params.clone(),
                            body: fn_def.body.clone(),
                            env: self.env.clone(),
                        })
                    }
                    None => {
                        // Check if it's a builtin
                        self.get_builtin(*def_id).ok_or(EvalError::UnboundVariable)
                    }
                }
            }

            ExprKind::Record(fields) => {
                let mut map = HashMap::new();
                for (name, expr) in fields {
                    map.insert(name.clone(), self.eval(expr)?);
                }
                Ok(Value::Record(Rc::new(map)))
            }

            ExprKind::List(items) => {
                let values: Result<Vec<_>, _> = items.iter().map(|e| self.eval(e)).collect();
                Ok(Value::List(Rc::new(values?)))
            }

            ExprKind::Tuple(items) => {
                let values: Result<Vec<_>, _> = items.iter().map(|e| self.eval(e)).collect();
                Ok(Value::Tuple(Rc::new(values?)))
            }

            ExprKind::Lambda(params, body) => Ok(Value::Closure {
                params: params.clone(),
                body: (**body).clone(),
                env: self.env.clone(),
            }),

            ExprKind::Call(func, args) => {
                let func_val = self.eval(func)?;
                let arg_vals: Result<Vec<_>, _> = args.iter().map(|e| self.eval(e)).collect();
                self.apply(func_val, arg_vals?)
            }

            ExprKind::Field(base, field) => {
                let base_val = self.eval(base)?;
                match base_val {
                    Value::Record(fields) => fields
                        .get(field)
                        .cloned()
                        .ok_or_else(|| EvalError::TypeError(format!("no field '{}'", field))),
                    _ => Err(EvalError::TypeError("not a record".to_string())),
                }
            }

            ExprKind::TupleIndex(base, index) => {
                let base_val = self.eval(base)?;
                match base_val {
                    Value::Tuple(items) => items.get(*index as usize).cloned().ok_or_else(|| {
                        EvalError::TypeError("tuple index out of bounds".to_string())
                    }),
                    _ => Err(EvalError::TypeError("not a tuple".to_string())),
                }
            }

            ExprKind::Binary(op, left, right) => {
                let left_val = self.eval(left)?;
                let right_val = self.eval(right)?;
                self.eval_binary(*op, left_val, right_val)
            }

            ExprKind::Unary(op, operand) => {
                let val = self.eval(operand)?;
                self.eval_unary(*op, val)
            }

            ExprKind::If(cond, then_branch, else_branch) => {
                let cond_val = self.eval(cond)?;
                if cond_val.is_truthy() {
                    self.eval(then_branch)
                } else {
                    self.eval(else_branch)
                }
            }

            ExprKind::Block(stmts, expr) => {
                let old_env = self.env.clone();
                self.env = self.env.child();

                for stmt in stmts {
                    match &stmt.kind {
                        neve_hir::StmtKind::Let(id, _, _, value) => {
                            let val = self.eval(value)?;
                            self.env.define(*id, val);
                        }
                        neve_hir::StmtKind::Expr(e) => {
                            self.eval(e)?;
                        }
                    }
                }

                let result = if let Some(e) = expr {
                    self.eval(e)?
                } else {
                    Value::Unit
                };

                self.env = old_env;
                Ok(result)
            }

            ExprKind::Match(scrutinee, arms) => {
                let val = self.eval(scrutinee)?;
                for arm in arms {
                    if let Some(bindings) = self.match_pattern(&arm.pattern, &val) {
                        let old_env = self.env.clone();
                        self.env = self.env.child();

                        for (id, value) in bindings {
                            self.env.define(id, value);
                        }

                        // Check guard if present
                        if let Some(guard) = &arm.guard {
                            let guard_val = self.eval(guard)?;
                            if !guard_val.is_truthy() {
                                self.env = old_env;
                                continue;
                            }
                        }

                        let result = self.eval(&arm.body)?;
                        self.env = old_env;
                        return Ok(result);
                    }
                }
                Err(EvalError::PatternMatchFailed)
            }

            ExprKind::Interpolated(parts) => {
                let mut result = String::new();
                for part in parts {
                    match part {
                        neve_hir::StringPart::Literal(s) => result.push_str(s),
                        neve_hir::StringPart::Expr(e) => {
                            let val = self.eval(e)?;
                            result.push_str(&Self::value_to_string(&val));
                        }
                    }
                }
                Ok(Value::String(Rc::new(result)))
            }
        }
    }

    fn get_builtin(&self, _def_id: DefId) -> Option<Value> {
        // Builtins are currently handled through AstEvaluator
        // TODO: Implement HIR-level builtins
        None
    }

    fn eval_literal(&self, lit: &Literal) -> Value {
        match lit {
            Literal::Int(n) => Value::Int(*n),
            Literal::Float(f) => Value::Float(*f),
            Literal::String(s) => Value::String(Rc::new(s.clone())),
            Literal::Char(c) => Value::Char(*c),
            Literal::Bool(b) => Value::Bool(*b),
            Literal::Unit => Value::Unit,
        }
    }

    fn eval_binary(&mut self, op: BinOp, left: Value, right: Value) -> Result<Value, EvalError> {
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
            BinOp::Eq => Ok(Value::Bool(Self::values_equal(&left, &right))),
            BinOp::Ne => Ok(Value::Bool(!Self::values_equal(&left, &right))),
            BinOp::Lt => self.compare(&left, &right).map(|o| Value::Bool(o.is_lt())),
            BinOp::Le => self.compare(&left, &right).map(|o| Value::Bool(o.is_le())),
            BinOp::Gt => self.compare(&left, &right).map(|o| Value::Bool(o.is_gt())),
            BinOp::Ge => self.compare(&left, &right).map(|o| Value::Bool(o.is_ge())),
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
                    let mut result: HashMap<String, Value> =
                        (*a).iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                    for (k, v) in b.iter() {
                        result.insert(k.clone(), v.clone());
                    }
                    Ok(Value::Record(Rc::new(result)))
                }
                _ => Err(EvalError::TypeError("cannot merge".to_string())),
            },
            BinOp::Pipe => {
                // a |> f  =>  f(a)
                self.apply(right, vec![left])
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
                x.len() == y.len()
                    && x.iter()
                        .zip(y.iter())
                        .all(|(a, b)| Self::values_equal(a, b))
            }
            (Value::Tuple(x), Value::Tuple(y)) => {
                x.len() == y.len()
                    && x.iter()
                        .zip(y.iter())
                        .all(|(a, b)| Self::values_equal(a, b))
            }
            _ => false,
        }
    }

    fn compare(&self, a: &Value, b: &Value) -> Result<std::cmp::Ordering, EvalError> {
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

    fn apply(&mut self, func: Value, args: Vec<Value>) -> Result<Value, EvalError> {
        // Tail call optimization: use iteration instead of recursion
        let mut current_func = func;
        let mut current_args = args;

        loop {
            match current_func {
                Value::Closure { params, body, env } => {
                    if current_args.len() != params.len() {
                        return Err(EvalError::WrongArity);
                    }

                    // Set up environment for function execution
                    let old_env = self.env.clone();
                    self.env = env.child();

                    for (param, arg) in params.iter().zip(current_args) {
                        self.env.define(param.id, arg);
                    }

                    // Evaluate the body and check if result is a tail call
                    match self.eval_with_tco(&body)? {
                        TcoResult::Value(v) => {
                            self.env = old_env;
                            return Ok(v);
                        }
                        TcoResult::TailCall(func, args) => {
                            // Tail call detected - loop instead of recurring
                            self.env = old_env;
                            current_func = func;
                            current_args = args;
                            continue;
                        }
                    }
                }
                Value::Builtin(builtin) => {
                    if current_args.len() != builtin.arity {
                        return Err(EvalError::WrongArity);
                    }
                    return (builtin.func)(&current_args).map_err(EvalError::TypeError);
                }
                Value::AstClosure(_) => {
                    // AstClosure not supported in HIR evaluator
                    return Err(EvalError::TypeError(
                        "AstClosure not supported in HIR evaluator".to_string(),
                    ));
                }
                _ => return Err(EvalError::NotAFunction),
            }
        }
    }

    /// Evaluate expression and detect tail calls.
    fn eval_with_tco(&mut self, expr: &Expr) -> Result<TcoResult, EvalError> {
        match &expr.kind {
            // Direct call in tail position
            ExprKind::Call(func, args) => {
                let func_val = self.eval(func)?;
                let arg_vals: Result<Vec<_>, _> = args.iter().map(|e| self.eval(e)).collect();
                Ok(TcoResult::TailCall(func_val, arg_vals?))
            }

            // If-then-else: evaluate condition, then the appropriate branch with TCO
            ExprKind::If(cond, then_branch, else_branch) => {
                let cond_val = self.eval(cond)?;
                match cond_val {
                    Value::Bool(true) => self.eval_with_tco(then_branch),
                    Value::Bool(false) => self.eval_with_tco(else_branch),
                    _ => Err(EvalError::TypeError(
                        "condition must be boolean".to_string(),
                    )),
                }
            }

            // Block: evaluate statements, then final expression with TCO
            ExprKind::Block(stmts, final_expr) => {
                for stmt in stmts {
                    self.eval_stmt(stmt)?;
                }

                if let Some(expr) = final_expr {
                    self.eval_with_tco(expr)
                } else {
                    Ok(TcoResult::Value(Value::Unit))
                }
            }

            // Match: evaluate scrutinee, match pattern, then evaluate arm with TCO
            ExprKind::Match(scrutinee, arms) => {
                let scrutinee_val = self.eval(scrutinee)?;

                for arm in arms {
                    if let Some(bindings) = self.match_pattern(&arm.pattern, &scrutinee_val) {
                        // Check guard if present
                        if let Some(guard) = &arm.guard {
                            let old_env = self.env.clone();
                            for (id, val) in bindings {
                                self.env.define(id, val);
                            }
                            let guard_val = self.eval(guard)?;
                            self.env = old_env;

                            if guard_val != Value::Bool(true) {
                                continue;
                            }
                        } else {
                            // No guard, just bind variables
                            for (id, val) in bindings {
                                self.env.define(id, val);
                            }
                        }

                        // Evaluate the arm body with TCO
                        return self.eval_with_tco(&arm.body);
                    }
                }

                Err(EvalError::PatternMatchFailed)
            }

            // All other expressions are not in tail position - evaluate normally
            _ => {
                let val = self.eval(expr)?;
                Ok(TcoResult::Value(val))
            }
        }
    }

    fn eval_stmt(&mut self, stmt: &neve_hir::Stmt) -> Result<(), EvalError> {
        match &stmt.kind {
            neve_hir::StmtKind::Let(id, _name, _ty, expr) => {
                let val = self.eval(expr)?;
                self.env.define(*id, val);
                Ok(())
            }
            neve_hir::StmtKind::Expr(expr) => {
                self.eval(expr)?;
                Ok(())
            }
        }
    }

    fn match_pattern(
        &self,
        pattern: &neve_hir::Pattern,
        value: &Value,
    ) -> Option<Vec<(LocalId, Value)>> {
        use neve_hir::PatternKind;

        match &pattern.kind {
            PatternKind::Wildcard => Some(Vec::new()),
            PatternKind::Var(id, _) => Some(vec![(*id, value.clone())]),
            PatternKind::Literal(lit) => {
                let lit_val = self.eval_literal(lit);
                if Self::values_equal(&lit_val, value) {
                    Some(Vec::new())
                } else {
                    None
                }
            }
            PatternKind::Tuple(patterns) => {
                if let Value::Tuple(values) = value {
                    if patterns.len() != values.len() {
                        return None;
                    }
                    let mut bindings = Vec::new();
                    for (p, v) in patterns.iter().zip(values.iter()) {
                        bindings.extend(self.match_pattern(p, v)?);
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
                        bindings.extend(self.match_pattern(p, v)?);
                    }
                    Some(bindings)
                } else {
                    None
                }
            }
            PatternKind::Record(fields) => {
                if let Value::Record(record) = value {
                    let mut bindings = Vec::new();
                    for (name, pat) in fields {
                        let val = record.get(name)?;
                        bindings.extend(self.match_pattern(pat, val)?);
                    }
                    Some(bindings)
                } else {
                    None
                }
            }
            PatternKind::Constructor(_, patterns) => {
                // Match Option/Result constructors
                match (patterns.as_slice(), value) {
                    ([p], Value::Some(v)) => self.match_pattern(p, v),
                    ([], Value::None) => Some(Vec::new()),
                    ([p], Value::Ok(v)) => self.match_pattern(p, v),
                    ([p], Value::Err(v)) => self.match_pattern(p, v),
                    _ => None,
                }
            }
        }
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
            Value::Thunk(thunk) => {
                use crate::value::ThunkState;
                match &*thunk.state() {
                    ThunkState::Evaluated(v) => Self::value_to_string(v),
                    ThunkState::Evaluating => "<thunk:evaluating>".to_string(),
                    ThunkState::Unevaluated { .. } => "<thunk>".to_string(),
                }
            }
        }
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neve_common::Span;
    use neve_hir::*;

    #[test]
    fn test_tail_call_optimization() {
        // This test verifies that TCO prevents stack overflow on deep recursion
        // We create a tail-recursive sum function: sum(n, acc) = if n == 0 then acc else sum(n-1, acc+n)

        let mut evaluator = Evaluator::new();

        // Create a simple tail-recursive function manually in HIR
        // fn sum(n: Int, acc: Int) -> Int = if n <= 0 then acc else sum(n - 1, acc + n)

        let span = Span::default();
        let n_id = LocalId(0);
        let acc_id = LocalId(1);
        let sum_def_id = DefId(0);

        // Build: if n <= 0 then acc else sum(n - 1, acc + n)
        let condition = Expr {
            kind: ExprKind::Binary(
                BinOp::Le,
                Box::new(Expr {
                    kind: ExprKind::Var(n_id),
                    ty: Ty {
                        kind: TyKind::Int,
                        span,
                    },
                    span,
                }),
                Box::new(Expr {
                    kind: ExprKind::Literal(Literal::Int(0)),
                    ty: Ty {
                        kind: TyKind::Int,
                        span,
                    },
                    span,
                }),
            ),
            ty: Ty {
                kind: TyKind::Bool,
                span,
            },
            span,
        };

        let then_branch = Expr {
            kind: ExprKind::Var(acc_id),
            ty: Ty {
                kind: TyKind::Int,
                span,
            },
            span,
        };

        // sum(n - 1, acc + n)
        let recursive_call = Expr {
            kind: ExprKind::Call(
                Box::new(Expr {
                    kind: ExprKind::Global(sum_def_id),
                    ty: Ty {
                        kind: TyKind::Int,
                        span,
                    },
                    span,
                }),
                vec![
                    Expr {
                        kind: ExprKind::Binary(
                            BinOp::Sub,
                            Box::new(Expr {
                                kind: ExprKind::Var(n_id),
                                ty: Ty {
                                    kind: TyKind::Int,
                                    span,
                                },
                                span,
                            }),
                            Box::new(Expr {
                                kind: ExprKind::Literal(Literal::Int(1)),
                                ty: Ty {
                                    kind: TyKind::Int,
                                    span,
                                },
                                span,
                            }),
                        ),
                        ty: Ty {
                            kind: TyKind::Int,
                            span,
                        },
                        span,
                    },
                    Expr {
                        kind: ExprKind::Binary(
                            BinOp::Add,
                            Box::new(Expr {
                                kind: ExprKind::Var(acc_id),
                                ty: Ty {
                                    kind: TyKind::Int,
                                    span,
                                },
                                span,
                            }),
                            Box::new(Expr {
                                kind: ExprKind::Var(n_id),
                                ty: Ty {
                                    kind: TyKind::Int,
                                    span,
                                },
                                span,
                            }),
                        ),
                        ty: Ty {
                            kind: TyKind::Int,
                            span,
                        },
                        span,
                    },
                ],
            ),
            ty: Ty {
                kind: TyKind::Int,
                span,
            },
            span,
        };

        let body = Expr {
            kind: ExprKind::If(
                Box::new(condition),
                Box::new(then_branch),
                Box::new(recursive_call),
            ),
            ty: Ty {
                kind: TyKind::Int,
                span,
            },
            span,
        };

        let fn_def = FnDef {
            name: "sum".to_string(),
            generics: vec![],
            params: vec![
                Param {
                    id: n_id,
                    name: "n".to_string(),
                    ty: Ty {
                        kind: TyKind::Int,
                        span,
                    },
                    span,
                },
                Param {
                    id: acc_id,
                    name: "acc".to_string(),
                    ty: Ty {
                        kind: TyKind::Int,
                        span,
                    },
                    span,
                },
            ],
            return_ty: Ty {
                kind: TyKind::Int,
                span,
            },
            body,
        };

        evaluator
            .globals
            .insert(sum_def_id, GlobalDef::Function(fn_def.clone()));

        // Call sum(100, 0) - should compute 1+2+...+100 = 5050
        let closure = Value::Closure {
            params: fn_def.params.clone(),
            body: fn_def.body.clone(),
            env: Environment::new(),
        };

        let result = evaluator.apply(closure, vec![Value::Int(100), Value::Int(0)]);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Value::Int(5050));
    }
}
