//! Pattern matching optimizations.
//! 模式匹配优化。
//!
//! This module provides pattern analysis and optimization for faster matching:
//! 本模块提供模式分析和优化以实现更快的匹配：
//!
//! - Pattern specificity calculation / 模式特异性计算
//! - Fast-path detection for common patterns / 常见模式的快速路径检测
//! - Match arm ordering hints / 匹配分支排序提示

use neve_syntax::{LiteralPattern, Pattern, PatternKind};

/// Pattern specificity score - higher means more specific.
/// 模式特异性分数 - 越高表示越具体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Specificity(u32);

impl Specificity {
    /// Wildcard pattern - matches everything. / 通配符模式 - 匹配所有。
    pub const WILDCARD: Self = Self(0);
    /// Variable binding - matches everything but binds. / 变量绑定 - 匹配所有但绑定。
    pub const VARIABLE: Self = Self(1);
    /// Or pattern - partial specificity. / Or 模式 - 部分特异性。
    pub const OR_BASE: Self = Self(10);
    /// Constructor pattern base. / 构造器模式基础。
    pub const CONSTRUCTOR_BASE: Self = Self(100);
    /// Literal pattern - most specific. / 字面量模式 - 最具体。
    pub const LITERAL: Self = Self(1000);

    /// Combine specificities (e.g., for tuple patterns).
    /// 组合特异性（例如用于元组模式）。
    pub fn combine(self, other: Self) -> Self {
        Self(self.0.saturating_add(other.0))
    }

    /// Get raw score. / 获取原始分数。
    pub fn score(self) -> u32 {
        self.0
    }
}

/// Calculate the specificity of a pattern.
/// 计算模式的特异性。
pub fn pattern_specificity(pattern: &Pattern) -> Specificity {
    match &pattern.kind {
        PatternKind::Wildcard => Specificity::WILDCARD,
        PatternKind::Var(ident) => {
            if ident.name == "_" {
                Specificity::WILDCARD
            } else {
                Specificity::VARIABLE
            }
        }
        PatternKind::Literal(_) => Specificity::LITERAL,
        PatternKind::Tuple(patterns) => {
            let mut spec = Specificity::CONSTRUCTOR_BASE;
            for p in patterns {
                spec = spec.combine(pattern_specificity(p));
            }
            spec
        }
        PatternKind::List(patterns) => {
            let mut spec = Specificity::CONSTRUCTOR_BASE;
            for p in patterns {
                spec = spec.combine(pattern_specificity(p));
            }
            // Add length bonus / 添加长度奖励
            spec = spec.combine(Specificity(patterns.len() as u32 * 10));
            spec
        }
        PatternKind::Record { fields, rest } => {
            let mut spec = Specificity::CONSTRUCTOR_BASE;
            for field in fields {
                if let Some(ref pat) = field.pattern {
                    spec = spec.combine(pattern_specificity(pat));
                } else {
                    spec = spec.combine(Specificity::VARIABLE);
                }
            }
            // Closed record (no rest) is more specific / 封闭记录（无 rest）更具体
            if !rest {
                spec = spec.combine(Specificity(50));
            }
            spec
        }
        PatternKind::ListRest { init, rest: _, tail } => {
            let mut spec = Specificity::CONSTRUCTOR_BASE;
            for p in init.iter().chain(tail.iter()) {
                spec = spec.combine(pattern_specificity(p));
            }
            spec
        }
        PatternKind::Constructor { path: _, args } => {
            let mut spec = Specificity::CONSTRUCTOR_BASE;
            for p in args {
                spec = spec.combine(pattern_specificity(p));
            }
            spec
        }
        PatternKind::Or(patterns) => {
            // Or patterns are as specific as their least specific alternative
            // Or 模式的特异性等于其最不具体的替代项
            patterns
                .iter()
                .map(pattern_specificity)
                .min()
                .unwrap_or(Specificity::OR_BASE)
        }
        PatternKind::Binding { pattern, .. } => {
            // Binding adds minimal specificity to inner pattern
            // 绑定为内部模式添加最小特异性
            pattern_specificity(pattern).combine(Specificity::VARIABLE)
        }
    }
}

