; Highlights for Neve — optimized for Helix themes
; See https://docs.helix-editor.com/themes.html for scope reference

; ---- Keywords ----
; Control flow
[
  "if"
  "then"
  "else"
  "match"
] @keyword.control

; Storage / declarations
[
  "let"
  "fn"
  "type"
  "struct"
  "enum"
  "trait"
  "impl"
] @keyword.storage

; Imports
[
  "import"
  "as"
] @keyword.control.import

; Modifiers
[
  "pub"
  "effect"
  "lazy"
] @keyword.directive

; ---- Literals ----
; Boolean constants
[
  "true"
  "false"
] @constant.builtin

; Numbers
(number) @number

; Strings
(string) @string
(char) @string

; Interpolated strings (special highlight)
(interpolated) @string.special

; Path literals (like filesystem paths)
(path_literal) @string.special.path

; ---- Functions ----
; Function definitions
(fn_def name: (ident) @function)

; Method calls (e.g., x.foo())
(method_call method: (ident) @function.method)

; Function calls — highlight the callee name
(call (path) @function.call)

; ---- Types ----
; Type definitions
(struct_def name: (ident) @type)
(enum_def name: (ident) @type)
(trait_def name: (ident) @type)
(type_alias name: (ident) @type)

; Enum variants (constructors)
(variant (ident) @constructor)

; ---- Variables & Parameters ----
; Parameters in function definitions
(param (pattern (ident) @parameter))

; Binding patterns (name @ pattern)
(binding_pattern (ident) @variable)

; ---- Properties & Fields ----
; Field access (x.field)
(field_access field: (ident) @property)

; Record fields in definitions
(field_def (ident) @property)

; ---- Identifiers ----
; Catch-all for other identifiers (will be overridden by more specific rules above)
(ident) @variable

; Module paths — first segment is namespace
(path (ident) @namespace)

; ---- Operators ----
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

; ---- Delimiters ----
[
  "(" ")"
  "[" "]"
  "{"
  "#{"
] @punctuation.bracket

; Closing brace (separate from opening for matching)
"}" @punctuation.bracket

; Punctuation
[
  "." "," ":" ";" "@"
] @punctuation.delimiter

; ---- Comments ----
(comment) @comment
