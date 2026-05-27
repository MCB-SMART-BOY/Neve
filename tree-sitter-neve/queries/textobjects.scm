; Structural text objects for Neve v3.0

(fn_def) @function.outer
(fn_def (expr) @function.inner)

(type_decl) @class.outer

(block) @block.outer

(match_arm) @conditional.outer
(match_arm (expr) @conditional.inner)

(comment) @comment.outer
(comment) @comment.inner

(param) @parameter.outer
(param (pattern) @parameter.inner)

(call) @call.outer
(call (expr) @call.inner)
