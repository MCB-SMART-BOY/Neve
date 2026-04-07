//! Type checker implementation.
//! 类型检查器实现。
//!
//! This module implements the main type checker for Neve.
//! It performs bidirectional type checking with Hindley-Milner inference.
//! 本模块实现 Neve 的主类型检查器。
//! 采用带有 Hindley-Milner 推断的双向类型检查。

use crate::builtin_types::{
    builtin_list, builtin_map, builtin_option, builtin_result, builtin_set, is_builtin_option_type,
    is_builtin_result_type,
};
use crate::errors::{
    TypeMismatchError, format_type, missing_assoc_type, missing_method, non_exhaustive_match,
    unbound_variable, unreachable_pattern, unused_variable,
};
use crate::infer::InferContext;
use crate::traits::{ImplInfo, TraitBound, TraitId, TraitInfo, TraitResolver};
use crate::unify::{Substitution, free_type_vars, generalize, instantiate, unify};
use neve_common::Span;
use neve_diagnostic::{Diagnostic, DiagnosticKind, ErrorCode, Label};
use neve_hir::{
    BinOp, DefId, EnumDef, Expr, ExprKind, FnDef, ImplDef, Item, ItemKind, Literal, LocalId,
    MatchArm, Module, Pattern, PatternKind, Stmt, StmtKind, StructDef, TraitDef, Ty, TyKind,
    TypeAlias, UnaryOp, builtin_constructor_name,
};
use std::collections::HashMap;

fn builtin_ty(kind: TyKind, span: Span) -> Ty {
    Ty { kind, span }
}

fn builtin_param(idx: u32, name: &str, span: Span) -> Ty {
    builtin_ty(TyKind::Param(idx, name.to_string()), span)
}

fn builtin_fn(params: Vec<Ty>, ret: Ty, span: Span) -> Ty {
    builtin_ty(TyKind::Fn(params, Box::new(ret)), span)
}

fn builtin_forall(params: Vec<&str>, body: Ty, span: Span) -> Ty {
    builtin_ty(
        TyKind::Forall(
            params.into_iter().map(|param| param.to_string()).collect(),
            Box::new(body),
        ),
        span,
    )
}

fn builtin_record(fields: Vec<(&str, Ty)>, span: Span) -> Ty {
    builtin_ty(
        TyKind::Record(
            fields
                .into_iter()
                .map(|(name, ty)| (name.to_string(), ty))
                .collect(),
        ),
        span,
    )
}

fn builtin_string_option(span: Span) -> Ty {
    builtin_option(builtin_ty(TyKind::String, span), span)
}

fn builtin_string_list(span: Span) -> Ty {
    builtin_list(builtin_ty(TyKind::String, span), span)
}

fn builtin_exec_result(span: Span) -> Ty {
    builtin_record(
        vec![
            ("code", builtin_ty(TyKind::Int, span)),
            ("success", builtin_ty(TyKind::Bool, span)),
            ("stdout", builtin_ty(TyKind::String, span)),
            ("stderr", builtin_ty(TyKind::String, span)),
        ],
        span,
    )
}

fn builtin_fetch_result(span: Span) -> Ty {
    builtin_record(
        vec![
            ("path", builtin_ty(TyKind::String, span)),
            ("hash", builtin_ty(TyKind::String, span)),
            ("cached", builtin_ty(TyKind::Bool, span)),
        ],
        span,
    )
}

/// Information about a local variable.
/// 局部变量的信息。
#[derive(Clone)]
struct LocalInfo {
    /// The type of the variable. / 变量的类型。
    ty: Ty,
    /// The variable name. / 变量名。
    name: String,
    /// Source location. / 源码位置。
    span: Span,
    /// Whether the variable has been used. / 变量是否被使用过。
    used: bool,
}

/// Information about a struct type definition.
/// 结构体类型定义的信息。
#[derive(Clone)]
struct StructInfo {
    /// Field types (name -> type). / 字段类型（名称 -> 类型）。
    fields: HashMap<String, Ty>,
}

/// Information about an enum type definition.
/// 枚举类型定义的信息。
#[derive(Clone)]
struct EnumInfo {
    /// Variant constructors (name -> field types). / 变体构造函数（名称 -> 字段类型）。
    variants: HashMap<String, Vec<Ty>>,
}

/// Information about a variant constructor.
/// 变体构造器的信息。
#[derive(Clone)]
struct VariantInfo {
    /// Enum definition ID. / 枚举定义 ID。
    enum_id: DefId,
    /// Variant name. / 变体名称。
    name: String,
    /// Field types. / 字段类型。
    fields: Vec<Ty>,
}

/// Information about a type alias.
/// 类型别名的信息。
#[derive(Clone)]
struct TypeAliasInfo {
    /// Target type. / 目标类型。
    target: Ty,
}

