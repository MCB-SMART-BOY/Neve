use neve_fmt::format;

fn assert_idempotent(source: &str) {
    let first = format(source).expect("first format should succeed");
    let second = format(&first).expect("second format should succeed");
    assert_eq!(
        first, second,
        "not idempotent:\n--- first ---\n{first}\n--- second ---\n{second}"
    );
}

#[test]
fn test_int() {
    assert_idempotent("let x = 42;");
}
#[test]
fn test_float() {
    assert_idempotent("let x = 3.14;");
}
#[test]
fn test_bool() {
    assert_idempotent("let x = true;");
}
#[test]
fn test_string() {
    assert_idempotent(r#"let x = "hello";"#);
}
#[test]
fn test_unit() {
    assert_idempotent("let x = ();");
}
#[test]
fn test_add() {
    assert_idempotent("let x = 1 + 2;");
}
#[test]
fn test_sub() {
    assert_idempotent("let x = 3 - 1;");
}
#[test]
fn test_mul() {
    assert_idempotent("let x = 2 * 3;");
}
#[test]
fn test_div() {
    assert_idempotent("let x = 10 / 2;");
}
#[test]
fn test_nested_arith() {
    assert_idempotent("let x = (1 + 2) * (3 - 1);");
}
#[test]
fn test_multi_let() {
    assert_idempotent("let x = 1;\nlet y = 2;");
}
#[test]
fn test_type_ann() {
    assert_idempotent("let x: Int = 42;");
}
#[test]
fn test_fn_simple() {
    assert_idempotent("fn add(x: Int, y: Int) -> Int = x + y;");
}
#[test]
fn test_fn_no_params() {
    assert_idempotent("fn answer() -> Int = 42;");
}
#[test]
fn test_fn_effect() {
    assert_idempotent("fn run() -> Unit effect = print(\"hello\");");
}

#[test]
fn test_list() {
    assert_idempotent("let xs = [1, 2, 3];");
}
#[test]
fn test_list_empty() {
    assert_idempotent("let xs = [];");
}
#[test]
fn test_record() {
    assert_idempotent("let r = #{x = 1, y = 2};");
}
#[test]
fn test_record_empty() {
    assert_idempotent("let r = #{};");
}

#[test]
fn test_match_wildcard() {
    assert_idempotent("let x = match 42 { _ -> 0 };");
}
#[test]
fn test_match_bool() {
    assert_idempotent("let x = match b { true -> 1, false -> 0 };");
}
#[test]
fn test_match_multi() {
    assert_idempotent("let x = match n { 0 -> \"zero\", 1 -> \"one\", _ -> \"many\" };");
}

#[test]
fn test_import() {
    assert_idempotent("import std.io as io;");
}
#[test]
fn test_enum() {
    assert_idempotent("enum Color { Red, Green, Blue };");
}
#[test]
fn test_struct() {
    assert_idempotent("struct Point { x: Int, y: Int };");
}
#[test]
fn test_trait() {
    assert_idempotent("trait Show { fn show(self) -> String; };");
}
