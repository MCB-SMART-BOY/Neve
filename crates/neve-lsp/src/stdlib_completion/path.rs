use super::CompletionSpec;

pub(super) fn specs() -> Vec<CompletionSpec> {
    vec![
        (
            "path.fromString",
            "Construct typed path",
            "path.fromString(${1:path})",
            "Path",
        ),
        (
            "path.joinPath",
            "Join typed path with child",
            "path.joinPath(${1:base}, ${2:child})",
            "Path",
        ),
        (
            "path.parentPath",
            "Get parent typed path",
            "path.parentPath(${1:path})",
            "Option<Path>",
        ),
        (
            "path.filenamePath",
            "Get typed path file name",
            "path.filenamePath(${1:path})",
            "Option<String>",
        ),
        (
            "path.extensionPath",
            "Get typed path extension",
            "path.extensionPath(${1:path})",
            "Option<String>",
        ),
        (
            "path.isAbsolutePath",
            "Check if typed path is absolute",
            "path.isAbsolutePath(${1:path})",
            "Bool",
        ),
        (
            "path.join",
            "Join string paths",
            "path.join(${1:base}, ${2:part})",
            "String",
        ),
        (
            "path.parent",
            "Get parent directory string",
            "path.parent(${1:path})",
            "Option<String>",
        ),
        (
            "path.filename",
            "Get file name string",
            "path.filename(${1:path})",
            "Option<String>",
        ),
        (
            "path.extension",
            "Get extension string",
            "path.extension(${1:path})",
            "Option<String>",
        ),
        (
            "path.is_absolute",
            "Check if string path is absolute",
            "path.is_absolute(${1:path})",
            "Bool",
        ),
    ]
}
