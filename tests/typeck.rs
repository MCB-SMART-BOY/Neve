//! Integration tests for neve-typeck crate.
//!
//! This file contains extensive edge case tests for type checking.

use neve_diagnostic::{Diagnostic, Severity};
use neve_hir::lower;
use neve_parser::parse;
use neve_typeck::TypeChecker;

fn check_source(source: &str) -> Vec<Diagnostic> {
    let (ast, parse_diags) = parse(source);
    if !parse_diags.is_empty() {
        // Return parse errors as diagnostics for tests that check parse failures
        return parse_diags;
    }

    let hir = lower(&ast);
    let mut checker = TypeChecker::new();
    checker.check(&hir);
    checker.diagnostics()
}

fn check_no_errors(source: &str) {
    let diags = check_source(source);
    assert!(diags.is_empty(), "unexpected errors: {:?}", diags);
}

fn check_has_errors(source: &str) {
    let diags = check_source(source);
    assert!(!diags.is_empty(), "expected type errors but got none");
}

fn assert_has_diagnostic(source: &str, severity: Severity, message_fragment: &str) {
    let diags = check_source(source);
    assert!(
        diags
            .iter()
            .any(|diag| { diag.severity == severity && diag.message.contains(message_fragment) }),
        "expected diagnostic containing '{message_fragment}', got {:?}",
        diags
    );
}

// ============================================================================
// 基本类型字面量
// ============================================================================

#[test]
fn test_typeck_int_literal() {
    check_no_errors("let x = 42;");
}

#[test]
fn test_typeck_float_literal() {
    check_no_errors("let x = 3.14;");
}

#[test]
fn test_typeck_bool_literal_true() {
    check_no_errors("let x = true;");
}

#[test]
fn test_typeck_bool_literal_false() {
    check_no_errors("let x = false;");
}

#[test]
fn test_typeck_string_literal() {
    check_no_errors("let x = \"hello\";");
}

#[test]
fn test_typeck_string_empty() {
    check_no_errors("let x = \"\";");
}

#[test]
fn test_typeck_char_literal() {
    check_no_errors("let x = 'a';");
}

// ============================================================================
// 算术运算
// ============================================================================

#[test]
fn test_typeck_int_addition() {
    check_no_errors("let x = 1 + 2;");
}

#[test]
fn test_typeck_int_subtraction() {
    check_no_errors("let x = 5 - 3;");
}

#[test]
fn test_typeck_int_multiplication() {
    check_no_errors("let x = 4 * 5;");
}

#[test]
fn test_typeck_int_division() {
    check_no_errors("let x = 10 / 2;");
}

#[test]
fn test_typeck_int_modulo() {
    check_no_errors("let x = 10 % 3;");
}

#[test]
fn test_typeck_complex_arithmetic() {
    check_no_errors("let x = 1 + 2 * 3 - 4 / 2;");
}

#[test]
fn test_typeck_nested_parentheses() {
    check_no_errors("let x = ((1 + 2) * (3 + 4));");
}

#[test]
fn test_typeck_float_arithmetic() {
    check_no_errors("let x = 1.0 + 2.0;");
}

#[test]
fn test_typeck_float_operations() {
    check_no_errors("let x = 1.5 * 2.5 - 0.5;");
}

// ============================================================================
// 比较运算
// ============================================================================

#[test]
fn test_typeck_less_than() {
    check_no_errors("let x = 1 < 2;");
}

#[test]
fn test_typeck_greater_than() {
    check_no_errors("let x = 2 > 1;");
}

#[test]
fn test_typeck_less_than_or_equal() {
    check_no_errors("let x = 1 <= 2;");
}

#[test]
fn test_typeck_greater_than_or_equal() {
    check_no_errors("let x = 2 >= 1;");
}

#[test]
fn test_typeck_equality() {
    check_no_errors("let x = 1 == 1;");
}

#[test]
fn test_typeck_inequality() {
    check_no_errors("let x = 1 != 2;");
}

#[test]
fn test_typeck_string_equality() {
    check_no_errors("let x = \"a\" == \"b\";");
}

#[test]
fn test_typeck_bool_equality() {
    check_no_errors("let x = true == false;");
}

