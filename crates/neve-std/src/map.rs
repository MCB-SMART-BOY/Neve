//! Map module for Neve standard library.
//!
//! Provides immutable hash map operations.

use neve_eval::Value;
use std::collections::HashMap;
use std::rc::Rc;

/// Returns all map builtins.
pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        // ============================================
        // Construction
        // ============================================
        
        // empty : Map k v
        // Creates an empty map
        ("Map.empty", Value::Map(Rc::new(HashMap::new()))),
        
        // singleton : k -> v -> Map k v
        // Creates a map with a single key-value pair
        ("Map.singleton", Value::BuiltinFn("Map.singleton", Rc::new(|args| {
            if args.len() != 2 {
                return Err("Map.singleton requires 2 arguments".into());
            }
            let key = format!("{:?}", args[0]);
            let mut map = HashMap::new();
            map.insert(key, args[1].clone());
            Ok(Value::Map(Rc::new(map)))
        }))),
        
        // fromList : List (k, v) -> Map k v
        // Creates a map from a list of key-value pairs
        ("Map.fromList", Value::BuiltinFn("Map.fromList", Rc::new(|args| {
            if args.len() != 1 {
                return Err("Map.fromList requires 1 argument".into());
            }
            match &args[0] {
                Value::List(pairs) => {
                    let mut map = HashMap::new();
                    for pair in pairs.iter() {
                        match pair {
                            Value::Tuple(elements) if elements.len() == 2 => {
                                let key = format!("{:?}", elements[0]);
                                map.insert(key, elements[1].clone());
                            }
                            Value::Record(fields) => {
                                if let (Some(k), Some(v)) = (fields.get("key"), fields.get("value")) {
                                    let key = format!("{:?}", k);
                                    map.insert(key, v.clone());
                                }
                            }
                            _ => return Err("Map.fromList expects list of tuples or records".into()),
                        }
                    }
                    Ok(Value::Map(Rc::new(map)))
                }
                _ => Err("Map.fromList expects a list".into()),
            }
        }))),
        
        // ============================================
        // Query
        // ============================================
        
        // get : k -> Map k v -> Option v
        // Looks up a value by key
        ("Map.get", Value::BuiltinFn("Map.get", Rc::new(|args| {
            if args.len() != 2 {
                return Err("Map.get requires 2 arguments".into());
            }
            let key = format!("{:?}", args[0]);
            match &args[1] {
                Value::Map(map) => {
                    match map.get(&key) {
                        Some(v) => Ok(Value::Variant("Some".into(), Box::new(v.clone()))),
                        None => Ok(Value::Variant("None".into(), Box::new(Value::Unit))),
                    }
                }
                _ => Err("Map.get expects a map as second argument".into()),
            }
        }))),
        
        // getWithDefault : k -> v -> Map k v -> v
        // Gets a value with a default if key doesn't exist
        ("Map.getWithDefault", Value::BuiltinFn("Map.getWithDefault", Rc::new(|args| {
            if args.len() != 3 {
                return Err("Map.getWithDefault requires 3 arguments".into());
            }
            let key = format!("{:?}", args[0]);
            match &args[2] {
                Value::Map(map) => {
                    Ok(map.get(&key).cloned().unwrap_or_else(|| args[1].clone()))
                }
                _ => Err("Map.getWithDefault expects a map as third argument".into()),
            }
        }))),
        
        // contains : k -> Map k v -> Bool
        // Checks if a key exists in the map
        ("Map.contains", Value::BuiltinFn("Map.contains", Rc::new(|args| {
            if args.len() != 2 {
                return Err("Map.contains requires 2 arguments".into());
            }
            let key = format!("{:?}", args[0]);
            match &args[1] {
                Value::Map(map) => Ok(Value::Bool(map.contains_key(&key))),
                _ => Err("Map.contains expects a map as second argument".into()),
            }
        }))),
        
        // size : Map k v -> Int
        // Returns the number of entries in the map
        ("Map.size", Value::BuiltinFn("Map.size", Rc::new(|args| {
            if args.len() != 1 {
                return Err("Map.size requires 1 argument".into());
            }
            match &args[0] {
                Value::Map(map) => Ok(Value::Int(map.len() as i64)),
                _ => Err("Map.size expects a map".into()),
            }
        }))),
        
        // isEmpty : Map k v -> Bool
        // Checks if the map is empty
        ("Map.isEmpty", Value::BuiltinFn("Map.isEmpty", Rc::new(|args| {
            if args.len() != 1 {
                return Err("Map.isEmpty requires 1 argument".into());
            }
            match &args[0] {
                Value::Map(map) => Ok(Value::Bool(map.is_empty())),
                _ => Err("Map.isEmpty expects a map".into()),
            }
        }))),
        
        // ============================================
        // Modification
        // ============================================
        
        // insert : k -> v -> Map k v -> Map k v
        // Inserts a key-value pair (overwrites if exists)
        ("Map.insert", Value::BuiltinFn("Map.insert", Rc::new(|args| {
            if args.len() != 3 {
                return Err("Map.insert requires 3 arguments".into());
            }
            let key = format!("{:?}", args[0]);
            match &args[2] {
                Value::Map(map) => {
                    let mut new_map = (**map).clone();
                    new_map.insert(key, args[1].clone());
                    Ok(Value::Map(Rc::new(new_map)))
                }
                _ => Err("Map.insert expects a map as third argument".into()),
            }
        }))),
        
        // remove : k -> Map k v -> Map k v
        // Removes a key from the map
        ("Map.remove", Value::BuiltinFn("Map.remove", Rc::new(|args| {
            if args.len() != 2 {
                return Err("Map.remove requires 2 arguments".into());
            }
            let key = format!("{:?}", args[0]);
            match &args[1] {
                Value::Map(map) => {
                    let mut new_map = (**map).clone();
                    new_map.remove(&key);
                    Ok(Value::Map(Rc::new(new_map)))
                }
                _ => Err("Map.remove expects a map as second argument".into()),
            }
        }))),
        
        // update : k -> (Option v -> Option v) -> Map k v -> Map k v
        // Updates a value at a key using a function
        // Note: Requires closure evaluation support
        ("Map.update", Value::BuiltinFn("Map.update", Rc::new(|args| {
            if args.len() != 3 {
                return Err("Map.update requires 3 arguments (key, fn, map)".into());
            }
            // Full implementation requires closure evaluation
            Err("Map.update requires closure evaluation support".into())
        }))),
        
        // ============================================
        // Combine
        // ============================================
        
        // union : Map k v -> Map k v -> Map k v
        // Combines two maps, preferring values from the first
        ("Map.union", Value::BuiltinFn("Map.union", Rc::new(|args| {
            if args.len() != 2 {
                return Err("Map.union requires 2 arguments".into());
            }
            match (&args[0], &args[1]) {
                (Value::Map(m1), Value::Map(m2)) => {
                    let mut result = (**m2).clone();
                    for (k, v) in m1.iter() {
                        result.insert(k.clone(), v.clone());
                    }
                    Ok(Value::Map(Rc::new(result)))
                }
                _ => Err("Map.union expects two maps".into()),
            }
        }))),
        
        // intersection : Map k v -> Map k v -> Map k v
        // Returns a map with keys present in both
        ("Map.intersection", Value::BuiltinFn("Map.intersection", Rc::new(|args| {
            if args.len() != 2 {
                return Err("Map.intersection requires 2 arguments".into());
            }
            match (&args[0], &args[1]) {
                (Value::Map(m1), Value::Map(m2)) => {
                    let mut result = HashMap::new();
                    for (k, v) in m1.iter() {
                        if m2.contains_key(k) {
                            result.insert(k.clone(), v.clone());
                        }
                    }
                    Ok(Value::Map(Rc::new(result)))
                }
                _ => Err("Map.intersection expects two maps".into()),
            }
        }))),
        
        // difference : Map k v -> Map k v -> Map k v
        // Returns keys in first map but not in second
        ("Map.difference", Value::BuiltinFn("Map.difference", Rc::new(|args| {
            if args.len() != 2 {
                return Err("Map.difference requires 2 arguments".into());
            }
            match (&args[0], &args[1]) {
                (Value::Map(m1), Value::Map(m2)) => {
                    let mut result = HashMap::new();
                    for (k, v) in m1.iter() {
                        if !m2.contains_key(k) {
                            result.insert(k.clone(), v.clone());
                        }
                    }
                    Ok(Value::Map(Rc::new(result)))
                }
                _ => Err("Map.difference expects two maps".into()),
            }
        }))),
        
        // ============================================
        // Conversion
        // ============================================
        
        // keys : Map k v -> List k
        // Returns all keys as a list
        ("Map.keys", Value::BuiltinFn("Map.keys", Rc::new(|args| {
            if args.len() != 1 {
                return Err("Map.keys requires 1 argument".into());
            }
            match &args[0] {
                Value::Map(map) => {
                    // Keys are stored as debug strings, return as strings
                    let keys: Vec<Value> = map.keys()
                        .map(|k| Value::String(Rc::new(k.clone())))
                        .collect();
                    Ok(Value::List(Rc::new(keys)))
                }
                _ => Err("Map.keys expects a map".into()),
            }
        }))),
        
        // values : Map k v -> List v
        // Returns all values as a list
        ("Map.values", Value::BuiltinFn("Map.values", Rc::new(|args| {
            if args.len() != 1 {
                return Err("Map.values requires 1 argument".into());
            }
            match &args[0] {
                Value::Map(map) => {
                    let values: Vec<Value> = map.values().cloned().collect();
                    Ok(Value::List(Rc::new(values)))
                }
                _ => Err("Map.values expects a map".into()),
            }
        }))),
        
        // toList : Map k v -> List (k, v)
        // Converts map to list of key-value pairs
        ("Map.toList", Value::BuiltinFn("Map.toList", Rc::new(|args| {
            if args.len() != 1 {
                return Err("Map.toList requires 1 argument".into());
            }
            match &args[0] {
                Value::Map(map) => {
                    let pairs: Vec<Value> = map.iter()
                        .map(|(k, v)| {
                            Value::Tuple(Rc::new(vec![
                                Value::String(Rc::new(k.clone())),
                                v.clone(),
                            ]))
                        })
                        .collect();
                    Ok(Value::List(Rc::new(pairs)))
                }
                _ => Err("Map.toList expects a map".into()),
            }
        }))),
        
        // ============================================
        // Higher-order (require closure evaluation)
        // ============================================
        
        // map : (v -> w) -> Map k v -> Map k w
        ("Map.map", Value::BuiltinFn("Map.map", Rc::new(|args| {
            if args.len() != 2 {
                return Err("Map.map requires 2 arguments (fn, map)".into());
            }
            Err("Map.map requires closure evaluation support".into())
        }))),
        
        // mapWithKey : (k -> v -> w) -> Map k v -> Map k w
        ("Map.mapWithKey", Value::BuiltinFn("Map.mapWithKey", Rc::new(|args| {
            if args.len() != 2 {
                return Err("Map.mapWithKey requires 2 arguments (fn, map)".into());
            }
            Err("Map.mapWithKey requires closure evaluation support".into())
        }))),
        
        // filter : (v -> Bool) -> Map k v -> Map k v
        ("Map.filter", Value::BuiltinFn("Map.filter", Rc::new(|args| {
            if args.len() != 2 {
                return Err("Map.filter requires 2 arguments (predicate, map)".into());
            }
            Err("Map.filter requires closure evaluation support".into())
        }))),
        
        // filterWithKey : (k -> v -> Bool) -> Map k v -> Map k v
        ("Map.filterWithKey", Value::BuiltinFn("Map.filterWithKey", Rc::new(|args| {
            if args.len() != 2 {
                return Err("Map.filterWithKey requires 2 arguments (predicate, map)".into());
            }
            Err("Map.filterWithKey requires closure evaluation support".into())
        }))),
        
        // fold : (b -> v -> b) -> b -> Map k v -> b
        ("Map.fold", Value::BuiltinFn("Map.fold", Rc::new(|args| {
            if args.len() != 3 {
                return Err("Map.fold requires 3 arguments (fn, init, map)".into());
            }
            Err("Map.fold requires closure evaluation support".into())
        }))),
        
        // foldWithKey : (b -> k -> v -> b) -> b -> Map k v -> b
        ("Map.foldWithKey", Value::BuiltinFn("Map.foldWithKey", Rc::new(|args| {
            if args.len() != 3 {
                return Err("Map.foldWithKey requires 3 arguments (fn, init, map)".into());
            }
            Err("Map.foldWithKey requires closure evaluation support".into())
        }))),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_builtin(name: &str) -> Value {
        builtins()
            .into_iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v)
            .unwrap()
    }

    fn call(f: &Value, args: Vec<Value>) -> Result<Value, String> {
        match f {
            Value::BuiltinFn(_, func) => func(args),
            _ => Err("Not a function".into()),
        }
    }

    fn str_val(s: &str) -> Value {
        Value::String(Rc::new(s.to_string()))
    }

    #[test]
    fn test_empty() {
        let empty = get_builtin("Map.empty");
        match empty {
            Value::Map(m) => assert!(m.is_empty()),
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_singleton() {
        let singleton = get_builtin("Map.singleton");
        let result = call(&singleton, vec![
            str_val("key"),
            Value::Int(42),
        ]).unwrap();
        
        match result {
            Value::Map(m) => assert_eq!(m.len(), 1),
            _ => panic!("Expected Map"),
        }
    }

    #[test]
    fn test_insert_and_get() {
        let empty = get_builtin("Map.empty");
        let insert = get_builtin("Map.insert");
        let get = get_builtin("Map.get");
        
        let m1 = call(&insert, vec![
            str_val("foo"),
            Value::Int(100),
            empty.clone(),
        ]).unwrap();
        
        let result = call(&get, vec![
            str_val("foo"),
            m1,
        ]).unwrap();
        
        match result {
            Value::Variant(tag, _) => assert_eq!(tag, "Some"),
            _ => panic!("Expected Some variant"),
        }
    }

    #[test]
    fn test_size() {
        let empty = get_builtin("Map.empty");
        let insert = get_builtin("Map.insert");
        let size = get_builtin("Map.size");
        
        let m1 = call(&insert, vec![
            str_val("a"),
            Value::Int(1),
            empty.clone(),
        ]).unwrap();
        
        let m2 = call(&insert, vec![
            str_val("b"),
            Value::Int(2),
            m1,
        ]).unwrap();
        
        let result = call(&size, vec![m2]).unwrap();
        assert_eq!(result, Value::Int(2));
    }

    #[test]
    fn test_remove() {
        let singleton = get_builtin("Map.singleton");
        let remove = get_builtin("Map.remove");
        let is_empty = get_builtin("Map.isEmpty");
        
        let m = call(&singleton, vec![
            str_val("key"),
            Value::Int(42),
        ]).unwrap();
        
        let m2 = call(&remove, vec![
            str_val("key"),
            m,
        ]).unwrap();
        
        let result = call(&is_empty, vec![m2]).unwrap();
        assert_eq!(result, Value::Bool(true));
    }

    #[test]
    fn test_union() {
        let singleton = get_builtin("Map.singleton");
        let union = get_builtin("Map.union");
        let size = get_builtin("Map.size");
        
        let m1 = call(&singleton, vec![
            str_val("a"),
            Value::Int(1),
        ]).unwrap();
        
        let m2 = call(&singleton, vec![
            str_val("b"),
            Value::Int(2),
        ]).unwrap();
        
        let combined = call(&union, vec![m1, m2]).unwrap();
        let result = call(&size, vec![combined]).unwrap();
        assert_eq!(result, Value::Int(2));
    }

    #[test]
    fn test_keys_and_values() {
        let singleton = get_builtin("Map.singleton");
        let keys = get_builtin("Map.keys");
        let values = get_builtin("Map.values");
        
        let m = call(&singleton, vec![
            str_val("key"),
            Value::Int(42),
        ]).unwrap();
        
        let k = call(&keys, vec![m.clone()]).unwrap();
        let v = call(&values, vec![m]).unwrap();
        
        match k {
            Value::List(list) => assert_eq!(list.len(), 1),
            _ => panic!("Expected List"),
        }
        
        match v {
            Value::List(list) => {
                assert_eq!(list.len(), 1);
                assert_eq!(list[0], Value::Int(42));
            }
            _ => panic!("Expected List"),
        }
    }
}
