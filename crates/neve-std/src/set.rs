//! Set module for Neve standard library.
//!
//! Provides immutable hash set operations.

use neve_eval::Value;
use std::collections::HashSet;
use std::rc::Rc;

/// Returns all set builtins.
pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        // ============================================
        // Construction
        // ============================================

        // empty : Set a
        // Creates an empty set
        ("Set.empty", Value::Set(Rc::new(HashSet::new()))),
        // singleton : a -> Set a
        // Creates a set with a single element
        (
            "Set.singleton",
            Value::BuiltinFn(
                "Set.singleton",
                Rc::new(|args| {
                    if args.len() != 1 {
                        return Err("Set.singleton requires 1 argument".into());
                    }
                    let key = format!("{:?}", args[0]);
                    let mut set = HashSet::new();
                    set.insert(key);
                    Ok(Value::Set(Rc::new(set)))
                }),
            ),
        ),
        // fromList : List a -> Set a
        // Creates a set from a list
        (
            "Set.fromList",
            Value::BuiltinFn(
                "Set.fromList",
                Rc::new(|args| {
                    if args.len() != 1 {
                        return Err("Set.fromList requires 1 argument".into());
                    }
                    match &args[0] {
                        Value::List(list) => {
                            let set: HashSet<String> =
                                list.iter().map(|v| format!("{:?}", v)).collect();
                            Ok(Value::Set(Rc::new(set)))
                        }
                        _ => Err("Set.fromList expects a list".into()),
                    }
                }),
            ),
        ),
        // ============================================
        // Query
        // ============================================

        // contains : a -> Set a -> Bool
        // Checks if an element is in the set
        (
            "Set.contains",
            Value::BuiltinFn(
                "Set.contains",
                Rc::new(|args| {
                    if args.len() != 2 {
                        return Err("Set.contains requires 2 arguments".into());
                    }
                    let key = format!("{:?}", args[0]);
                    match &args[1] {
                        Value::Set(set) => Ok(Value::Bool(set.contains(&key))),
                        _ => Err("Set.contains expects a set as second argument".into()),
                    }
                }),
            ),
        ),
        // size : Set a -> Int
        // Returns the number of elements in the set
        (
            "Set.size",
            Value::BuiltinFn(
                "Set.size",
                Rc::new(|args| {
                    if args.len() != 1 {
                        return Err("Set.size requires 1 argument".into());
                    }
                    match &args[0] {
                        Value::Set(set) => Ok(Value::Int(set.len() as i64)),
                        _ => Err("Set.size expects a set".into()),
                    }
                }),
            ),
        ),
        // isEmpty : Set a -> Bool
        // Checks if the set is empty
        (
            "Set.isEmpty",
            Value::BuiltinFn(
                "Set.isEmpty",
                Rc::new(|args| {
                    if args.len() != 1 {
                        return Err("Set.isEmpty requires 1 argument".into());
                    }
                    match &args[0] {
                        Value::Set(set) => Ok(Value::Bool(set.is_empty())),
                        _ => Err("Set.isEmpty expects a set".into()),
                    }
                }),
            ),
        ),
        // ============================================
        // Modification
        // ============================================

        // insert : a -> Set a -> Set a
        // Inserts an element into the set
        (
            "Set.insert",
            Value::BuiltinFn(
                "Set.insert",
                Rc::new(|args| {
                    if args.len() != 2 {
                        return Err("Set.insert requires 2 arguments".into());
                    }
                    let key = format!("{:?}", args[0]);
                    match &args[1] {
                        Value::Set(set) => {
                            let mut new_set = (**set).clone();
                            new_set.insert(key);
                            Ok(Value::Set(Rc::new(new_set)))
                        }
                        _ => Err("Set.insert expects a set as second argument".into()),
                    }
                }),
            ),
        ),
        // remove : a -> Set a -> Set a
        // Removes an element from the set
        (
            "Set.remove",
            Value::BuiltinFn(
                "Set.remove",
                Rc::new(|args| {
                    if args.len() != 2 {
                        return Err("Set.remove requires 2 arguments".into());
                    }
                    let key = format!("{:?}", args[0]);
                    match &args[1] {
                        Value::Set(set) => {
                            let mut new_set = (**set).clone();
                            new_set.remove(&key);
                            Ok(Value::Set(Rc::new(new_set)))
                        }
                        _ => Err("Set.remove expects a set as second argument".into()),
                    }
                }),
            ),
        ),
        // ============================================
        // Set Operations
        // ============================================

        // union : Set a -> Set a -> Set a
        // Returns the union of two sets
        (
            "Set.union",
            Value::BuiltinFn(
                "Set.union",
                Rc::new(|args| {
                    if args.len() != 2 {
                        return Err("Set.union requires 2 arguments".into());
                    }
                    match (&args[0], &args[1]) {
                        (Value::Set(s1), Value::Set(s2)) => {
                            let result: HashSet<String> = s1.union(&**s2).cloned().collect();
                            Ok(Value::Set(Rc::new(result)))
                        }
                        _ => Err("Set.union expects two sets".into()),
                    }
                }),
            ),
        ),
        // intersection : Set a -> Set a -> Set a
        // Returns the intersection of two sets
        (
            "Set.intersection",
            Value::BuiltinFn(
                "Set.intersection",
                Rc::new(|args| {
                    if args.len() != 2 {
                        return Err("Set.intersection requires 2 arguments".into());
                    }
                    match (&args[0], &args[1]) {
                        (Value::Set(s1), Value::Set(s2)) => {
                            let result: HashSet<String> = s1.intersection(&**s2).cloned().collect();
                            Ok(Value::Set(Rc::new(result)))
                        }
                        _ => Err("Set.intersection expects two sets".into()),
                    }
                }),
            ),
        ),
        // difference : Set a -> Set a -> Set a
        // Returns elements in first set but not in second
        (
            "Set.difference",
            Value::BuiltinFn(
                "Set.difference",
                Rc::new(|args| {
                    if args.len() != 2 {
                        return Err("Set.difference requires 2 arguments".into());
                    }
                    match (&args[0], &args[1]) {
                        (Value::Set(s1), Value::Set(s2)) => {
                            let result: HashSet<String> = s1.difference(&**s2).cloned().collect();
                            Ok(Value::Set(Rc::new(result)))
                        }
                        _ => Err("Set.difference expects two sets".into()),
                    }
                }),
            ),
        ),
        // symmetricDifference : Set a -> Set a -> Set a
        // Returns elements in either set but not in both
        (
            "Set.symmetricDifference",
            Value::BuiltinFn(
                "Set.symmetricDifference",
                Rc::new(|args| {
                    if args.len() != 2 {
                        return Err("Set.symmetricDifference requires 2 arguments".into());
                    }
                    match (&args[0], &args[1]) {
                        (Value::Set(s1), Value::Set(s2)) => {
                            let result: HashSet<String> =
                                s1.symmetric_difference(&**s2).cloned().collect();
                            Ok(Value::Set(Rc::new(result)))
                        }
                        _ => Err("Set.symmetricDifference expects two sets".into()),
                    }
                }),
            ),
        ),
        // isSubset : Set a -> Set a -> Bool
        // Checks if first set is a subset of second
        (
            "Set.isSubset",
            Value::BuiltinFn(
                "Set.isSubset",
                Rc::new(|args| {
                    if args.len() != 2 {
                        return Err("Set.isSubset requires 2 arguments".into());
                    }
                    match (&args[0], &args[1]) {
                        (Value::Set(s1), Value::Set(s2)) => Ok(Value::Bool(s1.is_subset(&**s2))),
                        _ => Err("Set.isSubset expects two sets".into()),
                    }
                }),
            ),
        ),
        // isSuperset : Set a -> Set a -> Bool
        // Checks if first set is a superset of second
        (
            "Set.isSuperset",
            Value::BuiltinFn(
                "Set.isSuperset",
                Rc::new(|args| {
                    if args.len() != 2 {
                        return Err("Set.isSuperset requires 2 arguments".into());
                    }
                    match (&args[0], &args[1]) {
                        (Value::Set(s1), Value::Set(s2)) => Ok(Value::Bool(s1.is_superset(&**s2))),
                        _ => Err("Set.isSuperset expects two sets".into()),
                    }
                }),
            ),
        ),
        // isDisjoint : Set a -> Set a -> Bool
        // Checks if two sets have no common elements
        (
            "Set.isDisjoint",
            Value::BuiltinFn(
                "Set.isDisjoint",
                Rc::new(|args| {
                    if args.len() != 2 {
                        return Err("Set.isDisjoint requires 2 arguments".into());
                    }
                    match (&args[0], &args[1]) {
                        (Value::Set(s1), Value::Set(s2)) => Ok(Value::Bool(s1.is_disjoint(&**s2))),
                        _ => Err("Set.isDisjoint expects two sets".into()),
                    }
                }),
            ),
        ),
        // ============================================
        // Conversion
        // ============================================

        // toList : Set a -> List a
        // Converts set to a list
        (
            "Set.toList",
            Value::BuiltinFn(
                "Set.toList",
                Rc::new(|args| {
                    if args.len() != 1 {
                        return Err("Set.toList requires 1 argument".into());
                    }
                    match &args[0] {
                        Value::Set(set) => {
                            // Elements are stored as debug strings
                            let list: Vec<Value> = set
                                .iter()
                                .map(|s| Value::String(Rc::new(s.clone())))
                                .collect();
                            Ok(Value::List(Rc::new(list)))
                        }
                        _ => Err("Set.toList expects a set".into()),
                    }
                }),
            ),
        ),
        // ============================================
        // Higher-order (require closure evaluation)
        // ============================================

        // map : (a -> b) -> Set a -> Set b
        (
            "Set.map",
            Value::BuiltinFn(
                "Set.map",
                Rc::new(|args| {
                    if args.len() != 2 {
                        return Err("Set.map requires 2 arguments (fn, set)".into());
                    }
                    Err("Set.map requires closure evaluation support".into())
                }),
            ),
        ),
        // filter : (a -> Bool) -> Set a -> Set a
        (
            "Set.filter",
            Value::BuiltinFn(
                "Set.filter",
                Rc::new(|args| {
                    if args.len() != 2 {
                        return Err("Set.filter requires 2 arguments (predicate, set)".into());
                    }
                    Err("Set.filter requires closure evaluation support".into())
                }),
            ),
        ),
        // fold : (b -> a -> b) -> b -> Set a -> b
        (
            "Set.fold",
            Value::BuiltinFn(
                "Set.fold",
                Rc::new(|args| {
                    if args.len() != 3 {
                        return Err("Set.fold requires 3 arguments (fn, init, set)".into());
                    }
                    Err("Set.fold requires closure evaluation support".into())
                }),
            ),
        ),
        // partition : (a -> Bool) -> Set a -> (Set a, Set a)
        (
            "Set.partition",
            Value::BuiltinFn(
                "Set.partition",
                Rc::new(|args| {
                    if args.len() != 2 {
                        return Err("Set.partition requires 2 arguments (predicate, set)".into());
                    }
                    Err("Set.partition requires closure evaluation support".into())
                }),
            ),
        ),
    ]
}
