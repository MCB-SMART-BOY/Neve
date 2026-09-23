//! Shared builtin type IDs and constructors.
//! 共享的内置类型 ID 与构造辅助函数。

use neve_common::Span;
pub use neve_hir::{
    BUILTIN_BYTES_TYPE_ID as BYTES_TYPE_ID, BUILTIN_COMMAND_TYPE_ID as COMMAND_TYPE_ID,
    BUILTIN_EVENT_TYPE_ID as EVENT_TYPE_ID, BUILTIN_LIST_TYPE_ID as LIST_TYPE_ID,
    BUILTIN_LIVE_TYPE_ID as LIVE_TYPE_ID, BUILTIN_MAP_TYPE_ID as MAP_TYPE_ID,
    BUILTIN_OPTION_TYPE_ID as OPTION_TYPE_ID, BUILTIN_PATH_TYPE_ID as PATH_TYPE_ID,
    BUILTIN_PIPELINE_TYPE_ID as PIPELINE_TYPE_ID,
    BUILTIN_PROCESS_RESULT_TYPE_ID as PROCESS_RESULT_TYPE_ID,
    BUILTIN_REDIRECT_TYPE_ID as REDIRECT_TYPE_ID, BUILTIN_RESULT_TYPE_ID as RESULT_TYPE_ID,
    BUILTIN_SET_TYPE_ID as SET_TYPE_ID, BUILTIN_STREAM_TYPE_ID as STREAM_TYPE_ID,
    BUILTIN_TASK_TYPE_ID as TASK_TYPE_ID,
};
use neve_hir::{DefId, Ty, TyKind};

pub fn builtin_list(elem: Ty, span: Span) -> Ty {
    Ty {
        kind: TyKind::Named(LIST_TYPE_ID, vec![elem]),
        span,
    }
}

pub fn builtin_option(elem: Ty, span: Span) -> Ty {
    Ty {
        kind: TyKind::Named(OPTION_TYPE_ID, vec![elem]),
        span,
    }
}

pub fn builtin_result(ok: Ty, err: Ty, span: Span) -> Ty {
    Ty {
        kind: TyKind::Named(RESULT_TYPE_ID, vec![ok, err]),
        span,
    }
}

pub fn builtin_map(key: Ty, value: Ty, span: Span) -> Ty {
    Ty {
        kind: TyKind::Named(MAP_TYPE_ID, vec![key, value]),
        span,
    }
}

pub fn builtin_set(elem: Ty, span: Span) -> Ty {
    Ty {
        kind: TyKind::Named(SET_TYPE_ID, vec![elem]),
        span,
    }
}

pub fn builtin_path(span: Span) -> Ty {
    Ty {
        kind: TyKind::Named(PATH_TYPE_ID, vec![]),
        span,
    }
}

pub fn builtin_bytes(span: Span) -> Ty {
    Ty {
        kind: TyKind::Named(BYTES_TYPE_ID, vec![]),
        span,
    }
}

pub fn builtin_command(span: Span) -> Ty {
    Ty {
        kind: TyKind::Named(COMMAND_TYPE_ID, vec![]),
        span,
    }
}

pub fn builtin_process_result(span: Span) -> Ty {
    Ty {
        kind: TyKind::Named(PROCESS_RESULT_TYPE_ID, vec![]),
        span,
    }
}

pub fn builtin_pipeline(span: Span) -> Ty {
    Ty {
        kind: TyKind::Named(PIPELINE_TYPE_ID, vec![]),
        span,
    }
}

pub fn builtin_redirect(span: Span) -> Ty {
    Ty {
        kind: TyKind::Named(REDIRECT_TYPE_ID, vec![]),
        span,
    }
}

pub fn builtin_task(output: Ty, span: Span) -> Ty {
    Ty {
        kind: TyKind::Named(TASK_TYPE_ID, vec![output]),
        span,
    }
}

pub fn builtin_event(inner: Ty, span: Span) -> Ty {
    Ty {
        kind: TyKind::Named(EVENT_TYPE_ID, vec![inner]),
        span,
    }
}

pub fn builtin_live(inner: Ty, span: Span) -> Ty {
    Ty {
        kind: TyKind::Named(LIVE_TYPE_ID, vec![inner]),
        span,
    }
}

pub fn builtin_stream(elem: Ty, span: Span) -> Ty {
    Ty {
        kind: TyKind::Named(STREAM_TYPE_ID, vec![elem]),
        span,
    }
}

pub fn is_builtin_option_type(def_id: DefId) -> bool {
    def_id == OPTION_TYPE_ID
}

pub fn is_builtin_result_type(def_id: DefId) -> bool {
    def_id == RESULT_TYPE_ID
}

/// Check if a DefId refers to the built-in Command type.
pub fn is_command_type(def_id: DefId) -> bool {
    def_id == COMMAND_TYPE_ID
}

pub fn builtin_type_name(def_id: DefId) -> Option<&'static str> {
    match def_id {
        LIST_TYPE_ID => Some("List"),
        OPTION_TYPE_ID => Some("Option"),
        RESULT_TYPE_ID => Some("Result"),
        MAP_TYPE_ID => Some("Map"),
        SET_TYPE_ID => Some("Set"),
        PATH_TYPE_ID => Some("Path"),
        BYTES_TYPE_ID => Some("Bytes"),
        COMMAND_TYPE_ID => Some("Command"),
        PROCESS_RESULT_TYPE_ID => Some("ProcessResult"),
        PIPELINE_TYPE_ID => Some("Pipeline"),
        REDIRECT_TYPE_ID => Some("Redirect"),
        TASK_TYPE_ID => Some("Task"),
        EVENT_TYPE_ID => Some("Event"),
        LIVE_TYPE_ID => Some("Live"),
        STREAM_TYPE_ID => Some("Stream"),
        _ => None,
    }
}

pub fn format_builtin_named_type(
    def_id: DefId,
    args: &[Ty],
    render: &impl Fn(&Ty) -> String,
) -> Option<String> {
    let name = builtin_type_name(def_id)?;
    if args.is_empty() {
        Some(name.to_string())
    } else {
        let rendered_args: Vec<_> = args.iter().map(render).collect();
        Some(format!("{name}[{}]", rendered_args.join(", ")))
    }
}
