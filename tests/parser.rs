//! Integration tests for neve-parser crate.

use neve_parser::parse;

// ============================================================================
// Basic Parsing Tests
// ============================================================================

#[test]
fn test_parse_let() {
    let (file, diags) = parse("let x = 42;");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_parse_fn() {
    let (file, diags) = parse("fn add(x: Int, y: Int) -> Int = x + y;");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_parse_record() {
    let (file, diags) = parse("let r = #{ x = 1, y = 2 };");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_parse_if_expr() {
    let (file, diags) = parse("let x = if true then 1 else 2;");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_parse_match_expr() {
    let (file, diags) = parse("let x = match y { Some(v) => v, None => 0 };");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_parse_lambda() {
    let (file, diags) = parse("let f = fn(x) x + 1;");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_parse_type_alias() {
    let (file, diags) = parse("type MyInt = Int;");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_parse_list() {
    let (file, diags) = parse("let xs = [1, 2, 3];");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_parse_binary_ops() {
    let (file, diags) = parse("let x = 1 + 2 * 3;");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_parse_pipe_operator() {
    let (file, diags) = parse("let x = y |> f |> g;");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

// ============================================================================
// Edge Cases - Let Bindings
// ============================================================================

#[test]
fn test_let_simple_value() {
    let (file, diags) = parse("let x = 1;");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_let_with_type_annotation() {
    let (file, diags) = parse("let x: Int = 42;");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_let_string() {
    let (file, diags) = parse(r#"let s = "hello";"#);
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_let_boolean_true() {
    let (file, diags) = parse("let b = true;");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_let_boolean_false() {
    let (file, diags) = parse("let b = false;");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_let_float() {
    let (file, diags) = parse("let f = 3.14;");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_let_negative_number() {
    let (file, diags) = parse("let n = -42;");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_multiple_lets() {
    let (file, diags) = parse("let x = 1; let y = 2; let z = 3;");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 3);
}

#[test]
fn test_let_underscore() {
    let (file, diags) = parse("let _ = 42;");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

// ============================================================================
// Edge Cases - Functions
// ============================================================================

#[test]
fn test_fn_no_params() {
    let (file, diags) = parse("fn foo() = 42;");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_fn_single_param() {
    let (file, diags) = parse("fn double(x) = x * 2;");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_fn_multiple_params() {
    let (file, diags) = parse("fn add3(a, b, c) = a + b + c;");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_fn_with_type_params() {
    let (file, diags) = parse("fn id(x: a) -> a = x;");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_fn_complex_return_type() {
    let (file, diags) = parse("fn pair(x: Int, y: Int) -> (Int, Int) = (x, y);");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_fn_calling_another() {
    let (file, diags) = parse("fn foo() = bar();");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_fn_recursive_reference() {
    let (file, diags) = parse("fn fact(n) = if n <= 1 then 1 else n * fact(n - 1);");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

// ============================================================================
// Edge Cases - Expressions
// ============================================================================

#[test]
fn test_parenthesized_expr() {
    let (_, diags) = parse("let x = (1 + 2);");
    assert!(diags.is_empty());
}

#[test]
fn test_nested_parens() {
    let (_, diags) = parse("let x = ((((1))));");
    assert!(diags.is_empty());
}

#[test]
fn test_complex_arithmetic() {
    let (_, diags) = parse("let x = 1 + 2 * 3 - 4 / 2;");
    assert!(diags.is_empty());
}

#[test]
fn test_comparison_chain() {
    let (_, diags) = parse("let x = 1 < 2;");
    assert!(diags.is_empty());
}

#[test]
fn test_logical_and() {
    let (_, diags) = parse("let x = true && false;");
    assert!(diags.is_empty());
}

#[test]
fn test_logical_or() {
    let (_, diags) = parse("let x = true || false;");
    assert!(diags.is_empty());
}

#[test]
fn test_logical_not() {
    let (_, diags) = parse("let x = !true;");
    assert!(diags.is_empty());
}

#[test]
fn test_mixed_logical() {
    let (_, diags) = parse("let x = true && false || !true;");
    assert!(diags.is_empty());
}

#[test]
fn test_string_concat() {
    let (_, diags) = parse(r#"let s = "hello" ++ " world";"#);
    assert!(diags.is_empty());
}

// ============================================================================
// Edge Cases - If Expressions
// ============================================================================

#[test]
fn test_if_simple() {
    let (_, diags) = parse("let x = if true then 1 else 0;");
    assert!(diags.is_empty());
}

#[test]
fn test_if_nested_condition() {
    let (_, diags) = parse("let x = if (a && b) then 1 else 0;");
    assert!(diags.is_empty());
}

#[test]
fn test_if_nested_then() {
    let (_, diags) = parse("let x = if a then if b then 1 else 2 else 3;");
    assert!(diags.is_empty());
}

#[test]
fn test_if_complex_branches() {
    let (_, diags) = parse("let x = if cond then foo(1, 2) else bar(3, 4);");
    assert!(diags.is_empty());
}

#[test]
fn test_if_with_comparison() {
    let (_, diags) = parse("let x = if n > 0 then n else 0;");
    assert!(diags.is_empty());
}

// ============================================================================
// Edge Cases - Lists
// ============================================================================

#[test]
fn test_empty_list() {
    let (_, diags) = parse("let xs = [];");
    assert!(diags.is_empty());
}

#[test]
fn test_single_element_list() {
    let (_, diags) = parse("let xs = [1];");
    assert!(diags.is_empty());
}

#[test]
fn test_list_trailing_comma() {
    let (file, diags) = parse("let xs = [1, 2, 3,];");
    // Trailing comma is allowed
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_nested_list() {
    let (_, diags) = parse("let xs = [[1, 2], [3, 4]];");
    assert!(diags.is_empty());
}

#[test]
fn test_list_of_strings() {
    let (_, diags) = parse(r#"let xs = ["a", "b", "c"];"#);
    assert!(diags.is_empty());
}

#[test]
fn test_list_with_expressions() {
    let (_, diags) = parse("let xs = [1 + 1, 2 * 2, 3 - 1];");
    assert!(diags.is_empty());
}

// ============================================================================
// Edge Cases - Records
// ============================================================================

#[test]
fn test_empty_record() {
    let (_, diags) = parse("let r = #{};");
    assert!(diags.is_empty());
}

#[test]
fn test_single_field_record() {
    let (_, diags) = parse("let r = #{ x = 1 };");
    assert!(diags.is_empty());
}

#[test]
fn test_record_trailing_comma() {
    let (file, diags) = parse("let r = #{ x = 1, y = 2, };");
    // Trailing comma is allowed
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 1);
}

#[test]
fn test_nested_record() {
    let (_, diags) = parse("let r = #{ inner = #{ x = 1 } };");
    assert!(diags.is_empty());
}

#[test]
fn test_record_with_expressions() {
    let (_, diags) = parse("let r = #{ sum = 1 + 2, product = 3 * 4 };");
    assert!(diags.is_empty());
}

#[test]
fn test_record_access() {
    let (_, diags) = parse("let x = r.field;");
    assert!(diags.is_empty());
}

#[test]
fn test_chained_record_access() {
    let (_, diags) = parse("let x = a.b.c.d;");
    assert!(diags.is_empty());
}

// ============================================================================
// Edge Cases - Tuples
// ============================================================================

#[test]
fn test_pair() {
    let (_, diags) = parse("let p = (1, 2);");
    assert!(diags.is_empty());
}

#[test]
fn test_triple() {
    let (_, diags) = parse("let t = (1, 2, 3);");
    assert!(diags.is_empty());
}

#[test]
fn test_nested_tuple() {
    let (_, diags) = parse("let t = ((1, 2), (3, 4));");
    assert!(diags.is_empty());
}

#[test]
fn test_mixed_tuple() {
    let (_, diags) = parse(r#"let t = (1, "hello", true);"#);
    assert!(diags.is_empty());
}

// ============================================================================
// Edge Cases - Function Calls
// ============================================================================

#[test]
fn test_call_no_args() {
    let (_, diags) = parse("let x = foo();");
    assert!(diags.is_empty());
}

#[test]
fn test_call_single_arg() {
    let (_, diags) = parse("let x = foo(42);");
    assert!(diags.is_empty());
}

#[test]
fn test_call_multiple_args() {
    let (_, diags) = parse("let x = foo(1, 2, 3);");
    assert!(diags.is_empty());
}

#[test]
fn test_call_with_expression_args() {
    let (_, diags) = parse("let x = foo(1 + 2, a * b);");
    assert!(diags.is_empty());
}

#[test]
fn test_nested_calls() {
    let (_, diags) = parse("let x = foo(bar(baz(1)));");
    assert!(diags.is_empty());
}

#[test]
fn test_method_chain() {
    let (_, diags) = parse("let x = obj.method1().method2();");
    assert!(diags.is_empty());
}

// ============================================================================
// Edge Cases - Pipe Operator
// ============================================================================

#[test]
fn test_simple_pipe() {
    let (_, diags) = parse("let x = 1 |> f;");
    assert!(diags.is_empty());
}

#[test]
fn test_long_pipe_chain() {
    let (_, diags) = parse("let x = input |> step1 |> step2 |> step3 |> step4;");
    assert!(diags.is_empty());
}

#[test]
fn test_pipe_with_calls() {
    let (_, diags) = parse("let x = data |> filter |> map;");
    assert!(diags.is_empty());
}

// ============================================================================
// Edge Cases - Type Annotations
// ============================================================================

#[test]
fn test_simple_type() {
    let (_, diags) = parse("let x: Int = 42;");
    assert!(diags.is_empty());
}

#[test]
fn test_function_type() {
    let (_, diags) = parse("fn apply(f: Int -> Int, x: Int) -> Int = f(x);");
    assert!(diags.is_empty());
}

#[test]
fn test_list_type() {
    let (_, diags) = parse("let xs: List<Int> = [1, 2, 3];");
    assert!(diags.is_empty());
}

#[test]
fn test_tuple_type() {
    let (_, diags) = parse("let p: (Int, String) = (1, \"hello\");");
    assert!(diags.is_empty());
}

// ============================================================================
// Error Recovery Tests
// ============================================================================

#[test]
fn test_recovery_missing_semicolon() {
    let (file, diags) = parse("let x = 42 let y = 10;");
    assert!(!diags.is_empty());
    assert!(file.items.len() >= 1);
}

#[test]
fn test_recovery_invalid_expression() {
    let (file, diags) = parse("let x = @@@; let y = 10;");
    assert!(!diags.is_empty());
    assert!(file.items.len() >= 1);
}

#[test]
fn test_recovery_multiple_errors() {
    let (file, diags) = parse("let x = ; let y = ; let z = 42;");
    assert!(diags.len() >= 2);
    assert!(file.items.len() >= 1);
}

#[test]
fn test_recovery_unbalanced_parens() {
    let (file, diags) = parse("let x = (1 + 2; let y = 3;");
    assert!(!diags.is_empty());
    assert!(file.items.len() >= 1);
}

#[test]
fn test_recovery_unbalanced_braces() {
    let (file, diags) = parse("let x = #{ a = 1; let y = 3;");
    assert!(!diags.is_empty());
    assert!(file.items.len() >= 1);
}

#[test]
fn test_recovery_missing_equals() {
    let (_, diags) = parse("let x 42;");
    assert!(!diags.is_empty());
}

#[test]
fn test_recovery_missing_value() {
    let (_, diags) = parse("let x = ;");
    assert!(!diags.is_empty());
}

#[test]
fn test_recovery_extra_comma() {
    let (_, diags) = parse("let xs = [1,, 2];");
    assert!(!diags.is_empty());
}

#[test]
fn test_recovery_missing_closing_bracket() {
    let (_, diags) = parse("let xs = [1, 2, 3;");
    assert!(!diags.is_empty());
}

#[test]
fn test_recovery_after_valid() {
    let (file, diags) = parse("let x = 1; let y = @; let z = 2;");
    assert!(!diags.is_empty());
    // Should still parse x and z
    assert!(file.items.len() >= 2);
}

// ============================================================================
// Edge Cases - Comments in Code
// ============================================================================

#[test]
fn test_comment_between_tokens() {
    let (_, diags) = parse("let -- comment\n x = 42;");
    assert!(diags.is_empty());
}

#[test]
fn test_comment_after_statement() {
    let (_, diags) = parse("let x = 42; -- this is x");
    assert!(diags.is_empty());
}

#[test]
fn test_multiple_line_comments() {
    let (file, diags) = parse("-- first\nlet x = 1;\n-- second\nlet y = 2;");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 2);
}

// ============================================================================
// Edge Cases - Whitespace Handling
// ============================================================================

#[test]
fn test_extra_whitespace() {
    let (_, diags) = parse("  let   x   =   42  ;  ");
    assert!(diags.is_empty());
}

#[test]
fn test_newlines_in_expression() {
    let (_, diags) = parse("let x = 1\n+ 2\n+ 3;");
    assert!(diags.is_empty());
}

#[test]
fn test_multiline_list() {
    let (_, diags) = parse("let xs = [\n  1,\n  2,\n  3\n];");
    assert!(diags.is_empty());
}

#[test]
fn test_multiline_record() {
    let (_, diags) = parse("let r = #{\n  x = 1,\n  y = 2\n};");
    assert!(diags.is_empty());
}

// ============================================================================
// Edge Cases - Empty/Minimal Input
// ============================================================================

#[test]
fn test_empty_file() {
    let (file, diags) = parse("");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 0);
}

#[test]
fn test_only_whitespace() {
    let (file, diags) = parse("   \n\t\n   ");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 0);
}

#[test]
fn test_only_comments() {
    let (file, diags) = parse("-- just comments\n-- more comments");
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 0);
}

// ============================================================================
// Stress Tests
// ============================================================================

#[test]
fn test_many_items() {
    let source: String = (0..100).map(|i| format!("let x{} = {};", i, i)).collect::<Vec<_>>().join("\n");
    let (file, diags) = parse(&source);
    assert!(diags.is_empty());
    assert_eq!(file.items.len(), 100);
}

#[test]
fn test_deeply_nested_parens() {
    let source = "let x = ".to_string() + &"(".repeat(50) + "1" + &")".repeat(50) + ";";
    let (_, diags) = parse(&source);
    assert!(diags.is_empty());
}

#[test]
fn test_long_expression() {
    let source = "let x = ".to_string() + &(0..100).map(|i| i.to_string()).collect::<Vec<_>>().join(" + ") + ";";
    let (_, diags) = parse(&source);
    assert!(diags.is_empty());
}

#[test]
fn test_large_list() {
    let source = "let xs = [".to_string() + &(0..500).map(|i| i.to_string()).collect::<Vec<_>>().join(", ") + "];";
    let (_, diags) = parse(&source);
    assert!(diags.is_empty());
}

#[test]
fn test_large_record() {
    let source = "let r = #{".to_string() + &(0..100).map(|i| format!("field{} = {}", i, i)).collect::<Vec<_>>().join(", ") + "};";
    let (_, diags) = parse(&source);
    assert!(diags.is_empty());
}