// ============================================================================
// 枚举构造器
// ============================================================================

#[test]
fn test_typeck_enum_constructor() {
    check_no_errors("enum Option { Some(Int), None }; let x = Some(1);");
}

#[test]
fn test_typeck_enum_match() {
    check_no_errors(
        "enum Option { Some(Int), None }; let x = Some(1); let y = match x { Some(v) -> v, None -> 0 };",
    );
}

#[test]
fn test_typeck_std_list_builtin_signatures_allow_valid_uses() {
    check_no_errors(
        r#"
            import std.list;
            let count: Int = list.len(list.range(1, 4));
            let keep: Bool = list.isEmpty([]);
        "#,
    );
}

#[test]
fn test_typeck_std_list_builtin_signatures_reject_wrong_annotation() {
    assert_has_diagnostic(
        r#"
            fn expectBool(x: Bool) -> Bool = x;
            import std.list (len);
            let wrong = expectBool(len([1, 2, 3]));
        "#,
        Severity::Error,
        "type mismatch",
    );
}

#[test]
fn test_typeck_std_string_builtin_signatures_allow_valid_uses() {
    check_no_errors(
        r#"
            import std.string as string;
            let size: Int = string.len("abc");
            let ok: Bool = string.contains("abc", "a");
        "#,
    );
}

#[test]
fn test_typeck_std_string_builtin_signatures_reject_wrong_annotation() {
    assert_has_diagnostic(
        r#"
            fn expectInt(x: Int) -> Int = x;
            import std.string as string;
            let wrong = expectInt(string.trim("abc"));
        "#,
        Severity::Error,
        "type mismatch",
    );
}

#[test]
fn test_typeck_std_option_builtins_allow_try_and_coalesce() {
    check_no_errors(
        r#"
            import std.option as option;
            let a = option.some(41)? + 1;
            let b = option.none ?? 5;
            let c = option.unwrap_or(option.some(3), 0);
        "#,
    );
}

#[test]
fn test_typeck_std_option_builtin_signatures_reject_wrong_use() {
    assert_has_diagnostic(
        r#"
            fn expectInt(x: Int) -> Int = x;
            import std.option as option;
            let wrong = expectInt(option.is_some(option.some(1)));
        "#,
        Severity::Error,
        "type mismatch",
    );
}

#[test]
fn test_typeck_std_result_builtins_allow_try_and_unwrap() {
    check_no_errors(
        r#"
            import std.result as result;
            let a = result.ok(41)? + 1;
            let b = result.unwrap(result.ok(7));
            let c = result.unwrap_err(result.err("boom"));
        "#,
    );
}

#[test]
fn test_typeck_std_result_builtin_signatures_reject_wrong_use() {
    assert_has_diagnostic(
        r#"
            fn expectInt(x: Int) -> Int = x;
            import std.result as result;
            let wrong = expectInt(result.is_ok(result.ok(1)));
        "#,
        Severity::Error,
        "type mismatch",
    );
}

#[test]
fn test_typeck_std_math_constants_have_float_type() {
    check_no_errors(
        r#"
            import std.math as math;
            let pi: Float = math.pi;
            let e: Float = math.e;
        "#,
    );
}

#[test]
fn test_typeck_std_math_constants_reject_wrong_annotation() {
    assert_has_diagnostic(
        r#"
            fn expectInt(x: Int) -> Int = x;
            import std.math as math;
            let wrong = expectInt(math.pi);
        "#,
        Severity::Error,
        "type mismatch",
    );
}

#[test]
fn test_typeck_std_path_builtins_allow_valid_uses() {
    check_no_errors(
        r#"
            import std.path as path;
            let joined: String = path.join("a", "b");
            let parent = path.parent("/tmp/file.txt") ?? "/";
            let abs: Bool = path.is_absolute("/tmp/file.txt");
        "#,
    );
}

#[test]
fn test_typeck_std_path_builtins_reject_wrong_use() {
    assert_has_diagnostic(
        r#"
            fn expectInt(x: Int) -> Int = x;
            import std.path as path;
            let wrong = expectInt(path.is_absolute("/tmp"));
        "#,
        Severity::Error,
        "type mismatch",
    );
}

