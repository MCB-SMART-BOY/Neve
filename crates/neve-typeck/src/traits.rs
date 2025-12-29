//! Trait resolution and checking.
//! 特征解析与检查。
//!
//! This module implements trait resolution for Neve's type system.
//! 本模块实现 Neve 类型系统的特征解析。
//!
//! It handles:
//! 主要处理：
//! - Trait definitions and their methods / 特征定义及其方法
//! - Impl blocks (both inherent and trait implementations) / 实现块（固有实现和特征实现）
//! - Trait bounds on generic parameters / 泛型参数的特征约束
//! - Trait resolution (finding the right impl for a type) / 特征解析（为类型找到正确的实现）
//! - Associated types and their resolution / 关联类型及其解析

use neve_common::Span;
use neve_hir::{DefId, GenericParam, ImplDef, TraitDef, Ty, TyKind};
use std::collections::HashMap;

/// A trait ID for internal tracking.
/// 用于内部跟踪的特征 ID。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraitId(pub u32);

/// An implementation ID.
/// 实现 ID。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImplId(pub u32);

/// Information about a registered trait.
/// 已注册特征的信息。
#[derive(Debug, Clone)]
pub struct TraitInfo {
    /// The trait definition ID. / 特征定义 ID。
    pub def_id: DefId,
    /// The trait name. / 特征名称。
    pub name: String,
    /// Generic parameters. / 泛型参数。
    pub generics: Vec<GenericParam>,
    /// Methods defined by this trait. / 该特征定义的方法。
    pub methods: Vec<TraitMethod>,
    /// Associated types. / 关联类型。
    pub assoc_types: Vec<AssocType>,
}

/// A method defined in a trait.
/// 特征中定义的方法。
#[derive(Debug, Clone)]
pub struct TraitMethod {
    /// Method name. / 方法名。
    pub name: String,
    /// Method signature (parameter types). / 方法签名（参数类型）。
    pub params: Vec<Ty>,
    /// Return type. / 返回类型。
    pub return_ty: Ty,
    /// Whether a default implementation exists. / 是否存在默认实现。
    pub has_default: bool,
}

/// An associated type in a trait (resolved info).
/// 特征中的关联类型（解析后的信息）。
#[derive(Debug, Clone)]
pub struct AssocType {
    /// The associated type name. / 关联类型名称。
    pub name: String,
    /// Bounds on the associated type. / 关联类型的约束。
    pub bounds: Vec<TraitBound>,
    /// Default type (if any). / 默认类型（如果有）。
    pub default: Option<Ty>,
}

/// An associated type implementation (resolved info).
/// 关联类型的实现（解析后的信息）。
#[derive(Debug, Clone)]
pub struct AssocTypeResolution {
    /// The associated type name. / 关联类型名称。
    pub name: String,
    /// The concrete type this is bound to. / 绑定的具体类型。
    pub ty: Ty,
}

/// A trait bound (e.g., `T: Show`).
/// 特征约束（例如 `T: Show`）。
#[derive(Debug, Clone)]
pub struct TraitBound {
    /// The trait being bounded on. / 被约束的特征。
    pub trait_id: TraitId,
    /// Type arguments to the trait. / 特征的类型参数。
    pub args: Vec<Ty>,
}

/// Information about an impl block.
/// 实现块的信息。
#[derive(Debug, Clone)]
pub struct ImplInfo {
    /// The impl definition ID. / 实现定义 ID。
    pub def_id: DefId,
    /// Generic parameters. / 泛型参数。
    pub generics: Vec<GenericParam>,
    /// The trait being implemented (None for inherent impls). / 被实现的特征（固有实现为 None）。
    pub trait_ref: Option<TraitRef>,
    /// The self type. / Self 类型。
    pub self_ty: Ty,
    /// Implemented methods. / 已实现的方法。
    pub methods: Vec<ImplMethod>,
    /// Associated type implementations. / 关联类型实现。
    pub assoc_types: Vec<AssocTypeResolution>,
}

/// Reference to a trait with type arguments.
/// 带有类型参数的特征引用。
#[derive(Debug, Clone)]
pub struct TraitRef {
    pub trait_id: TraitId,
    pub args: Vec<Ty>,
}

