; Auto-indentation rules for n3v3 v3.0

[
  "{"
] @indent

(if_expr "then" @indent)

"}" @outdent

(if_expr "else" @outdent)
