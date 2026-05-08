//! Event system, reactive, temporal — new additions live here.

use neve_eval::value::{BuiltinFn, EventKind, EventValue, LiveValue, Value};
use std::rc::Rc;

pub fn builtins() -> Vec<(&'static str, Value)> {
    vec![
        // Events / 事件
        // Reactive / 反应式
        (
            "io.reactive",
            Value::Builtin(BuiltinFn {
                name: "io.reactive",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Event(event) => Ok(Value::Live(Rc::new(LiveValue {
                        event: Rc::clone(event),
                        current: Rc::new(std::cell::RefCell::new(None)),
                        cancelled: Rc::new(std::cell::Cell::new(false)),
                    }))),
                    _ => Err("io.reactive expects an Event".to_string()),
                },
            }),
        ),
        (
            "io.liveNext",
            Value::Builtin(BuiltinFn {
                name: "io.liveNext",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Live(live) => {
                        if live.cancelled.get() {
                            return Err("io.liveNext: live cancelled".to_string());
                        }
                        // Poll the source event
                        let val = crate::io::poll_event(&live.event)?;
                        // Cache the value
                        *live.current.borrow_mut() = Some(val.clone());
                        Ok(val)
                    }
                    _ => Err("io.liveNext expects a Live value".to_string()),
                },
            }),
        ),
        (
            "io.liveCurrent",
            Value::Builtin(BuiltinFn {
                name: "io.liveCurrent",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Live(live) => {
                        let current = live.current.borrow();
                        Ok(match current.as_ref() {
                            Some(v) => Value::Some(Box::new(v.clone())),
                            None => Value::None,
                        })
                    }
                    _ => Err("io.liveCurrent expects a Live value".to_string()),
                },
            }),
        ),
        (
            "io.liveCancel",
            Value::Builtin(BuiltinFn {
                name: "io.liveCancel",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Live(live) => {
                        live.cancelled.set(true);
                        Ok(Value::Unit)
                    }
                    _ => Err("io.liveCancel expects a Live value".to_string()),
                },
            }),
        ),
        (
            "io.eventMap",
            Value::Builtin(BuiltinFn {
                name: "io.eventMap",
                arity: 2,
                func: |_args| Err("io.eventMap is evaluator-owned".to_string()),
            }),
        ),
        (
            "io.eventFilter",
            Value::Builtin(BuiltinFn {
                name: "io.eventFilter",
                arity: 2,
                func: |_args| Err("io.eventFilter is evaluator-owned".to_string()),
            }),
        ),
        (
            "io.watchFile",
            Value::Builtin(BuiltinFn {
                name: "io.watchFile",
                arity: 1,
                func: |args| match &args[0] {
                    Value::String(path) => {
                        let path = std::path::PathBuf::from(path.as_str());
                        Ok(Value::Event(Rc::new(EventValue {
                            kind: EventKind::FileWatch { path },
                        })))
                    }
                    Value::Path(path) => Ok(Value::Event(Rc::new(EventValue {
                        kind: EventKind::FileWatch {
                            path: std::path::PathBuf::from(path.as_ref()),
                        },
                    }))),
                    _ => Err("io.watchFile expects a String or Path".to_string()),
                },
            }),
        ),
        (
            "io.eventNext",
            Value::Builtin(BuiltinFn {
                name: "io.eventNext",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Event(event) => super::poll_event(event),
                    _ => Err("io.eventNext expects an Event".to_string()),
                },
            }),
        ),
        (
            "io.every",
            Value::Builtin(BuiltinFn {
                name: "io.every",
                arity: 1,
                func: |args| match &args[0] {
                    Value::Int(ms) => {
                        let ms: u64 = ms
                            .try_into()
                            .map_err(|_| "io.every: interval must be non-negative".to_string())?;
                        Ok(Value::Event(Rc::new(EventValue {
                            kind: EventKind::Timer { interval_ms: ms },
                        })))
                    }
                    _ => Err("io.every expects an Int (milliseconds)".to_string()),
                },
            }),
        ),
    ]
}
