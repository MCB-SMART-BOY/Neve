//! Type checker implementation.

use std::collections::HashMap;
use neve_common::Span;
use neve_diagnostic::{Diagnostic, DiagnosticKind, ErrorCode};
use neve_hir::{
    Module, Item, ItemKind, FnDef, TraitDef, ImplDef,
    Expr, ExprKind, Literal, BinOp, UnaryOp,
    Pattern, PatternKind, MatchArm,
    Stmt, StmtKind,
    Ty, TyKind, DefId, LocalId,
};
use crate::infer::InferContext;
use crate::unify::{Substitution, unify, instantiate, generalize, free_type_vars};
use crate::traits::{TraitResolver, TraitId};
use crate::errors::{TypeMismatchError, unbound_variable, unused_variable};

/// Information about a local variable.
#[derive(Clone)]
struct LocalInfo {
    ty: Ty,
    name: String,
    span: Span,
    used: bool,
}

/// The type checker.
pub struct TypeChecker {
    /// Type inference context for fresh type variables
    infer: InferContext,
    /// Substitution built during unification
    subst: Substitution,
    /// Types of global definitions
    globals: HashMap<DefId, Ty>,
    /// Span of global definitions for error reporting
    #[allow(dead_code)]
    global_spans: HashMap<DefId, Span>,
    /// Types of local variables with usage tracking
    locals: HashMap<LocalId, LocalInfo>,
    /// Trait resolver for trait/impl handling
    trait_resolver: TraitResolver,
    /// Map from def_id to trait_id
    trait_ids: HashMap<DefId, TraitId>,
    /// Collected diagnostics
    diagnostics: Vec<Diagnostic>,
    /// Whether to check for unused variables
    check_unused: bool,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            infer: InferContext::new(),
            subst: Substitution::new(),
            globals: HashMap::new(),
            global_spans: HashMap::new(),
            locals: HashMap::new(),
            trait_resolver: TraitResolver::new(),
            trait_ids: HashMap::new(),
            diagnostics: Vec::new(),
            check_unused: true,
        }
    }

    /// Create a type checker with unused variable checking disabled.
    pub fn without_unused_check() -> Self {
        Self {
            check_unused: false,
            ..Self::new()
        }
    }

    /// Type check a module.
    pub fn check(&mut self, module: &Module) {
        // First pass: collect all definitions (functions, traits, impls)
        for item in &module.items {
            self.collect_item(item);
        }

        // Second pass: check trait impls are complete
        self.check_all_impls();

        // Third pass: type check function bodies
        for item in &module.items {
            self.check_item(item);
        }
    }

    /// Check all registered impls for completeness.
    fn check_all_impls(&mut self) {
        // Collect trait info for checking
        let trait_infos: Vec<_> = self.trait_resolver.all_traits()
            .map(|(trait_id, info)| (*trait_id, info.clone()))
            .collect();

        // Check each trait's impls
        for (trait_id, trait_info) in trait_infos {
            let impl_infos: Vec<_> = self.trait_resolver.impls_for_trait(trait_id)
                .iter()
                .map(|info| (info.self_ty.clone(), info.methods.iter().map(|m| m.name.clone()).collect::<Vec<_>>()))
                .collect();

            for (self_ty, impl_methods) in impl_infos {
                let missing = self.check_impl_methods(&impl_methods, &trait_info);
                for method_name in missing {
                    self.diagnostics.push(
                        Diagnostic::error(DiagnosticKind::Type, Span::DUMMY, 
                            format!("missing required method '{}' in impl for {:?}", method_name, self_ty.kind))
                            .with_code(ErrorCode::TypeMismatch)
                    );
                }
            }
        }
    }

    /// Check if an impl provides all required methods.
    fn check_impl_methods(&self, impl_methods: &[String], trait_info: &crate::traits::TraitInfo) -> Vec<String> {
        let mut missing = Vec::new();
        for method in &trait_info.methods {
            if !method.has_default && !impl_methods.contains(&method.name) {
                missing.push(method.name.clone());
            }
        }
        missing
    }

    /// Get the trait resolver (for external use).
    pub fn trait_resolver(&self) -> &TraitResolver {
        &self.trait_resolver
    }

    /// Get the collected diagnostics.
    pub fn diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics.push(
            Diagnostic::error(DiagnosticKind::Type, span, message)
                .with_code(ErrorCode::TypeMismatch)
        );
    }

    fn emit(&mut self, diag: Diagnostic) {
        self.diagnostics.push(diag);
    }

    /// Check for unused variables and emit warnings.
    fn check_unused_locals(&mut self) {
        if !self.check_unused {
            return;
        }
        for info in self.locals.values() {
            if !info.used && !info.name.starts_with('_') {
                self.diagnostics.push(unused_variable(&info.name, info.span));
            }
        }
    }

    /// Mark a local variable as used.
    fn mark_used(&mut self, local_id: LocalId) {
        if let Some(info) = self.locals.get_mut(&local_id) {
            info.used = true;
        }
    }

    /// Define a local variable.
    fn define_local(&mut self, local_id: LocalId, name: String, ty: Ty, span: Span) {
        self.locals.insert(local_id, LocalInfo {
            ty,
            name,
            span,
            used: false,
        });
    }

    /// Get type of a local variable.
    fn get_local(&self, local_id: &LocalId) -> Option<Ty> {
        self.locals.get(local_id).map(|info| info.ty.clone())
    }

    fn fresh_var(&mut self) -> Ty {
        self.infer.fresh_var()
    }

    fn apply(&self, ty: &Ty) -> Ty {
        self.subst.apply(ty)
    }
    
    /// Check if a type variable has been resolved.
    pub fn is_resolved(&self, var: u32) -> bool {
        self.subst.get(var).is_some()
    }
    
    /// Get the resolved type for a type variable, if any.
    pub fn get_resolved(&self, var: u32) -> Option<Ty> {
        self.subst.get(var).map(|ty| self.apply(ty))
    }
    
    /// Check if a generic parameter has been bound.
    pub fn is_param_bound(&self, idx: u32) -> bool {
        self.subst.get_param(idx).is_some()
    }
    
    /// Get the bound type for a generic parameter, if any.
    pub fn get_param_binding(&self, idx: u32) -> Option<Ty> {
        self.subst.get_param(idx).map(|ty| self.apply(ty))
    }

    fn unify(&mut self, t1: &Ty, t2: &Ty, span: Span) -> bool {
        match unify(t1, t2, &mut self.subst) {
            Ok(()) => true,
            Err(msg) => {
                self.error(span, msg);
                false
            }
        }
    }

    // === First pass: collect signatures ===

    fn collect_item(&mut self, item: &Item) {
        match &item.kind {
            ItemKind::Fn(fn_def) => {
                let fn_ty = self.fn_signature(fn_def);
                self.globals.insert(item.id, fn_ty);
            }
            ItemKind::Trait(trait_def) => {
                self.collect_trait(item.id, trait_def);
            }
            ItemKind::Impl(impl_def) => {
                self.collect_impl(item.id, impl_def);
            }
            ItemKind::Struct(_) | ItemKind::Enum(_) | ItemKind::TypeAlias(_) => {
                // TODO: Handle struct/enum/type alias definitions
            }
        }
    }

    fn collect_trait(&mut self, def_id: DefId, trait_def: &TraitDef) {
        let trait_id = self.trait_resolver.register_trait(def_id, trait_def);
        self.trait_ids.insert(def_id, trait_id);
    }

    fn collect_impl(&mut self, def_id: DefId, impl_def: &ImplDef) {
        self.trait_resolver.register_impl(def_id, impl_def);
    }

    fn fn_signature(&mut self, fn_def: &FnDef) -> Ty {
        let param_tys: Vec<Ty> = fn_def.params.iter()
            .map(|p| self.resolve_type(&p.ty))
            .collect();
        
        let ret_ty = self.resolve_type(&fn_def.return_ty);
        
        let fn_ty = Ty {
            kind: TyKind::Fn(param_tys, Box::new(ret_ty)),
            span: Span::DUMMY,
        };

        // Wrap in Forall if there are generic parameters
        if fn_def.generics.is_empty() {
            fn_ty
        } else {
            let params: Vec<String> = fn_def.generics.iter()
                .map(|g| g.name.clone())
                .collect();
            Ty {
                kind: TyKind::Forall(params, Box::new(fn_ty)),
                span: Span::DUMMY,
            }
        }
    }

    fn resolve_type(&mut self, ty: &Ty) -> Ty {
        match &ty.kind {
            TyKind::Unknown => self.fresh_var(),
            TyKind::Param(idx, name) => {
                // Generic parameters stay as-is during signature collection
                Ty {
                    kind: TyKind::Param(*idx, name.clone()),
                    span: ty.span,
                }
            }
            TyKind::Named(id, args) => {
                let resolved_args: Vec<Ty> = args.iter()
                    .map(|a| self.resolve_type(a))
                    .collect();
                Ty {
                    kind: TyKind::Named(*id, resolved_args),
                    span: ty.span,
                }
            }
            TyKind::Fn(params, ret) => {
                let resolved_params: Vec<Ty> = params.iter()
                    .map(|p| self.resolve_type(p))
                    .collect();
                Ty {
                    kind: TyKind::Fn(resolved_params, Box::new(self.resolve_type(ret))),
                    span: ty.span,
                }
            }
            TyKind::Tuple(elems) => {
                let resolved_elems: Vec<Ty> = elems.iter()
                    .map(|e| self.resolve_type(e))
                    .collect();
                Ty {
                    kind: TyKind::Tuple(resolved_elems),
                    span: ty.span,
                }
            }
            _ => ty.clone(),
        }
    }

    // === Second pass: check bodies ===

    fn check_item(&mut self, item: &Item) {
        if let ItemKind::Fn(fn_def) = &item.kind {
            self.check_fn(item.id, fn_def);
        }
    }

    fn check_fn(&mut self, _id: DefId, fn_def: &FnDef) {
        // Create fresh type variables for generic parameters
        let mut generic_vars: HashMap<String, Ty> = HashMap::new();
        for (idx, param) in fn_def.generics.iter().enumerate() {
            let var = self.fresh_var();
            generic_vars.insert(param.name.clone(), var.clone());
            self.subst.bind_param(idx as u32, var);
        }

        // Bind parameter types (resolving generic references)
        // Parameters are considered used by default (they're part of the function signature)
        for param in &fn_def.params {
            let ty = self.resolve_type_with_generics(&param.ty, &generic_vars);
            self.locals.insert(param.id, LocalInfo {
                ty,
                name: param.name.clone(),
                span: param.span,
                used: true, // Parameters are always "used"
            });
        }

        // Infer body type
        let body_ty = self.infer_expr(&fn_def.body);

        // Unify with declared return type
        let ret_ty = self.resolve_type_with_generics(&fn_def.return_ty, &generic_vars);
        if !self.unify(&body_ty, &ret_ty, fn_def.body.span) {
            // Emit a more detailed error
            self.emit(
                TypeMismatchError::new(ret_ty, body_ty, fn_def.body.span)
                    .with_context("function return type")
                    .build()
            );
        }

        // Check for unused variables before clearing
        self.check_unused_locals();

        // Clear locals after checking function
        self.locals.clear();
    }

    /// Resolve a type, substituting generic parameters with their bound types.
    fn resolve_type_with_generics(&mut self, ty: &Ty, generics: &HashMap<String, Ty>) -> Ty {
        match &ty.kind {
            TyKind::Unknown => self.fresh_var(),
            TyKind::Param(_idx, name) => {
                generics.get(name).cloned().unwrap_or_else(|| {
                    self.error(ty.span, format!("unknown generic parameter: {}", name));
                    self.fresh_var()
                })
            }
            TyKind::Named(id, args) => {
                let resolved_args: Vec<Ty> = args.iter()
                    .map(|a| self.resolve_type_with_generics(a, generics))
                    .collect();
                Ty {
                    kind: TyKind::Named(*id, resolved_args),
                    span: ty.span,
                }
            }
            TyKind::Fn(params, ret) => {
                let resolved_params: Vec<Ty> = params.iter()
                    .map(|p| self.resolve_type_with_generics(p, generics))
                    .collect();
                Ty {
                    kind: TyKind::Fn(resolved_params, Box::new(self.resolve_type_with_generics(ret, generics))),
                    span: ty.span,
                }
            }
            TyKind::Tuple(elems) => {
                let resolved_elems: Vec<Ty> = elems.iter()
                    .map(|e| self.resolve_type_with_generics(e, generics))
                    .collect();
                Ty {
                    kind: TyKind::Tuple(resolved_elems),
                    span: ty.span,
                }
            }
            _ => ty.clone(),
        }
    }

    fn infer_expr(&mut self, expr: &Expr) -> Ty {
        let span = expr.span;
        match &expr.kind {
            ExprKind::Literal(lit) => self.infer_literal(lit),

            ExprKind::Var(local_id) => {
                self.mark_used(*local_id);
                self.get_local(local_id).unwrap_or_else(|| {
                    self.emit(unbound_variable("variable", span, None));
                    self.fresh_var()
                })
            }

            ExprKind::Global(def_id) => {
                self.globals.get(def_id).cloned().map(|ty| {
                    // Instantiate polymorphic types with fresh type variables
                    instantiate(&ty, &mut || self.fresh_var())
                }).unwrap_or_else(|| {
                    self.error(span, "undefined global");
                    self.fresh_var()
                })
            }

            ExprKind::List(items) => {
                let elem_ty = self.fresh_var();
                for item in items {
                    let item_ty = self.infer_expr(item);
                    self.unify(&elem_ty, &item_ty, item.span);
                }
                // For now, represent List<T> as a named type
                Ty {
                    kind: TyKind::Named(DefId(u32::MAX), vec![self.apply(&elem_ty)]),
                    span,
                }
            }

            ExprKind::Tuple(items) => {
                let elem_tys: Vec<Ty> = items.iter()
                    .map(|e| self.infer_expr(e))
                    .collect();
                Ty {
                    kind: TyKind::Tuple(elem_tys),
                    span,
                }
            }

            ExprKind::Record(fields) => {
                let field_tys: Vec<(String, Ty)> = fields.iter()
                    .map(|(name, e)| (name.clone(), self.infer_expr(e)))
                    .collect();
                Ty {
                    kind: TyKind::Record(field_tys),
                    span,
                }
            }

            ExprKind::Lambda(params, body) => {
                // Bind parameter types
                let param_tys: Vec<Ty> = params.iter()
                    .map(|p| {
                        let ty = self.resolve_type(&p.ty);
                        self.locals.insert(p.id, LocalInfo {
                            ty: ty.clone(),
                            name: p.name.clone(),
                            span: p.span,
                            used: true, // Lambda params considered used
                        });
                        ty
                    })
                    .collect();

                // Infer body
                let body_ty = self.infer_expr(body);

                // Remove locals
                for p in params {
                    self.locals.remove(&p.id);
                }

                Ty {
                    kind: TyKind::Fn(param_tys, Box::new(body_ty)),
                    span,
                }
            }

            ExprKind::Call(func, args) => {
                let func_ty = self.infer_expr(func);
                let arg_tys: Vec<Ty> = args.iter()
                    .map(|a| self.infer_expr(a))
                    .collect();

                let ret_ty = self.fresh_var();
                let expected_fn_ty = Ty {
                    kind: TyKind::Fn(arg_tys, Box::new(ret_ty.clone())),
                    span,
                };

                self.unify(&func_ty, &expected_fn_ty, span);
                self.apply(&ret_ty)
            }

            ExprKind::Field(base, field) => {
                let base_ty = self.infer_expr(base);
                let base_ty = self.apply(&base_ty);

                match &base_ty.kind {
                    TyKind::Record(fields) => {
                        for (name, ty) in fields {
                            if name == field {
                                return ty.clone();
                            }
                        }
                        self.error(span, format!("no field '{}' in record", field));
                        self.fresh_var()
                    }
                    _ => {
                        self.error(span, "field access on non-record type");
                        self.fresh_var()
                    }
                }
            }

            ExprKind::TupleIndex(base, index) => {
                let base_ty = self.infer_expr(base);
                let base_ty = self.apply(&base_ty);

                match &base_ty.kind {
                    TyKind::Tuple(elems) => {
                        if (*index as usize) < elems.len() {
                            elems[*index as usize].clone()
                        } else {
                            self.error(span, "tuple index out of bounds");
                            self.fresh_var()
                        }
                    }
                    _ => {
                        self.error(span, "tuple index on non-tuple type");
                        self.fresh_var()
                    }
                }
            }

            ExprKind::Binary(op, left, right) => {
                self.infer_binary(*op, left, right, span)
            }

            ExprKind::Unary(op, operand) => {
                self.infer_unary(*op, operand, span)
            }

            ExprKind::If(cond, then_br, else_br) => {
                let cond_ty = self.infer_expr(cond);
                self.unify(&cond_ty, &Ty { kind: TyKind::Bool, span: cond.span }, cond.span);

                let then_ty = self.infer_expr(then_br);
                let else_ty = self.infer_expr(else_br);
                self.unify(&then_ty, &else_ty, span);

                self.apply(&then_ty)
            }

            ExprKind::Match(scrutinee, arms) => {
                let scrutinee_ty = self.infer_expr(scrutinee);
                let result_ty = self.fresh_var();

                for arm in arms {
                    self.check_arm(arm, &scrutinee_ty, &result_ty);
                }

                self.apply(&result_ty)
            }

            ExprKind::Block(stmts, expr) => {
                for stmt in stmts {
                    self.check_stmt(stmt);
                }

                if let Some(e) = expr {
                    self.infer_expr(e)
                } else {
                    Ty { kind: TyKind::Unit, span }
                }
            }

            ExprKind::Interpolated(parts) => {
                // Check that all interpolated expressions are valid
                for part in parts {
                    if let neve_hir::StringPart::Expr(e) = part {
                        // We don't constrain the type of interpolated expressions
                        // Any type can be converted to string
                        let _ = self.infer_expr(e);
                    }
                }
                // Interpolated strings always have type String
                Ty { kind: TyKind::String, span }
            }
        }
    }

    fn infer_literal(&self, lit: &Literal) -> Ty {
        let kind = match lit {
            Literal::Int(_) => TyKind::Int,
            Literal::Float(_) => TyKind::Float,
            Literal::String(_) => TyKind::String,
            Literal::Char(_) => TyKind::Char,
            Literal::Bool(_) => TyKind::Bool,
            Literal::Unit => TyKind::Unit,
        };
        Ty { kind, span: Span::DUMMY }
    }

    fn infer_binary(&mut self, op: BinOp, left: &Expr, right: &Expr, span: Span) -> Ty {
        let left_ty = self.infer_expr(left);
        let right_ty = self.infer_expr(right);

        match op {
            // Arithmetic: Int -> Int -> Int or Float -> Float -> Float
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Pow => {
                self.unify(&left_ty, &right_ty, span);
                // For now, assume numeric types
                self.apply(&left_ty)
            }

            // Comparison: a -> a -> Bool
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                self.unify(&left_ty, &right_ty, span);
                Ty { kind: TyKind::Bool, span }
            }

            // Logical: Bool -> Bool -> Bool
            BinOp::And | BinOp::Or => {
                self.unify(&left_ty, &Ty { kind: TyKind::Bool, span: left.span }, left.span);
                self.unify(&right_ty, &Ty { kind: TyKind::Bool, span: right.span }, right.span);
                Ty { kind: TyKind::Bool, span }
            }

            // Concat: [a] -> [a] -> [a] or String -> String -> String
            BinOp::Concat => {
                self.unify(&left_ty, &right_ty, span);
                self.apply(&left_ty)
            }

            // Merge: {..} -> {..} -> {..}
            BinOp::Merge => {
                // Both should be records, result is merged record
                self.apply(&left_ty)
            }

            // Pipe: a -> (a -> b) -> b
            BinOp::Pipe => {
                let result_ty = self.fresh_var();
                let expected_fn = Ty {
                    kind: TyKind::Fn(vec![left_ty], Box::new(result_ty.clone())),
                    span,
                };
                self.unify(&right_ty, &expected_fn, right.span);
                self.apply(&result_ty)
            }
        }
    }

    fn infer_unary(&mut self, op: UnaryOp, operand: &Expr, span: Span) -> Ty {
        let operand_ty = self.infer_expr(operand);

        match op {
            UnaryOp::Neg => {
                // Numeric type
                self.apply(&operand_ty)
            }
            UnaryOp::Not => {
                self.unify(&operand_ty, &Ty { kind: TyKind::Bool, span: operand.span }, operand.span);
                Ty { kind: TyKind::Bool, span }
            }
        }
    }

    fn check_arm(&mut self, arm: &MatchArm, scrutinee_ty: &Ty, result_ty: &Ty) {
        // Check pattern against scrutinee type
        self.check_pattern(&arm.pattern, scrutinee_ty);

        // Check guard if present
        if let Some(guard) = &arm.guard {
            let guard_ty = self.infer_expr(guard);
            self.unify(&guard_ty, &Ty { kind: TyKind::Bool, span: guard.span }, guard.span);
        }

        // Check body and unify with result type
        let body_ty = self.infer_expr(&arm.body);
        self.unify(&body_ty, result_ty, arm.body.span);
    }

    fn check_pattern(&mut self, pattern: &Pattern, expected: &Ty) {
        match &pattern.kind {
            PatternKind::Wildcard => {}

            PatternKind::Var(local_id, name) => {
                self.define_local(*local_id, name.clone(), expected.clone(), pattern.span);
            }

            PatternKind::Literal(lit) => {
                let lit_ty = self.infer_literal(lit);
                self.unify(&lit_ty, expected, pattern.span);
            }

            PatternKind::Tuple(patterns) => {
                match &expected.kind {
                    TyKind::Tuple(elem_tys) if elem_tys.len() == patterns.len() => {
                        for (pat, ty) in patterns.iter().zip(elem_tys.iter()) {
                            self.check_pattern(pat, ty);
                        }
                    }
                    _ => {
                        self.error(pattern.span, "pattern does not match expected tuple");
                    }
                }
            }

            PatternKind::List(patterns) => {
                // Infer element type
                let elem_ty = self.fresh_var();
                for pat in patterns {
                    self.check_pattern(pat, &elem_ty);
                }
            }

            PatternKind::Record(fields) => {
                for (name, pat) in fields {
                    let field_ty = match &expected.kind {
                        TyKind::Record(field_tys) => {
                            field_tys.iter()
                                .find(|(n, _)| n == name)
                                .map(|(_, t)| t.clone())
                        }
                        _ => None,
                    };
                    
                    if let Some(ty) = field_ty {
                        self.check_pattern(pat, &ty);
                    } else {
                        self.error(pattern.span, format!("no field '{}' in record", name));
                    }
                }
            }

            PatternKind::Constructor(_def_id, patterns) => {
                // TODO: Look up constructor signature
                let arg_ty = self.fresh_var();
                for pat in patterns {
                    self.check_pattern(pat, &arg_ty);
                }
            }
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Let(local_id, name, ty, value) => {
                let value_ty = self.infer_expr(value);
                let declared_ty = self.resolve_type(ty);
                self.unify(&value_ty, &declared_ty, value.span);
                
                // Generalize the type for let-polymorphism
                // Collect environment type variables that shouldn't be generalized
                let env_vars: Vec<u32> = self.locals.values()
                    .flat_map(|info| free_type_vars(&info.ty))
                    .collect();
                let generalized_ty = generalize(&self.apply(&declared_ty), &env_vars);
                self.define_local(*local_id, name.clone(), generalized_ty, stmt.span);
            }
            StmtKind::Expr(e) => {
                self.infer_expr(e);
            }
        }
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

