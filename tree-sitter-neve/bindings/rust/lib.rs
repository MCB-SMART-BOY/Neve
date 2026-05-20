//! Neve grammar for tree-sitter.
//! tree-sitter 的 Neve 语法。

use tree_sitter::Language;

extern "C" {
    fn tree_sitter_neve() -> Language;
}

/// Get the tree-sitter Language for Neve.
/// 获取 Neve 的 tree-sitter 语言。
pub fn language() -> Language {
    unsafe { tree_sitter_neve() }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_can_load_grammar() {
        let _lang = super::language();
    }
}
