; Auto-indentation rules for Neve v3.0

[
  "{"
] @indent

(if_expr "then" @indent)

"}" @outdent

(if_expr "else" @outdent)