/// Pattern kind classification for fast-path matching.
/// 用于快速路径匹配的模式类型分类。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternClass {
    /// Always matches (wildcard, variable). / 总是匹配（通配符、变量）。
    Irrefutable,
    /// Matches a specific literal value. / 匹配特定字面量值。
    Literal,
    /// Matches a constructor (Some, None, Ok, Err, tuple, list).
    /// 匹配构造器（Some、None、Ok、Err、元组、列表）。
    Constructor,
    /// Matches a record structure. / 匹配记录结构。
    Record,
    /// Or pattern with alternatives. / 带有替代项的 Or 模式。
    Disjunction,
    /// Other complex patterns. / 其他复杂模式。
    Complex,
}

/// Classify a pattern for optimization hints.
/// 对模式进行分类以获取优化提示。
pub fn classify_pattern(pattern: &Pattern) -> PatternClass {
    match &pattern.kind {
        PatternKind::Wildcard => PatternClass::Irrefutable,
        PatternKind::Var(_) => PatternClass::Irrefutable,
        PatternKind::Literal(_) => PatternClass::Literal,
        PatternKind::Tuple(_)
        | PatternKind::List(_)
        | PatternKind::ListRest { .. }
        | PatternKind::Constructor { .. } => PatternClass::Constructor,
        PatternKind::Record { .. } => PatternClass::Record,
        PatternKind::Or(_) => PatternClass::Disjunction,
        PatternKind::Binding { pattern, .. } => classify_pattern(pattern),
    }
}

/// Check if a pattern is irrefutable (always matches).
/// 检查模式是否不可反驳（总是匹配）。
pub fn is_irrefutable(pattern: &Pattern) -> bool {
    match &pattern.kind {
        PatternKind::Wildcard => true,
        PatternKind::Var(_) => true,
        PatternKind::Binding { pattern, .. } => is_irrefutable(pattern),
        PatternKind::Tuple(patterns) => patterns.iter().all(is_irrefutable),
        PatternKind::Record { fields, rest } => {
            *rest && fields.iter().all(|f| {
                f.pattern.as_ref().map(is_irrefutable).unwrap_or(true)
            })
        }
        _ => false,
    }
}

/// Get discriminant hint for a pattern (what value component to check first).
/// 获取模式的判别提示（首先检查哪个值组件）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Discriminant {
    /// Check type/constructor tag. / 检查类型/构造器标签。
    Tag,
    /// Check literal value. / 检查字面量值。
    Value,
    /// Check collection length. / 检查集合长度。
    Length,
    /// Check record field presence. / 检查记录字段存在性。
    Field(String),
    /// No discriminant needed (irrefutable). / 不需要判别（不可反驳）。
    None,
}

/// Get the primary discriminant for a pattern.
/// 获取模式的主要判别。
pub fn get_discriminant(pattern: &Pattern) -> Discriminant {
    match &pattern.kind {
        PatternKind::Wildcard | PatternKind::Var(_) => Discriminant::None,
        PatternKind::Literal(_) => Discriminant::Value,
        PatternKind::Tuple(patterns) | PatternKind::List(patterns) => {
            if patterns.is_empty() {
                Discriminant::Length
            } else {
                Discriminant::Tag
            }
        }
        PatternKind::ListRest { .. } => Discriminant::Tag,
        PatternKind::Constructor { .. } => Discriminant::Tag,
        PatternKind::Record { fields, .. } => {
            if let Some(first) = fields.first() {
                Discriminant::Field(first.name.name.clone())
            } else {
                Discriminant::Tag
            }
        }
        PatternKind::Or(patterns) => {
            // Use discriminant of first alternative / 使用第一个替代项的判别
            patterns.first().map(get_discriminant).unwrap_or(Discriminant::None)
        }
        PatternKind::Binding { pattern, .. } => get_discriminant(pattern),
    }
}

/// Extract literal value from a literal pattern for direct comparison.
/// 从字面量模式中提取字面量值以进行直接比较。
#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
}

/// Try to extract a literal value from a pattern.
/// 尝试从模式中提取字面量值。
pub fn extract_literal(pattern: &Pattern) -> Option<LiteralValue> {
    match &pattern.kind {
        PatternKind::Literal(lit) => match lit {
            LiteralPattern::Int(n) => Some(LiteralValue::Int(*n)),
            LiteralPattern::Float(f) => Some(LiteralValue::Float(*f)),
            LiteralPattern::String(s) => Some(LiteralValue::String(s.clone())),
            LiteralPattern::Char(c) => Some(LiteralValue::Char(*c)),
            LiteralPattern::Bool(b) => Some(LiteralValue::Bool(*b)),
        },
        PatternKind::Binding { pattern, .. } => extract_literal(pattern),
        _ => None,
    }
}