#[test]
fn test_typeck_std_io_builtins_allow_valid_uses() {
    check_no_errors(
        r#"
            import std.io as io;
            let digest: String = io.hashString("abc");
            let cwd: String = io.currentDir();
            let env = io.getEnv("HOME") ?? "";
            let code: Int = io.execShell("printf neve").code;
            let output: String = io.exec("printf", ["neve"]).stdout;
        "#,
    );
}

#[test]
fn test_typeck_std_io_builtins_reject_wrong_use() {
    assert_has_diagnostic(
        r#"
            fn expectInt(x: Int) -> Int = x;
            import std.io as io;
            let wrong = expectInt(io.pathExists("."));
        "#,
        Severity::Error,
        "type mismatch",
    );
}

#[test]
fn test_typeck_std_fetch_builtins_allow_valid_uses() {
    check_no_errors(
        r#"
            import std.fetch as fetch;
            let path: String = fetch.path("Cargo.toml").path;
            let hash: String = fetch.urlWithHash("https://example.com/archive.tar.gz", "0000000000000000000000000000000000000000000000000000000000000000").hash;
            let cached: Bool = fetch.git("https://example.com/repo.git", "main").cached;
        "#,
    );
}

#[test]
fn test_typeck_std_fetch_builtins_reject_wrong_use() {
    assert_has_diagnostic(
        r#"
            fn expectInt(x: Int) -> Int = x;
            import std.fetch as fetch;
            let wrong = expectInt(fetch.path("Cargo.toml").cached);
        "#,
        Severity::Error,
        "type mismatch",
    );
}

#[test]
fn test_typeck_std_map_and_set_builtins_allow_valid_uses() {
    check_no_errors(
        r#"
            import std.Map;
            import std.Set;
            let map = Map.insert("a", 1, Map.empty);
            let value: Int = Map.getWithDefault("a", 0, map);
            let present: Bool = Map.contains("a", map);
            let set = Set.insert(1, Set.empty);
            let count: Int = Set.size(set);
            let hasOne: Bool = Set.contains(1, set);
        "#,
    );
}

#[test]
fn test_typeck_std_map_and_set_builtins_reject_wrong_use() {
    assert_has_diagnostic(
        r#"
            fn expectInt(x: Int) -> Int = x;
            import std.Map;
            import std.Set;
            let map = Map.insert("a", 1, Map.empty);
            let set = Set.insert(1, Set.empty);
            let wrong = expectInt(Map.contains("a", map)) + expectInt(Set.isEmpty(set));
        "#,
        Severity::Error,
        "type mismatch",
    );
}

// ============================================================================
// 逻辑运算
// ============================================================================

#[test]
fn test_typeck_logical_and() {
    check_no_errors("let x = true && false;");
}

#[test]
fn test_typeck_logical_or() {
    check_no_errors("let x = true || false;");
}

#[test]
fn test_typeck_logical_not() {
    check_no_errors("let x = !true;");
}

#[test]
fn test_typeck_complex_logical() {
    check_no_errors("let x = true && false || !true;");
}

#[test]
fn test_typeck_logical_with_comparison() {
    check_no_errors("let x = 1 < 2 && 3 > 2;");
}

#[test]
fn test_typeck_logical_and_wrong_type() {
    check_has_errors("let x = 1 && 2;");
}

#[test]
fn test_typeck_logical_or_wrong_type() {
    check_has_errors("let x = 1 || 2;");
}

#[test]
fn test_typeck_not_wrong_type() {
    check_has_errors("let x = !42;");
}

// ============================================================================
// 条件表达式
// ============================================================================

#[test]
fn test_typeck_if_then_else_int() {
    check_no_errors("let x = if true then 1 else 0;");
}

#[test]
fn test_typeck_if_then_else_string() {
    check_no_errors("let x = if true then \"yes\" else \"no\";");
}

#[test]
fn test_typeck_if_then_else_bool() {
    check_no_errors("let x = if true then true else false;");
}

#[test]
fn test_typeck_if_with_comparison() {
    check_no_errors("let x = if 1 < 2 then 10 else 20;");
}

#[test]
fn test_typeck_if_nested() {
    check_no_errors("let x = if true then if false then 1 else 2 else 3;");
}

