//! Type unification.

use neve_hir::{Ty, TyKind};
use std::collections::HashMap;

/// Substitution mapping type variables to types.
pub struct Substitution {
    /// Type variable bindings
    map: HashMap<u32, Ty>,
    /// Generic parameter bindings (index -> concrete type)
    params: HashMap<u32, Ty>,
}

impl Substitution {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            params: HashMap::new(),
        }
    }

    /// Apply substitution to a type.
    pub fn apply(&self, ty: &Ty) -> Ty {
        match &ty.kind {
            TyKind::Var(v) => {
                if let Some(t) = self.map.get(v) {
                    self.apply(t)
                } else {
                    ty.clone()
                }
            }
            TyKind::Param(idx, _name) => {
                if let Some(t) = self.params.get(idx) {
                    self.apply(t)
                } else {
                    ty.clone()
                }
            }
            TyKind::Fn(params, ret) => Ty {
                kind: TyKind::Fn(
                    params.iter().map(|t| self.apply(t)).collect(),
                    Box::new(self.apply(ret)),
                ),
                span: ty.span,
            },
            TyKind::Tuple(elems) => Ty {
                kind: TyKind::Tuple(elems.iter().map(|t| self.apply(t)).collect()),
                span: ty.span,
            },
            TyKind::Named(id, args) => Ty {
                kind: TyKind::Named(*id, args.iter().map(|t| self.apply(t)).collect()),
                span: ty.span,
            },
            TyKind::Record(fields) => Ty {
                kind: TyKind::Record(
                    fields
                        .iter()
                        .map(|(n, t)| (n.clone(), self.apply(t)))
                        .collect(),
                ),
                span: ty.span,
            },
            TyKind::Forall(params, body) => Ty {
                kind: TyKind::Forall(params.clone(), Box::new(self.apply(body))),
                span: ty.span,
            },
            _ => ty.clone(),
        }
    }

    /// Extend substitution with a new type variable binding.
    pub fn extend(&mut self, var: u32, ty: Ty) {
        self.map.insert(var, ty);
    }

    /// Bind a generic parameter to a concrete type.
    pub fn bind_param(&mut self, idx: u32, ty: Ty) {
        self.params.insert(idx, ty);
    }

    /// Get a bound type variable.
    pub fn get(&self, var: u32) -> Option<&Ty> {
        self.map.get(&var)
    }

    /// Get a bound generic parameter.
    pub fn get_param(&self, idx: u32) -> Option<&Ty> {
        self.params.get(&idx)
    }
}

impl Default for Substitution {
    fn default() -> Self {
        Self::new()
    }
}

/// Unify two types, returning an error message if they don't match.
pub fn unify(t1: &Ty, t2: &Ty, subst: &mut Substitution) -> Result<(), String> {
    let t1 = subst.apply(t1);
    let t2 = subst.apply(t2);

    match (&t1.kind, &t2.kind) {
        // Type variables
        (TyKind::Var(v1), TyKind::Var(v2)) if v1 == v2 => Ok(()),
        (TyKind::Var(v), _) => {
            if occurs_check(*v, &t2) {
                Err("infinite type".to_string())
            } else {
                subst.extend(*v, t2);
                Ok(())
            }
        }
        (_, TyKind::Var(v)) => {
            if occurs_check(*v, &t1) {
                Err("infinite type".to_string())
            } else {
                subst.extend(*v, t1);
                Ok(())
            }
        }

        // Generic type parameters
        (TyKind::Param(idx1, _), TyKind::Param(idx2, _)) if idx1 == idx2 => Ok(()),
        (TyKind::Param(idx, _), _) => {
            // Bind the generic parameter to the concrete type
            subst.bind_param(*idx, t2.clone());
            Ok(())
        }
        (_, TyKind::Param(idx, _)) => {
            subst.bind_param(*idx, t1.clone());
            Ok(())
        }

        // Primitive types
        (TyKind::Int, TyKind::Int) => Ok(()),
        (TyKind::Float, TyKind::Float) => Ok(()),
        (TyKind::Bool, TyKind::Bool) => Ok(()),
        (TyKind::Char, TyKind::Char) => Ok(()),
        (TyKind::String, TyKind::String) => Ok(()),
        (TyKind::Unit, TyKind::Unit) => Ok(()),

        // Function types
        (TyKind::Fn(p1, r1), TyKind::Fn(p2, r2)) => {
            if p1.len() != p2.len() {
                return Err("function arity mismatch".to_string());
            }
            for (a, b) in p1.iter().zip(p2.iter()) {
                unify(a, b, subst)?;
            }
            unify(r1, r2, subst)
        }

        // Tuple types
        (TyKind::Tuple(e1), TyKind::Tuple(e2)) => {
            if e1.len() != e2.len() {
                return Err("tuple length mismatch".to_string());
            }
            for (a, b) in e1.iter().zip(e2.iter()) {
                unify(a, b, subst)?;
            }
            Ok(())
        }

        // Named types with type arguments
        (TyKind::Named(id1, args1), TyKind::Named(id2, args2)) if id1 == id2 => {
            if args1.len() != args2.len() {
                return Err("type argument count mismatch".to_string());
            }
            for (a, b) in args1.iter().zip(args2.iter()) {
                unify(a, b, subst)?;
            }
            Ok(())
        }

        // Record types (structural)
        (TyKind::Record(f1), TyKind::Record(f2)) => {
            if f1.len() != f2.len() {
                return Err("record field count mismatch".to_string());
            }
            for ((n1, t1), (n2, t2)) in f1.iter().zip(f2.iter()) {
                if n1 != n2 {
                    return Err(format!("record field name mismatch: {} vs {}", n1, n2));
                }
                unify(t1, t2, subst)?;
            }
            Ok(())
        }

        // Forall types (polymorphic)
        (TyKind::Forall(params1, body1), TyKind::Forall(params2, body2)) => {
            if params1.len() != params2.len() {
                return Err("forall parameter count mismatch".to_string());
            }
            // Unify the bodies (parameters are already bound)
            unify(body1, body2, subst)
        }

        // Unknown types match anything (placeholder)
        (TyKind::Unknown, _) | (_, TyKind::Unknown) => Ok(()),

        _ => Err(format!("type mismatch: {:?} vs {:?}", t1.kind, t2.kind)),
    }
}

