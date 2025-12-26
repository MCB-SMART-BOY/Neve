//! Integration tests for neve-std crate.

use std::rc::Rc;
use neve_std::stdlib;
use neve_eval::Value;

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
    
    let result = call_builtin_fn(&singleton.unwrap(), vec![
        Value::String(Rc::new("key".to_string())),
        Value::Int(42),
    ]).unwrap();

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
    let map_builtins: Vec<_> = builtins.iter()
        .filter(|(name, _)| name.starts_with("Map."))
        .collect();
    assert!(!map_builtins.is_empty(), "No Map.* builtins found");
}

#[test]
fn test_stdlib_has_set_builtins() {
    let builtins = stdlib();
    let set_builtins: Vec<_> = builtins.iter()
        .filter(|(name, _)| name.starts_with("Set."))
        .collect();
    assert!(!set_builtins.is_empty(), "No Set.* builtins found");
}

#[test]
fn test_stdlib_has_list_builtins() {
    let builtins = stdlib();
    let list_builtins: Vec<_> = builtins.iter()
        .filter(|(name, _)| name.starts_with("list."))
        .collect();
    assert!(!list_builtins.is_empty(), "No list.* builtins found");
}
