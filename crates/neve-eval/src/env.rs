//! Evaluation environment.
//! 求值环境。
//!
//! This module provides the environment for variable bindings during evaluation.
//! 本模块提供求值过程中变量绑定的环境。

use crate::Value;
use neve_hir::LocalId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// An environment for variable bindings.
/// 变量绑定的环境。
///
/// The environment uses a parent-child chain to implement lexical scoping.
/// Each scope has its own bindings and a reference to its parent scope.
/// 环境使用父子链来实现词法作用域。
/// 每个作用域都有自己的绑定和对父作用域的引用。
#[derive(Clone)]
pub struct Environment {
    /// Variable bindings in this scope.
    /// 此作用域中的变量绑定。
    bindings: Rc<RefCell<HashMap<LocalId, Value>>>,
    /// Parent scope (if any).
    /// 父作用域（如果有）。
    parent: Option<Box<Environment>>,
}

impl Environment {
    /// Create a new empty environment.
    /// 创建一个新的空环境。
    pub fn new() -> Self {
        Self {
            bindings: Rc::new(RefCell::new(HashMap::new())),
            parent: None,
        }
    }

    /// Create a child environment.
    /// 创建一个子环境。
    ///
    /// The child environment can access bindings from its parent,
    /// but new bindings are added to the child scope only.
    /// 子环境可以访问其父环境的绑定，但新绑定只添加到子作用域中。
    pub fn child(&self) -> Self {
        Self {
            bindings: Rc::new(RefCell::new(HashMap::new())),
            parent: Some(Box::new(self.clone())),
        }
    }

    /// Define a variable in the current scope.
    /// 在当前作用域中定义变量。
    pub fn define(&self, id: LocalId, value: Value) {
        self.bindings.borrow_mut().insert(id, value);
    }

    /// Look up a variable.
    /// 查找变量。
    ///
    /// Searches the current scope first, then walks up the parent chain.
    /// 首先搜索当前作用域，然后向上遍历父链。
    pub fn get(&self, id: LocalId) -> Option<Value> {
        if let Some(value) = self.bindings.borrow().get(&id) {
            return Some(value.clone());
        }
        if let Some(parent) = &self.parent {
            return parent.get(id);
        }
        None
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}
