; Highlights for Neve

; Keywords
[
  "let"
  "fn"
  "import"
  "as"
  "type"
  "struct"
  "enum"
  "trait"
  "impl"
  "pub"
  "if"
  "then"
  "else"
  "match"
  "lazy"
  "effect"
] @keyword

; Boolean literals
[
  "true"
  "false"
] @boolean

; Identifiers
(ident) @variable

; Function definitions
(fn_def name: (ident) @function)

; Type definitions
(struct_def name: (ident) @type)
(enum_def name: (ident) @type)
(trait_def name: (ident) @type)
(type_alias name: (ident) @type)

; Parameters
(param (pattern (ident) @parameter))

; Method calls
(method_call method: (ident) @method)

; Field access
(field_access field: (ident) @property)

; Literals
(number) @number
(string) @string
(char) @string
(interpolated) @string
(path_literal) @string

; Operators
[
  "+" "-" "*" "/" "%" "^"
  "==" "!=" "<" ">" "<=" ">="
  "&&" "||"
  "++" "//"
  "|>"
  "->" "=>"
  "??" "?."
  "="
  "!"
] @operator

; Delimiters
[
  "(" ")"
  "[" "]"
  "{" "}"
  "#{"
] @punctuation.delimiter

; Punctuation
[
  "." "," ":" ";" "@"
] @punctuation

; Comments
(comment) @comment
