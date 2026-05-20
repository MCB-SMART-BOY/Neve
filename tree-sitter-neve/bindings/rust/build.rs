fn main() {
    let src_dir = std::path::Path::new("src");
    let mut c_config = cc::Build::new();
    c_config.include(src_dir);
    c_config
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-trigraphs");
    for path in &["parser.c"] {
        c_config.file(src_dir.join(path).to_str().unwrap());
    }
    c_config.compile("tree-sitter-neve");
    println!("cargo:rerun-if-changed=src/parser.c");
}
