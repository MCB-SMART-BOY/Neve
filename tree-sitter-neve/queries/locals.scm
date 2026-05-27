; Local variable scoping and definitions for Neve v3.0

; ---- Scopes ----
(source_file) @scope
(block) @scope
(fn_def) @scope
(type_decl) @scope
(trait_def) @scope
(impl_def) @scope
(lambda) @scope
(match_expr) @scope

; ---- Definitions ----
(fn_def name: (ident) @definition.function)
(binding (pattern (ident) @definition.var))
(param (pattern (ident) @definition.parameter))

(type_decl name: (ident) @definition.type)
(trait_def name: (ident) @definition.type)
(variant (ident) @definition.type)

(field_def (ident) @definition.field)
(use_stmt "as" (ident) @definition.import)
(import_stmt "as" (ident) @definition.import)
(generics (ident) @definition.type)
(binding_pattern (ident) @definition.var)

; ---- References ----
(ident) @reference
