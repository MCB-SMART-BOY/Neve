/-
  Environment infrastructure for type safety proofs.
  - EnvMatches: runtime environment matches typing context
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
-- Environment matching
-- ============================================================

/--
  EnvMatches Γ env: for each (x, τ) ∈ Γ, there exists (x, v) ∈ env with ⊢ v : τ.
  This connects the typing context to the runtime environment.
-/
inductive EnvMatches : Ctx → Env → Prop where
  | nil : EnvMatches [] []
  | cons (Γ : Ctx) (env : Env) (x : String) (v : Value) (τ : Ty) :
      EnvMatches Γ env → ValueTyping v τ →
      EnvMatches ((x, τ) :: Γ) ((x, v) :: env)

/--
  env_matches_lookup: if EnvMatches Γ env and (x, τ) ∈ Γ,
  then there exists v with (x, v) ∈ env and ⊢ v : τ.
-/
theorem env_matches_lookup (Γ : Ctx) (env : Env) (x : String) (τ : Ty)
    (h : EnvMatches Γ env) (hxin : (x, τ) ∈ Γ) :
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
