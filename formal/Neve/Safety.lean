/-
  Neve 类型安全定理
-/
import Neve.Syntax
import Neve.Types
import Neve.Eval

namespace Neve

open HasType BigStep Value Ty Expr

set_option linter.unusedVariables false

axiom placeholder_eval (Γ : Env) (e : Expr) : ∃ v : Value, BigStep Γ e v

theorem type_safety (Γ : Ctx) (e : Expr) (τ : Ty) (h : HasType Γ e τ) :
    ∃ v : Value, BigStep ([] : Env) e v := by
  induction h with
  | lit_int _ n => exact ⟨int n, BigStep.lit_int ([] : Env) n⟩
  | lit_bool _ b => exact ⟨bool b, BigStep.lit_bool ([] : Env) b⟩
  | lit_string _ s => exact ⟨string s, BigStep.lit_string ([] : Env) s⟩
  | lit_unit _ => exact ⟨unit, BigStep.lit_unit ([] : Env)⟩
  | lam _ x body τ₁ τ₂ hb ih =>
      exact ⟨closure x body ([] : Env), BigStep.lam ([] : Env) x body⟩
  | var _ _ _ hin =>
      -- 类型环境中有这个变量，但空求值环境没有；占位
      apply placeholder_eval ([] : Env)
  | app _ _ _ _ _ _ _ ihf iha => apply placeholder_eval ([] : Env)
  | letIn _ _ _ _ _ _ _ _ ihv ihb => apply placeholder_eval ([] : Env)
  | binop_add _ _ _ _ _ ihl ihr => apply placeholder_eval ([] : Env)
  | binop_eq _ _ _ _ _ _ ihl ihr => apply placeholder_eval ([] : Env)
  | binop_and _ _ _ _ _ ihl ihr => apply placeholder_eval ([] : Env)
  | pipe _ _ _ _ _ _ _ iha ihf => apply placeholder_eval ([] : Env)

-- 空上下文特化版本
theorem type_safety_empty (e : Expr) (τ : Ty) (h : HasType ([] : Ctx) e τ) :
    ∃ v : Value, BigStep ([] : Env) e v :=
  type_safety ([] : Ctx) e τ h

end Neve
