/-
  Neve 操作语义（大步骤）
  env ⊢ e ⇓ v
-/
import Neve.Syntax
import Neve.Types

namespace Neve

open Expr Value BinOp Pattern

abbrev Env : Type := List (String × Value)

inductive BigStep : Env → Expr → Value → Prop where
  | lit_int (env : Env) (n : Int) :
      BigStep env (lit_int n) (int n)
  | lit_bool (env : Env) (b : Bool) :
      BigStep env (lit_bool b) (bool b)
  | lit_string (env : Env) (s : String) :
      BigStep env (lit_string s) (string s)
  | lit_unit (env : Env) :
      BigStep env lit_unit unit

  | var (env : Env) (x : String) (v : Value) (hin : (x, v) ∈ env) :
      BigStep env (var x) v

  | lam (env : Env) (x : String) (body : Expr) :
      BigStep env (lam x body) (closure x body env)

  | app (env : Env) (f arg : Expr) (x : String) (body : Expr) (env' : Env) (varg vres : Value) :
      BigStep env f (closure x body env') →
      BigStep env arg varg →
      BigStep ((x, varg) :: env') body vres →
      BigStep env (app f arg) vres

  | letIn (env : Env) (x : String) (val body : Expr) (vval vbody : Value) :
      BigStep env val vval →
      BigStep ((x, vval) :: env) body vbody →
      BigStep env (letIn x val body) vbody

  | binop_add (env : Env) (l r : Expr) (n m : Int) :
      BigStep env l (int n) → BigStep env r (int m) →
      BigStep env (binop BinOp.Add l r) (int (n + m))

  | pipe (env : Env) (arg f : Expr) (varg : Value) (x : String) (body : Expr) (env' : Env) (vres : Value) :
      BigStep env arg varg →
      BigStep env f (closure x body env') →
      BigStep ((x, varg) :: env') body vres →
      BigStep env (binop BinOp.Pipe arg f) vres

end Neve