#[test]
fn test_typeck_if_deeply_nested() {
    check_no_errors("let x = if true then if true then if false then 1 else 2 else 3 else 4;");
}

#[test]
fn test_typeck_if_branch_type_mismatch() {
    check_has_errors("let x = if true then 1 else false;");
}

#[test]
fn test_typeck_if_branch_type_mismatch_string_int() {
    check_has_errors("let x = if true then \"hello\" else 42;");
}

#[test]
fn test_typeck_if_condition_not_bool() {
    check_has_errors("let x = if 42 then 1 else 0;");
}

#[test]
fn test_typeck_if_condition_not_bool_string() {
    check_has_errors("let x = if \"true\" then 1 else 0;");
}

// ============================================================================
// 元组
// ============================================================================

#[test]
fn test_typeck_tuple_pair() {
    check_no_errors("let x = (1, 2);");
}

#[test]
fn test_typeck_tuple_triple() {
    check_no_errors("let x = (1, 2, 3);");
}

#[test]
fn test_typeck_tuple_mixed_types() {
    check_no_errors("let x = (1, true, \"hello\");");
}

#[test]
fn test_typeck_tuple_nested() {
    check_no_errors("let x = ((1, 2), (3, 4));");
}

#[test]
fn test_typeck_tuple_deeply_nested() {
    check_no_errors("let x = (((1, 2), 3), 4);");
}

#[test]
fn test_typeck_tuple_with_expressions() {
    check_no_errors("let x = (1 + 2, 3 * 4);");
}

// ============================================================================
// 列表
// ============================================================================

#[test]
fn test_typeck_list_empty() {
    check_no_errors("let x = [];");
}

#[test]
fn test_typeck_list_single() {
    check_no_errors("let x = [1];");
}

#[test]
fn test_typeck_list_multiple() {
    check_no_errors("let x = [1, 2, 3];");
}

#[test]
fn test_typeck_list_strings() {
    check_no_errors("let x = [\"a\", \"b\", \"c\"];");
}

#[test]
fn test_typeck_list_bools() {
    check_no_errors("let x = [true, false, true];");
}

#[test]
fn test_typeck_list_nested() {
    check_no_errors("let x = [[1, 2], [3, 4]];");
}

#[test]
fn test_typeck_list_with_expressions() {
    check_no_errors("let x = [1 + 2, 3 * 4, 5 - 1];");
}

#[test]
fn test_typeck_list_heterogeneous() {
    // Lists must be homogeneous
    check_has_errors("let x = [1, true];");
}

#[test]
fn test_typeck_list_mixed_types() {
    check_has_errors("let x = [1, \"hello\"];");
}

// ============================================================================
// 记录
// ============================================================================

#[test]
fn test_typeck_record_single_field() {
    check_no_errors("let x = #{ a = 1 };");
}

#[test]
fn test_typeck_record_multiple_fields() {
    check_no_errors("let x = #{ a = 1, b = 2, c = 3 };");
}

#[test]
fn test_typeck_record_mixed_types() {
    check_no_errors("let x = #{ name = \"alice\", age = 30, active = true };");
}

#[test]
fn test_typeck_record_nested() {
    check_no_errors("let x = #{ inner = #{ a = 1 } };");
}

#[test]
fn test_typeck_record_with_expressions() {
    check_no_errors("let x = #{ sum = 1 + 2, product = 3 * 4 };");
}

#[test]
fn test_typeck_record_field_access_after_record_binding() {
    check_no_errors(
        r#"
            let config = #{ port = 40, host = "localhost" };
            let x = config.port;
        "#,
    );
}

#[test]
fn test_typeck_lazy_force() {
    check_no_errors("let thunk = lazy 42; let x = force(thunk);");
}

#[test]
fn test_typeck_lazy_predicates() {
    check_no_errors("let thunk = lazy 42; let x = isLazy(thunk); let y = isEvaluated(thunk);");
}

#[test]
fn test_typeck_or_pattern_with_shared_binding() {
    check_no_errors("let x = match (1, 2) { (0, v) | (1, v) -> v, _ -> 0 };");
}

