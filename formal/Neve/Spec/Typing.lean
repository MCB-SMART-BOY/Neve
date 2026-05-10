/-
  Neve 类型系统形式化 v4 — PatHasType 修复

  PatHasType 现在将原始上下文 Γ 作为 parameter：
    PatHasType Γ p τ Γ'  = "在上下文 Γ 中，模式 p 有类型 τ，扩展后上下文为 Γ'"

  关键变化：
  - wildcard: Γ' = Γ（不扩展）
  - var x τ:  Γ' = (x, τ) :: Γ（扩展一个变量）
  - lit_*:    Γ' = Γ（不扩展）

  这使得 AllArmsMatch.cons 中的 Γ' 被正确约束。
-/
import Neve.Spec.Syntax

namespace Neve

open Ty Expr Pattern Value Effect BinOp

-- === 类型环境 ===
abbrev Ctx : Type := List (String × Ty)

-- === 模式类型 / Pattern Typing ===
-- Γ is a PARAMETER (original context), the last Ctx is the INDEX (extended context)
inductive PatHasType (Γ : Ctx) : Pattern → Ty → Ctx → Prop where
  | wildcard (τ : Ty) : PatHasType Γ Pattern.wildcard τ Γ
  | var (x : String) (τ : Ty) : PatHasType Γ (Pattern.var x) τ ((x, τ) :: Γ)
  | lit_int (n : Int) : PatHasType Γ (Pattern.lit_int n) Ty.Int Γ
  | lit_bool (b : Bool) : PatHasType Γ (Pattern.lit_bool b) Ty.Bool Γ
  | lit_string (s : String) : PatHasType Γ (Pattern.lit_string s) Ty.String Γ
  | list_nil (τ : Ty) : PatHasType Γ (Pattern.list [] false) (Ty.List τ) Γ
  | list_cons (p ps : List Pattern) (head : Pattern) (τ : Ty) :
      PatHasType Γ head τ Γ → PatHasType Γ (Pattern.list ps false) (Ty.List τ) Γ →
      PatHasType Γ (Pattern.list (head :: ps) false) (Ty.List τ) Γ

-- === 类型检查 Γ ⊢ e : τ 与 AllArmsMatch（mutual 以解决前向引用）===
mutual
  inductive HasType : Ctx → Expr → Ty → Prop where
    | lit_int (Γ : Ctx) (n : Int) : HasType Γ (lit_int n) Ty.Int
    | lit_bool (Γ : Ctx) (b : Bool) : HasType Γ (lit_bool b) Ty.Bool
    | lit_string (Γ : Ctx) (s : String) : HasType Γ (lit_string s) Ty.String
    | lit_unit (Γ : Ctx) : HasType Γ lit_unit Ty.Unit

    | var (Γ : Ctx) (x : String) (τ : Ty) (hin : (x, τ) ∈ Γ) : HasType Γ (var x) τ

    -- λ with effect annotation
    | lam (Γ : Ctx) (x : String) (body : Expr) (τ₁ τ₂ : Ty) (eff : Effect) :
        HasType ((x, τ₁) :: Γ) body τ₂ →
        HasType Γ (lam x body) (Ty.Fn τ₁ τ₂ eff)

    -- application: function must be Fn τ₁ τ₂ eff
    | app (Γ : Ctx) (f arg : Expr) (τ₁ τ₂ : Ty) (eff : Effect) :
        HasType Γ f (Ty.Fn τ₁ τ₂ eff) → HasType Γ arg τ₁ →
        HasType Γ (app f arg) τ₂

    | letIn (Γ : Ctx) (x : String) (val body : Expr) (τ τ' : Ty) :
        HasType Γ val τ → HasType ((x, τ) :: Γ) body τ' →
        HasType Γ (letIn x val body) τ'

    -- match: scrutinee has type τs, all arms return type τ
    -- PatHasType now takes Γ (the outer context) as a parameter
    | matchOn (Γ : Ctx) (scrutinee : Expr) (arms : List (Pattern × Expr)) (τs τ : Ty) :
        HasType Γ scrutinee τs → AllArmsMatch Γ arms τs τ →
        HasType Γ (matchOn scrutinee arms) τ

    | binop_add (Γ : Ctx) (l r : Expr) :
        HasType Γ l Ty.Int → HasType Γ r Ty.Int →
        HasType Γ (binop BinOp.Add l r) Ty.Int

    | binop_sub (Γ : Ctx) (l r : Expr) :
        HasType Γ l Ty.Int → HasType Γ r Ty.Int →
        HasType Γ (binop BinOp.Sub l r) Ty.Int

    | binop_mul (Γ : Ctx) (l r : Expr) :
        HasType Γ l Ty.Int → HasType Γ r Ty.Int →
        HasType Γ (binop BinOp.Mul l r) Ty.Int

    | binop_div (Γ : Ctx) (l r : Expr) :
        HasType Γ l Ty.Int → HasType Γ r Ty.Int →
        HasType Γ (binop BinOp.Div l r) Ty.Int

    | binop_mod (Γ : Ctx) (l r : Expr) :
        HasType Γ l Ty.Int → HasType Γ r Ty.Int →
        HasType Γ (binop BinOp.Mod l r) Ty.Int

    | binop_eq (Γ : Ctx) (l r : Expr) (τ : Ty) :
        HasType Γ l τ → HasType Γ r τ →
        HasType Γ (binop BinOp.Eq l r) Ty.Bool

    | binop_and (Γ : Ctx) (l r : Expr) :
        HasType Γ l Ty.Bool → HasType Γ r Ty.Bool →
        HasType Γ (binop BinOp.And l r) Ty.Bool

    | binop_or (Γ : Ctx) (l r : Expr) :
        HasType Γ l Ty.Bool → HasType Γ r Ty.Bool →
        HasType Γ (binop BinOp.Or l r) Ty.Bool

    | pipe (Γ : Ctx) (arg f : Expr) (τ τ' : Ty) (eff : Effect) :
        HasType Γ arg τ → HasType Γ f (Ty.Fn τ τ' eff) →
        HasType Γ (binop BinOp.Pipe arg f) τ'

  -- === 所有 match arm 返回同一类型 ===
  inductive AllArmsMatch : Ctx → List (Pattern × Expr) → Ty → Ty → Prop where
    | nil (Γ : Ctx) (τs τ : Ty) : AllArmsMatch Γ [] τs τ
    | cons (Γ : Ctx) (p : Pattern) (e : Expr) (arms : List (Pattern × Expr)) (τs τ : Ty) (Γ' : Ctx) :
        PatHasType Γ p τs Γ' → HasType Γ' e τ → AllArmsMatch Γ arms τs τ →
        AllArmsMatch Γ ((p, e) :: arms) τs τ
end

def WellTyped (e : Expr) (τ : Ty) : Prop := HasType ([] : Ctx) e τ

end Neve