/// A method in an impl block.
/// 实现块中的方法。
#[derive(Debug, Clone)]
pub struct ImplMethod {
    pub name: String,
    pub params: Vec<Ty>,
    pub return_ty: Ty,
    pub span: Span,
}

/// The trait resolver - maintains trait and impl registries.
/// 特征解析器 - 维护特征和实现的注册表。
#[derive(Debug, Default)]
pub struct TraitResolver {
    /// Registered traits. / 已注册的特征。
    traits: HashMap<TraitId, TraitInfo>,
    /// Trait name to ID mapping. / 特征名称到 ID 的映射。
    trait_names: HashMap<String, TraitId>,
    /// Registered impls. / 已注册的实现。
    impls: HashMap<ImplId, ImplInfo>,
    /// Trait impls by trait ID. / 按特征 ID 索引的实现。
    trait_impls: HashMap<TraitId, Vec<ImplId>>,
    /// Inherent impls by type (simplified: using type name string). / 按类型索引的固有实现。
    inherent_impls: HashMap<String, Vec<ImplId>>,
    /// Next trait ID. / 下一个特征 ID。
    next_trait_id: u32,
    /// Next impl ID. / 下一个实现 ID。
    next_impl_id: u32,
}

impl TraitResolver {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a trait definition.
    /// 注册特征定义。
    pub fn register_trait(&mut self, def_id: DefId, def: &TraitDef) -> TraitId {
        let trait_id = TraitId(self.next_trait_id);
        self.next_trait_id += 1;

        let methods: Vec<TraitMethod> = def
            .items
            .iter()
            .map(|item| TraitMethod {
                name: item.name.clone(),
                params: item.params.clone(),
                return_ty: item.return_ty.clone(),
                has_default: item.default.is_some(),
            })
            .collect();

        // Extract associated types from the trait definition
        let assoc_types: Vec<AssocType> = def
            .assoc_types
            .iter()
            .map(|at| AssocType {
                name: at.name.clone(),
                bounds: at
                    .bounds
                    .iter()
                    .filter_map(|bound_ty| self.ty_to_trait_bound(bound_ty))
                    .collect(),
                default: at.default.clone(),
            })
            .collect();

        let info = TraitInfo {
            def_id,
            name: def.name.clone(),
            generics: def.generics.clone(),
            methods,
            assoc_types,
        };

        self.traits.insert(trait_id, info);
        self.trait_names.insert(def.name.clone(), trait_id);
        self.trait_impls.insert(trait_id, Vec::new());

        trait_id
    }

    /// Register an impl block.
    /// 注册实现块。
    pub fn register_impl(&mut self, def_id: DefId, def: &ImplDef) -> ImplId {
        let impl_id = ImplId(self.next_impl_id);
        self.next_impl_id += 1;

        let trait_ref = def
            .trait_ref
            .as_ref()
            .and_then(|ty| self.resolve_trait_ref(ty));

        let methods: Vec<ImplMethod> = def
            .items
            .iter()
            .map(|item| ImplMethod {
                name: item.name.clone(),
                params: item.params.iter().map(|p| p.ty.clone()).collect(),
                return_ty: item.return_ty.clone(),
                span: item.span,
            })
            .collect();

        // Extract associated type implementations
        let assoc_types: Vec<AssocTypeResolution> = def
            .assoc_type_impls
            .iter()
            .map(|ati| AssocTypeResolution {
                name: ati.name.clone(),
                ty: ati.ty.clone(),
            })
            .collect();

        let info = ImplInfo {
            def_id,
            generics: def.generics.clone(),
            trait_ref: trait_ref.clone(),
            self_ty: def.self_ty.clone(),
            methods,
            assoc_types,
        };

        self.impls.insert(impl_id, info);

        // Register with the appropriate index
        if let Some(trait_ref) = trait_ref {
            if let Some(impls) = self.trait_impls.get_mut(&trait_ref.trait_id) {
                impls.push(impl_id);
            }
        } else {
            // Inherent impl
            let type_key = self.type_key(&def.self_ty);
            self.inherent_impls
                .entry(type_key)
                .or_default()
                .push(impl_id);
        }

        impl_id
    }