#[test]
fn test_typeck_binding_pattern() {
    check_no_errors("let x = match 42 { n @ 42 -> n, _ -> 0 };");
}

#[test]
fn test_typeck_list_rest_pattern() {
    check_no_errors(
        "
        let x = match [1, 2, 3, 4] {
            [first, ..middle, last] -> match middle {
                [a, b] -> first + a + b + last,
                _ -> 0,
            },
            _ -> 0,
        };
        ",
    );
}

#[test]
fn test_typeck_try_on_option_like_enum() {
    check_no_errors(
        "
        enum Option { Some(Int), None };
        let x = Some(41)? + 1;
        ",
    );
}

#[test]
fn test_typeck_try_on_result_like_enum() {
    check_no_errors(
        "
        enum Result { Ok(Int), Err(String) };
        let x = Ok(41)? + 1;
        ",
    );
}

#[test]
fn test_typeck_coalesce_on_option_like_enum() {
    check_no_errors(
        "
        enum Option { Some(Int), None };
        let x = Some(41) ?? 0;
        ",
    );
}

#[test]
fn test_typeck_safe_field_coalesce_defaults_to_string() {
    check_no_errors(
        "
        let r = #{ name = \"test\" };
        let x = r?.missing ?? \"default\";
        ",
    );
}

#[test]
fn test_typeck_method_call_falls_back_to_function_call_semantics() {
    check_no_errors(
        "
        fn twice(x: Int) -> Int = x + x;
        let y = 21.twice();
        ",
    );
}

#[test]
fn test_typeck_method_call_resolves_trait_impl_signature() {
    check_no_errors(
        "
        trait Show { fn show(self) -> String; };
        impl Show for Int {
            fn show(self) -> String = toString(self);
        };
        let x = 1.show();
        ",
    );
}

// ============================================================================
// 函数定义
// ============================================================================

#[test]
fn test_typeck_function_simple() {
    check_no_errors("fn add_one(x) = x + 1;");
}

#[test]
fn test_typeck_function_two_params() {
    check_no_errors("fn add(a, b) = a + b;");
}

#[test]
fn test_typeck_function_three_params() {
    check_no_errors("fn sum3(a, b, c) = a + b + c;");
}

#[test]
fn test_typeck_function_returns_bool() {
    check_no_errors("fn is_positive(x) = x > 0;");
}

#[test]
fn test_typeck_function_with_if() {
    check_no_errors("fn abs(x) = if x < 0 then -x else x;");
}

#[test]
fn test_typeck_function_identity() {
    check_no_errors("fn identity(x) = x;");
}

#[test]
fn test_typeck_function_constant() {
    check_no_errors("fn always_42() = 42;");
}

#[test]
fn test_typeck_multiple_functions() {
    check_no_errors(
        "
        fn double(x) = x * 2;
        fn triple(x) = x * 3;
        fn quadruple(x) = double(double(x));
    ",
    );
}

// ============================================================================
// 函数调用
// ============================================================================

#[test]
fn test_typeck_function_call() {
    check_no_errors(
        "
        fn double(x) = x * 2;
        let y = double(21);
    ",
    );
}

#[test]
fn test_typeck_function_call_nested() {
    check_no_errors(
        "
        fn double(x) = x * 2;
        fn add_one(x) = x + 1;
        let y = add_one(double(5));
    ",
    );
}

#[test]
fn test_typeck_function_call_chain() {
    check_no_errors(
        "
        fn f(x) = x + 1;
        fn g(x) = x * 2;
        fn h(x) = x - 1;
        let y = h(g(f(10)));
    ",
    );
}

// ============================================================================
// 递归函数
// ============================================================================

#[test]
fn test_typeck_recursive_factorial() {
    check_no_errors(
        "
        fn fact(n) = if n <= 1 then 1 else n * fact(n - 1);
    ",
    );
}

#[test]
fn test_typeck_recursive_fibonacci() {
    check_no_errors(
        "
        fn fib(n) = if n <= 1 then n else fib(n - 1) + fib(n - 2);
    ",
    );
}

#[test]
fn test_typeck_recursive_sum() {
    check_no_errors(
        "
        fn sum_to(n) = if n <= 0 then 0 else n + sum_to(n - 1);
    ",
    );
}

