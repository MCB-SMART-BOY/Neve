/-
  Type Safety — Verified Pattern Matching Lemmas

  These lemmas reduce dependency on the progress_match axiom by proving
  specific, commonly-used pattern matching cases.

  Note: The current BigStep.matchOn rule only handles the case where
  the FIRST arm matches the scrutinee. Full multi-arm matching with
  fallthrough requires an additional semantic rule (future work).

  Already verified:
    - matchOn_wildcard     (in Safety.lean) — wildcard always matches
    - matchOn_wildcard_first (below)        — wildcard first arm
    - matchOn_lit_int_first  (below)        — matching literal as first arm
    - matchOn_unit_wildcard  (below)        — unit with wildcard first
-/
import Neve.Spec.Syntax
import Neve.Spec.Typing
import Neve.Spec.Eval
import Neve.Proofs.Values
import Neve.Proofs.Context
import Neve.Proofs.Safety

namespace Neve

open Ty Expr Value Pattern

set_option linter.unusedVariables false

-- ============================================================
-- matchOn_wildcard_first (re-export of Safety.matchOn_wildcard_verified)
-- ============================================================

/--
  If the first arm is a wildcard, the match always succeeds.
  This is exactly the lemma from Safety.lean, re-exported here.
-/
theorem matchOn_wildcard_first (Γ : Ctx) (env : Env) (scrutinee : Expr)
    (e : Expr) (rest : List (Pattern × Expr)) (τs τ : Ty)
    (hs : HasType Γ scrutinee τs) (hbody : HasType Γ e τ)
    (hrest : AllArmsMatch Γ rest τs τ) (henv : EnvMatches ValueTyping Γ env) :
    ∃ v : Value, BigStep env (Expr.matchOn scrutinee ((Pattern.wildcard, e) :: rest)) v
    ∧ ValueTyping v τ :=
  matchOn_wildcard_verified Γ env scrutinee e rest τs τ hs hbody hrest henv

-- ============================================================
-- matchOn_lit_int_first
-- ============================================================

/--
  Match on int where the scrutinee is known to evaluate to n,
  and the first arm is `lit_int n => eMatch`.

  The `heval_scrut` parameter gives us the exact evaluation result,
  so we can directly construct the BigStep derivation.
-/
theorem matchOn_lit_int_first (Γ : Ctx) (env : Env) (scrutinee : Expr)
    (n : Int) (eMatch : Expr) (rest : List (Pattern × Expr)) (τ : Ty)
    (hs : HasType Γ scrutinee Ty.Int)
    (hMatch : HasType Γ eMatch τ)
    (henv : EnvMatches ValueTyping Γ env)
    (heval_scrut : BigStep env scrutinee (Value.int n)) :
    ∃ v : Value, BigStep env
      (Expr.matchOn scrutinee ((Pattern.lit_int n, eMatch) :: rest)) v
    ∧ ValueTyping v τ := by
  have ihbody := progress_preservation Γ env eMatch τ hMatch henv
  rcases ihbody with ⟨vres, evalbody, tyvres⟩
  exact ⟨vres,
    BigStep.matchOn env scrutinee ((Pattern.lit_int n, eMatch) :: rest)
      (Value.int n) vres (Pattern.lit_int n) eMatch rest []
      heval_scrut rfl (Matches.lit_int n n rfl) evalbody,
    tyvres⟩

-- ============================================================
-- matchOn_unit_wildcard
-- ============================================================

/--
  Match on unit with a wildcard first arm.
  Since unit has only one value and wildcard matches everything,
  this match always succeeds.

  Note: This is a special case of matchOn_wildcard_first with τs = Ty.Unit.
-/
theorem matchOn_unit_wildcard (Γ : Ctx) (env : Env) (scrutinee : Expr)
    (eBody : Expr) (τ : Ty)
    (hs : HasType Γ scrutinee Ty.Unit)
    (hBody : HasType Γ eBody τ)
    (henv : EnvMatches ValueTyping Γ env) :
    ∃ v : Value, BigStep env
      (Expr.matchOn scrutinee [(Pattern.wildcard, eBody)]) v
    ∧ ValueTyping v τ :=
  -- This is a direct application of matchOn_wildcard_first with empty rest
  matchOn_wildcard_first Γ env scrutinee eBody [] Ty.Unit τ hs hBody
    (AllArmsMatch.nil Γ Ty.Unit τ) henv

-- ============================================================
-- matchOn_bool_first_arm
-- ============================================================

/--
  Match on bool where the scrutinee is known to evaluate to b,
  and the first arm is `lit_bool b => eMatch`.

  Note: This only covers the case where the matching arm is FIRST.
  For a full two-arm boolean match (true => e1, false => e2),
  the false case requires arm skipping which is not yet formalized
  in BigStep.matchOn. See the note at the end of this file.
