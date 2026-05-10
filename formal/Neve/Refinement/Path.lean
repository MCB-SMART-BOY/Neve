/-
  Path Resolution Refinement (M-1)

  Connects the Rust implementation of resolve_redirect_path
  (crates/neve-std/src/io/mod.rs) to its Lean specification
  (Verify/Path.lean).

  The refinement approach:
    1. Model the Rust function as an identical algorithm in Lean
    2. Prove that this model is equivalent to the Lean spec
    3. Conclude: the Rust implementation (when correct) satisfies the spec

  Security guarantee: Any Rust redirect path that contains `..`
  is replaced by the safe sentinel, preventing directory traversal.
-/
import Neve.Spec.Syntax
import Neve.Verify.Path
import Neve.Refinement.Types

namespace Neve.Refinement

open Neve (Path resolve_redirect_path safe_sentinel path_safe)

set_option linter.unusedVariables false

-- ============================================================
-- Path refinement relation
-- ============================================================

/--
  PathRefines: a Rust PathBuf refines a Lean Path.
  
  Since we cannot directly reference Rust's PathBuf from Lean,
  we model the refinement as component-list equality.
-/
def PathRefines (rust_path : List String) (lean_path : Path) : Prop :=
  rust_path = lean_path

/--
  Safe refinement: path equality + path_safe guarantee.
-/
def PathSafeRefines (rust_path : List String) (lean_path : Path) : Prop :=
  PathRefines rust_path lean_path ∧ path_safe lean_path

-- ============================================================
-- Rust model: identical algorithm in Lean
-- ============================================================

/--
  path_rust_resolve_redirect_path: exact transcription of the Rust implementation.
  
  Rust code (crates/neve-std/src/io/mod.rs):
    fn resolve_redirect_path(command: &Command, redirect: &Path) -> PathBuf {
        if redirect.components().any(|c| c == Component::ParentDir) {
            PathBuf::from("/dev/null/neve-blocked-traversal")
        } else if let Some(cwd) = command.cwd {
            cwd.join(redirect)
        } else {
            redirect.to_path_buf()
        }
    }
-/
def path_rust_resolve_redirect_path (cwd : Option (List String)) (redirect : List String) : List String :=
  if List.any redirect (λ c => c = "..") then
    ["dev", "null", "neve-blocked-traversal"]
  else match cwd with
    | none     => redirect
    | some cwd => cwd ++ redirect

-- ============================================================
-- Equivalence: Rust model = Lean spec
-- ============================================================

/--
  The Rust model is exactly equivalent to the Lean specification.
  
  This is a structural equivalence: both implement the same algorithm.
  The Lean spec (Verify/Path.lean) has:
    resolve_redirect_path cwd redirect =
      if has_parent_dir redirect then safe_sentinel
      else match cwd with | none => redirect | some cwd => cwd ++ redirect
  
  The Rust model has:
    path_rust_resolve_redirect_path cwd redirect =
      if List.any redirect (λ c => c = "..") then ["dev", "null", "neve-blocked-traversal"]
      else match cwd with | none => redirect | some cwd => cwd ++ redirect
  
  Since has_parent_dir redirect = List.any redirect (λ c => c = "..")
  and safe_sentinel = ["dev", "null", "neve-blocked-traversal"],
  these are syntactically identical.
-/
theorem path_rust_model_equals_lean_spec (cwd : Option (List String)) (redirect : List String) :
    path_rust_resolve_redirect_path cwd redirect =
    resolve_redirect_path cwd redirect := by
  unfold path_rust_resolve_redirect_path resolve_redirect_path has_parent_dir safe_sentinel
  rfl

-- ============================================================
-- Main refinement theorem
-- ============================================================

/--
  For any cwd (optional) and redirect path, the Rust model produces
  a result that refines the Lean specification AND is path_safe.
  
  This is the key security theorem:
    1. The Rust model = the Lean spec (structural equivalence)
    2. The Lean spec produces path_safe output (Verify/Path.lean)
    3. Therefore, the Rust model produces path_safe output
  
  For the actual Rust implementation to be verified, one must prove
  that it faithfully implements path_rust_resolve_redirect_path.
  This is an external proof obligation (e.g., via coq-of-rust or Aeneas).
-/
theorem path_rust_model_is_safe (cwd : Option (List String)) (redirect : List String)
    (hcwd_safe : ∀ p ∈ cwd, path_safe p) :
    PathSafeRefines (path_rust_resolve_redirect_path cwd redirect)
                    (resolve_redirect_path cwd redirect) := by
  -- Structural equivalence
  have heq : path_rust_resolve_redirect_path cwd redirect =
             resolve_redirect_path cwd redirect :=
    path_rust_model_equals_lean_spec cwd redirect
  -- Lean spec safety (from Verify/Path.lean)
  have hsafe : path_safe (resolve_redirect_path cwd redirect) :=
    path_safety_with_safe_cwd cwd redirect hcwd_safe
  -- Combine
  unfold PathSafeRefines PathRefines
  exact And.intro heq hsafe

/--
  Summary of the M-1 path safety refinement:
  
  Rust implementation → Rust model (manual transcription)
  Rust model = Lean spec (machine-checked above)
  Lean spec is safe (machine-checked in Verify/Path.lean)
  ─────────────────────────────────────────────
  ∴ Rust model is safe (machine-checked above)
  ∴ Rust implementation is safe (modulo transcription correctness)
  
  The remaining proof obligation (transcription correctness) requires
  external tooling. The Lean side is fully machine-checked.
-/
theorem path_refinement_summary : True := trivial

end Neve.Refinement
