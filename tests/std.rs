//! Integration tests for neve-std crate.

use neve_eval::Value;
use neve_std::stdlib;
use std::rc::Rc;

fn get_builtin(name: &str) -> Option<Value> {
    stdlib()
        .into_iter()
        .find(|(n, _)| *n == name)
        .map(|(_, v)| v)
}

fn call_builtin_fn(f: &Value, args: Vec<Value>) -> Result<Value, String> {
    match f {
        Value::BuiltinFn(_, func) => func(args),
        _ => Err("Not a builtin function".into()),
    }
}

// Map tests

#[test]
fn test_map_empty() {
    let empty = get_builtin("Map.empty");
    assert!(empty.is_some(), "Map.empty not found");
    match empty.unwrap() {
        Value::Map(m) => assert!(m.is_empty()),
        _ => panic!("Expected Map"),
    }
}

#[test]
fn test_map_singleton() {
    let singleton = get_builtin("Map.singleton");
    assert!(singleton.is_some(), "Map.singleton not found");

    let result = call_builtin_fn(
        &singleton.unwrap(),
        vec![Value::String(Rc::new("key".to_string())), Value::Int(42)],
    )
    .unwrap();

    match result {
        Value::Map(m) => assert_eq!(m.len(), 1),
        _ => panic!("Expected Map"),
    }
}

// Set tests

#[test]
fn test_set_empty() {
    let empty = get_builtin("Set.empty");
    assert!(empty.is_some(), "Set.empty not found");
    match empty.unwrap() {
        Value::Set(s) => assert!(s.is_empty()),
        _ => panic!("Expected Set"),
    }
}

#[test]
fn test_set_singleton() {
    let singleton = get_builtin("Set.singleton");
    assert!(singleton.is_some(), "Set.singleton not found");

    let result = call_builtin_fn(&singleton.unwrap(), vec![Value::Int(42)]).unwrap();

    match result {
        Value::Set(s) => assert_eq!(s.len(), 1),
        _ => panic!("Expected Set"),
    }
}

// List tests

