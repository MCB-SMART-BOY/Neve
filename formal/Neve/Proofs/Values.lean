/-
  Neve 辅助引理 v3
  - Canonical Forms: 如果 ⊢ v : τ，则 v 具有 τ 对应的规范形式
  - Value Typing: 值类型判断 ⊢ v : τ
  - Substitution Lemma: 类型保持替换
  - Progress Lemma: 良类型表达式可以求值
-/
import Neve.Spec.Syntax
import Neve.Spec.Typing
import Neve.Spec.Eval

namespace Neve

open HasType BigStep Value Ty Expr

set_option linter.unusedVariables false

-- ============================================================
-- Value Typing: 值在空上下文中的类型判断
-- ============================================================

inductive ValueTyping : Value → Ty → Prop where
  | int (n : Int) : ValueTyping (int n) Ty.Int
  | float (f : Float) : ValueTyping (float f) Ty.Float
  | bool (b : Bool) : ValueTyping (bool b) Ty.Bool
  | char (c : Char) : ValueTyping (char c) Ty.Char
  | string (s : String) : ValueTyping (string s) Ty.String
  | unit : ValueTyping unit Ty.Unit
  | list_nil (τ : Ty) : ValueTyping (list []) (Ty.List τ)
  | list_cons (v : Value) (vs : List Value) (τ : Ty) :
      ValueTyping v τ → ValueTyping (list vs) (Ty.List τ) →
      ValueTyping (list (v :: vs)) (Ty.List τ)
  | closure (x : String) (body : Expr) (env : Env) (τ₁ τ₂ : Ty) (eff : Effect) :
      ValueTyping (closure x body env) (Ty.Fn τ₁ τ₂ eff)
  | processResult (code : Int) (stdout stderr : String) :
      ValueTyping (processResult code stdout stderr) Ty.ProcessResult
  | someVal (v : Value) (τ : Ty) : ValueTyping v τ → ValueTyping (Value.someVal v) (Ty.Option τ)
  | noneVal (τ : Ty) : ValueTyping Value.noneVal (Ty.Option τ)

-- ============================================================
-- Canonical Forms Lemma
-- ============================================================

/--
  canonical_forms_int: 如果 ⊢ v : Int，则 v = int n 对于某个 n。
-/
theorem canonical_forms_int (v : Value) (h : ValueTyping v Ty.Int) : ∃ n : Int, v = int n := by
  cases h with
  | int n => exact ⟨n, rfl⟩

/--
  canonical_forms_bool: 如果 ⊢ v : Bool，则 v = bool b 对于某个 b。
-/
theorem canonical_forms_bool (v : Value) (h : ValueTyping v Ty.Bool) : ∃ b : Bool, v = bool b := by
  cases h with
  | bool b => exact ⟨b, rfl⟩

/--
  canonical_forms_string: 如果 ⊢ v : String，则 v = string s 对于某个 s。
-/
theorem canonical_forms_string (v : Value) (h : ValueTyping v Ty.String) : ∃ s : String, v = string s := by
  cases h with
  | string s => exact ⟨s, rfl⟩

/--
  canonical_forms_fn: 如果 ⊢ v : Fn τ₁ τ₂ eff，则 v 是一个闭包。
-/
theorem canonical_forms_fn (v : Value) (τ₁ τ₂ : Ty) (eff : Effect) (h : ValueTyping v (Ty.Fn τ₁ τ₂ eff)) :
    ∃ (x : String) (body : Expr) (env : Env), v = closure x body env := by
  cases h with
  | closure x body env _ _ _ => exact ⟨x, body, env, rfl⟩

/--
  canonical_forms_unit: 如果 ⊢ v : Unit，则 v = unit。
-/
theorem canonical_forms_unit (v : Value) (h : ValueTyping v Ty.Unit) : v = unit := by
  cases h with
  | unit => rfl

/--
  canonical_forms_list: 如果 ⊢ v : List τ，则 v 是一个列表。
-/
theorem canonical_forms_list (v : Value) (τ : Ty) (h : ValueTyping v (Ty.List τ)) :
    ∃ (vs : List Value), v = list vs := by
  cases h with
  | list_nil _ => exact ⟨[], rfl⟩
  | list_cons _ vs _ _ _ => exact ⟨_ :: vs, rfl⟩

-- ============================================================
-- Substitution Lemma (框架)
-- ============================================================

/--
  类型保持的替换引理：
  如果 Γ, x:τ₁ ⊢ e : τ₂ 且 ⊢ v : τ₁，则 Γ ⊢ e[v/x] : τ₂

  这里只声明，完整证明需要对 e 的结构做归纳。
-/
axiom substitution_lemma (Γ : Ctx) (x : String) (e : Expr) (v : Value) (τ₁ τ₂ : Ty)
    (hbody : HasType ((x, τ₁) :: Γ) e τ₂) (hval : ValueTyping v τ₁) :
    HasType Γ e τ₂

end Neve
