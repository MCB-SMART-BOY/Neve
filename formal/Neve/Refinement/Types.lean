/-
  Rust-Lean Refinement Bridge — Type Relations

  Defines the refinement relation ⊑ between Rust runtime values and their
  Lean formal counterparts. This is the foundation for proving that Rust
  implementations correctly refine their Lean specifications.

  Refinement Judgment:
    rust_val ⊑ lean_val : τ
    "rust_val refines lean_val at type τ"

  The refinement is indexed by the Rust value type, allowing heterogeneous
  relations: an Int refines Value.int, a Bool refines Value.bool, etc.

  Organization:
    - This file: refinement relations for base types
    - Refinement/Path.lean: path resolution refinement (M-1 fix)
    - Refinement/Environ.lean: env stripping refinement (M-4 fix)
    - Refinement/Limits.lean: buffer limits refinement (H-1, H-2 fixes)
-/
import Neve.Spec.Syntax
import Neve.Proofs.Values

namespace Neve.Refinement

open Ty Value

set_option linter.unusedVariables false

-- ============================================================
-- Refinement relation: rust_val ⊑ lean_val : τ
-- ============================================================

/--
  Refines {α} (r : α) (l : Value) (τ : Ty) : Prop
  
  Heterogeneous refinement relation indexed by the Rust value type α.
  This connects Rust runtime values (modeled as Lean values of type α)
  to Lean formal Value constructors at type τ.

  Example:
    Refines 42 (Value.int 42) Ty.Int       — integer refinement
    Refines true (Value.bool true) Ty.Bool — boolean refinement
    Refines "hi" (Value.string "hi") Ty.String — string refinement
-/
inductive Refines : {α : Type} → α → Value → Ty → Prop where
  | int (n : Int) : Refines n (Value.int n) Ty.Int
  | bool (b : Bool) : Refines b (Value.bool b) Ty.Bool
  | string (s : String) : Refines s (Value.string s) Ty.String
  | unit : Refines () Value.unit Ty.Unit
  | bytes (data : List Nat) :
      Refines data (Value.bytes data) Ty.Bytes
  | processResult (code : Int) (stdout stderr : String) :
      Refines (code, stdout, stderr) (Value.processResult code stdout stderr) Ty.ProcessResult

-- ============================================================
-- Refinement of functions
-- ============================================================

/--
  FunctionRefines (f_rust : α → β) (f_lean : α_lean → β_lean) : Prop

  A Rust function f_rust refines a Lean function f_lean if for all inputs
  that are related by the input refinement, the outputs are related by
  the output refinement.

  This is the fundamental judgment of the refinement bridge.
-/
def FunctionRefines {α β α_lean β_lean : Type}
    (f_rust : α → β) (f_lean : α_lean → β_lean)
    (R_in : α → α_lean → Prop) (R_out : β → β_lean → Prop) : Prop :=
  ∀ (a : α) (a_lean : α_lean), R_in a a_lean → R_out (f_rust a) (f_lean a_lean)

-- ============================================================
-- Refinement soundness
-- ============================================================

/--
  If a Rust value refines a Lean value at type τ, then the Lean value
  is well-typed (ValueTyping l τ).

  This connects the refinement bridge to the type safety proofs.
-/
theorem refinement_implies_value_typing {α : Type} (r : α) (l : Value) (τ : Ty)
    (href : Refines r l τ) : ValueTyping l τ := by
  cases href with
  | int n => exact ValueTyping.int n
  | bool b => exact ValueTyping.bool b
  | string s => exact ValueTyping.string s
  | unit => exact ValueTyping.unit
  | bytes data => exact ValueTyping.bytes data
  | processResult code stdout stderr =>
      exact ValueTyping.processResult code stdout stderr

end Neve.Refinement
