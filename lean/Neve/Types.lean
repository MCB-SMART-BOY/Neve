/-
  Neve 类型系统形式化
  Γ ⊢ e : τ  (inductive typing judgment)
-/
import Neve.Syntax

namespace Neve

open Ty Expr Pattern Value

-- === 类型环境 ===
abbrev Ctx : Type := List (String × Ty)

-- === 辅助关系：构造器模式匹配类型 ===
inductive PatHasType : Pattern → Ty → Ctx → Prop where
  | wildcard (τ : Ty) (Γ : Ctx) : PatHasType Pattern.wildcard τ Γ
  | var (x : String) (τ : Ty) (Γ : Ctx) : PatHasType (Pattern.var x) τ ((x, τ) :: Γ)

-- === 类型检查 Γ ⊢ e : τ ===
inductive HasType : Ctx → Expr → Ty → Prop where
  | lit_int (Γ : Ctx) (n : Int) : HasType Γ (lit_int n) Ty.Int
  | lit_bool (Γ : Ctx) (b : Bool) : HasType Γ (lit_bool b) Ty.Bool
  | lit_string (Γ : Ctx) (s : String) : HasType Γ (lit_string s) Ty.String
  | lit_unit (Γ : Ctx) : HasType Γ lit_unit Ty.Unit

  | var (Γ : Ctx) (x : String) (τ : Ty) (hin : (x, τ) ∈ Γ) : HasType Γ (var x) τ

  | lam (Γ : Ctx) (x : String) (body : Expr) (τ₁ τ₂ : Ty) :
      HasType ((x, τ₁) :: Γ) body τ₂ →
      HasType Γ (lam x body) (Ty.Fn τ₁ τ₂)

  | app (Γ : Ctx) (f arg : Expr) (τ₁ τ₂ : Ty) :
      HasType Γ f (Ty.Fn τ₁ τ₂) → HasType Γ arg τ₁ →
      HasType Γ (app f arg) τ₂

  | letIn (Γ : Ctx) (x : String) (val body : Expr) (τ τ' : Ty) :
      HasType Γ val τ → HasType ((x, τ) :: Γ) body τ' →
      HasType Γ (letIn x val body) τ'

  | binop_add (Γ : Ctx) (l r : Expr) :
      HasType Γ l Ty.Int → HasType Γ r Ty.Int →
      HasType Γ (binop BinOp.Add l r) Ty.Int

  | binop_eq (Γ : Ctx) (l r : Expr) (τ : Ty) :
      HasType Γ l τ → HasType Γ r τ →
      HasType Γ (binop BinOp.Eq l r) Ty.Bool

  | binop_and (Γ : Ctx) (l r : Expr) :
      HasType Γ l Ty.Bool → HasType Γ r Ty.Bool →
      HasType Γ (binop BinOp.And l r) Ty.Bool

  | pipe (Γ : Ctx) (arg f : Expr) (τ τ' : Ty) :
      HasType Γ arg τ → HasType Γ f (Ty.Fn τ τ') →
      HasType Γ (binop BinOp.Pipe arg f) τ'

def WellTyped (e : Expr) (τ : Ty) : Prop := HasType ([] : Ctx) e τ

end Neve
