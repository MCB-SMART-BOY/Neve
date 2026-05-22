; Structural text objects for Neve

; ---- Functions ----
(fn_def) @function.outer
(fn_def (expr) @function.inner)

; ---- Classes (structs and enums) ----
(struct_def) @class.outer
(enum_def) @class.outer

; ---- Blocks ----
(block) @block.outer

; ---- Conditionals (match arms) ----
(match_arm) @conditional.outer
(match_arm (expr) @conditional.inner)

; ---- Comments ----
(comment) @comment.outer
(comment) @comment.inner

; ---- Parameters ----
(param) @parameter.outer
(param (pattern) @parameter.inner)

; ---- Calls ----
(call) @call.outer
(call (expr) @call.inner)
