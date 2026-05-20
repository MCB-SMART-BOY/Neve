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
-- matchOn_lit_string_first
-- ============================================================

/--
  Match on string where the scrutinee is known to evaluate to s,
  and the first arm is `lit_string s => eMatch`.
-/
theorem matchOn_lit_string_first (Γ : Ctx) (env : Env) (scrutinee : Expr)
    (s : String) (eMatch : Expr) (rest : List (Pattern × Expr)) (τ : Ty)
    (hs : HasType Γ scrutinee Ty.String)
    (hMatch : HasType Γ eMatch τ)
    (henv : EnvMatches ValueTyping Γ env)
    (heval_scrut : BigStep env scrutinee (Value.string s)) :
    ∃ v : Value, BigStep env
      (Expr.matchOn scrutinee ((Pattern.lit_string s, eMatch) :: rest)) v
    ∧ ValueTyping v τ := by
  have ihbody := progress_preservation Γ env eMatch τ hMatch henv
  rcases ihbody with ⟨vres, evalbody, tyvres⟩
  exact ⟨vres,
    BigStep.matchOn env scrutinee ((Pattern.lit_string s, eMatch) :: rest)
      (Value.string s) vres (Pattern.lit_string s) eMatch rest []
      heval_scrut rfl (Matches.lit_string s s rfl) evalbody,
    tyvres⟩

-- ============================================================
-- matchOn_list_nil_first
-- ============================================================

/--
  Match on an empty list where the first arm is `[] => eMatch`.
-/
theorem matchOn_list_nil_first (Γ : Ctx) (env : Env) (scrutinee : Expr)
    (τelem : Ty) (eMatch : Expr) (rest : List (Pattern × Expr)) (τ : Ty)
    (hs : HasType Γ scrutinee (Ty.List τelem))
    (hMatch : HasType Γ eMatch τ)
    (henv : EnvMatches ValueTyping Γ env)
    (heval_scrut : BigStep env scrutinee (Value.list [])) :
    ∃ v : Value, BigStep env
      (Expr.matchOn scrutinee ((Pattern.list [] false, eMatch) :: rest)) v
    ∧ ValueTyping v τ := by
  have ihbody := progress_preservation Γ env eMatch τ hMatch henv
  rcases ihbody with ⟨vres, evalbody, tyvres⟩
  exact ⟨vres,
    BigStep.matchOn env scrutinee ((Pattern.list [] false, eMatch) :: rest)
      (Value.list []) vres (Pattern.list [] false) eMatch rest []
      heval_scrut rfl (Matches.list_nil τelem) evalbody,
    tyvres⟩

-- ============================================================
-- matchOn_wildcard_not_first_arm: wildcard as second arm
-- ============================================================

/--
  Match where the first arm does NOT match, but the second arm
  is a wildcard. Uses matchOn_fallthrough to skip the non-matching
  first arm, then matchOn_wildcard_first for the remaining arms.
  
  Example: `match x { 42 -> "answer", _ -> "other" }` where x ≠ 42
-/
theorem matchOn_wildcard_not_first_arm (Γ : Ctx) (env : Env) (scrutinee : Expr)
    (p : Pattern) (eNoMatch : Expr) (eWild : Expr) (rest : List (Pattern × Expr))
    (τs τ : Ty) (vscrut : Value)
    (hs : HasType Γ scrutinee τs) (hNoMatch : HasType Γ eNoMatch τ) (hWild : HasType Γ eWild τ)
    (hrest : AllArmsMatch Γ rest τs τ)
    (henv : EnvMatches ValueTyping Γ env)
    (heval_scrut : BigStep env scrutinee vscrut)
    (h_no_match : ∀ binds, ¬ Matches p vscrut binds) :
    ∃ v : Value, BigStep env
      (Expr.matchOn scrutinee ((p, eNoMatch) :: (Pattern.wildcard, eWild) :: rest)) v
    ∧ ValueTyping v τ := by
  -- The wildcard arm always matches when it becomes the first arm
  have h_wild := matchOn_wildcard_first Γ env scrutinee eWild rest τs τ hs hWild hrest henv
  rcases h_wild with ⟨vres, h_wild_eval, tyvres⟩
  exact ⟨vres,
    BigStep.matchOn_fallthrough env scrutinee
      ((p, eNoMatch) :: (Pattern.wildcard, eWild) :: rest)
      p eNoMatch ((Pattern.wildcard, eWild) :: rest)
      vscrut vres
      heval_scrut rfl h_no_match h_wild_eval,
    tyvres⟩

