use super::CompletionSpec;

pub(super) fn specs() -> Vec<CompletionSpec> {
    vec![
        ("list.empty", "Empty list", "list.empty", "List<T>"),
        (
            "list.singleton",
            "Single-element list",
            "list.singleton(${1:x})",
            "List<T>",
        ),
        ("list.len", "List length", "list.len(${1:xs})", "Int"),
        (
            "list.isEmpty",
            "Check emptiness",
            "list.isEmpty(${1:xs})",
            "Bool",
        ),
        (
            "list.head",
            "First element",
            "list.head(${1:xs})",
            "Option<T>",
        ),
        (
            "list.tail",
            "All but first",
            "list.tail(${1:xs})",
            "List<T>",
        ),
        (
            "list.last",
            "Last element",
            "list.last(${1:xs})",
            "Option<T>",
        ),
        ("list.init", "All but last", "list.init(${1:xs})", "List<T>"),
        (
            "list.get",
            "Get element by index",
            "list.get(${1:index}, ${2:xs})",
            "Option<T>",
        ),
        (
            "list.cons",
            "Prepend element",
            "list.cons(${1:x}, ${2:xs})",
            "List<T>",
        ),
        (
            "list.take",
            "Take prefix",
            "list.take(${1:n}, ${2:xs})",
            "List<T>",
        ),
        (
            "list.drop",
            "Drop prefix",
            "list.drop(${1:n}, ${2:xs})",
            "List<T>",
        ),
        (
            "list.contains",
            "Check membership",
            "list.contains(${1:x}, ${2:xs})",
            "Bool",
        ),
        (
            "list.indexOf",
            "Find element index",
            "list.indexOf(${1:x}, ${2:xs})",
            "Option<Int>",
        ),
        (
            "list.reverse",
            "Reverse list",
            "list.reverse(${1:xs})",
            "List<T>",
        ),
        (
            "list.map",
            "Map function over list",
            "list.map(${1:f}, ${2:xs})",
            "List<U>",
        ),
        (
            "list.filter",
            "Filter list",
            "list.filter(${1:pred}, ${2:xs})",
            "List<T>",
        ),
        (
            "list.fold",
            "Fold list",
            "list.fold(${1:init}, ${2:f}, ${3:xs})",
            "U",
        ),
        (
            "list.foldRight",
            "Right fold list",
            "list.foldRight(${1:init}, ${2:f}, ${3:xs})",
            "U",
        ),
        ("list.sum", "Sum integers", "list.sum(${1:xs})", "Int"),
        (
            "list.product",
            "Multiply integers",
            "list.product(${1:xs})",
            "Int",
        ),
        ("list.sort", "Sort list", "list.sort(${1:xs})", "List<T>"),
        (
            "list.max",
            "Maximum integer element",
            "list.max(${1:xs})",
            "Option<Int>",
        ),
        (
            "list.min",
            "Minimum integer element",
            "list.min(${1:xs})",
            "Option<Int>",
        ),
        (
            "list.range",
            "Create range",
            "list.range(${1:start}, ${2:end})",
            "List<Int>",
        ),
        (
            "list.replicate",
            "Repeat value",
            "list.replicate(${1:n}, ${2:value})",
            "List<T>",
        ),
        (
            "list.zip",
            "Zip two lists",
            "list.zip(${1:xs}, ${2:ys})",
            "List<(T, U)>",
        ),
        (
            "list.unzip",
            "Unzip pairs",
            "list.unzip(${1:pairs})",
            "(List<T>, List<U>)",
        ),
    ]
}
