fn main() {
    let src_dir = std::path::Path::new("../../src");
    let parser_c = src_dir.join("parser.c");

    println!("cargo:rerun-if-changed={}", parser_c.display());
    println!("cargo:rerun-if-changed=../../grammar.js");

    assert!(
        parser_c.is_file(),
        "generated tree-sitter parser is missing: {} (run ./scripts/build-grammar.sh)",
        parser_c.display()
    );

    cc::Build::new()
        .include(src_dir)
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs")
        .file(&parser_c)
        .compile("tree-sitter-neve");
}
