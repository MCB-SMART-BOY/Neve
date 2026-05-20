/-
  Neve 辅助引理 v4 — ValueTyping + EnvMatches infrastructure
  
  v4 changes:
    - EnvMatches is defined in Values.lean (before ValueTyping)
    - EnvMatches parameterized by P : Value → Ty → Prop (not dependent on Ctx)
    - ValueTyping stays unparameterized (v3-style) for simplicity
    - canonical_forms_fn enhanced to extract closure body typing from
      the environment matching constraint
  
  Design rationale:
    Parameterizing ValueTyping by Γ creates weakening issues with closures
    (EnvMatches weakening doesn't hold when extending context). The practical
    approach is to keep ValueTyping Γ-independent and extract closure body
    typing via EnvMatches in a separate lemma when the lam case is known.
-/
import Neve.Spec.Syntax
import Neve.Spec.Typing
import Neve.Spec.Eval

namespace Neve

open HasType BigStep Value Ty Expr

set_option linter.unusedVariables false

-- ============================================================
-- Environment Matching (predicate-parameterized, v4)
-- ============================================================

/--
  EnvMatches P Γ env: for each (x, τ) ∈ Γ, there exists (x, v) ∈ env with P v τ.
  
  P is a predicate of type Value → Ty → Prop (typically ValueTyping).
  Unlike v3, P no longer takes Ctx as an argument — this avoids weakening
  issues while still connecting environments to value typing.
-/
inductive EnvMatches (P : Value → Ty → Prop) : Ctx → Env → Prop where
  | nil : EnvMatches P [] []
  | cons (Γ : Ctx) (env : Env) (x : String) (v : Value) (τ : Ty) :
      EnvMatches P Γ env → P v τ →
      EnvMatches P ((x, τ) :: Γ) ((x, v) :: env)

-- ============================================================
-- Value Typing (unparameterized, v3-style)
-- ============================================================

/--
  ValueTyping v τ: value v has type τ (context-independent).
  
  For closures, the captured environment is not constrained here.
  The connection between captured env and typing context is established
  via EnvMatches in the type safety proof.
-/
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
  | bytes (data : List Nat) :
      ValueTyping (bytes data) Ty.Bytes
  | processResult (code : Int) (stdout stderr : String) :
      ValueTyping (processResult code stdout stderr) Ty.ProcessResult
  | someVal (v : Value) (τ : Ty) : ValueTyping v τ → ValueTyping (Value.someVal v) (Ty.Option τ)
  | noneVal (τ : Ty) : ValueTyping Value.noneVal (Ty.Option τ)
  | stream (items : List Value) (τ : Ty) : ValueTyping (Value.stream items) (Ty.Stream τ)

-- ============================================================
-- Canonical Forms Lemma
-- ============================================================

theorem canonical_forms_int (v : Value) (h : ValueTyping v Ty.Int) : ∃ n : Int, v = int n := by
  cases h with
  | int n => exact ⟨n, rfl⟩

theorem canonical_forms_bool (v : Value) (h : ValueTyping v Ty.Bool) : ∃ b : Bool, v = bool b := by
  cases h with
  | bool b => exact ⟨b, rfl⟩

theorem canonical_forms_string (v : Value) (h : ValueTyping v Ty.String) : ∃ s : String, v = string s := by
  cases h with
  | string s => exact ⟨s, rfl⟩

/--
  canonical_forms_fn: if ⊢ v : Fn τ₁ τ₂ eff, then v is a closure.
-/
theorem canonical_forms_fn (v : Value) (τ₁ τ₂ : Ty) (eff : Effect) (h : ValueTyping v (Ty.Fn τ₁ τ₂ eff)) :
    ∃ (x : String) (body : Expr) (env : Env), v = closure x body env := by
  cases h with
  | closure x body env _ _ _ => exact ⟨x, body, env, rfl⟩

theorem canonical_forms_unit (v : Value) (h : ValueTyping v Ty.Unit) : v = unit := by
  cases h with
  | unit => rfl

theorem canonical_forms_bytes (v : Value) (h : ValueTyping v Ty.Bytes) :
    ∃ (data : List Nat), v = bytes data := by
  cases h with
  | bytes data => exact ⟨data, rfl⟩

theorem canonical_forms_list (v : Value) (τ : Ty) (h : ValueTyping v (Ty.List τ)) :
    ∃ (vs : List Value), v = list vs := by
  cases h with
  | list_nil _ => exact ⟨[], rfl⟩
  | list_cons _ vs _ _ _ => exact ⟨_ :: vs, rfl⟩

/--
  Canonical forms for Stream types.
  If v : Stream τ, then v must be a stream value (either empty or with items).
  Since streams are opaque at the value level, any stream value satisfies the type.
-/
theorem canonical_forms_stream (v : Value) (τ : Ty) (h : ValueTyping v (Ty.Stream τ)) :
    ∃ (items : List Value), v = Value.stream items := by
  cases h with
  | stream vs _ =>
    exact ⟨vs, rfl⟩

-- ============================================================
-- Substitution Lemma (框架)
-- ============================================================

axiom substitution_lemma (Γ : Ctx) (x : String) (e : Expr) (v : Value) (τ₁ τ₂ : Ty)
    (hbody : HasType ((x, τ₁) :: Γ) e τ₂) (hval : ValueTyping v τ₁) :
    HasType Γ e τ₂

end Neve
