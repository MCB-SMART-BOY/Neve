//! Map module for Neve standard library.
//! Neve 标准库的 Map 模块。
//!
//! Provides immutable hash map operations.
//! 提供不可变哈希映射操作。

use neve_eval::Value;
use std::collections::HashMap;
use std::rc::Rc;

/// Returns all map builtins.
/// 返回所有映射内置函数。
pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        // ============================================
        // Construction / 构造
        // ============================================

        // empty : Map k v
        // Creates an empty map / 创建空映射
        ("Map.empty", Value::Map(Rc::new(HashMap::new()))),
        // singleton : k -> v -> Map k v
        // Creates a map with a single key-value pair / 创建包含单个键值对的映射
        (
            "Map.singleton",
            Value::BuiltinFn(
                "Map.singleton",
                Rc::new(|args| {
                    if args.len() != 2 {
                        return Err("Map.singleton requires 2 arguments".into());
                    }
                    let key = format!("{:?}", args[0]);
                    let mut map = HashMap::new();
                    map.insert(key, args[1].clone());
                    Ok(Value::Map(Rc::new(map)))
                }),
            ),
        ),
        // fromList : List (k, v) -> Map k v
        // Creates a map from a list of key-value pairs / 从键值对列表创建映射
        (
            "Map.fromList",
            Value::BuiltinFn(
                "Map.fromList",
                Rc::new(|args| {
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
                                        if let (Some(k), Some(v)) =
                                            (fields.get("key"), fields.get("value"))
                                        {
                                            let key = format!("{:?}", k);
                                            map.insert(key, v.clone());
                                        }
                                    }
                                    _ => {
                                        return Err(
                                            "Map.fromList expects list of tuples or records".into(),
                                        );
                                    }
                                }
                            }
                            Ok(Value::Map(Rc::new(map)))
                        }
                        _ => Err("Map.fromList expects a list".into()),
                    }
                }),
            ),
        ),
        // ============================================
        // Query / 查询
        // ============================================

        // get : k -> Map k v -> Option v
        // Looks up a value by key / 按键查找值
        (
            "Map.get",
            Value::BuiltinFn(
                "Map.get",
                Rc::new(|args| {
                    if args.len() != 2 {
                        return Err("Map.get requires 2 arguments".into());
                    }
                    let key = format!("{:?}", args[0]);
                    match &args[1] {
                        Value::Map(map) => match map.get(&key) {
                            Some(v) => Ok(Value::Variant("Some".into(), Box::new(v.clone()))),
                            None => Ok(Value::Variant("None".into(), Box::new(Value::Unit))),
                        },
                        _ => Err("Map.get expects a map as second argument".into()),
                    }
                }),
            ),
        ),
        // getWithDefault : k -> v -> Map k v -> v
        // Gets a value with a default if key doesn't exist / 获取值，如果键不存在则返回默认值
        (
            "Map.getWithDefault",
            Value::BuiltinFn(
                "Map.getWithDefault",
                Rc::new(|args| {
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
                }),
            ),
        ),
        // contains : k -> Map k v -> Bool
        // Checks if a key exists in the map / 检查键是否存在于映射中
        (
            "Map.contains",
            Value::BuiltinFn(
                "Map.contains",
                Rc::new(|args| {
                    if args.len() != 2 {
                        return Err("Map.contains requires 2 arguments".into());
                    }
                    let key = format!("{:?}", args[0]);
                    match &args[1] {
                        Value::Map(map) => Ok(Value::Bool(map.contains_key(&key))),
                        _ => Err("Map.contains expects a map as second argument".into()),
                    }
                }),
            ),
        ),
        // size : Map k v -> Int
        // Returns the number of entries in the map / 返回映射中的条目数
        (
            "Map.size",
            Value::BuiltinFn(
                "Map.size",
                Rc::new(|args| {
                    if args.len() != 1 {
                        return Err("Map.size requires 1 argument".into());
                    }
                    match &args[0] {
                        Value::Map(map) => Ok(Value::Int(map.len() as i64)),
                        _ => Err("Map.size expects a map".into()),
                    }
                }),
            ),
        ),
        // isEmpty : Map k v -> Bool
        // Checks if the map is empty / 检查映射是否为空
        (
            "Map.isEmpty",
            Value::BuiltinFn(
                "Map.isEmpty",
                Rc::new(|args| {
                    if args.len() != 1 {
                        return Err("Map.isEmpty requires 1 argument".into());
                    }
                    match &args[0] {
                        Value::Map(map) => Ok(Value::Bool(map.is_empty())),
                        _ => Err("Map.isEmpty expects a map".into()),
                    }
                }),
            ),
        ),
        // ============================================
        // Modification / 修改
        // ============================================

        // insert : k -> v -> Map k v -> Map k v
        // Inserts a key-value pair (overwrites if exists) / 插入键值对（如存在则覆盖）
        (
            "Map.insert",
            Value::BuiltinFn(
                "Map.insert",
                Rc::new(|args| {
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
                }),
            ),
        ),
        // remove : k -> Map k v -> Map k v
        // Removes a key from the map / 从映射中删除键
        (
            "Map.remove",
            Value::BuiltinFn(
                "Map.remove",
                Rc::new(|args| {
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
                }),
            ),
        ),
        // update : k -> (Option v -> Option v) -> Map k v -> Map k v
        // Updates a value at a key using a function / 使用函数更新键处的值
        // Note: Requires closure evaluation support / 注意：需要闭包求值支持
        (
            "Map.update",
            Value::BuiltinFn(
                "Map.update",
                Rc::new(|args| {
                    if args.len() != 3 {
                        return Err("Map.update requires 3 arguments (key, fn, map)".into());
                    }
                    // Full implementation requires closure evaluation
                    // 完整实现需要闭包求值
                    Err("Map.update requires closure evaluation support".into())
                }),
            ),
        ),
        // ============================================
        // Combine / 组合
        // ============================================

        // union : Map k v -> Map k v -> Map k v
        // Combines two maps, preferring values from the first / 合并两个映射，优先使用第一个的值
        (
            "Map.union",
            Value::BuiltinFn(
                "Map.union",
                Rc::new(|args| {
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
                }),
            ),
        ),
        // intersection : Map k v -> Map k v -> Map k v
        // Returns a map with keys present in both / 返回两者都有的键的映射
        (
            "Map.intersection",
            Value::BuiltinFn(
                "Map.intersection",
                Rc::new(|args| {
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
                }),
            ),
        ),
        // difference : Map k v -> Map k v -> Map k v
        // Returns keys in first map but not in second / 返回第一个映射有但第二个没有的键
        (
            "Map.difference",
            Value::BuiltinFn(
                "Map.difference",
                Rc::new(|args| {
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
                }),
            ),
        ),
        // ============================================
        // Conversion / 转换
        // ============================================

        // keys : Map k v -> List k
        // Returns all keys as a list / 返回所有键的列表
        (
            "Map.keys",
            Value::BuiltinFn(
                "Map.keys",
                Rc::new(|args| {
                    if args.len() != 1 {
                        return Err("Map.keys requires 1 argument".into());
                    }
                    match &args[0] {
                        Value::Map(map) => {
                            // Keys are stored as debug strings, return as strings
                            // 键存储为调试字符串，作为字符串返回
                            let keys: Vec<Value> = map
                                .keys()
                                .map(|k| Value::String(Rc::new(k.clone())))
                                .collect();
                            Ok(Value::List(Rc::new(keys)))
                        }
                        _ => Err("Map.keys expects a map".into()),
                    }
                }),
            ),
        ),
        // values : Map k v -> List v
        // Returns all values as a list / 返回所有值的列表
        (
            "Map.values",
            Value::BuiltinFn(
                "Map.values",
                Rc::new(|args| {
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
                }),
            ),
        ),
        // toList : Map k v -> List (k, v)
        // Converts map to list of key-value pairs / 将映射转换为键值对列表
        (
            "Map.toList",
            Value::BuiltinFn(
                "Map.toList",
                Rc::new(|args| {
                    if args.len() != 1 {
                        return Err("Map.toList requires 1 argument".into());
                    }
                    match &args[0] {
                        Value::Map(map) => {
                            let pairs: Vec<Value> = map
                                .iter()
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
                }),
            ),
        ),
        // ============================================
        // Higher-order (require closure evaluation)
        // 高阶函数（需要闭包求值）
        // ============================================

        // map : (v -> w) -> Map k v -> Map k w
        (
            "Map.map",
            Value::BuiltinFn(
                "Map.map",
                Rc::new(|args| {
                    if args.len() != 2 {
                        return Err("Map.map requires 2 arguments (fn, map)".into());
                    }
                    Err("Map.map requires closure evaluation support".into())
                }),
            ),
        ),
        // mapWithKey : (k -> v -> w) -> Map k v -> Map k w
        (
            "Map.mapWithKey",
            Value::BuiltinFn(
                "Map.mapWithKey",
                Rc::new(|args| {
                    if args.len() != 2 {
                        return Err("Map.mapWithKey requires 2 arguments (fn, map)".into());
                    }
                    Err("Map.mapWithKey requires closure evaluation support".into())
                }),
            ),
        ),
        // filter : (v -> Bool) -> Map k v -> Map k v
        (
            "Map.filter",
            Value::BuiltinFn(
                "Map.filter",
                Rc::new(|args| {
                    if args.len() != 2 {
                        return Err("Map.filter requires 2 arguments (predicate, map)".into());
                    }
                    Err("Map.filter requires closure evaluation support".into())
                }),
            ),
        ),
        // filterWithKey : (k -> v -> Bool) -> Map k v -> Map k v
        (
            "Map.filterWithKey",
            Value::BuiltinFn(
                "Map.filterWithKey",
                Rc::new(|args| {
                    if args.len() != 2 {
                        return Err(
                            "Map.filterWithKey requires 2 arguments (predicate, map)".into()
                        );
                    }
                    Err("Map.filterWithKey requires closure evaluation support".into())
                }),
            ),
        ),
        // fold : (b -> v -> b) -> b -> Map k v -> b
        (
            "Map.fold",
            Value::BuiltinFn(
                "Map.fold",
                Rc::new(|args| {
                    if args.len() != 3 {
                        return Err("Map.fold requires 3 arguments (fn, init, map)".into());
                    }
                    Err("Map.fold requires closure evaluation support".into())
                }),
            ),
        ),
        // foldWithKey : (b -> k -> v -> b) -> b -> Map k v -> b
        (
            "Map.foldWithKey",
            Value::BuiltinFn(
                "Map.foldWithKey",
                Rc::new(|args| {
                    if args.len() != 3 {
                        return Err("Map.foldWithKey requires 3 arguments (fn, init, map)".into());
                    }
                    Err("Map.foldWithKey requires closure evaluation support".into())
                }),
            ),
        ),
    ]
}
