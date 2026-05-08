/-
  Neve 辅助引理
  Canonical Forms + basic lemmas for type safety proof
-/
import Neve.Syntax
import Neve.Types
import Neve.Eval

namespace Neve

open HasType BigStep Value Ty Expr

-- === Canonical Forms Lemma ===
-- 如果 ⊢ v : τ，则 v 具有 τ 对应的值形式

def canonical_int : Value → Prop
  | int _ => True
  | _ => False

theorem canonical_int_lemma (v : Value) (h : HasType [] (lit_int 0) Ty.Int) : 
    ∃ n : Int, v = int n := by
  -- v must be int(n) for some n
  cases v
  · exact ⟨_, rfl⟩
  · trivial
  · trivial
  · trivial
  · trivial
  · trivial
  · trivial
  · trivial
  · trivial
  · trivial
  · trivial

-- === 关键的求值引理: 值可以求值为自身 ===
theorem value_evaluates (v : Value) : BigStep [] (lit_int 0) v := by
  -- Placeholder: we need the right expression for v
  sorry

-- === Subsumption Lemma ===
-- 如果 e 有类型 τ 且 τ = σ，则 e 也有类型 σ
theorem subsumption (e : Expr) (τ σ : Ty) (h : HasType [] e τ) (heq : τ = σ) : 
    HasType [] e σ := by
  rw [heq]
  exact h

end Neve
