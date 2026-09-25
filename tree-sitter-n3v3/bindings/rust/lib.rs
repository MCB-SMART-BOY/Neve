//! n3v3 grammar for tree-sitter.
//! tree-sitter 的 n3v3 语法。

use tree_sitter::Language;

unsafe extern "C" {
    fn tree_sitter_n3v3() -> Language;
}

/// Get the tree-sitter Language for n3v3.
/// 获取 n3v3 的 tree-sitter 语言。
pub fn language() -> Language {
    unsafe { tree_sitter_n3v3() }
}

#[cfg(test)]
mod tests {
    use tree_sitter::Parser;

    #[test]
    fn test_can_load_grammar() {
        let _lang = super::language();
    }

    #[test]
    fn test_can_parse_canonical_source() {
        let mut parser = Parser::new();
        parser
            .set_language(&super::language())
            .expect("n3v3 language should be accepted by tree-sitter");
        let tree = parser
            .parse("let answer = 42;\nfn identity(x: Int) -> Int = x;", None)
            .expect("tree-sitter should produce a parse tree");

        assert!(!tree.root_node().has_error());
    }
}