#[test]
fn test_typeck_mutually_recursive() {
    check_no_errors(
        "
        fn is_even(n) = if n == 0 then true else is_odd(n - 1);
        fn is_odd(n) = if n == 0 then false else is_even(n - 1);
    ",
    );
}

// ============================================================================
// 管道运算符
// ============================================================================

#[test]
fn test_typeck_pipe_simple() {
    check_no_errors(
        "
        fn double(x) = x * 2;
        let x = 5 |> double;
    ",
    );
}

#[test]
fn test_typeck_pipe_chain() {
    check_no_errors(
        "
        fn double(x) = x * 2;
        fn add_one(x) = x + 1;
        let x = 5 |> double |> add_one;
    ",
    );
}

#[test]
fn test_typeck_pipe_long_chain() {
    check_no_errors(
        "
        fn f(x) = x + 1;
        fn g(x) = x * 2;
        fn h(x) = x - 1;
        let x = 10 |> f |> g |> h |> f |> g;
    ",
    );
}

// ============================================================================
// 模式匹配
// ============================================================================

#[test]
fn test_typeck_match_literal() {
    check_no_errors(
        "
        let x = match 1 {
            0 -> 100,
            1 -> 200,
            _ -> 300
        };
    ",
    );
}

#[test]
fn test_typeck_match_wildcard() {
    check_no_errors(
        "
        let x = match 5 {
            _ -> 42
        };
    ",
    );
}

#[test]
fn test_typeck_match_binding() {
    check_no_errors(
        "
        let x = match 42 {
            n -> n + 1
        };
    ",
    );
}

#[test]
fn test_typeck_match_bool() {
    check_no_errors(
        "
        let x = match true {
            true -> 1,
            false -> 0
        };
    ",
    );
}

#[test]
fn test_typeck_match_tuple() {
    check_no_errors(
        "
        let x = match (1, 2) {
            (a, b) -> a + b
        };
    ",
    );
}

#[test]
fn test_typeck_match_nested_tuple() {
    check_no_errors(
        "
        let x = match ((1, 2), 3) {
            ((a, b), c) -> a + b + c
        };
    ",
    );
}

#[test]
fn test_typeck_match_arm_type_mismatch() {
    check_has_errors(
        "
        let x = match 1 {
            0 -> 100,
            1 -> true,
            _ -> 300
        };
    ",
    );
}

#[test]
fn test_typeck_match_returns_consistent_type() {
    check_no_errors(
        "
        let x = match 1 {
            0 -> false,
            _ -> true
        };
    ",
    );
}

#[test]
fn test_typeck_match_bool_non_exhaustive() {
    assert_has_diagnostic(
        "
        let x = match true {
            true -> 1
        };
        ",
        Severity::Error,
        "non-exhaustive pattern match",
    );
}

#[test]
fn test_typeck_match_enum_non_exhaustive() {
    assert_has_diagnostic(
        "
        enum Option { Some(Int), None };
        let x = match Some(1) {
            Some(value) -> value
        };
        ",
        Severity::Error,
        "non-exhaustive pattern match",
    );
}

#[test]
fn test_typeck_match_builtin_option_non_exhaustive() {
    assert_has_diagnostic(
        "
        import std.option as option;
        let x = match option.some(1) {
            Some(value) -> value
        };
        ",
        Severity::Error,
        "non-exhaustive pattern match",
    );
}

#[test]
fn test_typeck_match_builtin_result_non_exhaustive() {
    assert_has_diagnostic(
        "
        import std.result as result;
        let x = match result.ok(1) {
            Ok(value) -> value
        };
        ",
        Severity::Error,
        "non-exhaustive pattern match",
    );
}

#[test]
fn test_typeck_match_builtin_option_rejects_wrong_constructor() {
    assert_has_diagnostic(
        "
        import std.option as option;
        let x = match option.some(1) {
            Ok(value) -> value,
            None -> 0
        };
        ",
        Severity::Error,
        "constructor does not match expected builtin type",
    );
}

#[test]
fn test_typeck_match_unreachable_after_wildcard() {
    assert_has_diagnostic(
        "
        let x = match true {
            _ -> 1,
            false -> 0
        };
        ",
        Severity::Warning,
        "unreachable pattern",
    );
}

