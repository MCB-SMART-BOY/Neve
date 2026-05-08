//! Match pattern coverage / usefulness analysis.
//! match 模式覆盖与可达性分析。

use crate::builtin_types::{is_builtin_option_type, is_builtin_result_type};
use crate::check::{EnumInfo, VariantInfo};
use neve_common::Span;
use neve_hir::{
    DefId, Literal, MatchArm, Pattern, PatternKind, Ty, TyKind, builtin_constructor_name,
};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct UnreachableArm {
    pub(crate) span: Span,
    pub(crate) shadowed_by: Span,
    pub(crate) reason: RedundancyReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RedundancyReason {
    CoveredByPreviousArms,
    SubsetShadowed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ArmUsefulness {
    Useful,
    Redundant {
        witness_span: Span,
        reason: RedundancyReason,
    },
    GuardedIgnored,
    NotAnalyzed,
}

#[derive(Debug, Default)]
pub(crate) struct PatternAnalysisResult {
    pub(crate) missing_patterns: Vec<String>,
    pub(crate) arm_usefulness: Vec<ArmUsefulness>,
    pub(crate) unreachable_arms: Vec<UnreachableArm>,
    pub(crate) coverage_complete_at: Option<Span>,
}

impl PatternAnalysisResult {
    fn push_useful(&mut self) {
        self.arm_usefulness.push(ArmUsefulness::Useful);
    }

    fn push_guarded_ignored(&mut self) {
        self.arm_usefulness.push(ArmUsefulness::GuardedIgnored);
    }

    fn push_redundant(&mut self, witness_span: Span, reason: RedundancyReason) {
        self.arm_usefulness.push(ArmUsefulness::Redundant {
            witness_span,
            reason,
        });
    }

    fn rebuild_unreachable_arms(&mut self, arms: &[MatchArm]) {
        self.unreachable_arms = self
            .arm_usefulness
            .iter()
            .zip(arms.iter())
            .filter_map(|(usefulness, arm)| match usefulness {
                ArmUsefulness::Redundant {
                    witness_span,
                    reason,
                } => Some(UnreachableArm {
                    span: arm.span,
                    shadowed_by: *witness_span,
                    reason: *reason,
                }),
                _ => None,
            })
            .collect();
    }
}

pub(crate) struct PatternAnalysisContext<'a> {
    pub(crate) enums: &'a HashMap<DefId, EnumInfo>,
    pub(crate) variants: &'a HashMap<DefId, VariantInfo>,
}

pub(crate) fn analyze_match(
    scrutinee_ty: &Ty,
    arms: &[MatchArm],
    ctx: &PatternAnalysisContext<'_>,
) -> PatternAnalysisResult {
    let mut result = PatternAnalysisResult::default();

    match &scrutinee_ty.kind {
        TyKind::Bool => {
            let mut covers_true = false;
            let mut covers_false = false;
            let mut true_witness = None;
            let mut false_witness = None;

            for arm in arms {
                if let Some(previous_span) = result.coverage_complete_at {
                    result.push_redundant(previous_span, RedundancyReason::CoveredByPreviousArms);
                    continue;
                }
                if arm.guard.is_some() {
                    result.push_guarded_ignored();
                    continue;
                }
                let (arm_true, arm_false) = bool_pattern_coverage(&arm.pattern);
                let arm_can_match = arm_true || arm_false;
                if arm_can_match && (!arm_true || covers_true) && (!arm_false || covers_false) {
                    let witness = earliest_span(
                        [
                            arm_true.then_some(true_witness).flatten(),
                            arm_false.then_some(false_witness).flatten(),
                        ]
                        .into_iter()
                        .flatten(),
                    )
                    .unwrap_or(arm.span);
                    result.push_redundant(witness, RedundancyReason::SubsetShadowed);
                    continue;
                }
                result.push_useful();
                if pattern_is_irrefutable_for(&arm.pattern, scrutinee_ty, ctx) {
                    result.coverage_complete_at = Some(arm.span);
                    if !covers_true {
                        true_witness = Some(arm.span);
                    }
                    if !covers_false {
                        false_witness = Some(arm.span);
                    }
                    covers_true = true;
                    covers_false = true;
                    continue;
                }

                if arm_true && !covers_true {
                    true_witness = Some(arm.span);
                }
                if arm_false && !covers_false {
                    false_witness = Some(arm.span);
                }
                covers_true |= arm_true;
                covers_false |= arm_false;

                if covers_true && covers_false {
                    result.coverage_complete_at = Some(arm.span);
                }
            }

            if !covers_true {
                result.missing_patterns.push("true".to_string());
            }
            if !covers_false {
                result.missing_patterns.push("false".to_string());
            }
        }

        TyKind::Unit => {
            let mut covered = false;
            let mut unit_witness = None;

            for arm in arms {
                if let Some(previous_span) = result.coverage_complete_at {
                    result.push_redundant(previous_span, RedundancyReason::CoveredByPreviousArms);
                    continue;
                }
                if arm.guard.is_some() {
                    result.push_guarded_ignored();
                    continue;
                }
                let arm_covers_unit = pattern_covers_unit(&arm.pattern);
                if arm_covers_unit && covered {
                    result.push_redundant(
                        unit_witness.unwrap_or(arm.span),
                        RedundancyReason::SubsetShadowed,
                    );
                    continue;
                }
                result.push_useful();
                if pattern_is_irrefutable_for(&arm.pattern, scrutinee_ty, ctx) {
                    result.coverage_complete_at = Some(arm.span);
                    if !covered {
                        unit_witness = Some(arm.span);
                    }
                    covered = true;
                    continue;
                }

                if arm_covers_unit && !covered {
                    unit_witness = Some(arm.span);
                }
                covered |= arm_covers_unit;

                if covered {
                    result.coverage_complete_at = Some(arm.span);
                }
            }

            if !covered {
                result.missing_patterns.push("()".to_string());
            }
        }

        TyKind::Named(def_id, args) if is_builtin_option_type(*def_id) => {
            let payload_ty = args.first();
            let mut covers_some = false;
            let mut covers_none = false;
            let mut some_witness = None;
            let mut none_witness = None;

            for arm in arms {
                if let Some(previous_span) = result.coverage_complete_at {
                    result.push_redundant(previous_span, RedundancyReason::CoveredByPreviousArms);
                    continue;
                }
                if arm.guard.is_some() {
                    result.push_guarded_ignored();
                    continue;
                }
                let arm_can_match_some = pattern_can_match_builtin_variant(&arm.pattern, "Some");
                let arm_can_match_none = pattern_can_match_builtin_variant(&arm.pattern, "None");
                let arm_can_match = arm_can_match_some || arm_can_match_none;
                if arm_can_match
                    && (!arm_can_match_some || covers_some)
                    && (!arm_can_match_none || covers_none)
                {
                    let witness = earliest_span(
                        [
                            arm_can_match_some.then_some(some_witness).flatten(),
                            arm_can_match_none.then_some(none_witness).flatten(),
                        ]
                        .into_iter()
                        .flatten(),
                    )
                    .unwrap_or(arm.span);
                    result.push_redundant(witness, RedundancyReason::SubsetShadowed);
                    continue;
                }
                result.push_useful();
                if pattern_is_irrefutable_for(&arm.pattern, scrutinee_ty, ctx) {
                    result.coverage_complete_at = Some(arm.span);
                    if !covers_some {
                        some_witness = Some(arm.span);
                    }
                    if !covers_none {
                        none_witness = Some(arm.span);
                    }
                    covers_some = true;
                    covers_none = true;
                    continue;
                }

                let arm_covers_some =
                    pattern_covers_builtin_variant(&arm.pattern, "Some", payload_ty, ctx);
                let arm_covers_none =
                    pattern_covers_builtin_variant(&arm.pattern, "None", None, ctx);
                if arm_covers_some && !covers_some {
                    some_witness = Some(arm.span);
                }
                if arm_covers_none && !covers_none {
                    none_witness = Some(arm.span);
                }
                covers_some |= arm_covers_some;
                covers_none |= arm_covers_none;

                if covers_some && covers_none {
                    result.coverage_complete_at = Some(arm.span);
                }
            }

            if !covers_some {
                result.missing_patterns.push("Some(_)".to_string());
            }
            if !covers_none {
                result.missing_patterns.push("None".to_string());
            }
        }

        TyKind::Named(def_id, args) if is_builtin_result_type(*def_id) => {
            let ok_ty = args.first();
            let err_ty = args.get(1);
            let mut covers_ok = false;
            let mut covers_err = false;
            let mut ok_witness = None;
            let mut err_witness = None;

            for arm in arms {
                if let Some(previous_span) = result.coverage_complete_at {
                    result.push_redundant(previous_span, RedundancyReason::CoveredByPreviousArms);
                    continue;
                }
                if arm.guard.is_some() {
                    result.push_guarded_ignored();
                    continue;
                }
                let arm_can_match_ok = pattern_can_match_builtin_variant(&arm.pattern, "Ok");
                let arm_can_match_err = pattern_can_match_builtin_variant(&arm.pattern, "Err");
                let arm_can_match = arm_can_match_ok || arm_can_match_err;
                if arm_can_match
                    && (!arm_can_match_ok || covers_ok)
                    && (!arm_can_match_err || covers_err)
                {
                    let witness = earliest_span(
                        [
                            arm_can_match_ok.then_some(ok_witness).flatten(),
                            arm_can_match_err.then_some(err_witness).flatten(),
                        ]
                        .into_iter()
                        .flatten(),
                    )
                    .unwrap_or(arm.span);
                    result.push_redundant(witness, RedundancyReason::SubsetShadowed);
                    continue;
                }
                result.push_useful();
                if pattern_is_irrefutable_for(&arm.pattern, scrutinee_ty, ctx) {
                    result.coverage_complete_at = Some(arm.span);
                    if !covers_ok {
                        ok_witness = Some(arm.span);
                    }
                    if !covers_err {
                        err_witness = Some(arm.span);
                    }
                    covers_ok = true;
                    covers_err = true;
                    continue;
                }

                let arm_covers_ok = pattern_covers_builtin_variant(&arm.pattern, "Ok", ok_ty, ctx);
                let arm_covers_err =
                    pattern_covers_builtin_variant(&arm.pattern, "Err", err_ty, ctx);
                if arm_covers_ok && !covers_ok {
                    ok_witness = Some(arm.span);
                }
                if arm_covers_err && !covers_err {
                    err_witness = Some(arm.span);
                }
                covers_ok |= arm_covers_ok;
                covers_err |= arm_covers_err;

                if covers_ok && covers_err {
                    result.coverage_complete_at = Some(arm.span);
                }
            }

            if !covers_ok {
                result.missing_patterns.push("Ok(_)".to_string());
            }
            if !covers_err {
                result.missing_patterns.push("Err(_)".to_string());
            }
        }

        TyKind::Named(enum_id, _) if ctx.enums.contains_key(enum_id) => {
            let mut covered_variants: HashMap<String, Span> = HashMap::new();

            for arm in arms {
                if let Some(previous_span) = result.coverage_complete_at {
                    result.push_redundant(previous_span, RedundancyReason::CoveredByPreviousArms);
                    continue;
                }
                if arm.guard.is_some() {
                    result.push_guarded_ignored();
                    continue;
                }
                let Some(enum_info) = ctx.enums.get(enum_id) else {
                    continue;
                };
                let arm_possible_variants = enum_info
                    .variants
                    .keys()
                    .filter(|variant_name| {
                        pattern_can_match_enum_variant(&arm.pattern, *enum_id, variant_name, ctx)
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                if !arm_possible_variants.is_empty()
                    && arm_possible_variants
                        .iter()
                        .all(|variant_name| covered_variants.contains_key(variant_name))
                {
                    let witness =
                        earliest_span(arm_possible_variants.iter().filter_map(|variant_name| {
                            covered_variants.get(variant_name).copied()
                        }))
                        .unwrap_or(arm.span);
                    result.push_redundant(witness, RedundancyReason::SubsetShadowed);
                    continue;
                }
                result.push_useful();
                if pattern_is_irrefutable_for(&arm.pattern, scrutinee_ty, ctx) {
                    result.coverage_complete_at = Some(arm.span);
                    if let Some(enum_info) = ctx.enums.get(enum_id) {
                        covered_variants = enum_info
                            .variant_order
                            .iter()
                            .map(|variant_name| (variant_name.clone(), arm.span))
                            .collect();
                    }
                    continue;
                }

                for variant_name in &enum_info.variant_order {
                    if !covered_variants.contains_key(variant_name)
                        && pattern_covers_enum_variant(&arm.pattern, *enum_id, variant_name, ctx)
                    {
                        covered_variants.insert(variant_name.clone(), arm.span);
                    }
                }

                if covered_variants.len() == enum_info.variants.len() {
                    result.coverage_complete_at = Some(arm.span);
                }
            }

            let covered_variant_names = covered_variants.keys().cloned().collect::<Vec<_>>();
            result.missing_patterns = missing_enum_patterns(*enum_id, &covered_variant_names, ctx);
        }

        TyKind::Record(field_tys) => {
            let declared_fields: Vec<&str> = field_tys.iter().map(|(n, _)| n.as_str()).collect();
            for arm in arms {
                if arm.guard.is_some() {
                    result.push_guarded_ignored();
                    continue;
                }
                if let Some(previous) = result.coverage_complete_at {
                    result.push_redundant(previous, RedundancyReason::CoveredByPreviousArms);
                    continue;
                }
                // Check if this arm covers all declared fields
                let covers_all = matches!(
                    &arm.pattern.kind,
                    PatternKind::Wildcard | PatternKind::Var(_, _) | PatternKind::Binding(_, _, _)
                ) || match &arm.pattern.kind {
                    PatternKind::Record(fields) => {
                        let covered: Vec<&str> = fields.iter().map(|(n, _)| n.as_str()).collect();
                        declared_fields.iter().all(|f| covered.contains(f))
                    }
                    _ => false,
                };

                if covers_all {
                    result.push_useful();
                    result.coverage_complete_at = Some(arm.span);
                } else {
                    result.push_useful();
                }
            }
            if result.coverage_complete_at.is_none() {
                result
                    .missing_patterns
                    .push("full record pattern".to_string());
            }
        }

        TyKind::String | TyKind::Int | TyKind::Float | TyKind::Char => {
            // Primitive types have infinite values — only wildcard/variable is exhaustive
            for arm in arms {
                if arm.guard.is_some() {
                    result.push_guarded_ignored();
                    continue;
                }
                if let Some(previous) = result.coverage_complete_at {
                    result.push_redundant(previous, RedundancyReason::CoveredByPreviousArms);
                    continue;
                }
                if matches!(
                    &arm.pattern.kind,
                    PatternKind::Wildcard | PatternKind::Var(_, _) | PatternKind::Binding(_, _, _)
                ) {
                    result.push_useful();
                    result.coverage_complete_at = Some(arm.span);
                } else {
                    result.push_useful();
                }
            }
            if result.coverage_complete_at.is_none() {
                result.missing_patterns.push("_ (wildcard)".to_string());
            }
        }

        _ => {
            result.arm_usefulness = vec![ArmUsefulness::NotAnalyzed; arms.len()];
        }
    }

    if result.arm_usefulness.is_empty() {
        result.arm_usefulness = vec![ArmUsefulness::Useful; arms.len()];
    }
    result.rebuild_unreachable_arms(arms);
    result
}

fn earliest_span(spans: impl IntoIterator<Item = Span>) -> Option<Span> {
    spans.into_iter().min_by_key(|span| (span.start, span.end))
}

fn bool_pattern_coverage(pattern: &Pattern) -> (bool, bool) {
    match &pattern.kind {
        PatternKind::Wildcard | PatternKind::Var(_, _) => (true, true),
        PatternKind::Binding(_, _, inner) => bool_pattern_coverage(inner),
        PatternKind::Literal(Literal::Bool(value)) => (*value, !*value),
        PatternKind::Or(patterns) => patterns.iter().fold((false, false), |(t, f), pattern| {
            let (covers_true, covers_false) = bool_pattern_coverage(pattern);
            (t || covers_true, f || covers_false)
        }),
        _ => (false, false),
    }
}

fn pattern_covers_unit(pattern: &Pattern) -> bool {
    match &pattern.kind {
        PatternKind::Wildcard | PatternKind::Var(_, _) => true,
        PatternKind::Binding(_, _, inner) => pattern_covers_unit(inner),
        PatternKind::Literal(Literal::Unit) => true,
        PatternKind::Or(patterns) => patterns.iter().any(pattern_covers_unit),
        _ => false,
    }
}

fn pattern_is_irrefutable_for(
    pattern: &Pattern,
    expected: &Ty,
    ctx: &PatternAnalysisContext<'_>,
) -> bool {
    match &pattern.kind {
        PatternKind::Wildcard | PatternKind::Var(_, _) => true,
        PatternKind::Binding(_, _, inner) => pattern_is_irrefutable_for(inner, expected, ctx),
        PatternKind::Literal(Literal::Unit) => matches!(expected.kind, TyKind::Unit),
        PatternKind::Tuple(patterns) => match &expected.kind {
            TyKind::Tuple(elem_tys) if elem_tys.len() == patterns.len() => patterns
                .iter()
                .zip(elem_tys.iter())
                .all(|(pattern, ty)| pattern_is_irrefutable_for(pattern, ty, ctx)),
            _ => false,
        },
        PatternKind::Constructor(def_id, patterns) => {
            let Some(variant) = ctx.variants.get(def_id) else {
                return false;
            };
            let TyKind::Named(enum_id, _) = &expected.kind else {
                return false;
            };
            if variant.enum_id != *enum_id || variant.fields.len() != patterns.len() {
                return false;
            }
            let Some(enum_info) = ctx.enums.get(enum_id) else {
                return false;
            };
            if enum_info.variants.len() != 1 {
                return false;
            }
            patterns
                .iter()
                .zip(variant.fields.iter())
                .all(|(pattern, ty)| pattern_is_irrefutable_for(pattern, ty, ctx))
        }
        PatternKind::Or(patterns) => {
            let (covers_true, covers_false) = bool_pattern_coverage(pattern);
            matches!(expected.kind, TyKind::Bool) && covers_true && covers_false
                || patterns
                    .iter()
                    .any(|pattern| pattern_is_irrefutable_for(pattern, expected, ctx))
        }
        _ => false,
    }
}

fn pattern_covers_enum_variant(
    pattern: &Pattern,
    enum_id: DefId,
    variant_name: &str,
    ctx: &PatternAnalysisContext<'_>,
) -> bool {
    match &pattern.kind {
        PatternKind::Wildcard | PatternKind::Var(_, _) => true,
        PatternKind::Binding(_, _, inner) => {
            pattern_covers_enum_variant(inner, enum_id, variant_name, ctx)
        }
        PatternKind::Constructor(def_id, patterns) => {
            let Some(variant) = ctx.variants.get(def_id) else {
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
                .all(|(pattern, ty)| pattern_is_irrefutable_for(pattern, ty, ctx))
        }
        PatternKind::Or(patterns) => patterns
            .iter()
            .any(|pattern| pattern_covers_enum_variant(pattern, enum_id, variant_name, ctx)),
        _ => false,
    }
}

fn pattern_covers_builtin_variant(
    pattern: &Pattern,
    variant_name: &str,
    payload_ty: Option<&Ty>,
    ctx: &PatternAnalysisContext<'_>,
) -> bool {
    match &pattern.kind {
        PatternKind::Wildcard | PatternKind::Var(_, _) => true,
        PatternKind::Binding(_, _, inner) => {
            pattern_covers_builtin_variant(inner, variant_name, payload_ty, ctx)
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
                    payload_ty.is_some_and(|ty| pattern_is_irrefutable_for(pattern, ty, ctx))
                }
                (Some("None"), "None", []) => true,
                _ => false,
            }
        }
        PatternKind::Or(patterns) => patterns
            .iter()
            .any(|pattern| pattern_covers_builtin_variant(pattern, variant_name, payload_ty, ctx)),
        _ => false,
    }
}

fn pattern_can_match_builtin_variant(pattern: &Pattern, variant_name: &str) -> bool {
    match &pattern.kind {
        PatternKind::Wildcard | PatternKind::Var(_, _) => true,
        PatternKind::Binding(_, _, inner) => pattern_can_match_builtin_variant(inner, variant_name),
        PatternKind::Constructor(def_id, patterns) => matches!(
            (
                builtin_constructor_name(*def_id),
                variant_name,
                patterns.as_slice(),
            ),
            (Some("Some"), "Some", [_])
                | (Some("Ok"), "Ok", [_])
                | (Some("Err"), "Err", [_])
                | (Some("None"), "None", [])
        ),
        PatternKind::Or(patterns) => patterns
            .iter()
            .any(|pattern| pattern_can_match_builtin_variant(pattern, variant_name)),
        _ => false,
    }
}

fn pattern_can_match_enum_variant(
    pattern: &Pattern,
    enum_id: DefId,
    variant_name: &str,
    ctx: &PatternAnalysisContext<'_>,
) -> bool {
    match &pattern.kind {
        PatternKind::Wildcard | PatternKind::Var(_, _) => true,
        PatternKind::Binding(_, _, inner) => {
            pattern_can_match_enum_variant(inner, enum_id, variant_name, ctx)
        }
        PatternKind::Constructor(def_id, patterns) => {
            let Some(variant) = ctx.variants.get(def_id) else {
                return false;
            };
            variant.enum_id == enum_id
                && variant.name == variant_name
                && variant.fields.len() == patterns.len()
        }
        PatternKind::Or(patterns) => patterns
            .iter()
            .any(|pattern| pattern_can_match_enum_variant(pattern, enum_id, variant_name, ctx)),
        _ => false,
    }
}

fn missing_enum_patterns(
    enum_id: DefId,
    covered_variants: &[String],
    ctx: &PatternAnalysisContext<'_>,
) -> Vec<String> {
    let Some(enum_info) = ctx.enums.get(&enum_id) else {
        return Vec::new();
    };

    enum_info
        .variant_order
        .iter()
        .filter_map(|name| {
            let fields = enum_info.variants.get(name)?;
            if covered_variants.iter().any(|covered| covered == name) {
                return None;
            }
            if fields.is_empty() {
                Some(name.clone())
            } else {
                let placeholders = std::iter::repeat_n("_", fields.len()).collect::<Vec<_>>();
                Some(format!("{}({})", name, placeholders.join(", ")))
            }
        })
        .collect()
}