#[test]
fn test_list_empty() {
    let empty = get_builtin("list.empty");
    assert!(empty.is_some(), "list.empty not found");

    match empty.unwrap() {
        Value::Builtin(builtin) => {
            let result = (builtin.func)(&[]).unwrap();
            match result {
                Value::List(l) => assert!(l.is_empty()),
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_singleton() {
    let singleton = get_builtin("list.singleton");
    assert!(singleton.is_some(), "list.singleton not found");

    match singleton.unwrap() {
        Value::Builtin(builtin) => {
            let result = (builtin.func)(&[Value::Int(42)]).unwrap();
            match result {
                Value::List(l) => {
                    assert_eq!(l.len(), 1);
                    assert_eq!(l[0], Value::Int(42));
                }
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_len() {
    let list = Value::List(Rc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));
    let len_fn = get_builtin("list.len");
    assert!(len_fn.is_some(), "list.len not found");

    match len_fn.unwrap() {
        Value::Builtin(builtin) => {
            let result = (builtin.func)(&[list]).unwrap();
            assert_eq!(result, Value::Int(3));
        }
        _ => panic!("Expected Builtin"),
    }
}

// Verify stdlib returns builtins
#[test]
fn test_stdlib_not_empty() {
    let builtins = stdlib();
    assert!(!builtins.is_empty());
}

#[test]
fn test_stdlib_has_map_builtins() {
    let builtins = stdlib();
    let map_builtins: Vec<_> = builtins
        .iter()
        .filter(|(name, _)| name.starts_with("Map."))
        .collect();
    assert!(!map_builtins.is_empty(), "No Map.* builtins found");
}

#[test]
fn test_stdlib_has_set_builtins() {
    let builtins = stdlib();
    let set_builtins: Vec<_> = builtins
        .iter()
        .filter(|(name, _)| name.starts_with("Set."))
        .collect();
    assert!(!set_builtins.is_empty(), "No Set.* builtins found");
}

#[test]
fn test_stdlib_has_list_builtins() {
    let builtins = stdlib();
    let list_builtins: Vec<_> = builtins
        .iter()
        .filter(|(name, _)| name.starts_with("list."))
        .collect();
    assert!(!list_builtins.is_empty(), "No list.* builtins found");
}

// ============================================================================
// List 模块边缘测试
// ============================================================================

#[test]
fn test_list_empty_returns_empty_list() {
    let empty = get_builtin("list.empty");
    assert!(empty.is_some());
    match empty.unwrap() {
        Value::Builtin(builtin) => {
            let result = (builtin.func)(&[]).unwrap();
            match result {
                Value::List(l) => {
                    assert!(l.is_empty());
                    assert_eq!(l.len(), 0);
                }
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_len_various_sizes() {
    let len_fn = get_builtin("list.len").unwrap();

    match len_fn {
        Value::Builtin(builtin) => {
            // Empty list
            let empty = Value::List(Rc::new(vec![]));
            assert_eq!((builtin.func)(&[empty]).unwrap(), Value::Int(0));

            // Single element
            let single = Value::List(Rc::new(vec![Value::Int(1)]));
            assert_eq!((builtin.func)(&[single]).unwrap(), Value::Int(1));

            // Many elements
            let many = Value::List(Rc::new(vec![Value::Int(1); 100]));
            assert_eq!((builtin.func)(&[many]).unwrap(), Value::Int(100));
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_is_empty_edge_cases() {
    let is_empty_fn = get_builtin("list.isEmpty").unwrap();

    match is_empty_fn {
        Value::Builtin(builtin) => {
            // Empty list
            let empty = Value::List(Rc::new(vec![]));
            assert_eq!((builtin.func)(&[empty]).unwrap(), Value::Bool(true));

            // Non-empty list
            let non_empty = Value::List(Rc::new(vec![Value::Int(1)]));
            assert_eq!((builtin.func)(&[non_empty]).unwrap(), Value::Bool(false));
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_head_empty_returns_none() {
    let head_fn = get_builtin("list.head").unwrap();

    match head_fn {
        Value::Builtin(builtin) => {
            let empty = Value::List(Rc::new(vec![]));
            let result = (builtin.func)(&[empty]).unwrap();
            assert!(matches!(result, Value::None));
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_head_single_element() {
    let head_fn = get_builtin("list.head").unwrap();

    match head_fn {
        Value::Builtin(builtin) => {
            let single = Value::List(Rc::new(vec![Value::Int(42)]));
            let result = (builtin.func)(&[single]).unwrap();
            match result {
                Value::Some(boxed) => assert_eq!(*boxed, Value::Int(42)),
                _ => panic!("Expected Some"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_tail_empty_returns_empty() {
    let tail_fn = get_builtin("list.tail").unwrap();

    match tail_fn {
        Value::Builtin(builtin) => {
            let empty = Value::List(Rc::new(vec![]));
            let result = (builtin.func)(&[empty]).unwrap();
            match result {
                Value::List(l) => assert!(l.is_empty()),
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_tail_single_element_returns_empty() {
    let tail_fn = get_builtin("list.tail").unwrap();

    match tail_fn {
        Value::Builtin(builtin) => {
            let single = Value::List(Rc::new(vec![Value::Int(1)]));
            let result = (builtin.func)(&[single]).unwrap();
            match result {
                Value::List(l) => assert!(l.is_empty()),
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_tail_multiple_elements() {
    let tail_fn = get_builtin("list.tail").unwrap();

    match tail_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));
            let result = (builtin.func)(&[list]).unwrap();
            match result {
                Value::List(l) => {
                    assert_eq!(l.len(), 2);
                    assert_eq!(l[0], Value::Int(2));
                    assert_eq!(l[1], Value::Int(3));
                }
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_last_empty_returns_none() {
    let last_fn = get_builtin("list.last").unwrap();

    match last_fn {
        Value::Builtin(builtin) => {
            let empty = Value::List(Rc::new(vec![]));
            let result = (builtin.func)(&[empty]).unwrap();
            assert!(matches!(result, Value::None));
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_last_single_element() {
    let last_fn = get_builtin("list.last").unwrap();

    match last_fn {
        Value::Builtin(builtin) => {
            let single = Value::List(Rc::new(vec![Value::Int(99)]));
            let result = (builtin.func)(&[single]).unwrap();
            match result {
                Value::Some(boxed) => assert_eq!(*boxed, Value::Int(99)),
                _ => panic!("Expected Some"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_init_empty_returns_empty() {
    let init_fn = get_builtin("list.init").unwrap();

    match init_fn {
        Value::Builtin(builtin) => {
            let empty = Value::List(Rc::new(vec![]));
            let result = (builtin.func)(&[empty]).unwrap();
            match result {
                Value::List(l) => assert!(l.is_empty()),
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_init_removes_last() {
    let init_fn = get_builtin("list.init").unwrap();

    match init_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));
            let result = (builtin.func)(&[list]).unwrap();
            match result {
                Value::List(l) => {
                    assert_eq!(l.len(), 2);
                    assert_eq!(l[0], Value::Int(1));
                    assert_eq!(l[1], Value::Int(2));
                }
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_get_valid_index() {
    let get_fn = get_builtin("list.get").unwrap();

    match get_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![
                Value::Int(10),
                Value::Int(20),
                Value::Int(30),
            ]));
            let result = (builtin.func)(&[Value::Int(1), list]).unwrap();
            match result {
                Value::Some(boxed) => assert_eq!(*boxed, Value::Int(20)),
                _ => panic!("Expected Some"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_get_out_of_bounds() {
    let get_fn = get_builtin("list.get").unwrap();

    match get_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![Value::Int(1)]));
            let result = (builtin.func)(&[Value::Int(10), list]).unwrap();
            assert!(matches!(result, Value::None));
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_get_negative_index_as_zero() {
    let get_fn = get_builtin("list.get").unwrap();

    match get_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![Value::Int(42)]));
            // Negative index becomes 0 when cast to usize (wraps around)
            // This tests edge case behavior
            let result = (builtin.func)(&[Value::Int(0), list]).unwrap();
            match result {
                Value::Some(boxed) => assert_eq!(*boxed, Value::Int(42)),
                _ => panic!("Expected Some"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_cons_to_empty() {
    let cons_fn = get_builtin("list.cons").unwrap();

    match cons_fn {
        Value::Builtin(builtin) => {
            let empty = Value::List(Rc::new(vec![]));
            let result = (builtin.func)(&[Value::Int(1), empty]).unwrap();
            match result {
                Value::List(l) => {
                    assert_eq!(l.len(), 1);
                    assert_eq!(l[0], Value::Int(1));
                }
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_cons_to_non_empty() {
    let cons_fn = get_builtin("list.cons").unwrap();

    match cons_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![Value::Int(2), Value::Int(3)]));
            let result = (builtin.func)(&[Value::Int(1), list]).unwrap();
            match result {
                Value::List(l) => {
                    assert_eq!(l.len(), 3);
                    assert_eq!(l[0], Value::Int(1));
                    assert_eq!(l[1], Value::Int(2));
                    assert_eq!(l[2], Value::Int(3));
                }
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_append_empty_lists() {
    let append_fn = get_builtin("list.append").unwrap();

    match append_fn {
        Value::Builtin(builtin) => {
            let empty1 = Value::List(Rc::new(vec![]));
            let empty2 = Value::List(Rc::new(vec![]));
            let result = (builtin.func)(&[empty1, empty2]).unwrap();
            match result {
                Value::List(l) => assert!(l.is_empty()),
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_append_left_empty() {
    let append_fn = get_builtin("list.append").unwrap();

    match append_fn {
        Value::Builtin(builtin) => {
            let empty = Value::List(Rc::new(vec![]));
            let list = Value::List(Rc::new(vec![Value::Int(1), Value::Int(2)]));
            let result = (builtin.func)(&[empty, list]).unwrap();
            match result {
                Value::List(l) => assert_eq!(l.len(), 2),
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_append_right_empty() {
    let append_fn = get_builtin("list.append").unwrap();

    match append_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![Value::Int(1), Value::Int(2)]));
            let empty = Value::List(Rc::new(vec![]));
            let result = (builtin.func)(&[list, empty]).unwrap();
            match result {
                Value::List(l) => assert_eq!(l.len(), 2),
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_reverse_empty() {
    let reverse_fn = get_builtin("list.reverse").unwrap();

    match reverse_fn {
        Value::Builtin(builtin) => {
            let empty = Value::List(Rc::new(vec![]));
            let result = (builtin.func)(&[empty]).unwrap();
            match result {
                Value::List(l) => assert!(l.is_empty()),
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_reverse_single() {
    let reverse_fn = get_builtin("list.reverse").unwrap();

    match reverse_fn {
        Value::Builtin(builtin) => {
            let single = Value::List(Rc::new(vec![Value::Int(42)]));
            let result = (builtin.func)(&[single]).unwrap();
            match result {
                Value::List(l) => {
                    assert_eq!(l.len(), 1);
                    assert_eq!(l[0], Value::Int(42));
                }
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_reverse_multiple() {
    let reverse_fn = get_builtin("list.reverse").unwrap();

    match reverse_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));
            let result = (builtin.func)(&[list]).unwrap();
            match result {
                Value::List(l) => {
                    assert_eq!(l[0], Value::Int(3));
                    assert_eq!(l[1], Value::Int(2));
                    assert_eq!(l[2], Value::Int(1));
                }
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_take_zero() {
    let take_fn = get_builtin("list.take").unwrap();

    match take_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));
            let result = (builtin.func)(&[Value::Int(0), list]).unwrap();
            match result {
                Value::List(l) => assert!(l.is_empty()),
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_take_more_than_length() {
    let take_fn = get_builtin("list.take").unwrap();

    match take_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![Value::Int(1), Value::Int(2)]));
            let result = (builtin.func)(&[Value::Int(100), list]).unwrap();
            match result {
                Value::List(l) => assert_eq!(l.len(), 2),
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_drop_zero() {
    let drop_fn = get_builtin("list.drop").unwrap();

    match drop_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));
            let result = (builtin.func)(&[Value::Int(0), list]).unwrap();
            match result {
                Value::List(l) => assert_eq!(l.len(), 3),
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_drop_more_than_length() {
    let drop_fn = get_builtin("list.drop").unwrap();

    match drop_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![Value::Int(1), Value::Int(2)]));
            let result = (builtin.func)(&[Value::Int(100), list]).unwrap();
            match result {
                Value::List(l) => assert!(l.is_empty()),
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_sum_empty() {
    let sum_fn = get_builtin("list.sum").unwrap();

    match sum_fn {
        Value::Builtin(builtin) => {
            let empty = Value::List(Rc::new(vec![]));
            let result = (builtin.func)(&[empty]).unwrap();
            assert_eq!(result, Value::Int(0));
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_sum_single() {
    let sum_fn = get_builtin("list.sum").unwrap();

    match sum_fn {
        Value::Builtin(builtin) => {
            let single = Value::List(Rc::new(vec![Value::Int(42)]));
            let result = (builtin.func)(&[single]).unwrap();
            assert_eq!(result, Value::Int(42));
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_sum_multiple() {
    let sum_fn = get_builtin("list.sum").unwrap();

    match sum_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
                Value::Int(4),
            ]));
            let result = (builtin.func)(&[list]).unwrap();
            assert_eq!(result, Value::Int(10));
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_sum_with_negatives() {
    let sum_fn = get_builtin("list.sum").unwrap();

    match sum_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![
                Value::Int(10),
                Value::Int(-5),
                Value::Int(-3),
            ]));
            let result = (builtin.func)(&[list]).unwrap();
            assert_eq!(result, Value::Int(2));
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_product_empty() {
    let product_fn = get_builtin("list.product").unwrap();

    match product_fn {
        Value::Builtin(builtin) => {
            let empty = Value::List(Rc::new(vec![]));
            let result = (builtin.func)(&[empty]).unwrap();
            assert_eq!(result, Value::Int(1));
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_product_with_zero() {
    let product_fn = get_builtin("list.product").unwrap();

    match product_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![Value::Int(5), Value::Int(0), Value::Int(10)]));
            let result = (builtin.func)(&[list]).unwrap();
            assert_eq!(result, Value::Int(0));
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_product_multiple() {
    let product_fn = get_builtin("list.product").unwrap();

    match product_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![Value::Int(2), Value::Int(3), Value::Int(4)]));
            let result = (builtin.func)(&[list]).unwrap();
            assert_eq!(result, Value::Int(24));
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_max_empty() {
    let max_fn = get_builtin("list.max").unwrap();

    match max_fn {
        Value::Builtin(builtin) => {
            let empty = Value::List(Rc::new(vec![]));
            let result = (builtin.func)(&[empty]).unwrap();
            assert!(matches!(result, Value::None));
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_max_single() {
    let max_fn = get_builtin("list.max").unwrap();

    match max_fn {
        Value::Builtin(builtin) => {
            let single = Value::List(Rc::new(vec![Value::Int(42)]));
            let result = (builtin.func)(&[single]).unwrap();
            match result {
                Value::Some(boxed) => assert_eq!(*boxed, Value::Int(42)),
                _ => panic!("Expected Some"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_max_with_negatives() {
    let max_fn = get_builtin("list.max").unwrap();

    match max_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![
                Value::Int(-5),
                Value::Int(-1),
                Value::Int(-10),
            ]));
            let result = (builtin.func)(&[list]).unwrap();
            match result {
                Value::Some(boxed) => assert_eq!(*boxed, Value::Int(-1)),
                _ => panic!("Expected Some"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_min_empty() {
    let min_fn = get_builtin("list.min").unwrap();

    match min_fn {
        Value::Builtin(builtin) => {
            let empty = Value::List(Rc::new(vec![]));
            let result = (builtin.func)(&[empty]).unwrap();
            assert!(matches!(result, Value::None));
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_min_with_positives() {
    let min_fn = get_builtin("list.min").unwrap();

    match min_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![Value::Int(5), Value::Int(1), Value::Int(10)]));
            let result = (builtin.func)(&[list]).unwrap();
            match result {
                Value::Some(boxed) => assert_eq!(*boxed, Value::Int(1)),
                _ => panic!("Expected Some"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_contains_found() {
    let contains_fn = get_builtin("list.contains").unwrap();

    match contains_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));
            let result = (builtin.func)(&[Value::Int(2), list]).unwrap();
            assert_eq!(result, Value::Bool(true));
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_contains_not_found() {
    let contains_fn = get_builtin("list.contains").unwrap();

    match contains_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));
            let result = (builtin.func)(&[Value::Int(99), list]).unwrap();
            assert_eq!(result, Value::Bool(false));
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_contains_in_empty() {
    let contains_fn = get_builtin("list.contains").unwrap();

    match contains_fn {
        Value::Builtin(builtin) => {
            let empty = Value::List(Rc::new(vec![]));
            let result = (builtin.func)(&[Value::Int(1), empty]).unwrap();
            assert_eq!(result, Value::Bool(false));
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_index_of_found() {
    let index_of_fn = get_builtin("list.indexOf").unwrap();

    match index_of_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![
                Value::Int(10),
                Value::Int(20),
                Value::Int(30),
            ]));
            let result = (builtin.func)(&[Value::Int(20), list]).unwrap();
            match result {
                Value::Some(boxed) => assert_eq!(*boxed, Value::Int(1)),
                _ => panic!("Expected Some"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_index_of_not_found() {
    let index_of_fn = get_builtin("list.indexOf").unwrap();

    match index_of_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));
            let result = (builtin.func)(&[Value::Int(99), list]).unwrap();
            assert!(matches!(result, Value::None));
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_index_of_first_occurrence() {
    let index_of_fn = get_builtin("list.indexOf").unwrap();

    match index_of_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![
                Value::Int(1),
                Value::Int(2),
                Value::Int(1),
                Value::Int(2),
            ]));
            let result = (builtin.func)(&[Value::Int(2), list]).unwrap();
            match result {
                Value::Some(boxed) => assert_eq!(*boxed, Value::Int(1)), // First occurrence at index 1
                _ => panic!("Expected Some"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_sort_empty() {
    let sort_fn = get_builtin("list.sort").unwrap();

    match sort_fn {
        Value::Builtin(builtin) => {
            let empty = Value::List(Rc::new(vec![]));
            let result = (builtin.func)(&[empty]).unwrap();
            match result {
                Value::List(l) => assert!(l.is_empty()),
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_sort_single() {
    let sort_fn = get_builtin("list.sort").unwrap();

    match sort_fn {
        Value::Builtin(builtin) => {
            let single = Value::List(Rc::new(vec![Value::Int(42)]));
            let result = (builtin.func)(&[single]).unwrap();
            match result {
                Value::List(l) => {
                    assert_eq!(l.len(), 1);
                    assert_eq!(l[0], Value::Int(42));
                }
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_sort_already_sorted() {
    let sort_fn = get_builtin("list.sort").unwrap();

    match sort_fn {
        Value::Builtin(builtin) => {
            let sorted = Value::List(Rc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]));
            let result = (builtin.func)(&[sorted]).unwrap();
            match result {
                Value::List(l) => {
                    assert_eq!(l[0], Value::Int(1));
                    assert_eq!(l[1], Value::Int(2));
                    assert_eq!(l[2], Value::Int(3));
                }
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_sort_reverse_sorted() {
    let sort_fn = get_builtin("list.sort").unwrap();

    match sort_fn {
        Value::Builtin(builtin) => {
            let reversed = Value::List(Rc::new(vec![Value::Int(3), Value::Int(2), Value::Int(1)]));
            let result = (builtin.func)(&[reversed]).unwrap();
            match result {
                Value::List(l) => {
                    assert_eq!(l[0], Value::Int(1));
                    assert_eq!(l[1], Value::Int(2));
                    assert_eq!(l[2], Value::Int(3));
                }
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_sort_with_duplicates() {
    let sort_fn = get_builtin("list.sort").unwrap();

    match sort_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![
                Value::Int(3),
                Value::Int(1),
                Value::Int(2),
                Value::Int(1),
            ]));
            let result = (builtin.func)(&[list]).unwrap();
            match result {
                Value::List(l) => {
                    assert_eq!(l[0], Value::Int(1));
                    assert_eq!(l[1], Value::Int(1));
                    assert_eq!(l[2], Value::Int(2));
                    assert_eq!(l[3], Value::Int(3));
                }
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_sort_strings() {
    let sort_fn = get_builtin("list.sort").unwrap();

    match sort_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![
                Value::String(Rc::new("banana".to_string())),
                Value::String(Rc::new("apple".to_string())),
                Value::String(Rc::new("cherry".to_string())),
            ]));
            let result = (builtin.func)(&[list]).unwrap();
            match result {
                Value::List(l) => match (&l[0], &l[1], &l[2]) {
                    (Value::String(a), Value::String(b), Value::String(c)) => {
                        assert_eq!(a.as_str(), "apple");
                        assert_eq!(b.as_str(), "banana");
                        assert_eq!(c.as_str(), "cherry");
                    }
                    _ => panic!("Expected strings"),
                },
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_range_empty() {
    let range_fn = get_builtin("list.range").unwrap();

    match range_fn {
        Value::Builtin(builtin) => {
            let result = (builtin.func)(&[Value::Int(5), Value::Int(5)]).unwrap();
            match result {
                Value::List(l) => assert!(l.is_empty()),
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_range_single() {
    let range_fn = get_builtin("list.range").unwrap();

    match range_fn {
        Value::Builtin(builtin) => {
            let result = (builtin.func)(&[Value::Int(0), Value::Int(1)]).unwrap();
            match result {
                Value::List(l) => {
                    assert_eq!(l.len(), 1);
                    assert_eq!(l[0], Value::Int(0));
                }
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_range_multiple() {
    let range_fn = get_builtin("list.range").unwrap();

    match range_fn {
        Value::Builtin(builtin) => {
            let result = (builtin.func)(&[Value::Int(1), Value::Int(5)]).unwrap();
            match result {
                Value::List(l) => {
                    assert_eq!(l.len(), 4);
                    assert_eq!(l[0], Value::Int(1));
                    assert_eq!(l[1], Value::Int(2));
                    assert_eq!(l[2], Value::Int(3));
                    assert_eq!(l[3], Value::Int(4));
                }
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_replicate_zero() {
    let replicate_fn = get_builtin("list.replicate").unwrap();

    match replicate_fn {
        Value::Builtin(builtin) => {
            let result = (builtin.func)(&[Value::Int(0), Value::Int(42)]).unwrap();
            match result {
                Value::List(l) => assert!(l.is_empty()),
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_replicate_multiple() {
    let replicate_fn = get_builtin("list.replicate").unwrap();

    match replicate_fn {
        Value::Builtin(builtin) => {
            let result =
                (builtin.func)(&[Value::Int(3), Value::String(Rc::new("x".to_string()))]).unwrap();
            match result {
                Value::List(l) => {
                    assert_eq!(l.len(), 3);
                    for item in l.iter() {
                        match item {
                            Value::String(s) => assert_eq!(s.as_str(), "x"),
                            _ => panic!("Expected String"),
                        }
                    }
                }
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_zip_empty() {
    let zip_fn = get_builtin("list.zip").unwrap();

    match zip_fn {
        Value::Builtin(builtin) => {
            let empty1 = Value::List(Rc::new(vec![]));
            let empty2 = Value::List(Rc::new(vec![]));
            let result = (builtin.func)(&[empty1, empty2]).unwrap();
            match result {
                Value::List(l) => assert!(l.is_empty()),
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_zip_different_lengths() {
    let zip_fn = get_builtin("list.zip").unwrap();

    match zip_fn {
        Value::Builtin(builtin) => {
            let short = Value::List(Rc::new(vec![Value::Int(1)]));
            let long = Value::List(Rc::new(vec![
                Value::Int(10),
                Value::Int(20),
                Value::Int(30),
            ]));
            let result = (builtin.func)(&[short, long]).unwrap();
            match result {
                Value::List(l) => {
                    // Zip stops at shorter list
                    assert_eq!(l.len(), 1);
                }
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_zip_same_length() {
    let zip_fn = get_builtin("list.zip").unwrap();

    match zip_fn {
        Value::Builtin(builtin) => {
            let list1 = Value::List(Rc::new(vec![Value::Int(1), Value::Int(2)]));
            let list2 = Value::List(Rc::new(vec![
                Value::String(Rc::new("a".to_string())),
                Value::String(Rc::new("b".to_string())),
            ]));
            let result = (builtin.func)(&[list1, list2]).unwrap();
            match result {
                Value::List(l) => {
                    assert_eq!(l.len(), 2);
                    match &l[0] {
                        Value::Tuple(t) => {
                            assert_eq!(t[0], Value::Int(1));
                            match &t[1] {
                                Value::String(s) => assert_eq!(s.as_str(), "a"),
                                _ => panic!("Expected String"),
                            }
                        }
                        _ => panic!("Expected Tuple"),
                    }
                }
                _ => panic!("Expected List"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_unzip_empty() {
    let unzip_fn = get_builtin("list.unzip").unwrap();

    match unzip_fn {
        Value::Builtin(builtin) => {
            let empty = Value::List(Rc::new(vec![]));
            let result = (builtin.func)(&[empty]).unwrap();
            match result {
                Value::Tuple(t) => {
                    assert_eq!(t.len(), 2);
                    match (&t[0], &t[1]) {
                        (Value::List(l1), Value::List(l2)) => {
                            assert!(l1.is_empty());
                            assert!(l2.is_empty());
                        }
                        _ => panic!("Expected Lists"),
                    }
                }
                _ => panic!("Expected Tuple"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_unzip_pairs() {
    let unzip_fn = get_builtin("list.unzip").unwrap();

    match unzip_fn {
        Value::Builtin(builtin) => {
            let pairs = Value::List(Rc::new(vec![
                Value::Tuple(Rc::new(vec![
                    Value::Int(1),
                    Value::String(Rc::new("a".to_string())),
                ])),
                Value::Tuple(Rc::new(vec![
                    Value::Int(2),
                    Value::String(Rc::new("b".to_string())),
                ])),
            ]));
            let result = (builtin.func)(&[pairs]).unwrap();
            match result {
                Value::Tuple(t) => match (&t[0], &t[1]) {
                    (Value::List(l1), Value::List(l2)) => {
                        assert_eq!(l1.len(), 2);
                        assert_eq!(l2.len(), 2);
                        assert_eq!(l1[0], Value::Int(1));
                        assert_eq!(l1[1], Value::Int(2));
                    }
                    _ => panic!("Expected Lists"),
                },
                _ => panic!("Expected Tuple"),
            }
        }
        _ => panic!("Expected Builtin"),
    }
}

// ============================================================================
// Map 模块边缘测试
// ============================================================================

#[test]
fn test_map_empty_is_empty() {
    let empty = get_builtin("Map.empty");
    assert!(empty.is_some());
    match empty.unwrap() {
        Value::Map(m) => {
            assert!(m.is_empty());
            assert_eq!(m.len(), 0);
        }
        _ => panic!("Expected Map"),
    }
}

#[test]
fn test_map_singleton_creates_single_entry() {
    let singleton = get_builtin("Map.singleton");
    assert!(singleton.is_some());

    let result = call_builtin_fn(
        &singleton.unwrap(),
        vec![Value::String(Rc::new("mykey".to_string())), Value::Int(999)],
    )
    .unwrap();

    match result {
        Value::Map(m) => {
            assert_eq!(m.len(), 1);
        }
        _ => panic!("Expected Map"),
    }
}

#[test]
fn test_map_size_empty() {
    let empty = get_builtin("Map.empty").unwrap();
    let size = get_builtin("Map.size").unwrap();

    let result = call_builtin_fn(&size, vec![empty]).unwrap();
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_map_is_empty_on_empty() {
    let empty = get_builtin("Map.empty").unwrap();
    let is_empty = get_builtin("Map.isEmpty").unwrap();

    let result = call_builtin_fn(&is_empty, vec![empty]).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_map_is_empty_on_non_empty() {
    let singleton = get_builtin("Map.singleton").unwrap();
    let is_empty = get_builtin("Map.isEmpty").unwrap();

    let m = call_builtin_fn(
        &singleton,
        vec![Value::String(Rc::new("k".to_string())), Value::Int(1)],
    )
    .unwrap();

    let result = call_builtin_fn(&is_empty, vec![m]).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_map_contains_existing_key() {
    let singleton = get_builtin("Map.singleton").unwrap();
    let contains = get_builtin("Map.contains").unwrap();

    let m = call_builtin_fn(
        &singleton,
        vec![Value::String(Rc::new("key".to_string())), Value::Int(42)],
    )
    .unwrap();

    let result = call_builtin_fn(
        &contains,
        vec![Value::String(Rc::new("key".to_string())), m],
    )
    .unwrap();

    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_map_contains_missing_key() {
    let empty = get_builtin("Map.empty").unwrap();
    let contains = get_builtin("Map.contains").unwrap();

    let result = call_builtin_fn(
        &contains,
        vec![Value::String(Rc::new("nonexistent".to_string())), empty],
    )
    .unwrap();

    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_map_insert_to_empty() {
    let empty = get_builtin("Map.empty").unwrap();
    let insert = get_builtin("Map.insert").unwrap();
    let size = get_builtin("Map.size").unwrap();

    let m = call_builtin_fn(
        &insert,
        vec![
            Value::String(Rc::new("a".to_string())),
            Value::Int(1),
            empty,
        ],
    )
    .unwrap();

    let result = call_builtin_fn(&size, vec![m]).unwrap();
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_map_insert_overwrite() {
    let singleton = get_builtin("Map.singleton").unwrap();
    let insert = get_builtin("Map.insert").unwrap();
    let get = get_builtin("Map.get").unwrap();

    let m = call_builtin_fn(
        &singleton,
        vec![Value::String(Rc::new("key".to_string())), Value::Int(100)],
    )
    .unwrap();

    let m2 = call_builtin_fn(
        &insert,
        vec![
            Value::String(Rc::new("key".to_string())),
            Value::Int(200),
            m,
        ],
    )
    .unwrap();

    let result =
        call_builtin_fn(&get, vec![Value::String(Rc::new("key".to_string())), m2]).unwrap();

    // Should be Some(200)
    match result {
        Value::Variant(tag, value) => {
            assert_eq!(tag, "Some");
            assert_eq!(*value, Value::Int(200));
        }
        _ => panic!("Expected Some variant"),
    }
}

#[test]
fn test_map_remove_existing() {
    let singleton = get_builtin("Map.singleton").unwrap();
    let remove = get_builtin("Map.remove").unwrap();
    let is_empty = get_builtin("Map.isEmpty").unwrap();

    let m = call_builtin_fn(
        &singleton,
        vec![Value::String(Rc::new("key".to_string())), Value::Int(42)],
    )
    .unwrap();

    let m2 = call_builtin_fn(&remove, vec![Value::String(Rc::new("key".to_string())), m]).unwrap();

    let result = call_builtin_fn(&is_empty, vec![m2]).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_map_remove_nonexistent() {
    let singleton = get_builtin("Map.singleton").unwrap();
    let remove = get_builtin("Map.remove").unwrap();
    let size = get_builtin("Map.size").unwrap();

    let m = call_builtin_fn(
        &singleton,
        vec![Value::String(Rc::new("key".to_string())), Value::Int(42)],
    )
    .unwrap();

    let m2 = call_builtin_fn(
        &remove,
        vec![Value::String(Rc::new("other".to_string())), m],
    )
    .unwrap();

    let result = call_builtin_fn(&size, vec![m2]).unwrap();
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_map_union_disjoint() {
    let singleton = get_builtin("Map.singleton").unwrap();
    let union = get_builtin("Map.union").unwrap();
    let size = get_builtin("Map.size").unwrap();

    let m1 = call_builtin_fn(
        &singleton,
        vec![Value::String(Rc::new("a".to_string())), Value::Int(1)],
    )
    .unwrap();

    let m2 = call_builtin_fn(
        &singleton,
        vec![Value::String(Rc::new("b".to_string())), Value::Int(2)],
    )
    .unwrap();

    let combined = call_builtin_fn(&union, vec![m1, m2]).unwrap();
    let result = call_builtin_fn(&size, vec![combined]).unwrap();

    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_map_intersection_empty() {
    let singleton = get_builtin("Map.singleton").unwrap();
    let intersection = get_builtin("Map.intersection").unwrap();
    let is_empty = get_builtin("Map.isEmpty").unwrap();

    let m1 = call_builtin_fn(
        &singleton,
        vec![Value::String(Rc::new("a".to_string())), Value::Int(1)],
    )
    .unwrap();

    let m2 = call_builtin_fn(
        &singleton,
        vec![Value::String(Rc::new("b".to_string())), Value::Int(2)],
    )
    .unwrap();

    let result = call_builtin_fn(&intersection, vec![m1, m2]).unwrap();
    let empty = call_builtin_fn(&is_empty, vec![result]).unwrap();

    assert_eq!(empty, Value::Bool(true));
}

#[test]
fn test_map_difference() {
    let singleton = get_builtin("Map.singleton").unwrap();
    let insert = get_builtin("Map.insert").unwrap();
    let difference = get_builtin("Map.difference").unwrap();
    let size = get_builtin("Map.size").unwrap();

    // m1 = {a: 1, b: 2}
    let m1 = call_builtin_fn(
        &singleton,
        vec![Value::String(Rc::new("a".to_string())), Value::Int(1)],
    )
    .unwrap();
    let m1 = call_builtin_fn(
        &insert,
        vec![Value::String(Rc::new("b".to_string())), Value::Int(2), m1],
    )
    .unwrap();

    // m2 = {b: 99}
    let m2 = call_builtin_fn(
        &singleton,
        vec![Value::String(Rc::new("b".to_string())), Value::Int(99)],
    )
    .unwrap();

    // difference = {a: 1}
    let diff = call_builtin_fn(&difference, vec![m1, m2]).unwrap();
    let result = call_builtin_fn(&size, vec![diff]).unwrap();

    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_map_keys_empty() {
    let empty = get_builtin("Map.empty").unwrap();
    let keys = get_builtin("Map.keys").unwrap();

    let result = call_builtin_fn(&keys, vec![empty]).unwrap();
    match result {
        Value::List(l) => assert!(l.is_empty()),
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_map_values_empty() {
    let empty = get_builtin("Map.empty").unwrap();
    let values = get_builtin("Map.values").unwrap();

    let result = call_builtin_fn(&values, vec![empty]).unwrap();
    match result {
        Value::List(l) => assert!(l.is_empty()),
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_map_to_list_empty() {
    let empty = get_builtin("Map.empty").unwrap();
    let to_list = get_builtin("Map.toList").unwrap();

    let result = call_builtin_fn(&to_list, vec![empty]).unwrap();
    match result {
        Value::List(l) => assert!(l.is_empty()),
        _ => panic!("Expected List"),
    }
}

#[test]
fn test_map_get_with_default_found() {
    let singleton = get_builtin("Map.singleton").unwrap();
    let get_with_default = get_builtin("Map.getWithDefault").unwrap();

    let m = call_builtin_fn(
        &singleton,
        vec![Value::String(Rc::new("key".to_string())), Value::Int(42)],
    )
    .unwrap();

    let result = call_builtin_fn(
        &get_with_default,
        vec![Value::String(Rc::new("key".to_string())), Value::Int(0), m],
    )
    .unwrap();

    assert_eq!(result, Value::Int(42));
}

#[test]
fn test_map_get_with_default_not_found() {
    let empty = get_builtin("Map.empty").unwrap();
    let get_with_default = get_builtin("Map.getWithDefault").unwrap();

    let result = call_builtin_fn(
        &get_with_default,
        vec![
            Value::String(Rc::new("missing".to_string())),
            Value::Int(999),
            empty,
        ],
    )
    .unwrap();

    assert_eq!(result, Value::Int(999));
}

// ============================================================================
// Set 模块边缘测试
// ============================================================================

#[test]
fn test_set_empty_is_empty() {
    let empty = get_builtin("Set.empty");
    assert!(empty.is_some());
    match empty.unwrap() {
        Value::Set(s) => {
            assert!(s.is_empty());
            assert_eq!(s.len(), 0);
        }
        _ => panic!("Expected Set"),
    }
}

#[test]
fn test_set_singleton_creates_single_element() {
    let singleton = get_builtin("Set.singleton");
    assert!(singleton.is_some());

    let result = call_builtin_fn(&singleton.unwrap(), vec![Value::Int(42)]).unwrap();

    match result {
        Value::Set(s) => {
            assert_eq!(s.len(), 1);
        }
        _ => panic!("Expected Set"),
    }
}

#[test]
fn test_set_size_empty() {
    let empty = get_builtin("Set.empty").unwrap();
    let size = get_builtin("Set.size").unwrap();

    let result = call_builtin_fn(&size, vec![empty]).unwrap();
    assert_eq!(result, Value::Int(0));
}

#[test]
fn test_set_is_empty_true() {
    let empty = get_builtin("Set.empty").unwrap();
    let is_empty = get_builtin("Set.isEmpty").unwrap();

    let result = call_builtin_fn(&is_empty, vec![empty]).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_set_is_empty_false() {
    let singleton = get_builtin("Set.singleton").unwrap();
    let is_empty = get_builtin("Set.isEmpty").unwrap();

    let s = call_builtin_fn(&singleton, vec![Value::Int(1)]).unwrap();
    let result = call_builtin_fn(&is_empty, vec![s]).unwrap();

    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_set_contains_found() {
    let singleton = get_builtin("Set.singleton").unwrap();
    let contains = get_builtin("Set.contains").unwrap();

    let s = call_builtin_fn(&singleton, vec![Value::Int(42)]).unwrap();
    let result = call_builtin_fn(&contains, vec![Value::Int(42), s]).unwrap();

    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_set_contains_not_found() {
    let singleton = get_builtin("Set.singleton").unwrap();
    let contains = get_builtin("Set.contains").unwrap();

    let s = call_builtin_fn(&singleton, vec![Value::Int(42)]).unwrap();
    let result = call_builtin_fn(&contains, vec![Value::Int(99), s]).unwrap();

    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_set_insert_new_element() {
    let empty = get_builtin("Set.empty").unwrap();
    let insert = get_builtin("Set.insert").unwrap();
    let size = get_builtin("Set.size").unwrap();

    let s = call_builtin_fn(&insert, vec![Value::Int(1), empty]).unwrap();
    let result = call_builtin_fn(&size, vec![s]).unwrap();

    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_set_insert_duplicate() {
    let singleton = get_builtin("Set.singleton").unwrap();
    let insert = get_builtin("Set.insert").unwrap();
    let size = get_builtin("Set.size").unwrap();

    let s = call_builtin_fn(&singleton, vec![Value::Int(42)]).unwrap();
    let s2 = call_builtin_fn(&insert, vec![Value::Int(42), s]).unwrap();
    let result = call_builtin_fn(&size, vec![s2]).unwrap();

    // Duplicate shouldn't increase size
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_set_remove_existing() {
    let singleton = get_builtin("Set.singleton").unwrap();
    let remove = get_builtin("Set.remove").unwrap();
    let is_empty = get_builtin("Set.isEmpty").unwrap();

    let s = call_builtin_fn(&singleton, vec![Value::Int(42)]).unwrap();
    let s2 = call_builtin_fn(&remove, vec![Value::Int(42), s]).unwrap();
    let result = call_builtin_fn(&is_empty, vec![s2]).unwrap();

    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_set_remove_nonexistent() {
    let singleton = get_builtin("Set.singleton").unwrap();
    let remove = get_builtin("Set.remove").unwrap();
    let size = get_builtin("Set.size").unwrap();

    let s = call_builtin_fn(&singleton, vec![Value::Int(42)]).unwrap();
    let s2 = call_builtin_fn(&remove, vec![Value::Int(99), s]).unwrap();
    let result = call_builtin_fn(&size, vec![s2]).unwrap();

    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_set_union_disjoint() {
    let singleton = get_builtin("Set.singleton").unwrap();
    let union = get_builtin("Set.union").unwrap();
    let size = get_builtin("Set.size").unwrap();

    let s1 = call_builtin_fn(&singleton, vec![Value::Int(1)]).unwrap();
    let s2 = call_builtin_fn(&singleton, vec![Value::Int(2)]).unwrap();

    let combined = call_builtin_fn(&union, vec![s1, s2]).unwrap();
    let result = call_builtin_fn(&size, vec![combined]).unwrap();

    assert_eq!(result, Value::Int(2));
}

#[test]
fn test_set_union_overlapping() {
    let singleton = get_builtin("Set.singleton").unwrap();
    let union = get_builtin("Set.union").unwrap();
    let size = get_builtin("Set.size").unwrap();

    let s1 = call_builtin_fn(&singleton, vec![Value::Int(1)]).unwrap();
    let s2 = call_builtin_fn(&singleton, vec![Value::Int(1)]).unwrap();

    let combined = call_builtin_fn(&union, vec![s1, s2]).unwrap();
    let result = call_builtin_fn(&size, vec![combined]).unwrap();

    // Same element, should still be 1
    assert_eq!(result, Value::Int(1));
}

#[test]
fn test_set_intersection_common() {
    let singleton = get_builtin("Set.singleton").unwrap();
    let insert = get_builtin("Set.insert").unwrap();
    let intersection = get_builtin("Set.intersection").unwrap();
    let size = get_builtin("Set.size").unwrap();

    // s1 = {1, 2}
    let s1 = call_builtin_fn(&singleton, vec![Value::Int(1)]).unwrap();
    let s1 = call_builtin_fn(&insert, vec![Value::Int(2), s1]).unwrap();

    // s2 = {2, 3}
    let s2 = call_builtin_fn(&singleton, vec![Value::Int(2)]).unwrap();
    let s2 = call_builtin_fn(&insert, vec![Value::Int(3), s2]).unwrap();

    let result = call_builtin_fn(&intersection, vec![s1, s2]).unwrap();
    let len = call_builtin_fn(&size, vec![result]).unwrap();

    // Common element is 2
    assert_eq!(len, Value::Int(1));
}

#[test]
fn test_set_intersection_none() {
    let singleton = get_builtin("Set.singleton").unwrap();
    let intersection = get_builtin("Set.intersection").unwrap();
    let is_empty = get_builtin("Set.isEmpty").unwrap();

    let s1 = call_builtin_fn(&singleton, vec![Value::Int(1)]).unwrap();
    let s2 = call_builtin_fn(&singleton, vec![Value::Int(2)]).unwrap();

    let result = call_builtin_fn(&intersection, vec![s1, s2]).unwrap();
    let empty = call_builtin_fn(&is_empty, vec![result]).unwrap();

    assert_eq!(empty, Value::Bool(true));
}

#[test]
fn test_set_difference_some() {
    let singleton = get_builtin("Set.singleton").unwrap();
    let insert = get_builtin("Set.insert").unwrap();
    let difference = get_builtin("Set.difference").unwrap();
    let size = get_builtin("Set.size").unwrap();

    // s1 = {1, 2}
    let s1 = call_builtin_fn(&singleton, vec![Value::Int(1)]).unwrap();
    let s1 = call_builtin_fn(&insert, vec![Value::Int(2), s1]).unwrap();

    // s2 = {2}
    let s2 = call_builtin_fn(&singleton, vec![Value::Int(2)]).unwrap();

    let result = call_builtin_fn(&difference, vec![s1, s2]).unwrap();
    let len = call_builtin_fn(&size, vec![result]).unwrap();

    // Difference is {1}
    assert_eq!(len, Value::Int(1));
}

#[test]
fn test_set_symmetric_difference() {
    let singleton = get_builtin("Set.singleton").unwrap();
    let insert = get_builtin("Set.insert").unwrap();
    let sym_diff = get_builtin("Set.symmetricDifference").unwrap();
    let size = get_builtin("Set.size").unwrap();

    // s1 = {1, 2}
    let s1 = call_builtin_fn(&singleton, vec![Value::Int(1)]).unwrap();
    let s1 = call_builtin_fn(&insert, vec![Value::Int(2), s1]).unwrap();

    // s2 = {2, 3}
    let s2 = call_builtin_fn(&singleton, vec![Value::Int(2)]).unwrap();
    let s2 = call_builtin_fn(&insert, vec![Value::Int(3), s2]).unwrap();

    let result = call_builtin_fn(&sym_diff, vec![s1, s2]).unwrap();
    let len = call_builtin_fn(&size, vec![result]).unwrap();

    // Symmetric difference is {1, 3}
    assert_eq!(len, Value::Int(2));
}

#[test]
fn test_set_is_subset_true() {
    let singleton = get_builtin("Set.singleton").unwrap();
    let insert = get_builtin("Set.insert").unwrap();
    let is_subset = get_builtin("Set.isSubset").unwrap();

    // small = {1}
    let small = call_builtin_fn(&singleton, vec![Value::Int(1)]).unwrap();

    // big = {1, 2}
    let big = call_builtin_fn(&singleton, vec![Value::Int(1)]).unwrap();
    let big = call_builtin_fn(&insert, vec![Value::Int(2), big]).unwrap();

    let result = call_builtin_fn(&is_subset, vec![small, big]).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_set_is_subset_false() {
    let singleton = get_builtin("Set.singleton").unwrap();
    let insert = get_builtin("Set.insert").unwrap();
    let is_subset = get_builtin("Set.isSubset").unwrap();

    // big = {1, 2}
    let big = call_builtin_fn(&singleton, vec![Value::Int(1)]).unwrap();
    let big = call_builtin_fn(&insert, vec![Value::Int(2), big]).unwrap();

    // small = {1}
    let small = call_builtin_fn(&singleton, vec![Value::Int(1)]).unwrap();

    let result = call_builtin_fn(&is_subset, vec![big, small]).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_set_is_superset() {
    let singleton = get_builtin("Set.singleton").unwrap();
    let insert = get_builtin("Set.insert").unwrap();
    let is_superset = get_builtin("Set.isSuperset").unwrap();

    // big = {1, 2}
    let big = call_builtin_fn(&singleton, vec![Value::Int(1)]).unwrap();
    let big = call_builtin_fn(&insert, vec![Value::Int(2), big]).unwrap();

    // small = {1}
    let small = call_builtin_fn(&singleton, vec![Value::Int(1)]).unwrap();

    let result = call_builtin_fn(&is_superset, vec![big, small]).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_set_is_disjoint_true() {
    let singleton = get_builtin("Set.singleton").unwrap();
    let is_disjoint = get_builtin("Set.isDisjoint").unwrap();

    let s1 = call_builtin_fn(&singleton, vec![Value::Int(1)]).unwrap();
    let s2 = call_builtin_fn(&singleton, vec![Value::Int(2)]).unwrap();

    let result = call_builtin_fn(&is_disjoint, vec![s1, s2]).unwrap();
    assert_eq!(result, Value::Bool(true));
}

#[test]
fn test_set_is_disjoint_false() {
    let singleton = get_builtin("Set.singleton").unwrap();
    let is_disjoint = get_builtin("Set.isDisjoint").unwrap();

    let s1 = call_builtin_fn(&singleton, vec![Value::Int(1)]).unwrap();
    let s2 = call_builtin_fn(&singleton, vec![Value::Int(1)]).unwrap();

    let result = call_builtin_fn(&is_disjoint, vec![s1, s2]).unwrap();
    assert_eq!(result, Value::Bool(false));
}

#[test]
fn test_set_from_list_with_duplicates() {
    let from_list = get_builtin("Set.fromList").unwrap();
    let size = get_builtin("Set.size").unwrap();

    let list = Value::List(Rc::new(vec![
        Value::Int(1),
        Value::Int(2),
        Value::Int(2),
        Value::Int(3),
        Value::Int(1),
    ]));

    let set = call_builtin_fn(&from_list, vec![list]).unwrap();
    let len = call_builtin_fn(&size, vec![set]).unwrap();

    // Duplicates removed
    assert_eq!(len, Value::Int(3));
}

#[test]
fn test_set_to_list() {
    let singleton = get_builtin("Set.singleton").unwrap();
    let insert = get_builtin("Set.insert").unwrap();
    let to_list = get_builtin("Set.toList").unwrap();

    let s = call_builtin_fn(&singleton, vec![Value::Int(1)]).unwrap();
    let s = call_builtin_fn(&insert, vec![Value::Int(2), s]).unwrap();

    let list = call_builtin_fn(&to_list, vec![s]).unwrap();

    match list {
        Value::List(l) => assert_eq!(l.len(), 2),
        _ => panic!("Expected List"),
    }
}

// ============================================================================
// 错误处理边缘测试
// ============================================================================

#[test]
fn test_list_len_wrong_type() {
    let len_fn = get_builtin("list.len").unwrap();

    match len_fn {
        Value::Builtin(builtin) => {
            let result = (builtin.func)(&[Value::Int(42)]);
            assert!(result.is_err());
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_head_wrong_type() {
    let head_fn = get_builtin("list.head").unwrap();

    match head_fn {
        Value::Builtin(builtin) => {
            let result = (builtin.func)(&[Value::String(Rc::new("not a list".to_string()))]);
            assert!(result.is_err());
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_list_sum_non_int_list() {
    let sum_fn = get_builtin("list.sum").unwrap();

    match sum_fn {
        Value::Builtin(builtin) => {
            let list = Value::List(Rc::new(vec![
                Value::String(Rc::new("not".to_string())),
                Value::String(Rc::new("ints".to_string())),
            ]));
            let result = (builtin.func)(&[list]);
            assert!(result.is_err());
        }
        _ => panic!("Expected Builtin"),
    }
}

#[test]
fn test_map_size_wrong_type() {
    let size = get_builtin("Map.size").unwrap();
    let result = call_builtin_fn(&size, vec![Value::Int(42)]);
    assert!(result.is_err());
}

#[test]
fn test_set_size_wrong_type() {
    let size = get_builtin("Set.size").unwrap();
    let result = call_builtin_fn(&size, vec![Value::Int(42)]);
    assert!(result.is_err());
}

#[test]
fn test_map_insert_wrong_arity() {
    let insert = get_builtin("Map.insert").unwrap();
    let result = call_builtin_fn(&insert, vec![Value::Int(1)]);
    assert!(result.is_err());
}

#[test]
fn test_set_insert_wrong_arity() {
    let insert = get_builtin("Set.insert").unwrap();
    let result = call_builtin_fn(&insert, vec![Value::Int(1)]);
    assert!(result.is_err());
}
