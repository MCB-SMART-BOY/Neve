; Auto-indentation rules for Neve

; Indent after opening delimiters (blocks, structs, enums, traits, impls, match, records)
[
  "{"
  "#{"
] @indent

; Indent after 'then' in if expressions
(if_expr "then" @indent)

; Outdent on closing delimiter
"}" @outdent

; Outdent on 'else' in if expressions
(if_expr "else" @outdent)