/// The type checker.
/// 类型检查器。
pub struct TypeChecker {
    /// Type inference context for fresh type variables.
    /// 用于生成新类型变量的推断上下文。
    infer: InferContext,
    /// Substitution built during unification.
    /// 合一过程中构建的替换。
    subst: Substitution,
    /// Types of global definitions.
    /// 全局定义的类型。
    globals: HashMap<DefId, Ty>,
    /// Span of global definitions for error reporting.
    /// 全局定义的位置信息，用于错误报告。
    global_spans: HashMap<DefId, Span>,
    /// Types of local variables with usage tracking.
    /// 局部变量的类型及使用情况跟踪。
    locals: HashMap<LocalId, LocalInfo>,
    /// Final inferred types for local definitions keyed by LocalId.
    /// 按 LocalId 存储的局部定义最终推断类型。
    local_definitions: HashMap<LocalId, Ty>,
    /// Final inferred types for expressions keyed by source span.
    /// 按源码 span 存储的表达式最终推断类型。
    expr_types: HashMap<Span, Ty>,
    /// Trait resolver for trait/impl handling.
    /// 用于处理 trait/impl 的特征解析器。
    trait_resolver: TraitResolver,
    /// Map from def_id to trait_id.
    /// 定义 ID 到特征 ID 的映射。
    trait_ids: HashMap<DefId, TraitId>,
    /// Struct type definitions. / 结构体类型定义。
    structs: HashMap<DefId, StructInfo>,
    /// Enum type definitions. / 枚举类型定义。
    enums: HashMap<DefId, EnumInfo>,
    /// Variant constructors by DefId. / 按 DefId 存储的变体构造器。
    variants: HashMap<DefId, VariantInfo>,
    /// Type alias definitions. / 类型别名定义。
    type_aliases: HashMap<DefId, TypeAliasInfo>,
    /// Collected diagnostics.
    /// 收集的诊断信息。
    diagnostics: Vec<Diagnostic>,
    /// Resolved method call targets keyed by expression span.
    /// 按表达式 span 存储的方法调用解析结果。
    method_resolutions: HashMap<Span, DefId>,
    /// Whether to check for unused variables.
    /// 是否检查未使用的变量。
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
            local_definitions: HashMap::new(),
            expr_types: HashMap::new(),
            trait_resolver: TraitResolver::new(),
            trait_ids: HashMap::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            variants: HashMap::new(),
            type_aliases: HashMap::new(),
            diagnostics: Vec::new(),
            method_resolutions: HashMap::new(),
            check_unused: true,
        }
    }

    /// Create a type checker with preloaded global signatures.
    /// 使用预加载的全局签名创建类型检查器。
    pub fn with_global_env(
        globals: HashMap<DefId, Ty>,
        global_spans: HashMap<DefId, Span>,
    ) -> Self {
        Self {
            globals,
            global_spans,
            ..Self::new()
        }
    }

    /// Create a type checker with unused variable checking disabled.
    /// 创建一个禁用未使用变量检查的类型检查器。
    pub fn without_unused_check() -> Self {
        Self {
            check_unused: false,
            ..Self::new()
        }
    }

    /// Collect global signatures from a module without checking bodies.
    /// 在不检查函数体的情况下收集模块的全局签名。
    pub fn collect_signatures(module: &Module) -> (HashMap<DefId, Ty>, HashMap<DefId, Span>) {
        let mut checker = TypeChecker::new();
        for item in &module.items {
            checker.collect_item(item);
        }
        (checker.globals, checker.global_spans)
    }

    /// Type check a module.
    /// 对模块进行类型检查。
    pub fn check(&mut self, module: &Module) {
        // First pass: collect all definitions (functions, traits, impls)
        // 第一遍：收集所有定义（函数、特征、实现）
        for item in &module.items {
            self.collect_item(item);
        }

        // Second pass: check trait impls are complete
        // 第二遍：检查特征实现是否完整
        self.check_all_impls();

        // Third pass: type check function bodies
        // 第三遍：对函数体进行类型检查
        for item in &module.items {
            self.check_item(item);
        }
    }

    /// Check all registered impls for completeness.
    /// 检查所有已注册的实现是否完整。
    fn check_all_impls(&mut self) {
        // Collect trait info for checking
        let trait_infos: Vec<_> = self
            .trait_resolver
            .all_traits()
            .map(|(trait_id, info)| (*trait_id, info.clone()))
            .collect();

        // Check each trait's impls
        for (trait_id, trait_info) in trait_infos {
            let impl_ids = self.trait_resolver.impl_ids_for_trait(trait_id);

            for impl_id in impl_ids {
                let Some(impl_info) = self.trait_resolver.impl_info(impl_id).cloned() else {
                    continue;
                };

                let completeness = self.trait_resolver.check_impl_full_completeness(impl_id);
                if !completeness.is_complete() {
                    let span = self
                        .global_spans
                        .get(&impl_info.def_id)
                        .copied()
                        .unwrap_or(Span::DUMMY);

                    for method_name in completeness.missing_methods {
                        self.diagnostics.push(missing_method(
                            &method_name,
                            &trait_info.name,
                            &impl_info.self_ty,
                            span,
                        ));
                    }

                    for assoc_name in completeness.missing_assoc_types {
                        self.diagnostics.push(missing_assoc_type(
                            &assoc_name,
                            &trait_info.name,
                            &impl_info.self_ty,
                            span,
                        ));
                    }
                }

                self.check_assoc_type_bounds(&trait_info, &impl_info);
                self.check_impl_method_signatures(&trait_info, &impl_info);
            }
        }
    }

    fn check_impl_method_signatures(&mut self, trait_info: &TraitInfo, impl_info: &ImplInfo) {
        let assoc_types = self.impl_signature_assoc_types(trait_info, impl_info);

        for trait_method in &trait_info.methods {
            let Some(impl_method) = impl_info
                .methods
                .iter()
                .find(|method| method.name == trait_method.name)
            else {
                continue;
            };

            let expected_params: Vec<Ty> = trait_method
                .params
                .iter()
                .map(|ty| self.resolve_impl_signature_type(ty, &impl_info.self_ty, &assoc_types))
                .collect();
            let actual_params: Vec<Ty> = impl_method
                .params
                .iter()
                .map(|ty| self.resolve_impl_signature_type(ty, &impl_info.self_ty, &assoc_types))
                .collect();
            let expected_return = self.resolve_impl_signature_type(
                &trait_method.return_ty,
                &impl_info.self_ty,
                &assoc_types,
            );
            let actual_return = self.resolve_impl_signature_type(
                &impl_method.return_ty,
                &impl_info.self_ty,
                &assoc_types,
            );

            let params_match =
                expected_params.len() == actual_params.len()
                    && expected_params.iter().zip(actual_params.iter()).all(
                        |(expected, actual)| self.method_signature_ty_compatible(expected, actual),
                    );
            let return_match =
                self.method_signature_ty_compatible(&expected_return, &actual_return);

            if params_match && return_match {
                continue;
            }

            let expected_signature =
                Self::format_method_signature(&expected_params, &expected_return);
            let actual_signature = Self::format_method_signature(&actual_params, &actual_return);
            self.diagnostics.push(
                Diagnostic::error(
                    DiagnosticKind::Type,
                    impl_method.span,
                    format!(
                        "impl method `{}` does not match trait `{}` signature",
                        impl_method.name, trait_info.name
                    ),
                )
                .with_note(format!("trait expects {expected_signature}"))
                .with_note(format!("impl provides {actual_signature}")),
            );
        }
    }

    fn impl_signature_assoc_types(
        &self,
        trait_info: &TraitInfo,
        impl_info: &ImplInfo,
    ) -> HashMap<String, Ty> {
        let mut assoc_types: HashMap<String, Ty> = impl_info
            .assoc_types
            .iter()
            .map(|assoc| (assoc.name.clone(), assoc.ty.clone()))
            .collect();

        for assoc in &trait_info.assoc_types {
            if assoc_types.contains_key(&assoc.name) {
                continue;
            }
            if let Some(default) = &assoc.default {
                assoc_types.insert(assoc.name.clone(), default.clone());
            }
        }

        assoc_types
    }

    fn resolve_impl_signature_type(
        &self,
        ty: &Ty,
        self_ty: &Ty,
        assoc_types: &HashMap<String, Ty>,
    ) -> Ty {
        match &ty.kind {
            TyKind::SelfType => self_ty.clone(),
            TyKind::SelfAssoc(name) => assoc_types.get(name).cloned().unwrap_or_else(|| ty.clone()),
            TyKind::Named(id, args) => Ty {
                kind: TyKind::Named(
                    *id,
                    args.iter()
                        .map(|arg| self.resolve_impl_signature_type(arg, self_ty, assoc_types))
                        .collect(),
                ),
                span: ty.span,
            },
            TyKind::Fn(params, ret) => Ty {
                kind: TyKind::Fn(
                    params
                        .iter()
                        .map(|param| self.resolve_impl_signature_type(param, self_ty, assoc_types))
                        .collect(),
                    Box::new(self.resolve_impl_signature_type(ret, self_ty, assoc_types)),
                ),
                span: ty.span,
            },
            TyKind::Tuple(items) => Ty {
                kind: TyKind::Tuple(
                    items
                        .iter()
                        .map(|item| self.resolve_impl_signature_type(item, self_ty, assoc_types))
                        .collect(),
                ),
                span: ty.span,
            },
            TyKind::Record(fields) => Ty {
                kind: TyKind::Record(
                    fields
                        .iter()
                        .map(|(name, field_ty)| {
                            (
                                name.clone(),
                                self.resolve_impl_signature_type(field_ty, self_ty, assoc_types),
                            )
                        })
                        .collect(),
                ),
                span: ty.span,
            },
            TyKind::Forall(params, body) => Ty {
                kind: TyKind::Forall(
                    params.clone(),
                    Box::new(self.resolve_impl_signature_type(body, self_ty, assoc_types)),
                ),
                span: ty.span,
            },
            _ => ty.clone(),
        }
    }

    fn method_signature_ty_compatible(&self, expected: &Ty, actual: &Ty) -> bool {
        let expected = self.apply(expected);
        let actual = self.apply(actual);

        match (&expected.kind, &actual.kind) {
            (TyKind::Unknown, _) | (_, TyKind::Unknown) => true,
            (TyKind::Var(_), _) | (_, TyKind::Var(_)) => true,
            (TyKind::Param(_, _), _) | (_, TyKind::Param(_, _)) => true,
            (TyKind::SelfType, TyKind::SelfType) => true,
            (TyKind::SelfAssoc(left), TyKind::SelfAssoc(right)) => left == right,
            (TyKind::Int, TyKind::Int)
            | (TyKind::Float, TyKind::Float)
            | (TyKind::Bool, TyKind::Bool)
            | (TyKind::Char, TyKind::Char)
            | (TyKind::String, TyKind::String)
            | (TyKind::Unit, TyKind::Unit) => true,
            (TyKind::Named(left_id, left_args), TyKind::Named(right_id, right_args)) => {
                left_id == right_id
                    && left_args.len() == right_args.len()
                    && left_args
                        .iter()
                        .zip(right_args.iter())
                        .all(|(left, right)| self.method_signature_ty_compatible(left, right))
            }
            (TyKind::Fn(left_params, left_ret), TyKind::Fn(right_params, right_ret)) => {
                left_params.len() == right_params.len()
                    && left_params
                        .iter()
                        .zip(right_params.iter())
                        .all(|(left, right)| self.method_signature_ty_compatible(left, right))
                    && self.method_signature_ty_compatible(left_ret, right_ret)
            }
            (TyKind::Tuple(left), TyKind::Tuple(right)) => {
                left.len() == right.len()
                    && left
                        .iter()
                        .zip(right.iter())
                        .all(|(left, right)| self.method_signature_ty_compatible(left, right))
            }
            (TyKind::Record(left_fields), TyKind::Record(right_fields)) => {
                left_fields.len() == right_fields.len()
                    && left_fields.iter().zip(right_fields.iter()).all(
                        |((left_name, left_ty), (right_name, right_ty))| {
                            left_name == right_name
                                && self.method_signature_ty_compatible(left_ty, right_ty)
                        },
                    )
            }
            (TyKind::Forall(_, left), TyKind::Forall(_, right)) => {
                self.method_signature_ty_compatible(left, right)
            }
            _ => false,
        }
    }

    fn format_method_signature(params: &[Ty], ret: &Ty) -> String {
        let params = params
            .iter()
            .map(format_type)
            .collect::<Vec<_>>()
            .join(", ");
        format!("({params}) -> {}", format_type(ret))
    }

    /// Get the trait resolver (for external use).
    /// 获取特征解析器（供外部使用）。
    pub fn trait_resolver(&self) -> &TraitResolver {
        &self.trait_resolver
    }

    /// Get the span of a global definition by its DefId.
    /// 通过 DefId 获取全局定义的位置信息。
    pub fn global_span(&self, def_id: DefId) -> Option<Span> {
        self.global_spans.get(&def_id).copied()
    }

    /// Get struct field type by name.
    /// 通过名称获取结构体字段类型。
    pub fn struct_field_type(&self, def_id: DefId, field_name: &str) -> Option<Ty> {
        self.structs
            .get(&def_id)
            .and_then(|info| info.fields.get(field_name).cloned())
    }

    /// Get all struct field names.
    /// 获取所有结构体字段名称。
    pub fn struct_fields(&self, def_id: DefId) -> Option<Vec<String>> {
        self.structs
            .get(&def_id)
            .map(|info| info.fields.keys().cloned().collect())
    }

    /// Get enum variant field types by variant name.
    /// 通过变体名称获取枚举变体字段类型。
    pub fn enum_variant_types(&self, def_id: DefId, variant_name: &str) -> Option<Vec<Ty>> {
        self.enums
            .get(&def_id)
            .and_then(|info| info.variants.get(variant_name).cloned())
    }

    /// Get all enum variant names.
    /// 获取所有枚举变体名称。
    pub fn enum_variants(&self, def_id: DefId) -> Option<Vec<String>> {
        self.enums
            .get(&def_id)
            .map(|info| info.variants.keys().cloned().collect())
    }

    /// Resolve a type alias to its target type.
    /// 将类型别名解析为其目标类型。
    pub fn resolve_type_alias(&self, def_id: DefId) -> Option<Ty> {
        self.type_aliases
            .get(&def_id)
            .map(|info| info.target.clone())
    }

    /// Get the collected diagnostics.
    /// 获取收集的诊断信息。
    pub fn diagnostics(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    /// Borrow collected diagnostics without consuming the checker.
    /// 在不消费检查器的前提下借用已收集的诊断。
    pub fn diagnostics_ref(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Clear collected diagnostics while keeping accumulated semantic state.
    /// 清空已收集的诊断，但保留已累积的语义状态。
    pub fn clear_diagnostics(&mut self) {
        self.diagnostics.clear();
    }

    /// Get resolved method call targets.
    /// 获取已解析的方法调用目标。
    pub fn method_resolutions(&self) -> &HashMap<Span, DefId> {
        &self.method_resolutions
    }

    /// Clear resolved method call targets while keeping accumulated semantic state.
    /// 清空已解析的方法调用目标，但保留已累积的语义状态。
    pub fn clear_method_resolutions(&mut self) {
        self.method_resolutions.clear();
    }

    /// Look up the inferred type of a global definition.
    /// 查询某个全局定义推断出的类型。
    pub fn global_type(&self, def_id: DefId) -> Option<Ty> {
        self.globals.get(&def_id).map(|ty| self.apply(ty))
    }

    /// Look up the inferred type of a local definition.
    /// 查询某个局部定义推断出的类型。
    pub fn local_type(&self, local_id: LocalId) -> Option<Ty> {
        self.local_definitions
            .get(&local_id)
            .map(|ty| self.apply(ty))
    }

    /// Look up the inferred type of an expression by span.
    /// 按 span 查询表达式推断出的类型。
    pub fn expr_type(&self, span: Span) -> Option<Ty> {
        self.expr_types.get(&span).map(|ty| self.apply(ty))
    }

    fn format_trait_bound(&self, bound: &TraitBound) -> String {
        let trait_name = self
            .trait_resolver
            .get_trait(bound.trait_id)
            .map(|info| info.name.as_str())
            .unwrap_or("<unknown>");

        if bound.args.is_empty() {
            trait_name.to_string()
        } else {
            let args = bound
                .args
                .iter()
                .map(format_type)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}[{}]", trait_name, args)
        }
    }

    fn check_assoc_type_bounds(&mut self, trait_info: &TraitInfo, impl_info: &ImplInfo) {
        if trait_info.assoc_types.is_empty() {
            return;
        }

        for assoc_def in &trait_info.assoc_types {
            if assoc_def.bounds.is_empty() {
                continue;
            }

            let assoc_impl = impl_info
                .assoc_types
                .iter()
                .find(|assoc| assoc.name == assoc_def.name);

            let Some(assoc_impl) = assoc_impl else {
                continue;
            };

            let assoc_ty = &assoc_impl.ty;
            let assoc_ty_str = format_type(assoc_ty);

            for bound in &assoc_def.bounds {
                if self
                    .trait_resolver
                    .find_trait_impl(bound.trait_id, assoc_ty)
                    .is_some()
                {
                    continue;
                }

                let bound_name = self.format_trait_bound(bound);
                let message = format!(
                    "associated type '{}' in impl of trait '{}' must satisfy bound '{}'",
                    assoc_def.name, trait_info.name, bound_name
                );

                self.diagnostics.push(
                    Diagnostic::error(DiagnosticKind::Type, assoc_impl.span, message)
                        .with_code(ErrorCode::TraitNotImplemented)
                        .with_label(Label::new(
                            assoc_impl.span,
                            format!("this is `{}`", assoc_ty_str),
                        ))
                        .with_note(format!(
                            "`{}` does not implement `{}`",
                            assoc_ty_str, bound_name
                        )),
                );
            }
        }
    }

    fn error(&mut self, span: Span, message: impl Into<String>) {
        self.diagnostics.push(
            Diagnostic::error(DiagnosticKind::Type, span, message)
                .with_code(ErrorCode::TypeMismatch),
        );
    }

    fn emit(&mut self, diag: Diagnostic) {
        self.diagnostics.push(diag);
    }

    /// Check for unused variables and emit warnings.
    /// 检查未使用的变量并发出警告。
    fn check_unused_locals(&mut self) {
        if !self.check_unused {
            return;
        }
        for info in self.locals.values() {
            if !info.used && !info.name.starts_with('_') {
                self.diagnostics
                    .push(unused_variable(&info.name, info.span));
            }
        }
    }

    /// Mark a local variable as used.
    /// 标记局部变量为已使用。
    fn mark_used(&mut self, local_id: LocalId) {
        if let Some(info) = self.locals.get_mut(&local_id) {
            info.used = true;
        }
    }

    /// Define a local variable.
    /// 定义局部变量。
    fn define_local(&mut self, local_id: LocalId, name: String, ty: Ty, span: Span) {
        self.local_definitions.insert(local_id, ty.clone());
        self.locals.insert(
            local_id,
            LocalInfo {
                ty,
                name,
                span,
                used: false,
            },
        );
    }

    /// Get type of a local variable.
    /// 获取局部变量的类型。
    fn get_local(&self, local_id: &LocalId) -> Option<Ty> {
        self.locals.get(local_id).map(|info| info.ty.clone())
    }

    fn fresh_var(&mut self) -> Ty {
        self.infer.fresh_var()
    }

    fn apply(&self, ty: &Ty) -> Ty {
        self.subst.apply(ty)
    }

    fn builtin_type(&mut self, name: &str, span: Span) -> Option<Ty> {
        let polymorphic = match name {
            "force" => Ty {
                kind: TyKind::Forall(
                    vec!["a".to_string()],
                    Box::new(Ty {
                        kind: TyKind::Fn(
                            vec![Ty {
                                kind: TyKind::Param(0, "a".to_string()),
                                span,
                            }],
                            Box::new(Ty {
                                kind: TyKind::Param(0, "a".to_string()),
                                span,
                            }),
                        ),
                        span,
                    }),
                ),
                span,
            },
            "isLazy" | "isEvaluated" => Ty {
                kind: TyKind::Forall(
                    vec!["a".to_string()],
                    Box::new(Ty {
                        kind: TyKind::Fn(
                            vec![Ty {
                                kind: TyKind::Param(0, "a".to_string()),
                                span,
                            }],
                            Box::new(Ty {
                                kind: TyKind::Bool,
                                span,
                            }),
                        ),
                        span,
                    }),
                ),
                span,
            },
            "toString" => {
                let a = builtin_param(0, "a", span);
                builtin_forall(
                    Vec::from(["a"]),
                    builtin_fn(vec![a], builtin_ty(TyKind::String, span), span),
                    span,
                )
            }
            "list.empty" => {
                let a = builtin_param(0, "a", span);
                builtin_forall(Vec::from(["a"]), builtin_list(a, span), span)
            }
            "list.singleton" => {
                let a = builtin_param(0, "a", span);
                builtin_forall(
                    Vec::from(["a"]),
                    builtin_fn(vec![a.clone()], builtin_list(a, span), span),
                    span,
                )
            }
            "list.len" => {
                let a = builtin_param(0, "a", span);
                builtin_forall(
                    Vec::from(["a"]),
                    builtin_fn(
                        vec![builtin_list(a, span)],
                        builtin_ty(TyKind::Int, span),
                        span,
                    ),
                    span,
                )
            }
            "list.isEmpty" => {
                let a = builtin_param(0, "a", span);
                builtin_forall(
                    Vec::from(["a"]),
                    builtin_fn(
                        vec![builtin_list(a, span)],
                        builtin_ty(TyKind::Bool, span),
                        span,
                    ),
                    span,
                )
            }
            "list.tail" => {
                let a = builtin_param(0, "a", span);
                builtin_forall(
                    Vec::from(["a"]),
                    builtin_fn(
                        vec![builtin_list(a.clone(), span)],
                        builtin_list(a, span),
                        span,
                    ),
                    span,
                )
            }
            "list.append" => {
                let a = builtin_param(0, "a", span);
                let list_a = builtin_list(a.clone(), span);
                builtin_forall(
                    Vec::from(["a"]),
                    builtin_fn(vec![list_a.clone(), list_a.clone()], list_a, span),
                    span,
                )
            }
            "list.map" => {
                let a = builtin_param(0, "a", span);
                let b = builtin_param(1, "b", span);
                builtin_forall(
                    Vec::from(["a", "b"]),
                    builtin_fn(
                        vec![
                            builtin_fn(vec![a.clone()], b.clone(), span),
                            builtin_list(a, span),
                        ],
                        builtin_list(b, span),
                        span,
                    ),
                    span,
                )
            }
            "list.filter" => {
                let a = builtin_param(0, "a", span);
                builtin_forall(
                    Vec::from(["a"]),
                    builtin_fn(
                        vec![
                            builtin_fn(vec![a.clone()], builtin_ty(TyKind::Bool, span), span),
                            builtin_list(a.clone(), span),
                        ],
                        builtin_list(a, span),
                        span,
                    ),
                    span,
                )
            }
            "list.fold" => {
                let a = builtin_param(0, "a", span);
                let b = builtin_param(1, "b", span);
                builtin_forall(
                    Vec::from(["a", "b"]),
                    builtin_fn(
                        vec![
                            b.clone(),
                            builtin_fn(vec![b.clone(), a.clone()], b.clone(), span),
                            builtin_list(a, span),
                        ],
                        b,
                        span,
                    ),
                    span,
                )
            }
            "list.range" => builtin_fn(
                vec![builtin_ty(TyKind::Int, span), builtin_ty(TyKind::Int, span)],
                builtin_list(builtin_ty(TyKind::Int, span), span),
                span,
            ),
            "string.len" => builtin_fn(
                vec![builtin_ty(TyKind::String, span)],
                builtin_ty(TyKind::Int, span),
                span,
            ),
            "string.split" => builtin_fn(
                vec![
                    builtin_ty(TyKind::String, span),
                    builtin_ty(TyKind::String, span),
                ],
                builtin_list(builtin_ty(TyKind::String, span), span),
                span,
            ),
            "string.join" => builtin_fn(
                vec![
                    builtin_list(builtin_ty(TyKind::String, span), span),
                    builtin_ty(TyKind::String, span),
                ],
                builtin_ty(TyKind::String, span),
                span,
            ),
            "string.trim" | "string.upper" | "string.lower" => builtin_fn(
                vec![builtin_ty(TyKind::String, span)],
                builtin_ty(TyKind::String, span),
                span,
            ),
            "string.contains" | "string.startsWith" | "string.endsWith" => builtin_fn(
                vec![
                    builtin_ty(TyKind::String, span),
                    builtin_ty(TyKind::String, span),
                ],
                builtin_ty(TyKind::Bool, span),
                span,
            ),
            "string.replace" => builtin_fn(
                vec![
                    builtin_ty(TyKind::String, span),
                    builtin_ty(TyKind::String, span),
                    builtin_ty(TyKind::String, span),
                ],
                builtin_ty(TyKind::String, span),
                span,
            ),
            "string.substring" => builtin_fn(
                vec![
                    builtin_ty(TyKind::String, span),
                    builtin_ty(TyKind::Int, span),
                    builtin_ty(TyKind::Int, span),
                ],
                builtin_ty(TyKind::String, span),
                span,
            ),
            "string.isEmpty" => builtin_fn(
                vec![builtin_ty(TyKind::String, span)],
                builtin_ty(TyKind::Bool, span),
                span,
            ),
            "string.repeat" => builtin_fn(
                vec![
                    builtin_ty(TyKind::String, span),
                    builtin_ty(TyKind::Int, span),
                ],
                builtin_ty(TyKind::String, span),
                span,
            ),
            "string.lines" => builtin_fn(
                vec![builtin_ty(TyKind::String, span)],
                builtin_list(builtin_ty(TyKind::String, span), span),
                span,
            ),
            "string.chars" => builtin_fn(
                vec![builtin_ty(TyKind::String, span)],
                builtin_list(builtin_ty(TyKind::Char, span), span),
                span,
            ),
            "option.some" => {
                let a = builtin_param(0, "a", span);
                builtin_forall(
                    Vec::from(["a"]),
                    builtin_fn(vec![a.clone()], builtin_option(a, span), span),
                    span,
                )
            }
            "option.none" => {
                let a = builtin_param(0, "a", span);
                builtin_forall(Vec::from(["a"]), builtin_option(a, span), span)
            }
            "option.is_some" | "option.is_none" => {
                let a = builtin_param(0, "a", span);
                builtin_forall(
                    Vec::from(["a"]),
                    builtin_fn(
                        vec![builtin_option(a, span)],
                        builtin_ty(TyKind::Bool, span),
                        span,
                    ),
                    span,
                )
            }
            "option.unwrap" => {
                let a = builtin_param(0, "a", span);
                builtin_forall(
                    Vec::from(["a"]),
                    builtin_fn(vec![builtin_option(a.clone(), span)], a, span),
                    span,
                )
            }
            "option.unwrap_or" => {
                let a = builtin_param(0, "a", span);
                builtin_forall(
                    Vec::from(["a"]),
                    builtin_fn(vec![builtin_option(a.clone(), span), a.clone()], a, span),
                    span,
                )
            }
            "result.ok" => {
                let t = builtin_param(0, "t", span);
                let e = builtin_param(1, "e", span);
                builtin_forall(
                    Vec::from(["t", "e"]),
                    builtin_fn(vec![t.clone()], builtin_result(t, e, span), span),
                    span,
                )
            }
            "result.err" => {
                let t = builtin_param(0, "t", span);
                let e = builtin_param(1, "e", span);
                builtin_forall(
                    Vec::from(["t", "e"]),
                    builtin_fn(vec![e.clone()], builtin_result(t, e, span), span),
                    span,
                )
            }
            "result.is_ok" | "result.is_err" => {
                let t = builtin_param(0, "t", span);
                let e = builtin_param(1, "e", span);
                builtin_forall(
                    Vec::from(["t", "e"]),
                    builtin_fn(
                        vec![builtin_result(t, e, span)],
                        builtin_ty(TyKind::Bool, span),
                        span,
                    ),
                    span,
                )
            }
            "result.unwrap" => {
                let t = builtin_param(0, "t", span);
                let e = builtin_param(1, "e", span);
                builtin_forall(
                    Vec::from(["t", "e"]),
                    builtin_fn(vec![builtin_result(t.clone(), e, span)], t, span),
                    span,
                )
            }
            "result.unwrap_err" => {
                let t = builtin_param(0, "t", span);
                let e = builtin_param(1, "e", span);
                builtin_forall(
                    Vec::from(["t", "e"]),
                    builtin_fn(vec![builtin_result(t, e.clone(), span)], e, span),
                    span,
                )
            }
            "math.pi" | "math.e" | "math.inf" | "math.nan" => builtin_ty(TyKind::Float, span),
            "path.join" => builtin_fn(
                vec![
                    builtin_ty(TyKind::String, span),
                    builtin_ty(TyKind::String, span),
                ],
                builtin_ty(TyKind::String, span),
                span,
            ),
            "path.parent" | "path.filename" | "path.extension" => builtin_fn(
                vec![builtin_ty(TyKind::String, span)],
                builtin_option(builtin_ty(TyKind::String, span), span),
                span,
            ),
            "path.is_absolute" => builtin_fn(
                vec![builtin_ty(TyKind::String, span)],
                builtin_ty(TyKind::Bool, span),
                span,
            ),
            "io.readFile" | "io.hashFile" => builtin_fn(
                vec![builtin_ty(TyKind::String, span)],
                builtin_ty(TyKind::String, span),
                span,
            ),
            "io.readDir" => builtin_fn(
                vec![builtin_ty(TyKind::String, span)],
                builtin_string_list(span),
                span,
            ),
            "io.writeFile" | "io.appendFile" => builtin_fn(
                vec![
                    builtin_ty(TyKind::String, span),
                    builtin_ty(TyKind::String, span),
                ],
                builtin_ty(TyKind::Unit, span),
                span,
            ),
            "io.createDirAll" | "io.removeDirAll" | "io.pathExists" | "io.isDir" | "io.isFile" => {
                let ret_ty = match name {
                    "io.createDirAll" | "io.removeDirAll" => builtin_ty(TyKind::Unit, span),
                    _ => builtin_ty(TyKind::Bool, span),
                };
                builtin_fn(vec![builtin_ty(TyKind::String, span)], ret_ty, span)
            }
            "io.getEnv" => builtin_fn(
                vec![builtin_ty(TyKind::String, span)],
                builtin_string_option(span),
                span,
            ),
            "io.currentDir" | "io.currentSystem" => {
                builtin_fn(Vec::new(), builtin_ty(TyKind::String, span), span)
            }
            "io.homeDir" => builtin_fn(Vec::new(), builtin_string_option(span), span),
            "io.exec" => builtin_fn(
                vec![builtin_ty(TyKind::String, span), builtin_string_list(span)],
                builtin_exec_result(span),
                span,
            ),
            "io.execShell" => builtin_fn(
                vec![builtin_ty(TyKind::String, span)],
                builtin_exec_result(span),
                span,
            ),
            "fetch.url" | "fetch.path" => builtin_fn(
                vec![builtin_ty(TyKind::String, span)],
                builtin_fetch_result(span),
                span,
            ),
            "fetch.urlWithHash" | "fetch.pathWithHash" | "fetch.git" => builtin_fn(
                vec![
                    builtin_ty(TyKind::String, span),
                    builtin_ty(TyKind::String, span),
                ],
                builtin_fetch_result(span),
                span,
            ),
            "fetch.gitWithHash" => builtin_fn(
                vec![
                    builtin_ty(TyKind::String, span),
                    builtin_ty(TyKind::String, span),
                    builtin_ty(TyKind::String, span),
                ],
                builtin_fetch_result(span),
                span,
            ),
            "Map.empty" => {
                let k = builtin_param(0, "k", span);
                let v = builtin_param(1, "v", span);
                builtin_forall(Vec::from(["k", "v"]), builtin_map(k, v, span), span)
            }
            "Map.singleton" => {
                let k = builtin_param(0, "k", span);
                let v = builtin_param(1, "v", span);
                builtin_forall(
                    Vec::from(["k", "v"]),
                    builtin_fn(vec![k.clone(), v.clone()], builtin_map(k, v, span), span),
                    span,
                )
            }
            "Map.fromList" => {
                let k = builtin_param(0, "k", span);
                let v = builtin_param(1, "v", span);
                let pair = builtin_ty(TyKind::Tuple(vec![k.clone(), v.clone()]), span);
                builtin_forall(
                    Vec::from(["k", "v"]),
                    builtin_fn(
                        vec![builtin_list(pair, span)],
                        builtin_map(k, v, span),
                        span,
                    ),
                    span,
                )
            }
            "Map.get" => {
                let k = builtin_param(0, "k", span);
                let v = builtin_param(1, "v", span);
                builtin_forall(
                    Vec::from(["k", "v"]),
                    builtin_fn(
                        vec![k.clone(), builtin_map(k, v.clone(), span)],
                        builtin_option(v, span),
                        span,
                    ),
                    span,
                )
            }
            "Map.getWithDefault" => {
                let k = builtin_param(0, "k", span);
                let v = builtin_param(1, "v", span);
                builtin_forall(
                    Vec::from(["k", "v"]),
                    builtin_fn(
                        vec![k.clone(), v.clone(), builtin_map(k, v.clone(), span)],
                        v,
                        span,
                    ),
                    span,
                )
            }
            "Map.contains" => {
                let k = builtin_param(0, "k", span);
                let v = builtin_param(1, "v", span);
                builtin_forall(
                    Vec::from(["k", "v"]),
                    builtin_fn(
                        vec![k.clone(), builtin_map(k, v, span)],
                        builtin_ty(TyKind::Bool, span),
                        span,
                    ),
                    span,
                )
            }
            "Map.size" | "Map.isEmpty" => {
                let k = builtin_param(0, "k", span);
                let v = builtin_param(1, "v", span);
                let ret_ty = match name {
                    "Map.size" => builtin_ty(TyKind::Int, span),
                    _ => builtin_ty(TyKind::Bool, span),
                };
                builtin_forall(
                    Vec::from(["k", "v"]),
                    builtin_fn(vec![builtin_map(k, v, span)], ret_ty, span),
                    span,
                )
            }
            "Map.insert" => {
                let k = builtin_param(0, "k", span);
                let v = builtin_param(1, "v", span);
                builtin_forall(
                    Vec::from(["k", "v"]),
                    builtin_fn(
                        vec![
                            k.clone(),
                            v.clone(),
                            builtin_map(k.clone(), v.clone(), span),
                        ],
                        builtin_map(k, v, span),
                        span,
                    ),
                    span,
                )
            }
            "Map.remove" => {
                let k = builtin_param(0, "k", span);
                let v = builtin_param(1, "v", span);
                builtin_forall(
                    Vec::from(["k", "v"]),
                    builtin_fn(
                        vec![k.clone(), builtin_map(k.clone(), v.clone(), span)],
                        builtin_map(k, v, span),
                        span,
                    ),
                    span,
                )
            }
            "Map.union" | "Map.intersection" | "Map.difference" => {
                let k = builtin_param(0, "k", span);
                let v = builtin_param(1, "v", span);
                let map_kv = builtin_map(k, v, span);
                builtin_forall(
                    Vec::from(["k", "v"]),
                    builtin_fn(vec![map_kv.clone(), map_kv.clone()], map_kv, span),
                    span,
                )
            }
            "Set.empty" => {
                let a = builtin_param(0, "a", span);
                builtin_forall(Vec::from(["a"]), builtin_set(a, span), span)
            }
            "Set.singleton" => {
                let a = builtin_param(0, "a", span);
                builtin_forall(
                    Vec::from(["a"]),
                    builtin_fn(vec![a.clone()], builtin_set(a, span), span),
                    span,
                )
            }
            "Set.fromList" => {
                let a = builtin_param(0, "a", span);
                builtin_forall(
                    Vec::from(["a"]),
                    builtin_fn(
                        vec![builtin_list(a.clone(), span)],
                        builtin_set(a, span),
                        span,
                    ),
                    span,
                )
            }
            "Set.contains" => {
                let a = builtin_param(0, "a", span);
                builtin_forall(
                    Vec::from(["a"]),
                    builtin_fn(
                        vec![a.clone(), builtin_set(a, span)],
                        builtin_ty(TyKind::Bool, span),
                        span,
                    ),
                    span,
                )
            }
            "Set.size" | "Set.isEmpty" => {
                let a = builtin_param(0, "a", span);
                let ret_ty = match name {
                    "Set.size" => builtin_ty(TyKind::Int, span),
                    _ => builtin_ty(TyKind::Bool, span),
                };
                builtin_forall(
                    Vec::from(["a"]),
                    builtin_fn(vec![builtin_set(a, span)], ret_ty, span),
                    span,
                )
            }
            "Set.insert" | "Set.remove" => {
                let a = builtin_param(0, "a", span);
                builtin_forall(
                    Vec::from(["a"]),
                    builtin_fn(
                        vec![a.clone(), builtin_set(a.clone(), span)],
                        builtin_set(a, span),
                        span,
                    ),
                    span,
                )
            }
            "Set.union" | "Set.intersection" | "Set.difference" | "Set.symmetricDifference" => {
                let a = builtin_param(0, "a", span);
                let set_a = builtin_set(a, span);
                builtin_forall(
                    Vec::from(["a"]),
                    builtin_fn(vec![set_a.clone(), set_a.clone()], set_a, span),
                    span,
                )
            }
            "Set.isSubset" | "Set.isSuperset" | "Set.isDisjoint" => {
                let a = builtin_param(0, "a", span);
                let set_a = builtin_set(a, span);
                builtin_forall(
                    Vec::from(["a"]),
                    builtin_fn(
                        vec![set_a.clone(), set_a],
                        builtin_ty(TyKind::Bool, span),
                        span,
                    ),
                    span,
                )
            }
            _ if name.contains('.') => return Some(self.fresh_var()),
            _ => return None,
        };

        Some(instantiate(&polymorphic, &mut || self.fresh_var()))
    }

    fn enum_has_variants(&self, def_id: DefId, required: &[&str]) -> bool {
        let Some(info) = self.enums.get(&def_id) else {
            return false;
        };
        required
            .iter()
            .all(|name| info.variants.contains_key(*name))
    }

    fn try_payload_type(&self, def_id: DefId, variant_name: &str, span: Span) -> Option<Ty> {
        self.enums
            .get(&def_id)?
            .variants
            .get(variant_name)?
            .first()
            .cloned()
            .map(|mut ty| {
                ty.span = span;
                ty
            })
    }

    fn try_result_type(&mut self, inner_ty: Ty, span: Span) -> Ty {
        let inner_ty = self.apply(&inner_ty);
        match inner_ty.kind {
            TyKind::Named(def_id, args) if is_builtin_option_type(def_id) => {
                args.into_iter().next().unwrap_or_else(|| self.fresh_var())
            }
            TyKind::Named(def_id, args) if is_builtin_result_type(def_id) => {
                args.into_iter().next().unwrap_or_else(|| self.fresh_var())
            }
            TyKind::Named(def_id, _) if self.enum_has_variants(def_id, &["Some", "None"]) => self
                .try_payload_type(def_id, "Some", span)
                .unwrap_or_else(|| self.fresh_var()),
            TyKind::Named(def_id, _) if self.enum_has_variants(def_id, &["Ok", "Err"]) => self
                .try_payload_type(def_id, "Ok", span)
                .unwrap_or_else(|| self.fresh_var()),
            TyKind::Var(_) | TyKind::Unknown => self.fresh_var(),
            _ => {
                self.error(span, "`?` expects Option-like or Result-like value");
                self.fresh_var()
            }
        }
    }

    fn coalesce_result_type(&mut self, value_ty: Ty, default_ty: Ty, span: Span) -> Ty {
        let value_ty = self.apply(&value_ty);
        match value_ty.kind {
            TyKind::Named(def_id, args) if is_builtin_option_type(def_id) => {
                let payload_ty = args.into_iter().next().unwrap_or_else(|| self.fresh_var());
                self.unify(&payload_ty, &default_ty, span);
                self.apply(&payload_ty)
            }
            TyKind::Named(def_id, _) if self.enum_has_variants(def_id, &["Some", "None"]) => {
                let payload_ty = self
                    .try_payload_type(def_id, "Some", span)
                    .unwrap_or_else(|| self.fresh_var());
                self.unify(&payload_ty, &default_ty, span);
                self.apply(&payload_ty)
            }
            _ => {
                self.unify(&value_ty, &default_ty, span);
                self.apply(&value_ty)
            }
        }
    }

    fn bool_pattern_coverage(&self, pattern: &Pattern) -> (bool, bool) {
        match &pattern.kind {
            PatternKind::Wildcard | PatternKind::Var(_, _) => (true, true),
            PatternKind::Binding(_, _, inner) => self.bool_pattern_coverage(inner),
            PatternKind::Literal(Literal::Bool(value)) => (*value, !*value),
            PatternKind::Or(patterns) => patterns.iter().fold((false, false), |(t, f), pattern| {
                let (covers_true, covers_false) = self.bool_pattern_coverage(pattern);
                (t || covers_true, f || covers_false)
            }),
            _ => (false, false),
        }
    }

    fn pattern_covers_unit(&self, pattern: &Pattern) -> bool {
        match &pattern.kind {
            PatternKind::Wildcard | PatternKind::Var(_, _) => true,
            PatternKind::Binding(_, _, inner) => self.pattern_covers_unit(inner),
            PatternKind::Literal(Literal::Unit) => true,
            PatternKind::Or(patterns) => patterns
                .iter()
                .any(|pattern| self.pattern_covers_unit(pattern)),
            _ => false,
        }
    }

    fn pattern_is_irrefutable_for(&self, pattern: &Pattern, expected: &Ty) -> bool {
        let expected = self.apply(expected);
        match &pattern.kind {
            PatternKind::Wildcard | PatternKind::Var(_, _) => true,
            PatternKind::Binding(_, _, inner) => self.pattern_is_irrefutable_for(inner, &expected),
            PatternKind::Literal(Literal::Unit) => matches!(expected.kind, TyKind::Unit),
            PatternKind::Tuple(patterns) => match &expected.kind {
                TyKind::Tuple(elem_tys) if elem_tys.len() == patterns.len() => patterns
                    .iter()
                    .zip(elem_tys.iter())
                    .all(|(pattern, ty)| self.pattern_is_irrefutable_for(pattern, ty)),
                _ => false,
            },
            PatternKind::Constructor(def_id, patterns) => {
                let Some(variant) = self.variants.get(def_id) else {
                    return false;
                };
                let TyKind::Named(enum_id, _) = expected.kind else {
                    return false;
                };
                if variant.enum_id != enum_id {
                    return false;
                }
                let Some(enum_info) = self.enums.get(&enum_id) else {
                    return false;
                };
                if enum_info.variants.len() != 1 || variant.fields.len() != patterns.len() {
                    return false;
                }
                patterns
                    .iter()
                    .zip(variant.fields.iter())
                    .all(|(pattern, ty)| self.pattern_is_irrefutable_for(pattern, ty))
            }
            PatternKind::Or(patterns) => {
                let (covers_true, covers_false) = self.bool_pattern_coverage(pattern);
                matches!(expected.kind, TyKind::Bool) && covers_true && covers_false
                    || patterns
                        .iter()
                        .any(|pattern| self.pattern_is_irrefutable_for(pattern, &expected))
            }
            _ => false,
        }
    }

    fn pattern_covers_enum_variant(
        &self,
        pattern: &Pattern,
        enum_id: DefId,
        variant_name: &str,
    ) -> bool {
        match &pattern.kind {
            PatternKind::Wildcard | PatternKind::Var(_, _) => true,
            PatternKind::Binding(_, _, inner) => {
                self.pattern_covers_enum_variant(inner, enum_id, variant_name)
            }
            PatternKind::Constructor(def_id, patterns) => {
                let Some(variant) = self.variants.get(def_id) else {
                    return false;
                };
                if variant.enum_id != enum_id
                    || variant.name != variant_name
                    || variant.fields.len() != patterns.len()
                {
                    return false;
                }
                patterns
                    .iter()
                    .zip(variant.fields.iter())
                    .all(|(pattern, ty)| self.pattern_is_irrefutable_for(pattern, ty))
            }
            PatternKind::Or(patterns) => patterns
                .iter()
                .any(|pattern| self.pattern_covers_enum_variant(pattern, enum_id, variant_name)),
            _ => false,
        }
    }

    fn pattern_covers_builtin_variant(
        &self,
        pattern: &Pattern,
        variant_name: &str,
        payload_ty: Option<&Ty>,
    ) -> bool {
        match &pattern.kind {
            PatternKind::Wildcard | PatternKind::Var(_, _) => true,
            PatternKind::Binding(_, _, inner) => {
                self.pattern_covers_builtin_variant(inner, variant_name, payload_ty)
            }
            PatternKind::Constructor(def_id, patterns) => {
                match (
                    builtin_constructor_name(*def_id),
                    variant_name,
                    patterns.as_slice(),
                ) {
                    (Some("Some"), "Some", [pattern])
                    | (Some("Ok"), "Ok", [pattern])
                    | (Some("Err"), "Err", [pattern]) => {
                        payload_ty.is_some_and(|ty| self.pattern_is_irrefutable_for(pattern, ty))
                    }
                    (Some("None"), "None", []) => true,
                    _ => false,
                }
            }
            PatternKind::Or(patterns) => patterns.iter().any(|pattern| {
                self.pattern_covers_builtin_variant(pattern, variant_name, payload_ty)
            }),
            _ => false,
        }
    }

    fn missing_enum_patterns(&self, enum_id: DefId, covered_variants: &[String]) -> Vec<String> {
        let Some(enum_info) = self.enums.get(&enum_id) else {
            return Vec::new();
        };

        enum_info
            .variants
            .iter()
            .filter(|(name, _)| !covered_variants.iter().any(|covered| covered == *name))
            .map(|(name, fields)| {
                if fields.is_empty() {
                    name.clone()
                } else {
                    let placeholders = std::iter::repeat_n("_", fields.len()).collect::<Vec<_>>();
                    format!("{}({})", name, placeholders.join(", "))
                }
            })
            .collect()
    }

    fn check_match_coverage(&mut self, scrutinee_ty: &Ty, arms: &[MatchArm], span: Span) {
        let scrutinee_ty = self.apply(scrutinee_ty);
        let mut coverage_complete_span = None;

        match &scrutinee_ty.kind {
            TyKind::Bool => {
                let mut covers_true = false;
                let mut covers_false = false;

                for arm in arms {
                    if let Some(previous_span) = coverage_complete_span {
                        self.diagnostics
                            .push(unreachable_pattern(arm.span, previous_span));
                        continue;
                    }
                    if arm.guard.is_some() {
                        continue;
                    }
                    if self.pattern_is_irrefutable_for(&arm.pattern, &scrutinee_ty) {
                        coverage_complete_span = Some(arm.span);
                        covers_true = true;
                        covers_false = true;
                        continue;
                    }

                    let (arm_true, arm_false) = self.bool_pattern_coverage(&arm.pattern);
                    covers_true |= arm_true;
                    covers_false |= arm_false;

                    if covers_true && covers_false {
                        coverage_complete_span = Some(arm.span);
                    }
                }

                let mut missing = Vec::new();
                if !covers_true {
                    missing.push("true".to_string());
                }
                if !covers_false {
                    missing.push("false".to_string());
                }
                if !missing.is_empty() {
                    self.diagnostics.push(non_exhaustive_match(&missing, span));
                }
            }

            TyKind::Unit => {
                let mut covered = false;

                for arm in arms {
                    if let Some(previous_span) = coverage_complete_span {
                        self.diagnostics
                            .push(unreachable_pattern(arm.span, previous_span));
                        continue;
                    }
                    if arm.guard.is_some() {
                        continue;
                    }
                    if self.pattern_is_irrefutable_for(&arm.pattern, &scrutinee_ty) {
                        coverage_complete_span = Some(arm.span);
                        covered = true;
                        continue;
                    }

                    covered |= self.pattern_covers_unit(&arm.pattern);

                    if covered {
                        coverage_complete_span = Some(arm.span);
                    }
                }

                if !covered {
                    self.diagnostics
                        .push(non_exhaustive_match(&["()".to_string()], span));
                }
            }

            TyKind::Named(def_id, args) if is_builtin_option_type(*def_id) => {
                let payload_ty = args.first().cloned();
                let mut covers_some = false;
                let mut covers_none = false;

                for arm in arms {
                    if let Some(previous_span) = coverage_complete_span {
                        self.diagnostics
                            .push(unreachable_pattern(arm.span, previous_span));
                        continue;
                    }
                    if arm.guard.is_some() {
                        continue;
                    }
                    if self.pattern_is_irrefutable_for(&arm.pattern, &scrutinee_ty) {
                        coverage_complete_span = Some(arm.span);
                        covers_some = true;
                        covers_none = true;
                        continue;
                    }

                    covers_some |= self.pattern_covers_builtin_variant(
                        &arm.pattern,
                        "Some",
                        payload_ty.as_ref(),
                    );
                    covers_none |= self.pattern_covers_builtin_variant(&arm.pattern, "None", None);

                    if covers_some && covers_none {
                        coverage_complete_span = Some(arm.span);
                    }
                }

                let mut missing = Vec::new();
                if !covers_some {
                    missing.push("Some(_)".to_string());
                }
                if !covers_none {
                    missing.push("None".to_string());
                }
                if !missing.is_empty() {
                    self.diagnostics.push(non_exhaustive_match(&missing, span));
                }
            }

            TyKind::Named(def_id, args) if is_builtin_result_type(*def_id) => {
                let ok_ty = args.first().cloned();
                let err_ty = args.get(1).cloned();
                let mut covers_ok = false;
                let mut covers_err = false;

                for arm in arms {
                    if let Some(previous_span) = coverage_complete_span {
                        self.diagnostics
                            .push(unreachable_pattern(arm.span, previous_span));
                        continue;
                    }
                    if arm.guard.is_some() {
                        continue;
                    }
                    if self.pattern_is_irrefutable_for(&arm.pattern, &scrutinee_ty) {
                        coverage_complete_span = Some(arm.span);
                        covers_ok = true;
                        covers_err = true;
                        continue;
                    }

                    covers_ok |=
                        self.pattern_covers_builtin_variant(&arm.pattern, "Ok", ok_ty.as_ref());
                    covers_err |=
                        self.pattern_covers_builtin_variant(&arm.pattern, "Err", err_ty.as_ref());

                    if covers_ok && covers_err {
                        coverage_complete_span = Some(arm.span);
                    }
                }

                let mut missing = Vec::new();
                if !covers_ok {
                    missing.push("Ok(_)".to_string());
                }
                if !covers_err {
                    missing.push("Err(_)".to_string());
                }
                if !missing.is_empty() {
                    self.diagnostics.push(non_exhaustive_match(&missing, span));
                }
            }

            TyKind::Named(enum_id, _) if self.enums.contains_key(enum_id) => {
                let mut covered_variants = Vec::new();

                for arm in arms {
                    if let Some(previous_span) = coverage_complete_span {
                        self.diagnostics
                            .push(unreachable_pattern(arm.span, previous_span));
                        continue;
                    }
                    if arm.guard.is_some() {
                        continue;
                    }
                    if self.pattern_is_irrefutable_for(&arm.pattern, &scrutinee_ty) {
                        coverage_complete_span = Some(arm.span);
                        if let Some(enum_info) = self.enums.get(enum_id) {
                            covered_variants = enum_info.variants.keys().cloned().collect();
                        }
                        continue;
                    }

                    let Some(enum_info) = self.enums.get(enum_id) else {
                        continue;
                    };

                    for variant_name in enum_info.variants.keys() {
                        if !covered_variants
                            .iter()
                            .any(|covered| covered == variant_name)
                            && self.pattern_covers_enum_variant(
                                &arm.pattern,
                                *enum_id,
                                variant_name,
                            )
                        {
                            covered_variants.push(variant_name.clone());
                        }
                    }

                    if covered_variants.len() == enum_info.variants.len() {
                        coverage_complete_span = Some(arm.span);
                    }
                }

                let missing = self.missing_enum_patterns(*enum_id, &covered_variants);
                if !missing.is_empty() {
                    self.diagnostics.push(non_exhaustive_match(&missing, span));
                }
            }

            _ => {}
        }
    }

    /// Check if a type variable has been resolved.
    /// 检查类型变量是否已被解析。
    pub fn is_resolved(&self, var: u32) -> bool {
        self.subst.get(var).is_some()
    }

    /// Get the resolved type for a type variable, if any.
    /// 获取类型变量的解析结果（如果有）。
    pub fn get_resolved(&self, var: u32) -> Option<Ty> {
        self.subst.get(var).map(|ty| self.apply(ty))
    }

    /// Check if a generic parameter has been bound.
    /// 检查泛型参数是否已被绑定。
    pub fn is_param_bound(&self, idx: u32) -> bool {
        self.subst.get_param(idx).is_some()
    }

    /// Get the bound type for a generic parameter, if any.
    /// 获取泛型参数的绑定类型（如果有）。
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

    // ===== First pass: collect signatures 第一遍：收集签名 =====

    fn collect_item(&mut self, item: &Item) {
        // Record span for all global definitions (for error reporting)
        // 记录所有全局定义的位置信息（用于错误报告）
        self.global_spans.insert(item.id, item.span);

        match &item.kind {
            ItemKind::Fn(fn_def) => {
                if fn_def.params.is_empty() {
                    let mut value_ty = self.resolve_type(&fn_def.return_ty);
                    if !fn_def.generics.is_empty() {
                        let params: Vec<String> =
                            fn_def.generics.iter().map(|g| g.name.clone()).collect();
                        value_ty = Ty {
                            kind: TyKind::Forall(params, Box::new(value_ty)),
                            span: Span::DUMMY,
                        };
                    }
                    self.globals.insert(item.id, value_ty);
                } else {
                    let fn_ty = self.fn_signature(fn_def);
                    self.globals.insert(item.id, fn_ty);
                }
            }
            ItemKind::Trait(trait_def) => {
                self.collect_trait(item.id, trait_def);
            }
            ItemKind::Impl(impl_def) => {
                self.collect_impl(item.id, impl_def);
            }
            ItemKind::Struct(struct_def) => {
                self.collect_struct(item.id, struct_def);
            }
            ItemKind::Enum(enum_def) => {
                self.collect_enum(item.id, enum_def);
            }
            ItemKind::TypeAlias(type_alias) => {
                self.collect_type_alias(item.id, type_alias);
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

    /// Collect struct type definition.
    /// 收集结构体类型定义。
    fn collect_struct(&mut self, def_id: DefId, struct_def: &StructDef) {
        let mut fields = HashMap::new();
        for field in &struct_def.fields {
            fields.insert(field.name.clone(), field.ty.clone());
        }

        let info = StructInfo { fields };

        self.structs.insert(def_id, info);

        // Register the struct type in globals as a type constructor
        // 将结构体类型注册为类型构造函数
        let struct_ty = Ty {
            kind: TyKind::Named(def_id, Vec::new()),
            span: Span::DUMMY,
        };
        self.globals.insert(def_id, struct_ty);
    }

    /// Collect enum type definition.
    /// 收集枚举类型定义。
    fn collect_enum(&mut self, def_id: DefId, enum_def: &EnumDef) {
        let mut variants = HashMap::new();
        for variant in &enum_def.variants {
            variants.insert(variant.name.clone(), variant.fields.clone());
        }

        let info = EnumInfo { variants };

        self.enums.insert(def_id, info);

        // Register the enum type in globals as a type constructor
        // 将枚举类型注册为类型构造函数
        let enum_ty = Ty {
            kind: TyKind::Named(def_id, Vec::new()),
            span: Span::DUMMY,
        };
        self.globals.insert(def_id, enum_ty);

        // Register variant constructors
        // 注册变体构造器
        for variant in &enum_def.variants {
            let fields = variant.fields.clone();
            self.variants.insert(
                variant.id,
                VariantInfo {
                    enum_id: def_id,
                    name: variant.name.clone(),
                    fields: fields.clone(),
                },
            );

            let ctor_ty = Ty {
                kind: TyKind::Fn(
                    fields,
                    Box::new(Ty {
                        kind: TyKind::Named(def_id, Vec::new()),
                        span: Span::DUMMY,
                    }),
                ),
                span: Span::DUMMY,
            };
            self.globals.insert(variant.id, ctor_ty);
        }
    }

    /// Collect type alias definition.
    /// 收集类型别名定义。
    fn collect_type_alias(&mut self, def_id: DefId, type_alias: &TypeAlias) {
        let info = TypeAliasInfo {
            target: type_alias.ty.clone(),
        };

        self.type_aliases.insert(def_id, info);

        // Register the alias as pointing to the target type
        // 将别名注册为指向目标类型
        self.globals.insert(def_id, type_alias.ty.clone());
    }

    fn fn_signature(&mut self, fn_def: &FnDef) -> Ty {
        let param_tys: Vec<Ty> = fn_def
            .params
            .iter()
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
            let params: Vec<String> = fn_def.generics.iter().map(|g| g.name.clone()).collect();
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
            TyKind::SelfType => Ty {
                kind: TyKind::SelfType,
                span: ty.span,
            },
            TyKind::SelfAssoc(name) => Ty {
                kind: TyKind::SelfAssoc(name.clone()),
                span: ty.span,
            },
            TyKind::Named(id, args) => {
                let resolved_args: Vec<Ty> = args.iter().map(|a| self.resolve_type(a)).collect();
                Ty {
                    kind: TyKind::Named(*id, resolved_args),
                    span: ty.span,
                }
            }
            TyKind::Fn(params, ret) => {
                let resolved_params: Vec<Ty> =
                    params.iter().map(|p| self.resolve_type(p)).collect();
                Ty {
                    kind: TyKind::Fn(resolved_params, Box::new(self.resolve_type(ret))),
                    span: ty.span,
                }
            }
            TyKind::Tuple(elems) => {
                let resolved_elems: Vec<Ty> = elems.iter().map(|e| self.resolve_type(e)).collect();
                Ty {
                    kind: TyKind::Tuple(resolved_elems),
                    span: ty.span,
                }
            }
            _ => ty.clone(),
        }
    }

    // ===== Second pass: check bodies 第二遍：检查函数体 =====

    fn check_item(&mut self, item: &Item) {
        match &item.kind {
            ItemKind::Fn(fn_def) => self.check_fn(item.id, fn_def),
            ItemKind::Impl(impl_def) => self.check_impl_methods(impl_def),
            _ => {}
        }
    }

    fn check_fn(&mut self, id: DefId, fn_def: &FnDef) {
        // Create fresh type variables for generic parameters
        let mut generic_vars: HashMap<String, Ty> = HashMap::new();
        for (idx, param) in fn_def.generics.iter().enumerate() {
            let var = self.fresh_var();
            generic_vars.insert(param.name.clone(), var.clone());
            self.subst.bind_param(idx as u32, var);
        }

        // Bind parameter types (resolving generic references)
        // Parameters are considered used by default (they're part of the function signature)
        let mut param_tys = Vec::with_capacity(fn_def.params.len());
        for param in &fn_def.params {
            let ty = self.resolve_type_with_generics(&param.ty, &generic_vars);
            param_tys.push(ty.clone());
            self.local_definitions.insert(param.id, ty.clone());
            self.locals.insert(
                param.id,
                LocalInfo {
                    ty,
                    name: param.name.clone(),
                    span: param.span,
                    used: true, // Parameters are always "used"
                },
            );
        }

        // Infer body type
        let body_ty = self.infer_expr(&fn_def.body);

        // Unify with declared return type
        let ret_ty = self.resolve_type_with_generics(&fn_def.return_ty, &generic_vars);
        if !self.unify(&body_ty, &ret_ty, fn_def.body.span) {
            // Emit a more detailed error
            self.emit(
                TypeMismatchError::new(ret_ty.clone(), body_ty.clone(), fn_def.body.span)
                    .with_context("function return type")
                    .build(),
            );
        }

        // Refine the global type after body checking so later items in the same
        // module can see the actual inferred type instead of the placeholder signature.
        let refined_ret_ty = self.apply(&ret_ty);
        let refined_param_tys: Vec<Ty> = param_tys.iter().map(|ty| self.apply(ty)).collect();
        let refined_global_ty = if refined_param_tys.is_empty() {
            refined_ret_ty
        } else {
            Ty {
                kind: TyKind::Fn(refined_param_tys, Box::new(refined_ret_ty)),
                span: Span::DUMMY,
            }
        };
        let refined_global_ty =
            self.reify_named_generics(refined_global_ty, &fn_def.generics, &generic_vars);
        let refined_global_ty = if fn_def.generics.is_empty() {
            refined_global_ty
        } else {
            let params: Vec<String> = fn_def.generics.iter().map(|g| g.name.clone()).collect();
            Ty {
                kind: TyKind::Forall(params, Box::new(refined_global_ty)),
                span: Span::DUMMY,
            }
        };
        self.globals.insert(id, refined_global_ty);

        // Check for unused variables before clearing
        self.check_unused_locals();

        // Clear locals after checking function
        self.locals.clear();
    }

    fn check_impl_methods(&mut self, impl_def: &ImplDef) {
        let generic_vars = self.fresh_generic_bindings(&impl_def.generics);
        let self_ty = self.resolve_type_with_generics(&impl_def.self_ty, &generic_vars);
        let assoc_types = self.impl_assoc_type_bindings(impl_def, &generic_vars, &self_ty);

        for item in &impl_def.items {
            self.check_impl_item(item, &self_ty, &generic_vars, &assoc_types);
        }
    }

    fn check_impl_item(
        &mut self,
        item: &neve_hir::ImplItem,
        self_ty: &Ty,
        impl_generics: &HashMap<String, Ty>,
        assoc_types: &HashMap<String, Ty>,
    ) {
        let mut generic_vars = impl_generics.clone();
        for param in &item.generics {
            generic_vars.insert(param.name.clone(), self.fresh_var());
        }

        let mut param_tys = Vec::with_capacity(item.params.len());
        for (index, param) in item.params.iter().enumerate() {
            let ty = if index == 0
                && param.name == "self"
                && matches!(param.ty.kind, TyKind::Unknown)
            {
                self_ty.clone()
            } else {
                self.resolve_type_with_context(&param.ty, &generic_vars, Some(self_ty), assoc_types)
            };
            param_tys.push(ty.clone());
            self.local_definitions.insert(param.id, ty.clone());
            self.locals.insert(
                param.id,
                LocalInfo {
                    ty,
                    name: param.name.clone(),
                    span: param.span,
                    used: true,
                },
            );
        }

        let body_ty = self.infer_expr(&item.body);
        let ret_ty = self.resolve_type_with_context(
            &item.return_ty,
            &generic_vars,
            Some(self_ty),
            assoc_types,
        );
        if !self.unify(&body_ty, &ret_ty, item.body.span) {
            self.emit(
                TypeMismatchError::new(ret_ty.clone(), body_ty.clone(), item.body.span)
                    .with_context(format!("impl method `{}` return type", item.name))
                    .build(),
            );
        }

        let refined_ret_ty = self.apply(&ret_ty);
        let refined_param_tys: Vec<Ty> = param_tys.iter().map(|ty| self.apply(ty)).collect();
        let method_ty = Ty {
            kind: TyKind::Fn(refined_param_tys, Box::new(refined_ret_ty)),
            span: Span::DUMMY,
        };
        let method_ty = self.reify_named_generics(method_ty, &item.generics, &generic_vars);
        let method_ty = if item.generics.is_empty() {
            method_ty
        } else {
            let params: Vec<String> = item.generics.iter().map(|g| g.name.clone()).collect();
            Ty {
                kind: TyKind::Forall(params, Box::new(method_ty)),
                span: Span::DUMMY,
            }
        };
        self.globals.insert(item.id, method_ty);

        self.check_unused_locals();
        self.locals.clear();
    }

    fn fresh_generic_bindings(
        &mut self,
        generics: &[neve_hir::GenericParam],
    ) -> HashMap<String, Ty> {
        let mut generic_vars = HashMap::new();
        for param in generics {
            generic_vars.insert(param.name.clone(), self.fresh_var());
        }
        generic_vars
    }

    fn reify_named_generics(
        &self,
        ty: Ty,
        generics: &[neve_hir::GenericParam],
        generic_vars: &HashMap<String, Ty>,
    ) -> Ty {
        let var_to_param: HashMap<u32, (u32, String)> = generics
            .iter()
            .enumerate()
            .filter_map(|(index, param)| {
                match generic_vars.get(&param.name).map(|ty| self.apply(ty)) {
                    Some(Ty {
                        kind: TyKind::Var(var),
                        ..
                    }) => Some((var, (index as u32, param.name.clone()))),
                    _ => None,
                }
            })
            .collect();
        Self::replace_generic_vars_with_params(&ty, &var_to_param)
    }

    fn replace_generic_vars_with_params(ty: &Ty, var_to_param: &HashMap<u32, (u32, String)>) -> Ty {
        match &ty.kind {
            TyKind::Var(var) => {
                if let Some((index, name)) = var_to_param.get(var) {
                    Ty {
                        kind: TyKind::Param(*index, name.clone()),
                        span: ty.span,
                    }
                } else {
                    ty.clone()
                }
            }
            TyKind::Named(id, args) => Ty {
                kind: TyKind::Named(
                    *id,
                    args.iter()
                        .map(|arg| Self::replace_generic_vars_with_params(arg, var_to_param))
                        .collect(),
                ),
                span: ty.span,
            },
            TyKind::Fn(params, ret) => Ty {
                kind: TyKind::Fn(
                    params
                        .iter()
                        .map(|param| Self::replace_generic_vars_with_params(param, var_to_param))
                        .collect(),
                    Box::new(Self::replace_generic_vars_with_params(ret, var_to_param)),
                ),
                span: ty.span,
            },
            TyKind::Tuple(items) => Ty {
                kind: TyKind::Tuple(
                    items
                        .iter()
                        .map(|item| Self::replace_generic_vars_with_params(item, var_to_param))
                        .collect(),
                ),
                span: ty.span,
            },
            TyKind::Record(fields) => Ty {
                kind: TyKind::Record(
                    fields
                        .iter()
                        .map(|(name, field_ty)| {
                            (
                                name.clone(),
                                Self::replace_generic_vars_with_params(field_ty, var_to_param),
                            )
                        })
                        .collect(),
                ),
                span: ty.span,
            },
            TyKind::Forall(params, inner) => Ty {
                kind: TyKind::Forall(
                    params.clone(),
                    Box::new(Self::replace_generic_vars_with_params(inner, var_to_param)),
                ),
                span: ty.span,
            },
            _ => ty.clone(),
        }
    }

    /// Resolve a type, substituting generic parameters with their bound types.
    /// 解析类型，将泛型参数替换为其绑定的类型。
    fn resolve_type_with_generics(&mut self, ty: &Ty, generics: &HashMap<String, Ty>) -> Ty {
        self.resolve_type_with_context(ty, generics, None, &HashMap::new())
    }

    fn resolve_type_with_context(
        &mut self,
        ty: &Ty,
        generics: &HashMap<String, Ty>,
        self_ty: Option<&Ty>,
        assoc_types: &HashMap<String, Ty>,
    ) -> Ty {
        match &ty.kind {
            TyKind::Unknown => self.fresh_var(),
            TyKind::Param(_idx, name) => generics.get(name).cloned().unwrap_or_else(|| {
                self.error(ty.span, format!("unknown generic parameter: {}", name));
                self.fresh_var()
            }),
            TyKind::SelfType => self_ty.cloned().unwrap_or(Ty {
                kind: TyKind::SelfType,
                span: ty.span,
            }),
            TyKind::SelfAssoc(name) => assoc_types.get(name).cloned().unwrap_or_else(|| {
                self.error(ty.span, format!("unknown associated type `Self.{name}`"));
                self.fresh_var()
            }),
            TyKind::Named(id, args) => {
                let resolved_args: Vec<Ty> = args
                    .iter()
                    .map(|a| self.resolve_type_with_context(a, generics, self_ty, assoc_types))
                    .collect();
                Ty {
                    kind: TyKind::Named(*id, resolved_args),
                    span: ty.span,
                }
            }
            TyKind::Fn(params, ret) => {
                let resolved_params: Vec<Ty> = params
                    .iter()
                    .map(|p| self.resolve_type_with_context(p, generics, self_ty, assoc_types))
                    .collect();
                Ty {
                    kind: TyKind::Fn(
                        resolved_params,
                        Box::new(self.resolve_type_with_context(
                            ret,
                            generics,
                            self_ty,
                            assoc_types,
                        )),
                    ),
                    span: ty.span,
                }
            }
            TyKind::Tuple(elems) => {
                let resolved_elems: Vec<Ty> = elems
                    .iter()
                    .map(|e| self.resolve_type_with_context(e, generics, self_ty, assoc_types))
                    .collect();
                Ty {
                    kind: TyKind::Tuple(resolved_elems),
                    span: ty.span,
                }
            }
            TyKind::Record(fields) => {
                let resolved_fields = fields
                    .iter()
                    .map(|(name, field_ty)| {
                        (
                            name.clone(),
                            self.resolve_type_with_context(
                                field_ty,
                                generics,
                                self_ty,
                                assoc_types,
                            ),
                        )
                    })
                    .collect();
                Ty {
                    kind: TyKind::Record(resolved_fields),
                    span: ty.span,
                }
            }
            _ => ty.clone(),
        }
    }

    fn impl_assoc_type_bindings(
        &mut self,
        impl_def: &ImplDef,
        generics: &HashMap<String, Ty>,
        self_ty: &Ty,
    ) -> HashMap<String, Ty> {
        let mut assoc_types = HashMap::new();

        for assoc in &impl_def.assoc_type_impls {
            let resolved =
                self.resolve_type_with_context(&assoc.ty, generics, Some(self_ty), &assoc_types);
            assoc_types.insert(assoc.name.clone(), resolved);
        }

        if let Some(trait_ref) = &impl_def.trait_ref
            && let TyKind::Named(def_id, _) = trait_ref.kind
            && let Some(trait_id) = self.trait_ids.get(&def_id).copied()
            && let Some(trait_info) = self.trait_resolver.get_trait(trait_id)
        {
            let defaults: Vec<(String, Ty)> = trait_info
                .assoc_types
                .iter()
                .filter_map(|assoc| {
                    assoc
                        .default
                        .as_ref()
                        .map(|default| (assoc.name.clone(), default.clone()))
                })
                .collect();
            for (name, default) in defaults {
                if assoc_types.contains_key(&name) {
                    continue;
                }
                let resolved =
                    self.resolve_type_with_context(&default, generics, Some(self_ty), &assoc_types);
                assoc_types.insert(name, resolved);
            }
        }

        assoc_types
    }

    fn infer_expr(&mut self, expr: &Expr) -> Ty {
        let span = expr.span;
        let ty = match &expr.kind {
            ExprKind::Literal(lit) => self.infer_literal(lit),

            ExprKind::Var(local_id) => {
                self.mark_used(*local_id);
                self.get_local(local_id).unwrap_or_else(|| {
                    self.emit(unbound_variable("variable", span, None));
                    self.fresh_var()
                })
            }

            ExprKind::Global(def_id) => {
                if let Some(ty) = self.globals.get(def_id).cloned() {
                    // Instantiate polymorphic types with fresh type variables
                    instantiate(&ty, &mut || self.fresh_var())
                } else if def_id.0 == u32::MAX {
                    self.error(span, "undefined global");
                    self.fresh_var()
                } else {
                    self.fresh_var()
                }
            }

            ExprKind::Builtin(name) => self.builtin_type(name, span).unwrap_or_else(|| {
                self.error(span, format!("unknown builtin: {name}"));
                self.fresh_var()
            }),

            ExprKind::List(items) => {
                let elem_ty = self.fresh_var();
                for item in items {
                    let item_ty = self.infer_expr(item);
                    self.unify(&elem_ty, &item_ty, item.span);
                }
                builtin_list(self.apply(&elem_ty), span)
            }

            ExprKind::Tuple(items) => {
                let elem_tys: Vec<Ty> = items.iter().map(|e| self.infer_expr(e)).collect();
                Ty {
                    kind: TyKind::Tuple(elem_tys),
                    span,
                }
            }

            ExprKind::Record(fields) => {
                let field_tys: Vec<(String, Ty)> = fields
                    .iter()
                    .map(|(name, e)| (name.clone(), self.infer_expr(e)))
                    .collect();
                Ty {
                    kind: TyKind::Record(field_tys),
                    span,
                }
            }

            ExprKind::Lambda(params, body) => {
                // Bind parameter types
                let param_tys: Vec<Ty> = params
                    .iter()
                    .map(|p| {
                        let ty = self.resolve_type(&p.ty);
                        self.local_definitions.insert(p.id, ty.clone());
                        self.locals.insert(
                            p.id,
                            LocalInfo {
                                ty: ty.clone(),
                                name: p.name.clone(),
                                span: p.span,
                                used: true, // Lambda params considered used
                            },
                        );
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
                let arg_tys: Vec<Ty> = args.iter().map(|a| self.infer_expr(a)).collect();

                let ret_ty = self.fresh_var();
                let expected_fn_ty = Ty {
                    kind: TyKind::Fn(arg_tys, Box::new(ret_ty.clone())),
                    span,
                };

                self.unify(&func_ty, &expected_fn_ty, span);
                self.apply(&ret_ty)
            }

            ExprKind::MethodCall {
                receiver,
                method,
                target,
                args,
            } => {
                let receiver_ty = self.infer_expr(receiver);
                let applied_receiver_ty = self.apply(&receiver_ty);

                if let Some(resolution) = self
                    .trait_resolver
                    .resolve_method(&applied_receiver_ty, method)
                {
                    self.method_resolutions
                        .insert(span, resolution.method_def_id);

                    if let Some(self_param_ty) = resolution.params.first() {
                        self.unify(&receiver_ty, self_param_ty, receiver.span);
                    }

                    if args.len() + 1 != resolution.params.len() {
                        self.error(
                            span,
                            format!(
                                "method '{}' expects {} arguments, got {}",
                                method,
                                resolution.params.len().saturating_sub(1),
                                args.len()
                            ),
                        );
                    }

                    for (arg, param_ty) in args.iter().zip(resolution.params.iter().skip(1)) {
                        let arg_ty = self.infer_expr(arg);
                        self.unify(&arg_ty, param_ty, arg.span);
                    }

                    resolution.return_ty
                } else {
                    let func_ty = self.infer_expr(target);
                    let mut arg_tys = vec![receiver_ty];
                    arg_tys.extend(args.iter().map(|arg| self.infer_expr(arg)));

                    let ret_ty = self.fresh_var();
                    let expected_fn_ty = Ty {
                        kind: TyKind::Fn(arg_tys, Box::new(ret_ty.clone())),
                        span,
                    };

                    self.unify(&func_ty, &expected_fn_ty, span);
                    self.apply(&ret_ty)
                }
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

            ExprKind::SafeField { base, .. } => {
                let _ = self.infer_expr(base);
                self.fresh_var()
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

            ExprKind::Binary(op, left, right) => self.infer_binary(*op, left, right, span),

            ExprKind::Unary(op, operand) => self.infer_unary(*op, operand, span),

            ExprKind::If(cond, then_br, else_br) => {
                let cond_ty = self.infer_expr(cond);
                self.unify(
                    &cond_ty,
                    &Ty {
                        kind: TyKind::Bool,
                        span: cond.span,
                    },
                    cond.span,
                );

                let then_ty = self.infer_expr(then_br);
                let else_ty = self.infer_expr(else_br);
                self.unify(&then_ty, &else_ty, span);

                self.apply(&then_ty)
            }

            ExprKind::Coalesce { value, default } => {
                let value_ty = self.infer_expr(value);
                let default_ty = self.infer_expr(default);
                self.coalesce_result_type(value_ty, default_ty, span)
            }

            ExprKind::Match(scrutinee, arms) => {
                let scrutinee_ty = self.infer_expr(scrutinee);
                let result_ty = self.fresh_var();

                for arm in arms {
                    self.check_arm(arm, &scrutinee_ty, &result_ty);
                }

                self.check_match_coverage(&scrutinee_ty, arms, span);

                self.apply(&result_ty)
            }

            ExprKind::Block(stmts, expr) => {
                for stmt in stmts {
                    self.check_stmt(stmt);
                }

                if let Some(e) = expr {
                    self.infer_expr(e)
                } else {
                    Ty {
                        kind: TyKind::Unit,
                        span,
                    }
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
                Ty {
                    kind: TyKind::String,
                    span,
                }
            }

            ExprKind::Let {
                pattern,
                ty,
                value,
                body,
            } => {
                let value_ty = self.infer_expr(value);
                let declared_ty = ty
                    .as_ref()
                    .map(|t| self.resolve_type(t))
                    .unwrap_or_else(|| value_ty.clone());
                self.unify(&value_ty, &declared_ty, value.span);
                self.check_pattern(pattern, &declared_ty);
                self.infer_expr(body)
            }

            ExprKind::Try(inner) => {
                let inner_ty = self.infer_expr(inner);
                self.try_result_type(inner_ty, span)
            }

            ExprKind::Lazy(inner) => self.infer_expr(inner),

            ExprKind::ListComp { body, generators } => {
                for generator in generators {
                    let iter_ty = self.infer_expr(&generator.iter);
                    let elem_ty = self.fresh_var();
                    let list_ty = builtin_list(elem_ty.clone(), generator.span);
                    self.unify(&iter_ty, &list_ty, generator.iter.span);
                    self.check_pattern(&generator.pattern, &elem_ty);
                    if let Some(condition) = &generator.condition {
                        let cond_ty = self.infer_expr(condition);
                        self.unify(
                            &cond_ty,
                            &Ty {
                                kind: TyKind::Bool,
                                span: condition.span,
                            },
                            condition.span,
                        );
                    }
                }
                let body_ty = self.infer_expr(body);
                builtin_list(self.apply(&body_ty), span)
            }

            ExprKind::Error(message) => {
                self.error(span, message.clone());
                self.fresh_var()
            }
        };

        let ty = self.apply(&ty);
        self.expr_types.insert(span, ty.clone());
        ty
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
        Ty {
            kind,
            span: Span::DUMMY,
        }
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
                Ty {
                    kind: TyKind::Bool,
                    span,
                }
            }

            // Logical: Bool -> Bool -> Bool
            BinOp::And | BinOp::Or => {
                self.unify(
                    &left_ty,
                    &Ty {
                        kind: TyKind::Bool,
                        span: left.span,
                    },
                    left.span,
                );
                self.unify(
                    &right_ty,
                    &Ty {
                        kind: TyKind::Bool,
                        span: right.span,
                    },
                    right.span,
                );
                Ty {
                    kind: TyKind::Bool,
                    span,
                }
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
                self.unify(
                    &operand_ty,
                    &Ty {
                        kind: TyKind::Bool,
                        span: operand.span,
                    },
                    operand.span,
                );
                Ty {
                    kind: TyKind::Bool,
                    span,
                }
            }
        }
    }

    fn check_arm(&mut self, arm: &MatchArm, scrutinee_ty: &Ty, result_ty: &Ty) {
        // Check pattern against scrutinee type
        self.check_pattern(&arm.pattern, scrutinee_ty);

        // Check guard if present
        if let Some(guard) = &arm.guard {
            let guard_ty = self.infer_expr(guard);
            self.unify(
                &guard_ty,
                &Ty {
                    kind: TyKind::Bool,
                    span: guard.span,
                },
                guard.span,
            );
        }

        // Check body and unify with result type
        let body_ty = self.infer_expr(&arm.body);
        self.unify(&body_ty, result_ty, arm.body.span);
    }

    fn pattern_binding_ids(pattern: &Pattern, bindings: &mut Vec<LocalId>) {
        match &pattern.kind {
            PatternKind::Wildcard | PatternKind::Literal(_) => {}
            PatternKind::Var(local_id, _) => bindings.push(*local_id),
            PatternKind::Binding(local_id, _, inner) => {
                bindings.push(*local_id);
                Self::pattern_binding_ids(inner, bindings);
            }
            PatternKind::Tuple(patterns)
            | PatternKind::List(patterns)
            | PatternKind::Constructor(_, patterns)
            | PatternKind::Or(patterns) => {
                for pattern in patterns {
                    Self::pattern_binding_ids(pattern, bindings);
                }
            }
            PatternKind::ListRest { init, rest, tail } => {
                for pattern in init {
                    Self::pattern_binding_ids(pattern, bindings);
                }
                if let Some(pattern) = rest {
                    Self::pattern_binding_ids(pattern, bindings);
                }
                for pattern in tail {
                    Self::pattern_binding_ids(pattern, bindings);
                }
            }
            PatternKind::Record(fields) => {
                for (_, pattern) in fields {
                    Self::pattern_binding_ids(pattern, bindings);
                }
            }
        }
    }

    fn pattern_binding_signature(pattern: &Pattern) -> Vec<u32> {
        let mut bindings = Vec::new();
        Self::pattern_binding_ids(pattern, &mut bindings);
        let mut ids: Vec<u32> = bindings.into_iter().map(|id| id.0).collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    fn check_pattern(&mut self, pattern: &Pattern, expected: &Ty) {
        match &pattern.kind {
            PatternKind::Wildcard => {}

            PatternKind::Var(local_id, name) => {
                self.define_local(*local_id, name.clone(), expected.clone(), pattern.span);
            }

            PatternKind::Binding(local_id, name, inner) => {
                self.define_local(*local_id, name.clone(), expected.clone(), pattern.span);
                self.check_pattern(inner, expected);
            }

            PatternKind::Literal(lit) => {
                let lit_ty = self.infer_literal(lit);
                self.unify(&lit_ty, expected, pattern.span);
            }

            PatternKind::Tuple(patterns) => match &expected.kind {
                TyKind::Tuple(elem_tys) if elem_tys.len() == patterns.len() => {
                    for (pat, ty) in patterns.iter().zip(elem_tys.iter()) {
                        self.check_pattern(pat, ty);
                    }
                }
                _ => {
                    self.error(pattern.span, "pattern does not match expected tuple");
                }
            },

            PatternKind::List(patterns) => {
                let elem_ty = self.fresh_var();
                let list_ty = builtin_list(elem_ty.clone(), pattern.span);
                self.unify(&list_ty, expected, pattern.span);
                for pat in patterns {
                    self.check_pattern(pat, &elem_ty);
                }
            }

            PatternKind::ListRest { init, rest, tail } => {
                let elem_ty = self.fresh_var();
                let list_ty = builtin_list(elem_ty.clone(), pattern.span);
                self.unify(&list_ty, expected, pattern.span);
                for pat in init {
                    self.check_pattern(pat, &elem_ty);
                }
                if let Some(pattern) = rest {
                    self.check_pattern(pattern, &list_ty);
                }
                for pat in tail {
                    self.check_pattern(pat, &elem_ty);
                }
            }

            PatternKind::Record(fields) => {
                for (name, pat) in fields {
                    let field_ty = match &expected.kind {
                        TyKind::Record(field_tys) => field_tys
                            .iter()
                            .find(|(n, _)| n == name)
                            .map(|(_, t)| t.clone()),
                        _ => None,
                    };

                    if let Some(ty) = field_ty {
                        self.check_pattern(pat, &ty);
                    } else {
                        self.error(pattern.span, format!("no field '{}' in record", name));
                    }
                }
            }

            PatternKind::Constructor(def_id, patterns) => {
                if let Some(name) = builtin_constructor_name(*def_id) {
                    let expected = self.apply(expected);
                    let option_ctor = matches!(name, "Some" | "None");
                    let result_ctor = matches!(name, "Ok" | "Err");
                    if matches!(expected.kind, TyKind::Named(def_id, _) if option_ctor && is_builtin_result_type(def_id) || result_ctor && is_builtin_option_type(def_id))
                    {
                        self.error(
                            pattern.span,
                            "constructor does not match expected builtin type",
                        );
                        return;
                    }

                    match name {
                        "Some" => {
                            let payload_ty = self.fresh_var();
                            self.unify(
                                &builtin_option(payload_ty.clone(), pattern.span),
                                &expected,
                                pattern.span,
                            );
                            if patterns.len() != 1 {
                                self.error(
                                    pattern.span,
                                    format!(
                                        "constructor expects 1 field(s), got {}",
                                        patterns.len()
                                    ),
                                );
                                return;
                            }
                            self.check_pattern(&patterns[0], &self.apply(&payload_ty));
                        }
                        "None" => {
                            let elem_ty = self.fresh_var();
                            self.unify(
                                &builtin_option(elem_ty, pattern.span),
                                &expected,
                                pattern.span,
                            );
                            if !patterns.is_empty() {
                                self.error(
                                    pattern.span,
                                    format!(
                                        "constructor expects 0 field(s), got {}",
                                        patterns.len()
                                    ),
                                );
                            }
                        }
                        "Ok" => {
                            let ok_ty = self.fresh_var();
                            let err_ty = self.fresh_var();
                            self.unify(
                                &builtin_result(ok_ty.clone(), err_ty, pattern.span),
                                &expected,
                                pattern.span,
                            );
                            if patterns.len() != 1 {
                                self.error(
                                    pattern.span,
                                    format!(
                                        "constructor expects 1 field(s), got {}",
                                        patterns.len()
                                    ),
                                );
                                return;
                            }
                            self.check_pattern(&patterns[0], &self.apply(&ok_ty));
                        }
                        "Err" => {
                            let ok_ty = self.fresh_var();
                            let err_ty = self.fresh_var();
                            self.unify(
                                &builtin_result(ok_ty, err_ty.clone(), pattern.span),
                                &expected,
                                pattern.span,
                            );
                            if patterns.len() != 1 {
                                self.error(
                                    pattern.span,
                                    format!(
                                        "constructor expects 1 field(s), got {}",
                                        patterns.len()
                                    ),
                                );
                                return;
                            }
                            self.check_pattern(&patterns[0], &self.apply(&err_ty));
                        }
                        _ => {}
                    }
                    return;
                }

                if let Some(variant) = self.variants.get(def_id) {
                    let enum_id = variant.enum_id;
                    let fields = variant.fields.clone();
                    let enum_ty = Ty {
                        kind: TyKind::Named(enum_id, Vec::new()),
                        span: pattern.span,
                    };
                    self.unify(&enum_ty, expected, pattern.span);

                    if fields.len() != patterns.len() {
                        self.error(
                            pattern.span,
                            format!(
                                "constructor expects {} field(s), got {}",
                                fields.len(),
                                patterns.len()
                            ),
                        );
                        return;
                    }

                    for (pat, ty) in patterns.iter().zip(fields.iter()) {
                        self.check_pattern(pat, ty);
                    }
                } else {
                    let expected = self.apply(expected);
                    if matches!(expected.kind, TyKind::Named(def_id, _) if is_builtin_option_type(def_id) || is_builtin_result_type(def_id))
                    {
                        self.error(
                            pattern.span,
                            "constructor does not match expected builtin type",
                        );
                        return;
                    }
                    // Unknown constructor, use fresh type variables
                    // 未知构造函数，使用新类型变量
                    for pat in patterns {
                        let arg_ty = self.fresh_var();
                        self.check_pattern(pat, &arg_ty);
                    }
                }
            }

            PatternKind::Or(patterns) => {
                let saved_locals = self.locals.clone();
                let mut first_signature = None;
                let mut merged_locals = None;

                for pattern in patterns {
                    self.locals = saved_locals.clone();
                    self.check_pattern(pattern, expected);

                    let signature = Self::pattern_binding_signature(pattern);
                    if let Some(first) = &first_signature {
                        if first != &signature {
                            self.error(
                                pattern.span,
                                "or-pattern alternatives must bind the same variables",
                            );
                        }
                    } else {
                        first_signature = Some(signature);
                        merged_locals = Some(self.locals.clone());
                    }
                }

                self.locals = merged_locals.unwrap_or(saved_locals);
            }
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match &stmt.kind {
            StmtKind::Let { pattern, ty, value } => {
                let value_ty = self.infer_expr(value);
                let declared_ty = ty
                    .as_ref()
                    .map(|ty| self.resolve_type(ty))
                    .unwrap_or_else(|| value_ty.clone());
                self.unify(&value_ty, &declared_ty, value.span);

                let binding_ids = Self::pattern_binding_signature(pattern);
                let env_vars: Vec<u32> = self
                    .locals
                    .values()
                    .flat_map(|info| free_type_vars(&info.ty))
                    .collect();
                self.check_pattern(pattern, &declared_ty);

                for binding_id in binding_ids {
                    let local_id = LocalId(binding_id);
                    if let Some(local_ty) = self.locals.get(&local_id).map(|local| local.ty.clone())
                    {
                        let generalized_ty = generalize(&self.apply(&local_ty), &env_vars);
                        if let Some(local) = self.locals.get_mut(&local_id) {
                            local.ty = generalized_ty.clone();
                        }
                        self.local_definitions.insert(local_id, generalized_ty);
                    }
                }
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
