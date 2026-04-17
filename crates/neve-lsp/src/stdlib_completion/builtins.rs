use super::CompletionSpec;

pub(super) fn specs() -> Vec<CompletionSpec> {
    vec![
        (
            "assert",
            "Assert condition",
            "assert(${1:cond}, ${2:msg})",
            "Unit",
        ),
        ("force", "Force lazy value", "force(${1:lazy_expr})", "T"),
    ]
}