-- ============================================================
-- matchOn_wildcard_third_arm: wildcard as third arm
-- ============================================================

/--
  Match with THREE arms where the first two don't match and
  the third is a wildcard. Uses matchOn_fallthrough TWICE:
  once to skip the first non-matching arm, then
  matchOn_wildcard_not_first_arm to handle the remaining two arms.
  
  Example: `match x { 0 -> "zero", 1 -> "one", _ -> "other" }`
  where x is neither 0 nor 1.
-/
theorem matchOn_wildcard_third_arm (Γ : Ctx) (env : Env) (scrutinee : Expr)
    (e0 e1 eWild : Expr) (n0 n1 m : Int) (τ : Ty)
    (hs : HasType Γ scrutinee Ty.Int)
    (h0 : HasType Γ e0 τ) (h1 : HasType Γ e1 τ) (hWild : HasType Γ eWild τ)
    (henv : EnvMatches ValueTyping Γ env)
    (h_scrut_val : BigStep env scrutinee (Value.int m))
    (h_neq0 : n0 ≠ m) (h_neq1 : n1 ≠ m) :
    ∃ v : Value, BigStep env
      (Expr.matchOn scrutinee
        [(Pattern.lit_int n0, e0), (Pattern.lit_int n1, e1), (Pattern.wildcard, eWild)]) v
    ∧ ValueTyping v τ := by
  -- First arm (lit_int n0) doesn't match scrutinee (Value.int m)
  have h_no_match0 : ∀ (binds : Env), ¬ Matches (Pattern.lit_int n0) (Value.int m) binds := by
    intro binds h_match
    cases h_match with
    | lit_int n' m' heq =>
      apply h_neq0
      exact heq
  -- Second arm (lit_int n1) also doesn't match
  have h_no_match1 : ∀ (binds : Env), ¬ Matches (Pattern.lit_int n1) (Value.int m) binds := by
    intro binds h_match
    cases h_match with
    | lit_int n' m' heq =>
      apply h_neq1
      exact heq
  -- The remaining two arms are handled by matchOn_wildcard_not_first_arm
  have h_rest := matchOn_wildcard_not_first_arm Γ env scrutinee
    (Pattern.lit_int n1) e1 eWild [] Ty.Int τ (Value.int m)
    hs h1 hWild (AllArmsMatch.nil Γ Ty.Int τ) henv
    h_scrut_val h_no_match1
  rcases h_rest with ⟨vres, h_rest_eval, tyvres⟩
  -- Fallthrough from the first non-matching arm to the rest
  exact ⟨vres,
    BigStep.matchOn_fallthrough env scrutinee
      [(Pattern.lit_int n0, e0), (Pattern.lit_int n1, e1), (Pattern.wildcard, eWild)]
      (Pattern.lit_int n0) e0 [(Pattern.lit_int n1, e1), (Pattern.wildcard, eWild)]
      (Value.int m) vres
      h_scrut_val rfl h_no_match0 h_rest_eval,
    tyvres⟩

-- ============================================================
-- Summary
-- ============================================================

/--
  Pattern matching coverage status:

  Machine-checked (no axiom needed):
    1. matchOn_wildcard_first        — wildcard as first arm (any type)
    2. matchOn_lit_int_first         — integer literal as first arm
    3. matchOn_unit_wildcard         — unit value with wildcard
    4. matchOn_bool_first_arm        — boolean literal as first arm (when value matches)
    5. matchOn_bool_full             — full two-arm boolean match (uses fallthrough rule)
    6. matchOn_lit_string_first      — string literal as first arm
    7. matchOn_list_nil_first        — empty list as first arm
    8. matchOn_wildcard_not_first_arm — wildcard matches even when not first arm
    9. matchOn_wildcard_third_arm    — wildcard as third arm after two non-matching
    

  Requires axiom (progress_match):
    - Enum constructor patterns (Some/None, Ok/Err)
    - Record patterns, list rest patterns
    - Binding patterns (name @ pat), or patterns (a | b)
    - General N-arm matches with fallthrough (pattern generalized from lemmas 8-9)

  Future work:
    1. Generalize matchOn_wildcard_third_arm to N-arm fallthrough
    2. Prove pattern exhaustiveness for finite types (Bool, Unit)
    3. Extend to constructor patterns via canonical forms enumeration
-/
theorem pattern_matching_lemmas_summary : True := trivial

end Neve
