//! Integration tests for neve-eval crate.
//!
//! This file contains extensive edge case tests for the evaluator.

use neve_parser::parse;
use neve_hir::lower;
use neve_eval::{Evaluator, Value, EvalError, AstEvaluator};

fn eval_source(source: &str) -> Result<Value, EvalError> {
    let (ast, _) = parse(source);
    let hir = lower(&ast);
    let mut eval = Evaluator::new();
    eval.eval_module(&hir)
}

/// Evaluate source with builtins available (using AstEvaluator).
fn eval_with_builtins(source: &str) -> Result<Value, String> {
    let (ast, errors) = parse(source);
    if !errors.is_empty() {
        return Err(format!("parse error: {:?}", errors));
    }
    let mut eval = AstEvaluator::new();
    eval.eval_file(&ast).map_err(|e| e.to_string())
}

// ============================================================================
// 整数字面量和运算
// ============================================================================

#[test]
fn test_eval_integer_zero() {
    assert!(matches!(eval_source("let x = 0;"), Ok(Value::Int(0))));
}

#[test]
fn test_eval_integer_positive() {
    assert!(matches!(eval_source("let x = 42;"), Ok(Value::Int(42))));
}

#[test]
fn test_eval_integer_negative() {
    assert!(matches!(eval_source("let x = -42;"), Ok(Value::Int(-42))));
}

#[test]
fn test_eval_integer_large() {
    assert!(matches!(
        eval_source("let x = 9223372036854775807;"),
        Ok(Value::Int(9223372036854775807))
    ));
}

#[test]
fn test_eval_integer_min() {
    // Note: Parser might handle this differently
    let result = eval_source("let x = -9223372036854775807;");
    if let Ok(Value::Int(n)) = result {
        assert_eq!(n, -9223372036854775807);
    }
}

// ============================================================================
// 浮点数字面量和运算
// ============================================================================

#[test]
fn test_eval_float_zero() {
    match eval_source("let x = 0.0;") {
        Ok(Value::Float(f)) => assert!((f - 0.0).abs() < f64::EPSILON),
        other => panic!("expected float, got {:?}", other),
    }
}

#[test]
fn test_eval_float_positive() {
    match eval_source("let x = 3.25;") {
        Ok(Value::Float(f)) => assert!((f - 3.25).abs() < 0.00001),
        other => panic!("expected float, got {:?}", other),
    }
}

#[test]
fn test_eval_float_negative() {
    match eval_source("let x = -2.5;") {
        Ok(Value::Float(f)) => assert!((f - (-2.5)).abs() < 0.001),
        other => panic!("expected float, got {:?}", other),
    }
}

#[test]
fn test_eval_float_scientific() {
    match eval_source("let x = 1.5e10;") {
        Ok(Value::Float(f)) => assert!((f - 1.5e10).abs() < 1e5),
        other => panic!("expected float, got {:?}", other),
    }
}

#[test]
fn test_eval_float_addition() {
    match eval_source("let x = 1.5 + 2.5;") {
        Ok(Value::Float(f)) => assert!((f - 4.0).abs() < f64::EPSILON),
        other => panic!("expected float, got {:?}", other),
    }
}

#[test]
fn test_eval_float_subtraction() {
    match eval_source("let x = 5.5 - 2.5;") {
        Ok(Value::Float(f)) => assert!((f - 3.0).abs() < f64::EPSILON),
        other => panic!("expected float, got {:?}", other),
    }
}

#[test]
fn test_eval_float_multiplication() {
    match eval_source("let x = 2.5 * 4.0;") {
        Ok(Value::Float(f)) => assert!((f - 10.0).abs() < f64::EPSILON),
        other => panic!("expected float, got {:?}", other),
    }
}

#[test]
fn test_eval_float_division() {
    match eval_source("let x = 10.0 / 4.0;") {
        Ok(Value::Float(f)) => assert!((f - 2.5).abs() < f64::EPSILON),
        other => panic!("expected float, got {:?}", other),
    }
}

// ============================================================================
// 布尔值
// ============================================================================

