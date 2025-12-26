//! Evaluation environment.

use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use neve_hir::LocalId;
use crate::Value;

/// An environment for variable bindings.
#[derive(Clone)]
pub struct Environment {
    bindings: Rc<RefCell<HashMap<LocalId, Value>>>,
    parent: Option<Box<Environment>>,
}

impl Environment {
    /// Create a new empty environment.
    pub fn new() -> Self {
        Self {
            bindings: Rc::new(RefCell::new(HashMap::new())),
            parent: None,
        }
    }

    /// Create a child environment.
    pub fn child(&self) -> Self {
        Self {
            bindings: Rc::new(RefCell::new(HashMap::new())),
            parent: Some(Box::new(self.clone())),
        }
    }

    /// Define a variable in the current scope.
    pub fn define(&self, id: LocalId, value: Value) {
        self.bindings.borrow_mut().insert(id, value);
    }

    /// Look up a variable.
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