-/
theorem matchOn_bool_first_arm (Γ : Ctx) (env : Env) (scrutinee : Expr)
    (b : Bool) (eMatch : Expr) (rest : List (Pattern × Expr)) (τ : Ty)
    (hs : HasType Γ scrutinee Ty.Bool)
    (hMatch : HasType Γ eMatch τ)
    (henv : EnvMatches ValueTyping Γ env)
    (heval_scrut : BigStep env scrutinee (Value.bool b)) :
    ∃ v : Value, BigStep env
      (Expr.matchOn scrutinee ((Pattern.lit_bool b, eMatch) :: rest)) v
    ∧ ValueTyping v τ := by
  have ihbody := progress_preservation Γ env eMatch τ hMatch henv
  rcases ihbody with ⟨vres, evalbody, tyvres⟩
  exact ⟨vres,
    BigStep.matchOn env scrutinee ((Pattern.lit_bool b, eMatch) :: rest)
      (Value.bool b) vres (Pattern.lit_bool b) eMatch rest []
      heval_scrut rfl (Matches.lit_bool b b rfl) evalbody,
    tyvres⟩

-- ============================================================
-- matchOn_bool_full: two-arm boolean match (true => e1, false => e2)
-- ============================================================

/--
  Full two-arm boolean match, works for both true and false scrutinees.
  
  Uses the new matchOn_fallthrough rule (Eval.lean v2) to skip the
  non-matching first arm when the scrutinee is false.
  
  This eliminates the need for the progress_match axiom for boolean matches.
-/
theorem matchOn_bool_full (Γ : Ctx) (env : Env) (scrutinee : Expr)
    (eTrue eFalse : Expr) (τ : Ty)
    (hs : HasType Γ scrutinee Ty.Bool)
    (hTrue : HasType Γ eTrue τ) (hFalse : HasType Γ eFalse τ)
    (henv : EnvMatches ValueTyping Γ env) :
    ∃ v : Value, BigStep env
      (Expr.matchOn scrutinee
        [(Pattern.lit_bool true, eTrue), (Pattern.lit_bool false, eFalse)]) v
    ∧ ValueTyping v τ := by
  have ihs := progress_preservation Γ env scrutinee Ty.Bool hs henv
  rcases ihs with ⟨vscrut, evals, tyvs⟩
  rcases canonical_forms_bool vscrut tyvs with ⟨b, hb⟩
  subst hb
  match b with
  | true =>
      have ihbody := progress_preservation Γ env eTrue τ hTrue henv
      rcases ihbody with ⟨vres, evalbody, tyvres⟩
      exact ⟨vres,
        BigStep.matchOn env scrutinee
          [(Pattern.lit_bool true, eTrue), (Pattern.lit_bool false, eFalse)]
          (Value.bool true) vres (Pattern.lit_bool true) eTrue
          [(Pattern.lit_bool false, eFalse)] []
          evals rfl (Matches.lit_bool true true rfl) evalbody,
        tyvres⟩
  | false =>
      have h_nomatch : ∀ binds, ¬ Matches (Pattern.lit_bool true) (Value.bool false) binds := by
        intro binds h
        cases h with
        | lit_bool b1 b2 heq => injection heq
      have ihbody := progress_preservation Γ env eFalse τ hFalse henv
      rcases ihbody with ⟨vres, evalbody, tyvres⟩
      have h_rest : BigStep env
        (Expr.matchOn scrutinee [(Pattern.lit_bool false, eFalse)])
        vres :=
        BigStep.matchOn env scrutinee [(Pattern.lit_bool false, eFalse)]
          (Value.bool false) vres (Pattern.lit_bool false) eFalse [] []
          evals rfl (Matches.lit_bool false false rfl) evalbody
      exact ⟨vres,
        BigStep.matchOn_fallthrough env scrutinee
          [(Pattern.lit_bool true, eTrue), (Pattern.lit_bool false, eFalse)]
          (Pattern.lit_bool true) eTrue [(Pattern.lit_bool false, eFalse)]
          (Value.bool false) vres
          evals rfl h_nomatch h_rest,
        tyvres⟩

-- ============================================================
-- Summary
-- ============================================================

/--
  Pattern matching coverage status:

  Machine-checked (no axiom needed):
    1. matchOn_wildcard_first  — wildcard as first arm (any type)
    2. matchOn_lit_int_first   — integer literal as first arm
    3. matchOn_unit_wildcard   — unit value with wildcard
    4. matchOn_bool_first_arm  — boolean literal as first arm (when value matches)
    5. matchOn_bool_full       — full two-arm boolean match (uses fallthrough rule)

  Requires axiom (progress_match):
    - Multi-arm boolean matches where false is the scrutinee
      (requires arm skipping — BigStep.matchOn semantic extension needed)
    - Enum constructor patterns (Some/None, Ok/Err)
    - Record patterns, tuple patterns, list rest patterns
    - Binding patterns (name @ pat), or patterns (a | b)
    - General N-arm matches with fallthrough

  Future work:
    1. Add BigStep.matchOn_fallthrough rule to Eval.lean for arm skipping
    2. Prove pattern exhaustiveness for finite types (Bool, Unit)
    3. Extend to constructor patterns via canonical forms enumeration
-/
theorem pattern_matching_lemmas_summary : True := trivial

end Neve