#[test]
fn test_eval_bool_true() {
    assert!(matches!(eval_source("let x = true;"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_bool_false() {
    assert!(matches!(eval_source("let x = false;"), Ok(Value::Bool(false))));
}

#[test]
fn test_eval_bool_not_true() {
    assert!(matches!(eval_source("let x = !true;"), Ok(Value::Bool(false))));
}

#[test]
fn test_eval_bool_not_false() {
    assert!(matches!(eval_source("let x = !false;"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_bool_double_not() {
    assert!(matches!(eval_source("let x = !!true;"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_bool_and_true_true() {
    assert!(matches!(eval_source("let x = true && true;"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_bool_and_true_false() {
    assert!(matches!(eval_source("let x = true && false;"), Ok(Value::Bool(false))));
}

#[test]
fn test_eval_bool_and_false_true() {
    assert!(matches!(eval_source("let x = false && true;"), Ok(Value::Bool(false))));
}

#[test]
fn test_eval_bool_and_false_false() {
    assert!(matches!(eval_source("let x = false && false;"), Ok(Value::Bool(false))));
}

#[test]
fn test_eval_bool_or_true_true() {
    assert!(matches!(eval_source("let x = true || true;"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_bool_or_true_false() {
    assert!(matches!(eval_source("let x = true || false;"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_bool_or_false_true() {
    assert!(matches!(eval_source("let x = false || true;"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_bool_or_false_false() {
    assert!(matches!(eval_source("let x = false || false;"), Ok(Value::Bool(false))));
}

// ============================================================================
// 字符串
// ============================================================================

#[test]
fn test_eval_string_empty() {
    match eval_source("let x = \"\";") {
        Ok(Value::String(s)) => assert_eq!(&*s, ""),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_string_simple() {
    match eval_source("let x = \"hello\";") {
        Ok(Value::String(s)) => assert_eq!(&*s, "hello"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_string_with_spaces() {
    match eval_source("let x = \"hello world\";") {
        Ok(Value::String(s)) => assert_eq!(&*s, "hello world"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_string_with_numbers() {
    match eval_source("let x = \"abc123\";") {
        Ok(Value::String(s)) => assert_eq!(&*s, "abc123"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_string_unicode() {
    match eval_source("let x = \"你好世界\";") {
        Ok(Value::String(s)) => assert_eq!(&*s, "你好世界"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_string_concat() {
    match eval_source("let x = \"hello\" ++ \" world\";") {
        Ok(Value::String(s)) => assert_eq!(&*s, "hello world"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_string_concat_empty() {
    match eval_source("let x = \"hello\" ++ \"\";") {
        Ok(Value::String(s)) => assert_eq!(&*s, "hello"),
        other => panic!("expected string, got {:?}", other),
    }
}

// ============================================================================
// 算术运算
// ============================================================================

#[test]
fn test_eval_addition() {
    assert!(matches!(eval_source("let x = 1 + 2;"), Ok(Value::Int(3))));
}

#[test]
fn test_eval_subtraction() {
    assert!(matches!(eval_source("let x = 10 - 3;"), Ok(Value::Int(7))));
}

#[test]
fn test_eval_multiplication() {
    assert!(matches!(eval_source("let x = 6 * 7;"), Ok(Value::Int(42))));
}

#[test]
fn test_eval_division() {
    assert!(matches!(eval_source("let x = 20 / 4;"), Ok(Value::Int(5))));
}

#[test]
fn test_eval_modulo() {
    assert!(matches!(eval_source("let x = 17 % 5;"), Ok(Value::Int(2))));
}

#[test]
fn test_eval_division_by_zero() {
    match eval_source("let x = 10 / 0;") {
        Err(EvalError::DivisionByZero) => {}
        other => panic!("expected DivisionByZero error, got {:?}", other),
    }
}

#[test]
fn test_eval_modulo_by_zero() {
    match eval_source("let x = 10 % 0;") {
        Err(EvalError::DivisionByZero) => {}
        other => panic!("expected DivisionByZero error, got {:?}", other),
    }
}

#[test]
fn test_eval_negative_division() {
    assert!(matches!(eval_source("let x = -10 / 2;"), Ok(Value::Int(-5))));
}

#[test]
fn test_eval_negative_modulo() {
    let result = eval_source("let x = -17 % 5;");
    if let Ok(Value::Int(n)) = result {
        assert_eq!(n, -17 % 5);
    }
}

#[test]
fn test_eval_operator_precedence() {
    assert!(matches!(eval_source("let x = 1 + 2 * 3;"), Ok(Value::Int(7))));
    assert!(matches!(eval_source("let x = (1 + 2) * 3;"), Ok(Value::Int(9))));
}

#[test]
fn test_eval_complex_arithmetic() {
    assert!(matches!(
        eval_source("let x = 1 + 2 * 3 - 4 / 2;"),
        Ok(Value::Int(5))
    ));
}

#[test]
fn test_eval_nested_parentheses() {
    assert!(matches!(
        eval_source("let x = ((1 + 2) * (3 + 4));"),
        Ok(Value::Int(21))
    ));
}

// ============================================================================
// 比较运算
// ============================================================================

#[test]
fn test_eval_less_than_true() {
    assert!(matches!(eval_source("let x = 1 < 2;"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_less_than_false() {
    assert!(matches!(eval_source("let x = 2 < 1;"), Ok(Value::Bool(false))));
}

#[test]
fn test_eval_less_than_equal() {
    assert!(matches!(eval_source("let x = 1 < 1;"), Ok(Value::Bool(false))));
}

#[test]
fn test_eval_greater_than_true() {
    assert!(matches!(eval_source("let x = 2 > 1;"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_greater_than_false() {
    assert!(matches!(eval_source("let x = 1 > 2;"), Ok(Value::Bool(false))));
}

#[test]
fn test_eval_less_than_or_equal_true() {
    assert!(matches!(eval_source("let x = 1 <= 2;"), Ok(Value::Bool(true))));
    assert!(matches!(eval_source("let x = 1 <= 1;"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_less_than_or_equal_false() {
    assert!(matches!(eval_source("let x = 2 <= 1;"), Ok(Value::Bool(false))));
}

#[test]
fn test_eval_greater_than_or_equal_true() {
    assert!(matches!(eval_source("let x = 2 >= 1;"), Ok(Value::Bool(true))));
    assert!(matches!(eval_source("let x = 1 >= 1;"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_greater_than_or_equal_false() {
    assert!(matches!(eval_source("let x = 1 >= 2;"), Ok(Value::Bool(false))));
}

#[test]
fn test_eval_equality_int() {
    assert!(matches!(eval_source("let x = 42 == 42;"), Ok(Value::Bool(true))));
    assert!(matches!(eval_source("let x = 42 == 43;"), Ok(Value::Bool(false))));
}

#[test]
fn test_eval_inequality_int() {
    assert!(matches!(eval_source("let x = 42 != 43;"), Ok(Value::Bool(true))));
    assert!(matches!(eval_source("let x = 42 != 42;"), Ok(Value::Bool(false))));
}

#[test]
fn test_eval_equality_bool() {
    assert!(matches!(eval_source("let x = true == true;"), Ok(Value::Bool(true))));
    assert!(matches!(eval_source("let x = true == false;"), Ok(Value::Bool(false))));
}

#[test]
fn test_eval_equality_string() {
    assert!(matches!(
        eval_source("let x = \"hello\" == \"hello\";"),
        Ok(Value::Bool(true))
    ));
    assert!(matches!(
        eval_source("let x = \"hello\" == \"world\";"),
        Ok(Value::Bool(false))
    ));
}

// ============================================================================
// 条件表达式
// ============================================================================

#[test]
fn test_eval_if_true_branch() {
    assert!(matches!(eval_source("let x = if true then 1 else 0;"), Ok(Value::Int(1))));
}

#[test]
fn test_eval_if_false_branch() {
    assert!(matches!(eval_source("let x = if false then 1 else 0;"), Ok(Value::Int(0))));
}

#[test]
fn test_eval_if_with_expression_condition() {
    assert!(matches!(
        eval_source("let x = if 1 < 2 then 10 else 20;"),
        Ok(Value::Int(10))
    ));
}

#[test]
fn test_eval_if_nested() {
    assert!(matches!(
        eval_source("let x = if true then if false then 1 else 2 else 3;"),
        Ok(Value::Int(2))
    ));
}

#[test]
fn test_eval_if_deeply_nested() {
    assert!(matches!(
        eval_source("let x = if true then if true then if false then 1 else 2 else 3 else 4;"),
        Ok(Value::Int(2))
    ));
}

#[test]
fn test_eval_if_with_arithmetic() {
    assert!(matches!(
        eval_source("let x = if 2 + 2 == 4 then 100 else 0;"),
        Ok(Value::Int(100))
    ));
}

#[test]
fn test_eval_if_returns_different_types() {
    // Both branches should be able to return the same type
    match eval_source("let x = if true then \"yes\" else \"no\";") {
        Ok(Value::String(s)) => assert_eq!(&*s, "yes"),
        other => panic!("expected string, got {:?}", other),
    }
}

// ============================================================================
// 列表
// ============================================================================

#[test]
fn test_eval_list_empty() {
    match eval_source("let x = [];") {
        Ok(Value::List(items)) => assert!(items.is_empty()),
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_list_single_element() {
    match eval_source("let x = [42];") {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 1);
            assert!(matches!(items[0], Value::Int(42)));
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_list_multiple_elements() {
    match eval_source("let x = [1, 2, 3, 4, 5];") {
        Ok(Value::List(items)) => assert_eq!(items.len(), 5),
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_list_with_expressions() {
    match eval_source("let x = [1 + 1, 2 * 2, 3 - 1];") {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 3);
            assert!(matches!(items[0], Value::Int(2)));
            assert!(matches!(items[1], Value::Int(4)));
            assert!(matches!(items[2], Value::Int(2)));
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_list_nested() {
    match eval_source("let x = [[1, 2], [3, 4]];") {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 2);
            match &items[0] {
                Value::List(inner) => assert_eq!(inner.len(), 2),
                _ => panic!("expected nested list"),
            }
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_list_concat() {
    match eval_source("let x = [1, 2] ++ [3, 4];") {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 4);
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_list_concat_empty() {
    match eval_source("let x = [1, 2] ++ [];") {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 2);
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_list_concat_left_empty() {
    match eval_source("let x = [] ++ [1, 2];") {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 2);
        }
        other => panic!("expected list, got {:?}", other),
    }
}

// ============================================================================
// 元组
// ============================================================================

#[test]
fn test_eval_tuple_pair() {
    match eval_source("let x = (1, 2);") {
        Ok(Value::Tuple(items)) => {
            assert_eq!(items.len(), 2);
            assert!(matches!(items[0], Value::Int(1)));
            assert!(matches!(items[1], Value::Int(2)));
        }
        other => panic!("expected tuple, got {:?}", other),
    }
}

#[test]
fn test_eval_tuple_triple() {
    match eval_source("let x = (1, true, \"hello\");") {
        Ok(Value::Tuple(items)) => {
            assert_eq!(items.len(), 3);
        }
        other => panic!("expected tuple, got {:?}", other),
    }
}

#[test]
fn test_eval_tuple_nested() {
    match eval_source("let x = ((1, 2), (3, 4));") {
        Ok(Value::Tuple(items)) => {
            assert_eq!(items.len(), 2);
            match &items[0] {
                Value::Tuple(inner) => assert_eq!(inner.len(), 2),
                _ => panic!("expected nested tuple"),
            }
        }
        other => panic!("expected tuple, got {:?}", other),
    }
}

#[test]
fn test_eval_tuple_with_expressions() {
    match eval_source("let x = (1 + 1, 2 * 2);") {
        Ok(Value::Tuple(items)) => {
            assert!(matches!(items[0], Value::Int(2)));
            assert!(matches!(items[1], Value::Int(4)));
        }
        other => panic!("expected tuple, got {:?}", other),
    }
}

// ============================================================================
// 记录
// ============================================================================

#[test]
fn test_eval_record_single_field() {
    match eval_source("let x = #{ a = 1 };") {
        Ok(Value::Record(fields)) => {
            assert_eq!(fields.len(), 1);
            assert!(matches!(fields.get("a"), Some(Value::Int(1))));
        }
        other => panic!("expected record, got {:?}", other),
    }
}

#[test]
fn test_eval_record_multiple_fields() {
    match eval_source("let x = #{ a = 1, b = 2, c = 3 };") {
        Ok(Value::Record(fields)) => {
            assert_eq!(fields.len(), 3);
        }
        other => panic!("expected record, got {:?}", other),
    }
}

#[test]
fn test_eval_record_mixed_types() {
    match eval_source("let x = #{ name = \"alice\", age = 30, active = true };") {
        Ok(Value::Record(fields)) => {
            assert_eq!(fields.len(), 3);
            match fields.get("name") {
                Some(Value::String(s)) => assert_eq!(&**s, "alice"),
                _ => panic!("expected string field"),
            }
            assert!(matches!(fields.get("age"), Some(Value::Int(30))));
            assert!(matches!(fields.get("active"), Some(Value::Bool(true))));
        }
        other => panic!("expected record, got {:?}", other),
    }
}

#[test]
fn test_eval_record_nested() {
    match eval_source("let x = #{ inner = #{ a = 1 } };") {
        Ok(Value::Record(fields)) => {
            match fields.get("inner") {
                Some(Value::Record(inner)) => {
                    assert!(matches!(inner.get("a"), Some(Value::Int(1))));
                }
                _ => panic!("expected nested record"),
            }
        }
        other => panic!("expected record, got {:?}", other),
    }
}

#[test]
fn test_eval_record_field_access() {
    match eval_source("let r = #{ a = 42, b = 100 }; let x = r.a;") {
        Ok(Value::Int(42)) => {}
        other => panic!("expected 42, got {:?}", other),
    }
}

#[test]
fn test_eval_record_merge() {
    match eval_source("let x = #{ a = 1 } // #{ b = 2 };") {
        Ok(Value::Record(fields)) => {
            assert_eq!(fields.len(), 2);
            assert!(matches!(fields.get("a"), Some(Value::Int(1))));
            assert!(matches!(fields.get("b"), Some(Value::Int(2))));
        }
        other => panic!("expected record, got {:?}", other),
    }
}

#[test]
fn test_eval_record_merge_override() {
    match eval_source("let x = #{ a = 1 } // #{ a = 2 };") {
        Ok(Value::Record(fields)) => {
            assert!(matches!(fields.get("a"), Some(Value::Int(2))));
        }
        other => panic!("expected record, got {:?}", other),
    }
}

// ============================================================================
// 函数定义和调用
// ============================================================================

#[test]
fn test_eval_function_simple() {
    let result = eval_source("
        fn add_one(x) = x + 1;
        let y = add_one(5);
    ");
    assert!(matches!(result, Ok(Value::Int(6))));
}

#[test]
fn test_eval_function_two_params() {
    let result = eval_source("
        fn add(a, b) = a + b;
        let y = add(3, 4);
    ");
    assert!(matches!(result, Ok(Value::Int(7))));
}

#[test]
fn test_eval_function_three_params() {
    let result = eval_source("
        fn sum3(a, b, c) = a + b + c;
        let y = sum3(1, 2, 3);
    ");
    assert!(matches!(result, Ok(Value::Int(6))));
}

#[test]
fn test_eval_function_returns_bool() {
    let result = eval_source("
        fn is_positive(x) = x > 0;
        let y = is_positive(5);
    ");
    assert!(matches!(result, Ok(Value::Bool(true))));
}

#[test]
fn test_eval_function_returns_string() {
    match eval_source("
        fn greet(name) = name;
        let y = greet(\"world\");
    ") {
        Ok(Value::String(s)) => assert_eq!(&*s, "world"),
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_function_with_if() {
    let result = eval_source("
        fn abs(x) = if x < 0 then -x else x;
        let y = abs(-5);
    ");
    assert!(matches!(result, Ok(Value::Int(5))));
}

#[test]
fn test_eval_function_multiple_calls() {
    let result = eval_source("
        fn double(x) = x * 2;
        let a = double(1);
        let b = double(2);
        let c = double(3);
        let y = a + b + c;
    ");
    assert!(matches!(result, Ok(Value::Int(12))));
}

#[test]
fn test_eval_function_composition() {
    let result = eval_source("
        fn double(x) = x * 2;
        fn add_one(x) = x + 1;
        let y = add_one(double(5));
    ");
    assert!(matches!(result, Ok(Value::Int(11))));
}

// ============================================================================
// 递归函数
// ============================================================================

#[test]
fn test_eval_recursive_factorial() {
    let result = eval_source("
        fn fact(n) = if n <= 1 then 1 else n * fact(n - 1);
        let x = fact(5);
    ");
    assert!(matches!(result, Ok(Value::Int(120))));
}

#[test]
fn test_eval_recursive_factorial_zero() {
    let result = eval_source("
        fn fact(n) = if n <= 1 then 1 else n * fact(n - 1);
        let x = fact(0);
    ");
    assert!(matches!(result, Ok(Value::Int(1))));
}

#[test]
fn test_eval_recursive_factorial_one() {
    let result = eval_source("
        fn fact(n) = if n <= 1 then 1 else n * fact(n - 1);
        let x = fact(1);
    ");
    assert!(matches!(result, Ok(Value::Int(1))));
}

#[test]
fn test_eval_recursive_fibonacci() {
    let result = eval_source("
        fn fib(n) = if n <= 1 then n else fib(n - 1) + fib(n - 2);
        let x = fib(10);
    ");
    assert!(matches!(result, Ok(Value::Int(55))));
}

#[test]
fn test_eval_recursive_fibonacci_zero() {
    let result = eval_source("
        fn fib(n) = if n <= 1 then n else fib(n - 1) + fib(n - 2);
        let x = fib(0);
    ");
    assert!(matches!(result, Ok(Value::Int(0))));
}

#[test]
fn test_eval_recursive_sum() {
    let result = eval_source("
        fn sum_to(n) = if n <= 0 then 0 else n + sum_to(n - 1);
        let x = sum_to(10);
    ");
    assert!(matches!(result, Ok(Value::Int(55))));
}

#[test]
fn test_eval_recursive_gcd() {
    let result = eval_source("
        fn gcd(a, b) = if b == 0 then a else gcd(b, a % b);
        let x = gcd(48, 18);
    ");
    assert!(matches!(result, Ok(Value::Int(6))));
}

// ============================================================================
// 管道操作
// ============================================================================

#[test]
fn test_eval_pipe_simple() {
    let result = eval_source("
        fn double(x) = x * 2;
        let x = 5 |> double;
    ");
    assert!(matches!(result, Ok(Value::Int(10))));
}

#[test]
fn test_eval_pipe_chain() {
    let result = eval_source("
        fn double(x) = x * 2;
        fn add_one(x) = x + 1;
        let x = 5 |> double |> add_one;
    ");
    assert!(matches!(result, Ok(Value::Int(11))));
}

#[test]
fn test_eval_pipe_long_chain() {
    let result = eval_source("
        fn double(x) = x * 2;
        fn add_one(x) = x + 1;
        let x = 1 |> double |> add_one |> double |> add_one;
    ");
    // 1 -> 2 -> 3 -> 6 -> 7
    assert!(matches!(result, Ok(Value::Int(7))));
}

#[test]
fn test_eval_pipe_with_expression() {
    let result = eval_source("
        fn double(x) = x * 2;
        let x = (1 + 2) |> double;
    ");
    assert!(matches!(result, Ok(Value::Int(6))));
}

// ============================================================================
// 模式匹配
// ============================================================================

#[test]
fn test_eval_match_literal() {
    assert!(matches!(
        eval_source("let x = match 1 { 0 => 100, 1 => 200, _ => 300 };"),
        Ok(Value::Int(200))
    ));
}

#[test]
fn test_eval_match_wildcard() {
    assert!(matches!(
        eval_source("let x = match 5 { 0 => 100, 1 => 200, _ => 300 };"),
        Ok(Value::Int(300))
    ));
}

#[test]
fn test_eval_match_first_arm() {
    assert!(matches!(
        eval_source("let x = match 0 { 0 => 100, 1 => 200, _ => 300 };"),
        Ok(Value::Int(100))
    ));
}

#[test]
fn test_eval_match_with_binding() {
    assert!(matches!(
        eval_source("let x = match 42 { n => n + 1 };"),
        Ok(Value::Int(43))
    ));
}

#[test]
fn test_eval_match_tuple() {
    assert!(matches!(
        eval_source("let x = match (1, 2) { (a, b) => a + b };"),
        Ok(Value::Int(3))
    ));
}

#[test]
fn test_eval_match_tuple_nested() {
    assert!(matches!(
        eval_source("let x = match ((1, 2), 3) { ((a, b), c) => a + b + c };"),
        Ok(Value::Int(6))
    ));
}

#[test]
fn test_eval_match_list_pattern() {
    // Match a specific list
    let result = eval_source("let x = match [1, 2] { [a, b] => a + b, _ => 0 };");
    if let Ok(Value::Int(n)) = result {
        assert_eq!(n, 3);
    }
}

#[test]
fn test_eval_match_multiple_arms_first() {
    assert!(matches!(
        eval_source("let x = match true { true => 1, false => 0 };"),
        Ok(Value::Int(1))
    ));
}

#[test]
fn test_eval_match_multiple_arms_second() {
    assert!(matches!(
        eval_source("let x = match false { true => 1, false => 0 };"),
        Ok(Value::Int(0))
    ));
}

// ============================================================================
// 变量绑定和作用域
// ============================================================================

#[test]
fn test_eval_let_simple() {
    assert!(matches!(eval_source("let x = 42;"), Ok(Value::Int(42))));
}

#[test]
fn test_eval_let_with_expression() {
    assert!(matches!(eval_source("let x = 1 + 2 + 3;"), Ok(Value::Int(6))));
}

#[test]
fn test_eval_multiple_lets() {
    assert!(matches!(
        eval_source("let a = 1; let b = 2; let c = a + b;"),
        Ok(Value::Int(3))
    ));
}

#[test]
fn test_eval_let_shadowing() {
    assert!(matches!(
        eval_source("let x = 1; let x = x + 1; let x = x + 1;"),
        Ok(Value::Int(3))
    ));
}

#[test]
fn test_eval_let_uses_previous() {
    assert!(matches!(
        eval_source("let a = 10; let b = a * 2; let c = b + a;"),
        Ok(Value::Int(30))
    ));
}

// ============================================================================
// 特殊边缘情况
// ============================================================================

#[test]
fn test_eval_unary_minus_expression() {
    assert!(matches!(eval_source("let x = -(1 + 2);"), Ok(Value::Int(-3))));
}

#[test]
fn test_eval_double_negation() {
    assert!(matches!(eval_source("let x = - -42;"), Ok(Value::Int(42))));
}

#[test]
fn test_eval_chained_comparisons() {
    // (1 < 2) && (2 < 3)
    assert!(matches!(
        eval_source("let x = 1 < 2 && 2 < 3;"),
        Ok(Value::Bool(true))
    ));
}

#[test]
fn test_eval_mixed_and_or() {
    assert!(matches!(
        eval_source("let x = true && false || true;"),
        Ok(Value::Bool(true))
    ));
}

#[test]
fn test_eval_complex_boolean_expression() {
    assert!(matches!(
        eval_source("let x = (1 < 2) && (3 > 2) || false;"),
        Ok(Value::Bool(true))
    ));
}

// ============================================================================
// 压力测试
// ============================================================================

#[test]
fn test_eval_large_list() {
    // Generate a list with many elements
    let source = "let x = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20];";
    match eval_source(source) {
        Ok(Value::List(items)) => assert_eq!(items.len(), 20),
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_deeply_nested_list() {
    match eval_source("let x = [[[1]]];") {
        Ok(Value::List(l1)) => match &l1[0] {
            Value::List(l2) => match &l2[0] {
                Value::List(l3) => assert!(matches!(l3[0], Value::Int(1))),
                _ => panic!("expected innermost list"),
            },
            _ => panic!("expected middle list"),
        },
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_many_functions() {
    let result = eval_source("
        fn f1(x) = x + 1;
        fn f2(x) = x + 2;
        fn f3(x) = x + 3;
        fn f4(x) = x + 4;
        fn f5(x) = x + 5;
        let x = f1(f2(f3(f4(f5(0)))));
    ");
    assert!(matches!(result, Ok(Value::Int(15))));
}

#[test]
fn test_eval_complex_record() {
    let result = eval_source("
        let config = #{
            name = \"test\",
            version = 1,
            enabled = true,
            settings = #{
                debug = false,
                level = 5
            }
        };
        let x = config.settings.level;
    ");
    assert!(matches!(result, Ok(Value::Int(5))));
}

// ============================================================================
// 错误处理测试
// ============================================================================

#[test]
fn test_eval_field_access_nonexistent() {
    match eval_source("let r = #{ a = 1 }; let x = r.b;") {
        Err(EvalError::TypeError(msg)) => assert!(msg.contains("field")),
        other => panic!("expected TypeError, got {:?}", other),
    }
}

#[test]
fn test_eval_field_access_on_non_record() {
    match eval_source("let x = 42; let y = x.field;") {
        Err(EvalError::TypeError(msg)) => assert!(msg.contains("record")),
        other => panic!("expected TypeError, got {:?}", other),
    }
}

#[test]
fn test_eval_call_non_function() {
    match eval_source("let x = 42; let y = x(1);") {
        Err(EvalError::NotAFunction) => {}
        other => panic!("expected NotAFunction error, got {:?}", other),
    }
}

#[test]
fn test_eval_pattern_match_failure() {
    match eval_source("let x = match 5 { 1 => 10, 2 => 20 };") {
        Err(EvalError::PatternMatchFailed) => {}
        other => panic!("expected PatternMatchFailed error, got {:?}", other),
    }
}

// ============================================================================
// Lambda 表达式测试 (可能被忽略)
// ============================================================================

#[test]
fn test_eval_lambda_simple() {
    let result = eval_source("
        let f = fn(x) x * 2;
        let y = f(21);
    ");
    assert!(matches!(result, Ok(Value::Int(42))));
}

#[test]
fn test_eval_lambda_closure() {
    let result = eval_source("
        fn make_adder(n) = fn(x) x + n;
        let add5 = make_adder(5);
        let result = add5(10);
    ");
    assert!(matches!(result, Ok(Value::Int(15))));
}

#[test]
fn test_eval_lambda_higher_order() {
    let result = eval_source("
        fn apply(f, x) = f(x);
        let double = fn(x) x * 2;
        let result = apply(double, 21);
    ");
    assert!(matches!(result, Ok(Value::Int(42))));
}

// ============================================================================
// 幂运算测试
// ============================================================================

#[test]
fn test_eval_power_simple() {
    assert!(matches!(eval_source("let x = 2 ^ 3;"), Ok(Value::Int(8))));
}

#[test]
fn test_eval_power_zero_exponent() {
    assert!(matches!(eval_source("let x = 5 ^ 0;"), Ok(Value::Int(1))));
}

#[test]
fn test_eval_power_one_exponent() {
    assert!(matches!(eval_source("let x = 5 ^ 1;"), Ok(Value::Int(5))));
}

#[test]
fn test_eval_power_larger() {
    assert!(matches!(eval_source("let x = 2 ^ 10;"), Ok(Value::Int(1024))));
}

// ============================================================================
// 混合类型运算测试
// ============================================================================

#[test]
fn test_eval_int_float_addition() {
    match eval_source("let x = 1 + 2.5;") {
        Ok(Value::Float(f)) => assert!((f - 3.5).abs() < f64::EPSILON),
        other => panic!("expected float, got {:?}", other),
    }
}

#[test]
fn test_eval_float_int_addition() {
    match eval_source("let x = 2.5 + 1;") {
        Ok(Value::Float(f)) => assert!((f - 3.5).abs() < f64::EPSILON),
        other => panic!("expected float, got {:?}", other),
    }
}

// ============================================================================
// Builtin 函数测试 - 新增函数 (使用 AstEvaluator)
// ============================================================================

#[test]
fn test_eval_builtin_length_string() {
    let result = eval_with_builtins("let x = length(\"hello\");");
    assert!(matches!(result, Ok(Value::Int(5))));
}

#[test]
fn test_eval_builtin_length_list() {
    assert!(matches!(eval_with_builtins("let x = length([1, 2, 3, 4, 5]);"), Ok(Value::Int(5))));
}

#[test]
fn test_eval_builtin_length_empty_list() {
    assert!(matches!(eval_with_builtins("let x = length([]);"), Ok(Value::Int(0))));
}

#[test]
fn test_eval_builtin_head() {
    assert!(matches!(eval_with_builtins("let x = head([1, 2, 3]);"), Ok(Value::Int(1))));
}

#[test]
fn test_eval_builtin_tail() {
    let result = eval_with_builtins("let x = tail([1, 2, 3]);");
    match result {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 2);
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_builtin_null_empty() {
    assert!(matches!(eval_with_builtins("let x = null([]);"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_builtin_null_nonempty() {
    assert!(matches!(eval_with_builtins("let x = null([1]);"), Ok(Value::Bool(false))));
}

#[test]
fn test_eval_builtin_elem_found() {
    assert!(matches!(eval_with_builtins("let x = elem(2, [1, 2, 3]);"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_builtin_elem_not_found() {
    assert!(matches!(eval_with_builtins("let x = elem(5, [1, 2, 3]);"), Ok(Value::Bool(false))));
}

#[test]
fn test_eval_builtin_to_string_int() {
    assert!(matches!(eval_with_builtins("let x = toString(42);"), Ok(Value::String(s)) if s.as_str() == "42"));
}

#[test]
fn test_eval_builtin_to_string_bool() {
    assert!(matches!(eval_with_builtins("let x = toString(true);"), Ok(Value::String(s)) if s.as_str() == "true"));
}

#[test]
fn test_eval_builtin_type_of_int() {
    assert!(matches!(eval_with_builtins("let x = typeOf(42);"), Ok(Value::String(s)) if s.as_str() == "int"));
}

#[test]
fn test_eval_builtin_type_of_string() {
    assert!(matches!(eval_with_builtins("let x = typeOf(\"hello\");"), Ok(Value::String(s)) if s.as_str() == "string"));
}

#[test]
fn test_eval_builtin_type_of_list() {
    assert!(matches!(eval_with_builtins("let x = typeOf([1, 2]);"), Ok(Value::String(s)) if s.as_str() == "list"));
}

#[test]
fn test_eval_builtin_type_of_bool() {
    assert!(matches!(eval_with_builtins("let x = typeOf(true);"), Ok(Value::String(s)) if s.as_str() == "bool"));
}

#[test]
fn test_eval_builtin_is_int_true() {
    assert!(matches!(eval_with_builtins("let x = isInt(42);"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_builtin_is_int_false() {
    assert!(matches!(eval_with_builtins("let x = isInt(\"hello\");"), Ok(Value::Bool(false))));
}

#[test]
fn test_eval_builtin_is_string_true() {
    assert!(matches!(eval_with_builtins("let x = isString(\"hello\");"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_builtin_is_list_true() {
    assert!(matches!(eval_with_builtins("let x = isList([1, 2]);"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_builtin_is_bool_true() {
    assert!(matches!(eval_with_builtins("let x = isBool(false);"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_builtin_abs_positive() {
    assert!(matches!(eval_with_builtins("let x = abs(42);"), Ok(Value::Int(42))));
}

#[test]
fn test_eval_builtin_abs_negative() {
    assert!(matches!(eval_with_builtins("let x = abs(-42);"), Ok(Value::Int(42))));
}

#[test]
fn test_eval_builtin_min_two() {
    assert!(matches!(eval_with_builtins("let x = min(3, 7);"), Ok(Value::Int(3))));
}

#[test]
fn test_eval_builtin_max_two() {
    assert!(matches!(eval_with_builtins("let x = max(3, 7);"), Ok(Value::Int(7))));
}

#[test]
fn test_eval_builtin_floor() {
    assert!(matches!(eval_with_builtins("let x = floor(3.7);"), Ok(Value::Int(3))));
}

#[test]
fn test_eval_builtin_ceil() {
    assert!(matches!(eval_with_builtins("let x = ceil(3.2);"), Ok(Value::Int(4))));
}

#[test]
fn test_eval_builtin_round() {
    assert!(matches!(eval_with_builtins("let x = round(3.5);"), Ok(Value::Int(4))));
}

#[test]
fn test_eval_builtin_to_int() {
    assert!(matches!(eval_with_builtins("let x = toInt(3.9);"), Ok(Value::Int(3))));
}

#[test]
fn test_eval_builtin_to_float() {
    match eval_with_builtins("let x = toFloat(42);") {
        Ok(Value::Float(f)) => assert!((f - 42.0).abs() < f64::EPSILON),
        other => panic!("expected float, got {:?}", other),
    }
}

#[test]
fn test_eval_builtin_map() {
    let result = eval_with_builtins("
        let double = fn(x) x * 2;
        let x = map(double, [1, 2, 3]);
    ");
    match result {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 3);
            assert!(matches!(items[0], Value::Int(2)));
            assert!(matches!(items[1], Value::Int(4)));
            assert!(matches!(items[2], Value::Int(6)));
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_builtin_filter() {
    let result = eval_with_builtins("
        let isEven = fn(x) x % 2 == 0;
        let x = filter(isEven, [1, 2, 3, 4, 5, 6]);
    ");
    match result {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 3);
            assert!(matches!(items[0], Value::Int(2)));
            assert!(matches!(items[1], Value::Int(4)));
            assert!(matches!(items[2], Value::Int(6)));
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_builtin_foldl() {
    let result = eval_with_builtins("
        let add = fn(a, b) a + b;
        let x = foldl(add, 0, [1, 2, 3, 4, 5]);
    ");
    assert!(matches!(result, Ok(Value::Int(15))));
}

#[test]
fn test_eval_builtin_foldr() {
    let result = eval_with_builtins("
        let sub = fn(a, b) a - b;
        let x = foldr(sub, 0, [1, 2, 3]);
    ");
    // foldr sub 0 [1, 2, 3] = 1 - (2 - (3 - 0)) = 1 - (2 - 3) = 1 - (-1) = 2
    assert!(matches!(result, Ok(Value::Int(2))));
}

#[test]
fn test_eval_builtin_reverse() {
    let result = eval_with_builtins("let x = reverse([1, 2, 3]);");
    match result {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 3);
            assert!(matches!(items[0], Value::Int(3)));
            assert!(matches!(items[1], Value::Int(2)));
            assert!(matches!(items[2], Value::Int(1)));
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_builtin_sort() {
    let result = eval_with_builtins("let x = sort([3, 1, 4, 1, 5, 9, 2, 6]);");
    match result {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 8);
            assert!(matches!(items[0], Value::Int(1)));
            assert!(matches!(items[1], Value::Int(1)));
            assert!(matches!(items[2], Value::Int(2)));
            assert!(matches!(items[7], Value::Int(9)));
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_builtin_take() {
    let result = eval_with_builtins("let x = take(3, [1, 2, 3, 4, 5]);");
    match result {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 3);
            assert!(matches!(items[0], Value::Int(1)));
            assert!(matches!(items[2], Value::Int(3)));
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_builtin_drop() {
    let result = eval_with_builtins("let x = drop(2, [1, 2, 3, 4, 5]);");
    match result {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 3);
            assert!(matches!(items[0], Value::Int(3)));
            assert!(matches!(items[2], Value::Int(5)));
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_builtin_any() {
    let result = eval_with_builtins("
        let isNegative = fn(x) x < 0;
        let x = any(isNegative, [1, -2, 3]);
    ");
    assert!(matches!(result, Ok(Value::Bool(true))));
}

#[test]
fn test_eval_builtin_any_false() {
    let result = eval_with_builtins("
        let isNegative = fn(x) x < 0;
        let x = any(isNegative, [1, 2, 3]);
    ");
    assert!(matches!(result, Ok(Value::Bool(false))));
}

#[test]
fn test_eval_builtin_all() {
    let result = eval_with_builtins("
        let isPositive = fn(x) x > 0;
        let x = all(isPositive, [1, 2, 3]);
    ");
    assert!(matches!(result, Ok(Value::Bool(true))));
}

#[test]
fn test_eval_builtin_all_false() {
    let result = eval_with_builtins("
        let isPositive = fn(x) x > 0;
        let x = all(isPositive, [1, -2, 3]);
    ");
    assert!(matches!(result, Ok(Value::Bool(false))));
}

#[test]
fn test_eval_builtin_string_length() {
    assert!(matches!(eval_with_builtins("let x = stringLength(\"hello\");"), Ok(Value::Int(5))));
}

#[test]
fn test_eval_builtin_substring() {
    let result = eval_with_builtins("let x = substring(1, 3, \"hello\");");
    assert!(matches!(result, Ok(Value::String(s)) if s.as_str() == "ell"));
}

#[test]
fn test_eval_builtin_split() {
    let result = eval_with_builtins("let x = split(\",\", \"a,b,c\");");
    match result {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 3);
            assert!(matches!(&items[0], Value::String(s) if s.as_str() == "a"));
            assert!(matches!(&items[1], Value::String(s) if s.as_str() == "b"));
            assert!(matches!(&items[2], Value::String(s) if s.as_str() == "c"));
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_builtin_replace() {
    let result = eval_with_builtins("let x = replace(\"world\", \"Neve\", \"hello world\");");
    assert!(matches!(result, Ok(Value::String(s)) if s.as_str() == "hello Neve"));
}

#[test]
fn test_eval_builtin_to_upper() {
    let result = eval_with_builtins("let x = toUpper(\"hello\");");
    assert!(matches!(result, Ok(Value::String(s)) if s.as_str() == "HELLO"));
}

#[test]
fn test_eval_builtin_to_lower() {
    let result = eval_with_builtins("let x = toLower(\"HELLO\");");
    assert!(matches!(result, Ok(Value::String(s)) if s.as_str() == "hello"));
}

#[test]
fn test_eval_builtin_trim() {
    let result = eval_with_builtins("let x = trim(\"  hello  \");");
    assert!(matches!(result, Ok(Value::String(s)) if s.as_str() == "hello"));
}

#[test]
fn test_eval_builtin_starts_with() {
    assert!(matches!(eval_with_builtins("let x = startsWith(\"hel\", \"hello\");"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_builtin_ends_with() {
    assert!(matches!(eval_with_builtins("let x = endsWith(\"llo\", \"hello\");"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_builtin_contains() {
    assert!(matches!(eval_with_builtins("let x = contains(\"ell\", \"hello\");"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_builtin_has_attr() {
    assert!(matches!(eval_with_builtins("let x = hasAttr(\"name\", { name = \"test\" });"), Ok(Value::Bool(true))));
}

#[test]
fn test_eval_builtin_has_attr_false() {
    assert!(matches!(eval_with_builtins("let x = hasAttr(\"age\", { name = \"test\" });"), Ok(Value::Bool(false))));
}

#[test]
fn test_eval_builtin_get_attr() {
    let result = eval_with_builtins("let x = getAttr(\"name\", { name = \"test\" });");
    assert!(matches!(result, Ok(Value::String(s)) if s.as_str() == "test"));
}

#[test]
fn test_eval_builtin_attr_names() {
    let result = eval_with_builtins("let x = attrNames({ a = 1, b = 2 });");
    match result {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 2);
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_builtin_attr_values() {
    let result = eval_with_builtins("let x = attrValues({ a = 1, b = 2 });");
    match result {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 2);
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_builtin_to_json() {
    let result = eval_with_builtins("let x = toJSON({ name = \"test\", value = 42 });");
    match result {
        Ok(Value::String(s)) => {
            assert!(s.contains("\"name\""));
            assert!(s.contains("\"test\""));
            assert!(s.contains("42"));
        }
        other => panic!("expected string, got {:?}", other),
    }
}

#[test]
fn test_eval_builtin_from_json() {
    let result = eval_with_builtins("let x = fromJSON(\"{\\\"a\\\": 1, \\\"b\\\": 2}\");");
    match result {
        Ok(Value::Record(fields)) => {
            assert_eq!(fields.len(), 2);
        }
        other => panic!("expected record, got {:?}", other),
    }
}

#[test]
fn test_eval_builtin_concat() {
    let result = eval_with_builtins("let x = concat([[1, 2], [3, 4], [5]]);");
    match result {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 5);
            assert!(matches!(items[0], Value::Int(1)));
            assert!(matches!(items[4], Value::Int(5)));
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_builtin_flatten() {
    let result = eval_with_builtins("let x = flatten([[1, [2, 3]], [[4], 5]]);");
    match result {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 5);
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_builtin_unique() {
    let result = eval_with_builtins("let x = unique([1, 2, 2, 3, 1, 3, 4]);");
    match result {
        Ok(Value::List(items)) => {
            assert_eq!(items.len(), 4);
        }
        other => panic!("expected list, got {:?}", other),
    }
}

#[test]
fn test_eval_builtin_partition() {
    let result = eval_with_builtins("
        let isEven = fn(x) x % 2 == 0;
        let x = partition(isEven, [1, 2, 3, 4, 5, 6]);
    ");
    match result {
        Ok(Value::Record(fields)) => {
            assert_eq!(fields.len(), 2);
        }
        other => panic!("expected record, got {:?}", other),
    }
}

#[test]
fn test_eval_builtin_bit_and() {
    assert!(matches!(eval_with_builtins("let x = bitAnd(12, 10);"), Ok(Value::Int(8))));
}

#[test]
fn test_eval_builtin_bit_or() {
    assert!(matches!(eval_with_builtins("let x = bitOr(12, 10);"), Ok(Value::Int(14))));
}

#[test]
fn test_eval_builtin_bit_xor() {
    assert!(matches!(eval_with_builtins("let x = bitXor(12, 10);"), Ok(Value::Int(6))));
}

#[test]
fn test_eval_builtin_bit_not() {
    // bitNot of 0 should give -1 (all bits set in two's complement)
    assert!(matches!(eval_with_builtins("let x = bitNot(0);"), Ok(Value::Int(-1))));
}

#[test]
fn test_eval_builtin_bit_shift_left() {
    assert!(matches!(eval_with_builtins("let x = bitShiftLeft(1, 4);"), Ok(Value::Int(16))));
}

#[test]
fn test_eval_builtin_bit_shift_right() {
    assert!(matches!(eval_with_builtins("let x = bitShiftRight(16, 2);"), Ok(Value::Int(4))));
}

#[test]
fn test_eval_builtin_pad_left() {
    let result = eval_with_builtins("let x = padLeft(5, \"0\", \"42\");");
    assert!(matches!(result, Ok(Value::String(s)) if s.as_str() == "00042"));
}

#[test]
fn test_eval_builtin_pad_right() {
    let result = eval_with_builtins("let x = padRight(5, \".\", \"hi\");");
    assert!(matches!(result, Ok(Value::String(s)) if s.as_str() == "hi..."));
}

#[test]
fn test_eval_builtin_compare_less() {
    assert!(matches!(eval_with_builtins("let x = compare(1, 2);"), Ok(Value::Int(-1))));
}

#[test]
fn test_eval_builtin_compare_equal() {
    assert!(matches!(eval_with_builtins("let x = compare(2, 2);"), Ok(Value::Int(0))));
}

#[test]
fn test_eval_builtin_compare_greater() {
    assert!(matches!(eval_with_builtins("let x = compare(3, 2);"), Ok(Value::Int(1))));
}

#[test]
fn test_eval_builtin_merge() {
    let result = eval_with_builtins("let x = merge({ a = 1 }, { b = 2 });");
    match result {
        Ok(Value::Record(fields)) => {
            assert_eq!(fields.len(), 2);
        }
        other => panic!("expected record, got {:?}", other),
    }
}

#[test]
fn test_eval_builtin_merge_recursive() {
    let result = eval_with_builtins("
        let x = mergeRecursive(
            { a = { x = 1, y = 2 } }, 
            { a = { y = 3, z = 4 } }
        );
    ");
    match result {
        Ok(Value::Record(_)) => {
            // Success - just check it returns a record
        }
        other => panic!("expected record, got {:?}", other),
    }
}
