/-
  Environment infrastructure for type safety proofs (v4).
  
  EnvMatches is parameterized by P : Value → Ty → Prop
  and defined in Proofs/Values.lean before ValueTyping.
  
  This file provides:
    - env_matches_lookup: variable lookup preserves typing
-/
import Neve.Spec.Syntax
import Neve.Spec.Typing
import Neve.Spec.Eval
import Neve.Proofs.Values

namespace Neve

open Ty Expr Value

set_option linter.unusedVariables false

-- ============================================================
-- env_matches_lookup
-- ============================================================

/--
  env_matches_lookup: if EnvMatches ValueTyping Γ env and (x, τ) ∈ Γ,
  then there exists v with (x, v) ∈ env and ⊢ v : τ.
-/
theorem env_matches_lookup (Γ : Ctx) (env : Env) (x : String) (τ : Ty)
    (h : EnvMatches ValueTyping Γ env) (hxin : (x, τ) ∈ Γ) :
    ∃ v : Value, (x, v) ∈ env ∧ ValueTyping v τ := by
  induction h with
  | nil => 
      simp at hxin
  | cons Γ' env' y v σ hrest hvty ih =>
      match hxin with
      | List.Mem.head _ =>
          exact ⟨v, by simp, hvty⟩
      | List.Mem.tail _ hm =>
          rcases ih hm with ⟨v', hv', hvty'⟩
          exact ⟨v', by
            apply List.mem_cons_of_mem
            exact hv', hvty'⟩

end Neve