#[test]
fn test_typeck_match_builtin_option_unreachable_after_complete_coverage() {
    assert_has_diagnostic(
        "
        import std.option as option;
        let x = match option.some(1) {
            Some(_) | None -> 1,
            Some(value) -> value
        };
        ",
        Severity::Warning,
        "unreachable pattern",
    );
}

// ============================================================================
// 一元运算符
// ============================================================================

#[test]
fn test_typeck_unary_neg_int() {
    check_no_errors("let x = -42;");
}

#[test]
fn test_typeck_unary_neg_float() {
    check_no_errors("let x = -3.14;");
}

#[test]
fn test_typeck_unary_not_bool() {
    check_no_errors("let x = !true;");
}

#[test]
fn test_typeck_double_neg() {
    check_no_errors("let x = - -42;");
}

#[test]
fn test_typeck_double_not() {
    check_no_errors("let x = !!true;");
}

#[test]
fn test_typeck_unary_neg_expression() {
    check_no_errors("let x = -(1 + 2);");
}

// ============================================================================
// 字符串连接
// ============================================================================

#[test]
fn test_typeck_string_concat() {
    check_no_errors("let x = \"hello\" ++ \" world\";");
}

#[test]
fn test_typeck_string_concat_chain() {
    check_no_errors("let x = \"a\" ++ \"b\" ++ \"c\";");
}

// ============================================================================
// 列表连接
// ============================================================================

#[test]
fn test_typeck_list_concat() {
    check_no_errors("let x = [1, 2] ++ [3, 4];");
}

#[test]
fn test_typeck_list_concat_empty() {
    check_no_errors("let x = [] ++ [1, 2];");
}

#[test]
fn test_typeck_list_concat_chain() {
    check_no_errors("let x = [1] ++ [2] ++ [3];");
}

// ============================================================================
// 记录合并
// ============================================================================

#[test]
fn test_typeck_record_merge() {
    check_no_errors("let x = #{ a = 1 } // #{ b = 2 };");
}

#[test]
fn test_typeck_record_merge_override() {
    check_no_errors("let x = #{ a = 1 } // #{ a = 2 };");
}

#[test]
fn test_typeck_record_merge_chain() {
    check_no_errors("let x = #{ a = 1 } // #{ b = 2 } // #{ c = 3 };");
}

// ============================================================================
// 多重 let 绑定
// ============================================================================

#[test]
fn test_typeck_multiple_lets() {
    check_no_errors("let a = 1; let b = 2; let c = a + b;");
}

#[test]
fn test_typeck_let_shadowing() {
    // 顶层 let 之间不能互相引用，这里测试单独的 let
    check_no_errors("let x = 1 + 1;");
}

#[test]
fn test_typeck_let_uses_previous() {
    // 在函数内部可以使用前面定义的变量
    check_no_errors(
        "
        fn test() = {
            let a = 10;
            let b = a * 2;
            a + b
        };
    ",
    );
}

// ============================================================================
// 复杂表达式
// ============================================================================

#[test]
fn test_typeck_complex_expression_1() {
    check_no_errors("let x = if 1 + 2 > 2 then (3, 4) else (5, 6);");
}

#[test]
fn test_typeck_complex_expression_2() {
    check_no_errors(
        "
        fn f(x) = x * 2;
        let x = if true then f(5) else f(10);
    ",
    );
}

#[test]
fn test_typeck_complex_expression_3() {
    check_no_errors(
        "
        let x = match (1, 2) {
            (0, _) -> 0,
            (_, 0) -> 0,
            (a, b) -> a * b
        };
    ",
    );
}

// ============================================================================
// Lambda 表达式
// ============================================================================

#[test]
fn test_typeck_lambda_simple() {
    check_no_errors("let f = fn(x) x + 1;");
}

#[test]
fn test_typeck_lambda_multiple_params() {
    check_no_errors("let f = fn(x, y) x + y;");
}

#[test]
fn test_typeck_closure_in_function() {
    // 在函数内定义闭包
    check_no_errors(
        "
        fn make_adder(n) = fn(x) x + n;
    ",
    );
}

// ============================================================================
// 类型推导边缘情况
// ============================================================================