    /// Resolve a type to a trait reference.
    /// 将类型解析为特征引用。
    fn resolve_trait_ref(&self, ty: &Ty) -> Option<TraitRef> {
        match &ty.kind {
            TyKind::Named(def_id, args) => {
                // Try to find a trait with this def_id
                for (trait_id, info) in &self.traits {
                    if info.def_id == *def_id {
                        return Some(TraitRef {
                            trait_id: *trait_id,
                            args: args.clone(),
                        });
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Convert a type (representing a trait bound) to a TraitBound.
    /// 将类型（表示特征约束）转换为 TraitBound。
    fn ty_to_trait_bound(&self, ty: &Ty) -> Option<TraitBound> {
        self.resolve_trait_ref(ty).map(|trait_ref| TraitBound {
            trait_id: trait_ref.trait_id,
            args: trait_ref.args,
        })
    }

    /// Get a simple key for a type (for inherent impl lookup).
    /// 获取类型的简单键（用于固有实现查找）。
    fn type_key(&self, ty: &Ty) -> String {
        match &ty.kind {
            TyKind::Int => "Int".to_string(),
            TyKind::Float => "Float".to_string(),
            TyKind::Bool => "Bool".to_string(),
            TyKind::Char => "Char".to_string(),
            TyKind::String => "String".to_string(),
            TyKind::Unit => "()".to_string(),
            TyKind::Named(def_id, _) => format!("Named({})", def_id.0),
            TyKind::Tuple(elems) => format!("Tuple({})", elems.len()),
            TyKind::Record(_) => "Record".to_string(),
            TyKind::Fn(_, _) => "Fn".to_string(),
            TyKind::Var(v) => format!("Var({})", v),
            TyKind::Param(idx, name) => format!("Param({}, {})", idx, name),
            TyKind::Forall(params, _) => format!("Forall({})", params.len()),
            TyKind::Unknown => "Unknown".to_string(),
        }
    }

    /// Look up a trait by name.
    /// 按名称查找特征。
    pub fn lookup_trait(&self, name: &str) -> Option<TraitId> {
        self.trait_names.get(name).copied()
    }

    /// Get trait info by ID.
    /// 按 ID 获取特征信息。
    pub fn get_trait(&self, id: TraitId) -> Option<&TraitInfo> {
        self.traits.get(&id)
    }

    /// Get impl info by ID.
    /// 按 ID 获取实现信息。
    pub fn get_impl(&self, id: ImplId) -> Option<&ImplInfo> {
        self.impls.get(&id)
    }

    /// Find implementations of a trait for a specific type.
    /// 为特定类型查找特征的实现。
    pub fn find_trait_impl(&self, trait_id: TraitId, self_ty: &Ty) -> Option<ImplId> {
        let impls = self.trait_impls.get(&trait_id)?;

        for impl_id in impls {
            if let Some(info) = self.impls.get(impl_id)
                && self.types_match(&info.self_ty, self_ty)
            {
                return Some(*impl_id);
            }
        }

        None
    }

    /// Find inherent impls for a type.
    /// 查找类型的固有实现。
    pub fn find_inherent_impls(&self, self_ty: &Ty) -> Vec<ImplId> {
        let key = self.type_key(self_ty);
        self.inherent_impls.get(&key).cloned().unwrap_or_default()
    }

    /// Check if two types match (simplified - ignores generics for now).
    /// 检查两个类型是否匹配（简化版本 - 暂时忽略泛型）。
    fn types_match(&self, t1: &Ty, t2: &Ty) -> bool {
        match (&t1.kind, &t2.kind) {
            (TyKind::Int, TyKind::Int) => true,
            (TyKind::Float, TyKind::Float) => true,
            (TyKind::Bool, TyKind::Bool) => true,
            (TyKind::Char, TyKind::Char) => true,
            (TyKind::String, TyKind::String) => true,
            (TyKind::Unit, TyKind::Unit) => true,
            (TyKind::Named(id1, _), TyKind::Named(id2, _)) => id1 == id2,
            (TyKind::Var(_), _) | (_, TyKind::Var(_)) => true, // Type vars match anything
            _ => false,
        }
    }

    /// Resolve a method call on a type.
    /// 解析类型上的方法调用。
    pub fn resolve_method(&self, self_ty: &Ty, method_name: &str) -> Option<MethodResolution> {
        // First, check inherent impls
        for impl_id in self.find_inherent_impls(self_ty) {
            if let Some(info) = self.impls.get(&impl_id) {
                for method in &info.methods {
                    if method.name == method_name {
                        return Some(MethodResolution {
                            impl_id,
                            method_name: method_name.to_string(),
                            self_ty: info.self_ty.clone(),
                            params: method.params.clone(),
                            return_ty: method.return_ty.clone(),
                        });
                    }
                }
            }
        }

        // Then check trait impls
        for (trait_id, impl_ids) in &self.trait_impls {
            // Check if trait has this method
            if let Some(trait_info) = self.traits.get(trait_id) {
                let has_method = trait_info.methods.iter().any(|m| m.name == method_name);

                if has_method {
                    // Find an impl that matches our type
                    for impl_id in impl_ids {
                        if let Some(info) = self.impls.get(impl_id)
                            && self.types_match(&info.self_ty, self_ty)
                        {
                            for method in &info.methods {
                                if method.name == method_name {
                                    return Some(MethodResolution {
                                        impl_id: *impl_id,
                                        method_name: method_name.to_string(),
                                        self_ty: info.self_ty.clone(),
                                        params: method.params.clone(),
                                        return_ty: method.return_ty.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Check that an impl provides all required trait methods.
    /// 检查实现是否提供了所有必需的特征方法。
    pub fn check_impl_completeness(&self, impl_id: ImplId) -> Vec<String> {
        let mut missing = Vec::new();

        if let Some(info) = self.impls.get(&impl_id)
            && let Some(trait_ref) = &info.trait_ref
            && let Some(trait_info) = self.traits.get(&trait_ref.trait_id)
        {
            let impl_method_names: Vec<_> = info.methods.iter().map(|m| m.name.as_str()).collect();

            for trait_method in &trait_info.methods {
                if !trait_method.has_default
                    && !impl_method_names.contains(&trait_method.name.as_str())
                {
                    missing.push(trait_method.name.clone());
                }
            }
        }

        missing
    }

    /// Get all traits.
    /// 获取所有特征。
    pub fn all_traits(&self) -> impl Iterator<Item = (&TraitId, &TraitInfo)> {
        self.traits.iter()
    }

    /// Get all impls for a trait.
    /// 获取特征的所有实现。
    pub fn impls_for_trait(&self, trait_id: TraitId) -> Vec<&ImplInfo> {
        self.trait_impls
            .get(&trait_id)
            .map(|impl_ids| {
                impl_ids
                    .iter()
                    .filter_map(|id| self.impls.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Resolve an associated type for a given type and trait.
    /// Returns the concrete type that the associated type is bound to.
    /// 解析给定类型和特征的关联类型。
    /// 返回关联类型绑定的具体类型。
    pub fn resolve_assoc_type(
        &self,
        self_ty: &Ty,
        trait_id: TraitId,
        assoc_type_name: &str,
    ) -> Option<Ty> {
        // Find the impl for this type and trait
        let impl_id = self.find_trait_impl(trait_id, self_ty)?;
        let impl_info = self.impls.get(&impl_id)?;

        // Look for the associated type in the impl
        for assoc in &impl_info.assoc_types {
            if assoc.name == assoc_type_name {
                return Some(assoc.ty.clone());
            }
        }

        // Check if the trait has a default for this associated type
        let trait_info = self.traits.get(&trait_id)?;
        for assoc in &trait_info.assoc_types {
            if assoc.name == assoc_type_name {
                return assoc.default.clone();
            }
        }

        None
    }

    /// Get all associated types defined by a trait.
    /// 获取特征定义的所有关联类型。
    pub fn trait_assoc_types(&self, trait_id: TraitId) -> Vec<&AssocType> {
        self.traits
            .get(&trait_id)
            .map(|info| info.assoc_types.iter().collect())
            .unwrap_or_default()
    }

    /// Check that an impl provides all required associated types.
    /// 检查实现是否提供了所有必需的关联类型。
    pub fn check_impl_assoc_types(&self, impl_id: ImplId) -> Vec<String> {
        let mut missing = Vec::new();

        if let Some(info) = self.impls.get(&impl_id)
            && let Some(trait_ref) = &info.trait_ref
            && let Some(trait_info) = self.traits.get(&trait_ref.trait_id)
        {
            let impl_assoc_names: Vec<_> =
                info.assoc_types.iter().map(|a| a.name.as_str()).collect();

            for trait_assoc in &trait_info.assoc_types {
                // Required if no default
                if trait_assoc.default.is_none()
                    && !impl_assoc_names.contains(&trait_assoc.name.as_str())
                {
                    missing.push(trait_assoc.name.clone());
                }
            }
        }

        missing
    }

    /// Get the full completeness check for an impl (methods + associated types).
    /// 获取实现的完整性检查（方法 + 关联类型）。
    pub fn check_impl_full_completeness(&self, impl_id: ImplId) -> ImplCompleteness {
        ImplCompleteness {
            missing_methods: self.check_impl_completeness(impl_id),
            missing_assoc_types: self.check_impl_assoc_types(impl_id),
        }
    }
}

/// Result of checking impl completeness.
/// 实现完整性检查的结果。
#[derive(Debug, Clone)]
pub struct ImplCompleteness {
    pub missing_methods: Vec<String>,
    pub missing_assoc_types: Vec<String>,
}

impl ImplCompleteness {
    pub fn is_complete(&self) -> bool {
        self.missing_methods.is_empty() && self.missing_assoc_types.is_empty()
    }
}

/// Result of resolving a method call.
/// 方法调用解析的结果。
#[derive(Debug, Clone)]
pub struct MethodResolution {
    pub impl_id: ImplId,
    pub method_name: String,
    pub self_ty: Ty,
    pub params: Vec<Ty>,
    pub return_ty: Ty,
}

/// Trait constraint for type checking.
/// 类型检查的特征约束。
#[derive(Debug, Clone)]
pub struct TraitConstraint {
    /// The type that must implement the trait. / 必须实现特征的类型。
    pub ty: Ty,
    /// The trait bound. / 特征约束。
    pub bound: TraitBound,
    /// Where this constraint was introduced. / 约束引入的位置。
    pub span: Span,
}

/// Constraint solver for trait bounds.
/// 特征约束的求解器。
#[derive(Debug, Default)]
pub struct ConstraintSolver {
    /// Pending constraints to solve. / 待求解的约束。
    constraints: Vec<TraitConstraint>,
    /// Reference to trait resolver. / 特征解析器引用。
    trait_resolver: TraitResolver,
}

impl ConstraintSolver {
    pub fn new(resolver: TraitResolver) -> Self {
        Self {
            constraints: Vec::new(),
            trait_resolver: resolver,
        }
    }

    /// Add a constraint to be solved.
    /// 添加待求解的约束。
    pub fn add_constraint(&mut self, constraint: TraitConstraint) {
        self.constraints.push(constraint);
    }

    /// Solve all pending constraints.
    /// Returns a list of unsatisfied constraints.
    /// 求解所有待处理的约束。
    /// 返回未满足的约束列表。
    pub fn solve(&self) -> Vec<UnsatisfiedConstraint> {
        let mut unsatisfied = Vec::new();

        for constraint in &self.constraints {
            if !self.is_satisfied(&constraint.ty, &constraint.bound) {
                unsatisfied.push(UnsatisfiedConstraint {
                    ty: constraint.ty.clone(),
                    bound: constraint.bound.clone(),
                    span: constraint.span,
                });
            }
        }

        unsatisfied
    }

    /// Check if a type satisfies a trait bound.
    /// 检查类型是否满足特征约束。
    fn is_satisfied(&self, ty: &Ty, bound: &TraitBound) -> bool {
        self.trait_resolver
            .find_trait_impl(bound.trait_id, ty)
            .is_some()
    }

    /// Get the trait resolver.
    /// 获取特征解析器。
    pub fn resolver(&self) -> &TraitResolver {
        &self.trait_resolver
    }

    /// Get mutable trait resolver.
    /// 获取可变的特征解析器。
    pub fn resolver_mut(&mut self) -> &mut TraitResolver {
        &mut self.trait_resolver
    }
}

/// An unsatisfied trait constraint.
/// 未满足的特征约束。
#[derive(Debug, Clone)]
pub struct UnsatisfiedConstraint {
    pub ty: Ty,
    pub bound: TraitBound,
    pub span: Span,
}
