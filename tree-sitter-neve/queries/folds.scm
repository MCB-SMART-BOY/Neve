; Code folding rules for Neve
; Defines which nodes can be folded (collapsed) in the editor

; Fold all block-like constructs
(block) @fold
(struct_def) @fold
(enum_def) @fold
(trait_def) @fold
(impl_def) @fold
(match_expr) @fold

; Fold function bodies
(fn_def) @fold

; Fold record literals (if they span multiple lines)
(record) @fold

; Fold list literals (if they span multiple lines)
(list) @fold
