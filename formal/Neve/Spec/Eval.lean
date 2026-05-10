/-
  Neve 操作语义（大步骤）
  env ⊢ e ⇓ v
  Big-step operational semantics for Neve core calculus.

  v2: Complete binop rules, matchOn with pattern matching, lit_float, lit_char
-/
import Neve.Spec.Syntax
import Neve.Spec.Typing

namespace Neve

open Expr Value BinOp Pattern

abbrev Env : Type := List (String × Value)

-- === 模式匹配（mutual 以解决前向引用）===
-- env' ⊢ p matches v → bindings
mutual
  inductive Matches : Pattern → Value → Env → Prop where
    | wildcard (v : Value) :
        Matches Pattern.wildcard v []

    | var (x : String) (v : Value) :
        Matches (Pattern.var x) v [(x, v)]

    | lit_int (n m : Int) (heq : n = m) :
        Matches (Pattern.lit_int n) (int m) []

    | lit_bool (b₁ b₂ : Bool) (heq : b₁ = b₂) :
        Matches (Pattern.lit_bool b₁) (bool b₂) []

    | lit_string (s₁ s₂ : String) (heq : s₁ = s₂) :
        Matches (Pattern.lit_string s₁) (string s₂) []

    | tuple (ps : List Pattern) (vs : List Value) (binds : Env) :
        MatchesList ps vs binds →
        Matches (Pattern.tuple ps) (tuple vs) binds

    | list_nil (τ : Ty) :
        Matches (Pattern.list [] false) (list []) []

    | list_cons (p : Pattern) (ps : List Pattern) (v : Value) (vs : List Value) (hds : Env) (tls : Env) :
        Matches p v hds →
        Matches (Pattern.list ps false) (list vs) tls →
        Matches (Pattern.list (p :: ps) false) (list (v :: vs)) (hds ++ tls)

  -- === 多模式匹配辅助 ===
  inductive MatchesList : List Pattern → List Value → Env → Prop where
    | nil : MatchesList [] [] []
    | cons (p : Pattern) (ps : List Pattern) (v : Value) (vs : List Value) (binds_hd binds_tl : Env) :
        Matches p v binds_hd →
        MatchesList ps vs binds_tl →
        MatchesList (p :: ps) (v :: vs) (binds_hd ++ binds_tl)
end

