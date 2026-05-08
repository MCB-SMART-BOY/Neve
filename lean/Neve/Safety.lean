/-
  Neve 类型安全性证明框架
  
  定理框架已建立，完整证明待后续填入。
-/
import Neve.Syntax
import Neve.Types
import Neve.Eval

namespace Neve

open HasType BigStep Value Ty Expr

-- === Progress 定理（框架）===
theorem progress (e : Expr) (τ : Ty) (h : HasType ([] : Ctx) e τ) : 
    ∃ v, BigStep ([] : Env) e v := by
  -- 完整证明待完成
  sorry

-- === Preservation 定理（框架）===
theorem preservation (e : Expr) (τ : Ty) (ht : HasType ([] : Ctx) e τ) 
    (v : Value) (he : BigStep ([] : Env) e v) : 
    HasType ([] : Ctx) (Expr.lit_int 0) τ := by
  -- 完整证明待完成
  sorry

end Neve