#[test]
fn test_typeck_polymorphic_identity() {
    // 单次调用多态函数是可以的
    check_no_errors(
        "
        fn id(x) = x;
        let a = id(42);
    ",
    );
}

#[test]
fn test_typeck_polymorphic_const() {
    // 单次调用多态函数是可以的
    check_no_errors(
        "
        fn const_val(x, y) = x;
        let a = const_val(1, true);
    ",
    );
}

#[test]
fn test_typeck_higher_order_function() {
    check_no_errors(
        "
        fn apply(f, x) = f(x);
        fn double(x) = x * 2;
        let y = apply(double, 21);
    ",
    );
}

// ============================================================================
// 压力测试
// ============================================================================

#[test]
fn test_typeck_many_lets() {
    check_no_errors(
        "
        let a = 1;
        let b = 2;
        let c = 3;
        let d = 4;
        let e = 5;
        let f = a + b + c + d + e;
    ",
    );
}

#[test]
fn test_typeck_many_functions() {
    check_no_errors(
        "
        fn f1(x) = x + 1;
        fn f2(x) = x + 2;
        fn f3(x) = x + 3;
        fn f4(x) = x + 4;
        fn f5(x) = x + 5;
        let y = f1(f2(f3(f4(f5(0)))));
    ",
    );
}

#[test]
fn test_typeck_deeply_nested_if() {
    check_no_errors(
        "
        let x = if true then
            if true then
                if true then
                    if true then
                        1
                    else
                        2
                else
                    3
            else
                4
        else
            5;
    ",
    );
}

#[test]
fn test_typeck_complex_match() {
    check_no_errors(
        "
        let x = match (1, (2, 3)) {
            (0, _) -> 0,
            (_, (0, _)) -> 1,
            (_, (_, 0)) -> 2,
            (a, (b, c)) -> a + b + c
        };
    ",
    );
}

// ============================================================================
// Trait 关联类型
// ============================================================================

#[test]
fn test_typeck_assoc_type_bounds_satisfied() {
    check_no_errors(
        "
        trait Show { };
        trait Iterator { type Item: Show; };
        struct Foo {};
        impl Show for Int { };
        impl Iterator for Foo { type Item = Int; };
    ",
    );
}

#[test]
fn test_typeck_assoc_type_bounds_missing_impl() {
    check_has_errors(
        "
        trait Show { };
        trait Iterator { type Item: Show; };
        struct Foo {};
        impl Iterator for Foo { type Item = Int; };
    ",
    );
}

#[test]
fn test_typeck_impl_method_body_return_type_mismatch() {
    assert_has_diagnostic(
        "
        struct Counter {};
        impl Counter {
            fn value(self) -> Int = true;
        };
        ",
        Severity::Error,
        "impl method `value` return type",
    );
}

#[test]
fn test_typeck_trait_impl_method_body_return_type_mismatch() {
    assert_has_diagnostic(
        "
        trait Show {
            fn show(self) -> Int;
        };
        struct Counter {};
        impl Show for Counter {
            fn show(self) -> Int = true;
        };
        ",
        Severity::Error,
        "impl method `show` return type",
    );
}

#[test]
fn test_typeck_to_string_builtin() {
    check_no_errors("let x = toString(42);");
}

// ============================================================================
// 错误检测测试
// ============================================================================

#[test]
fn test_typeck_detects_type_error_in_if() {
    check_has_errors("let x = if true then 1 else \"string\";");
}

#[test]
fn test_typeck_detects_type_error_in_list() {
    check_has_errors("let x = [1, 2, true];");
}

#[test]
fn test_typeck_detects_non_bool_condition() {
    check_has_errors("let x = if 42 then 1 else 2;");
}

#[test]
fn test_typeck_detects_logical_on_non_bool() {
    check_has_errors("let x = 1 && true;");
}

#[test]
fn test_typeck_detects_not_on_non_bool() {
    check_has_errors("let x = !\"hello\";");
}

#[test]
fn test_typeck_detects_match_arm_mismatch() {
    check_has_errors(
        "
        let x = match 1 {
            0 -> 0,
            _ -> \"not zero\"
        };
    ",
    );
}
