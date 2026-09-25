use super::CompletionSpec;

pub(super) fn specs() -> Vec<CompletionSpec> {
    vec![
        ("string.len", "String length", "string.len(${1:s})", "Int"),
        (
            "string.chars",
            "Characters of string",
            "string.chars(${1:s})",
            "List[Char]",
        ),
        (
            "string.split",
            "Split string",
            "string.split(${1:s}, ${2:sep})",
            "List[String]",
        ),
        (
            "string.join",
            "Join strings",
            "string.join(${1:xs}, ${2:sep})",
            "String",
        ),
        (
            "string.trim",
            "Trim whitespace",
            "string.trim(${1:s})",
            "String",
        ),
        (
            "string.upper",
            "To uppercase",
            "string.upper(${1:s})",
            "String",
        ),
        (
            "string.lower",
            "To lowercase",
            "string.lower(${1:s})",
            "String",
        ),
        (
            "string.contains",
            "Check if contains",
            "string.contains(${1:s}, ${2:needle})",
            "Bool",
        ),
        (
            "string.startsWith",
            "Check prefix",
            "string.startsWith(${1:s}, ${2:prefix})",
            "Bool",
        ),
        (
            "string.endsWith",
            "Check suffix",
            "string.endsWith(${1:s}, ${2:suffix})",
            "Bool",
        ),
        (
            "string.replace",
            "Replace substring",
            "string.replace(${1:s}, ${2:from}, ${3:to})",
            "String",
        ),
        (
            "string.substring",
            "Substring by range",
            "string.substring(${1:s}, ${2:start}, ${3:end})",
            "String",
        ),
        (
            "string.isEmpty",
            "Check if string is empty",
            "string.isEmpty(${1:s})",
            "Bool",
        ),
        (
            "string.repeat",
            "Repeat string",
            "string.repeat(${1:s}, ${2:n})",
            "String",
        ),
        (
            "string.lines",
            "Split string into lines",
            "string.lines(${1:s})",
            "List[String]",
        ),
    ]
}
