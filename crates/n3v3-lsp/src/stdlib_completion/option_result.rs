use super::CompletionSpec;

pub(super) fn specs() -> Vec<CompletionSpec> {
    vec![
        (
            "option.some",
            "Wrap in Some",
            "option.some(${1:x})",
            "Option<T>",
        ),
        ("option.none", "None value", "option.none", "Option<T>"),
        (
            "option.is_some",
            "Check if Some",
            "option.is_some(${1:opt})",
            "Bool",
        ),
        (
            "option.is_none",
            "Check if None",
            "option.is_none(${1:opt})",
            "Bool",
        ),
        (
            "option.unwrap",
            "Unwrap option",
            "option.unwrap(${1:opt})",
            "T",
        ),
        (
            "option.unwrap_or",
            "Unwrap or default",
            "option.unwrap_or(${1:opt}, ${2:default})",
            "T",
        ),
        (
            "result.ok",
            "Wrap in Ok",
            "result.ok(${1:x})",
            "Result<T, E>",
        ),
        (
            "result.err",
            "Wrap in Err",
            "result.err(${1:e})",
            "Result<T, E>",
        ),
        (
            "result.is_ok",
            "Check if Ok",
            "result.is_ok(${1:res})",
            "Bool",
        ),
        (
            "result.is_err",
            "Check if Err",
            "result.is_err(${1:res})",
            "Bool",
        ),
        (
            "result.unwrap",
            "Unwrap result",
            "result.unwrap(${1:res})",
            "T",
        ),
        (
            "result.unwrap_err",
            "Unwrap error",
            "result.unwrap_err(${1:res})",
            "E",
        ),
    ]
}
