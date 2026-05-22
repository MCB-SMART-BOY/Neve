; Local variable scoping and definitions for Neve

; ---- Scopes ----
(source_file) @scope
(block) @scope
(fn_def) @scope
(struct_def) @scope
(enum_def) @scope
(trait_def) @scope
(impl_def) @scope
(lambda) @scope
(match_expr) @scope

; ---- Definitions ----
; Function definitions
(fn_def name: (ident) @definition.function)

; Variable definitions (let bindings)
(let_stmt (pattern (ident) @definition.var))

; Parameters
(param (pattern (ident) @definition.parameter))

; Type definitions
(struct_def name: (ident) @definition.type)
(enum_def name: (ident) @definition.type)
(trait_def name: (ident) @definition.type)
(type_alias name: (ident) @definition.type)
(variant (ident) @definition.type)

; Field definitions
(field_def (ident) @definition.field)

; Import aliases
(import_stmt "as" (ident) @definition.import)

; Generic type parameters
(generics (ident) @definition.type)

; Binding patterns (e.g., `name @ pattern`)
(binding_pattern (ident) @definition.var)

; ---- References ----
; All other identifier uses
(ident) @reference