/// Optimization hints for a match expression.
/// 匹配表达式的优化提示。
#[derive(Debug, Clone)]
pub struct MatchHints {
    /// Whether all arms are irrefutable (only last should be).
    /// 是否所有分支都不可反驳（只有最后一个应该是）。
    pub has_irrefutable_non_last: bool,
    /// Whether there's a catch-all pattern. / 是否有全捕获模式。
    pub has_catchall: bool,
    /// Dominant discriminant type. / 主要判别类型。
    pub primary_discriminant: Discriminant,
    /// Number of literal patterns (for switch optimization).
    /// 字面量模式的数量（用于 switch 优化）。
    pub literal_count: usize,
    /// Whether arms could be reordered for efficiency.
    /// 分支是否可以重新排序以提高效率。
    pub could_reorder: bool,
}

/// Analyze a match expression's arms for optimization hints.
/// 分析匹配表达式的分支以获取优化提示。
pub fn analyze_match(patterns: &[&Pattern]) -> MatchHints {
    let mut has_irrefutable_non_last = false;
    let mut has_catchall = false;
    let mut literal_count = 0;
    let mut discriminants = Vec::new();

    for (i, pattern) in patterns.iter().enumerate() {
        let is_last = i == patterns.len() - 1;
        
        if is_irrefutable(pattern) {
            has_catchall = true;
            if !is_last {
                has_irrefutable_non_last = true;
            }
        }

        if extract_literal(pattern).is_some() {
            literal_count += 1;
        }

        discriminants.push(get_discriminant(pattern));
    }

    // Find primary discriminant (most common) / 找到主要判别（最常见的）
    let primary_discriminant = discriminants
        .first()
        .cloned()
        .unwrap_or(Discriminant::None);

    // Could reorder if no irrefutable patterns before last
    // 如果最后一个之前没有不可反驳的模式，则可以重新排序
    let could_reorder = !has_irrefutable_non_last && patterns.len() > 2;

    MatchHints {
        has_irrefutable_non_last,
        has_catchall,
        primary_discriminant,
        literal_count,
        could_reorder,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use neve_common::{BytePos, Span};
    use neve_syntax::Ident;

    fn make_span() -> Span {
        Span::new(BytePos::ZERO, BytePos::ZERO)
    }

    fn wildcard() -> Pattern {
        Pattern {
            kind: PatternKind::Wildcard,
            span: make_span(),
        }
    }

    fn var(name: &str) -> Pattern {
        Pattern {
            kind: PatternKind::Var(Ident {
                name: name.to_string(),
                span: make_span(),
            }),
            span: make_span(),
        }
    }

    fn int_lit(n: i64) -> Pattern {
        Pattern {
            kind: PatternKind::Literal(LiteralPattern::Int(n)),
            span: make_span(),
        }
    }

    #[test]
    fn test_specificity_ordering() {
        assert!(Specificity::WILDCARD < Specificity::VARIABLE);
        assert!(Specificity::VARIABLE < Specificity::CONSTRUCTOR_BASE);
        assert!(Specificity::CONSTRUCTOR_BASE < Specificity::LITERAL);
    }

    #[test]
    fn test_pattern_specificity() {
        assert_eq!(pattern_specificity(&wildcard()), Specificity::WILDCARD);
        assert_eq!(pattern_specificity(&var("x")), Specificity::VARIABLE);
        assert_eq!(pattern_specificity(&int_lit(42)), Specificity::LITERAL);
    }

    #[test]
    fn test_is_irrefutable() {
        assert!(is_irrefutable(&wildcard()));
        assert!(is_irrefutable(&var("x")));
        assert!(!is_irrefutable(&int_lit(42)));
    }

    #[test]
    fn test_classify_pattern() {
        assert_eq!(classify_pattern(&wildcard()), PatternClass::Irrefutable);
        assert_eq!(classify_pattern(&var("x")), PatternClass::Irrefutable);
        assert_eq!(classify_pattern(&int_lit(42)), PatternClass::Literal);
    }
}