-- === 大步操作语义 / Big-step operational semantics ===
inductive BigStep : Env → Expr → Value → Prop where
  -- Literals
  | lit_int (env : Env) (n : Int) :
      BigStep env (lit_int n) (int n)
  | lit_float (env : Env) (f : Float) :
      BigStep env (lit_float f) (float f)
  | lit_bool (env : Env) (b : Bool) :
      BigStep env (lit_bool b) (bool b)
  | lit_char (env : Env) (c : Char) :
      BigStep env (lit_char c) (char c)
  | lit_string (env : Env) (s : String) :
      BigStep env (lit_string s) (string s)
  | lit_unit (env : Env) :
      BigStep env lit_unit unit

  -- Variable
  | var (env : Env) (x : String) (v : Value) (hin : (x, v) ∈ env) :
      BigStep env (var x) v

  -- Lambda
  | lam (env : Env) (x : String) (body : Expr) :
      BigStep env (lam x body) (closure x body env)

  -- Application
  | app (env : Env) (f arg : Expr) (x : String) (body : Expr) (env' : Env) (varg vres : Value) :
      BigStep env f (closure x body env') →
      BigStep env arg varg →
      BigStep ((x, varg) :: env') body vres →
      BigStep env (app f arg) vres

  -- Let binding
  | letIn (env : Env) (x : String) (val body : Expr) (vval vbody : Value) :
      BigStep env val vval →
      BigStep ((x, vval) :: env) body vbody →
      BigStep env (letIn x val body) vbody

  -- BinOp: Arithmetic
  | binop_add (env : Env) (l r : Expr) (n m : Int) :
      BigStep env l (int n) → BigStep env r (int m) →
      BigStep env (binop BinOp.Add l r) (int (n + m))

  | binop_sub (env : Env) (l r : Expr) (n m : Int) :
      BigStep env l (int n) → BigStep env r (int m) →
      BigStep env (binop BinOp.Sub l r) (int (n - m))

  | binop_mul (env : Env) (l r : Expr) (n m : Int) :
      BigStep env l (int n) → BigStep env r (int m) →
      BigStep env (binop BinOp.Mul l r) (int (n * m))

  | binop_div (env : Env) (l r : Expr) (n m : Int) (hnz : m ≠ 0) :
      BigStep env l (int n) → BigStep env r (int m) →
      BigStep env (binop BinOp.Div l r) (int (n / m))

  | binop_mod (env : Env) (l r : Expr) (n m : Int) (hnz : m ≠ 0) :
      BigStep env l (int n) → BigStep env r (int m) →
      BigStep env (binop BinOp.Mod l r) (int (n % m))

  -- Division by zero: returns 0 (safe default, type safety preserved)
  | binop_div_zero (env : Env) (l r : Expr) (n : Int) :
      BigStep env l (int n) → BigStep env r (int 0) →
      BigStep env (binop BinOp.Div l r) (int 0)

  -- Modulo by zero: returns 0 (safe default, type safety preserved)
  | binop_mod_zero (env : Env) (l r : Expr) (n : Int) :
      BigStep env l (int n) → BigStep env r (int 0) →
      BigStep env (binop BinOp.Mod l r) (int 0)

  -- BinOp: Equality (polymorphic - works for any value type)
  | binop_eq_true (env : Env) (l r : Expr) (v₁ v₂ : Value) :
      BigStep env l v₁ → BigStep env r v₂ → v₁ = v₂ →
      BigStep env (binop BinOp.Eq l r) (bool true)

  | binop_eq_false (env : Env) (l r : Expr) (v₁ v₂ : Value) :
      BigStep env l v₁ → BigStep env r v₂ → v₁ ≠ v₂ →
      BigStep env (binop BinOp.Eq l r) (bool false)

  -- BinOp: Boolean logic
  | binop_and_true (env : Env) (l r : Expr) :
      BigStep env l (bool true) → BigStep env r (bool true) →
      BigStep env (binop BinOp.And l r) (bool true)

  | binop_and_false_l (env : Env) (l r : Expr) :
      BigStep env l (bool false) →
      BigStep env (binop BinOp.And l r) (bool false)

  | binop_and_false_r (env : Env) (l r : Expr) :
      BigStep env l (bool true) → BigStep env r (bool false) →
      BigStep env (binop BinOp.And l r) (bool false)

  | binop_or_true_l (env : Env) (l r : Expr) :
      BigStep env l (bool true) →
      BigStep env (binop BinOp.Or l r) (bool true)

  | binop_or_true_r (env : Env) (l r : Expr) :
      BigStep env l (bool false) → BigStep env r (bool true) →
      BigStep env (binop BinOp.Or l r) (bool true)

  | binop_or_false (env : Env) (l r : Expr) :
      BigStep env l (bool false) → BigStep env r (bool false) →
      BigStep env (binop BinOp.Or l r) (bool false)

  -- BinOp: Pipe (forward pipe, like |>) — same semantics as application
  | pipe (env : Env) (arg f : Expr) (varg : Value) (x : String) (body : Expr) (env' : Env) (vres : Value) :
      BigStep env arg varg →
      BigStep env f (closure x body env') →
      BigStep ((x, varg) :: env') body vres →
      BigStep env (binop BinOp.Pipe arg f) vres

  -- Match expression
  | matchOn (env : Env) (scrutinee : Expr) (arms : List (Pattern × Expr)) (vscrut vres : Value)
             (p : Pattern) (e : Expr) (rest : List (Pattern × Expr)) (binds : Env) :
      BigStep env scrutinee vscrut →
      -- Find the first matching arm
      (p, e) :: rest = arms →
      Matches p vscrut binds →
      BigStep (binds ++ env) e vres →
      BigStep env (matchOn scrutinee arms) vres

  -- Match fallthrough: first arm does NOT match, skip to remaining arms
  | matchOn_fallthrough (env : Env) (scrutinee : Expr) (arms : List (Pattern × Expr))
             (p : Pattern) (e : Expr) (rest : List (Pattern × Expr)) (vscrut vres : Value) :
      BigStep env scrutinee vscrut →
      (p, e) :: rest = arms →
      (∀ binds, ¬ Matches p vscrut binds) →
      BigStep env (matchOn scrutinee rest) vres →
      BigStep env (matchOn scrutinee arms) vres

end Neve