/// Check if a type variable occurs in a type (for infinite type prevention).
fn occurs_check(var: u32, ty: &Ty) -> bool {
    match &ty.kind {
        TyKind::Var(v) => *v == var,
        TyKind::Param(_, _) => false,
        TyKind::Fn(params, ret) => {
            params.iter().any(|t| occurs_check(var, t)) || occurs_check(var, ret)
        }
        TyKind::Tuple(elems) => elems.iter().any(|t| occurs_check(var, t)),
        TyKind::Named(_, args) => args.iter().any(|t| occurs_check(var, t)),
        TyKind::Record(fields) => fields.iter().any(|(_, t)| occurs_check(var, t)),
        TyKind::Forall(_, body) => occurs_check(var, body),
        _ => false,
    }
}

/// Instantiate a polymorphic type by replacing type parameters with fresh variables.
pub fn instantiate(ty: &Ty, fresh_var: &mut impl FnMut() -> Ty) -> Ty {
    match &ty.kind {
        TyKind::Forall(params, body) => {
            let mut subst = Substitution::new();
            for (idx, _name) in params.iter().enumerate() {
                subst.bind_param(idx as u32, fresh_var());
            }
            subst.apply(body)
        }
        _ => ty.clone(),
    }
}

/// Generalize a type by wrapping free type variables in Forall.
pub fn generalize(ty: &Ty, env_vars: &[u32]) -> Ty {
    let free_vars = free_type_vars(ty);
    let params: Vec<String> = free_vars
        .iter()
        .filter(|v| !env_vars.contains(v))
        .map(|v| format!("t{}", v))
        .collect();
    
    if params.is_empty() {
        ty.clone()
    } else {
        Ty {
            kind: TyKind::Forall(params, Box::new(ty.clone())),
            span: ty.span,
        }
    }
}

/// Collect free type variables from a type.
pub fn free_type_vars(ty: &Ty) -> Vec<u32> {
    let mut vars = Vec::new();
    collect_free_vars(ty, &mut vars);
    vars.sort();
    vars.dedup();
    vars
}

fn collect_free_vars(ty: &Ty, vars: &mut Vec<u32>) {
    match &ty.kind {
        TyKind::Var(v) => vars.push(*v),
        TyKind::Fn(params, ret) => {
            for p in params {
                collect_free_vars(p, vars);
            }
            collect_free_vars(ret, vars);
        }
        TyKind::Tuple(elems) => {
            for e in elems {
                collect_free_vars(e, vars);
            }
        }
        TyKind::Named(_, args) => {
            for a in args {
                collect_free_vars(a, vars);
            }
        }
        TyKind::Record(fields) => {
            for (_, t) in fields {
                collect_free_vars(t, vars);
            }
        }
        TyKind::Forall(_, body) => {
            collect_free_vars(body, vars);
        }
        _ => {}
    }
}
