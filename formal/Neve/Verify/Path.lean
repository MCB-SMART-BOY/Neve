/-
  Path Safety: redirect paths must not escape the sandbox.

  Rust reference: crates/neve-std/src/io/mod.rs → resolve_redirect_path
  Security audit: M-1 (path traversal via `..` in redirect targets)

  Verified properties:
  1. sentinel_is_safe           — the sentinel path has no `..`
  2. path_safe_append           — safe paths compose under concatenation
  3. traversal_is_caught        — any input with `..` → sentinel
  4. path_safety_with_safe_cwd  — safe cwd + safe redirect → safe result
-/

namespace Neve

-- ============================================================
-- Path model
-- ============================================================

abbrev Path : Type := List String

def safe_sentinel : Path := ["dev", "null", "neve-blocked-traversal"]

-- ============================================================
-- Path predicates
-- ============================================================

/--
  Decidable check: does the path contain `..`?
  Uses Bool (not Prop) so it works in `if` statements.
  Rust: `path.components().any(|c| c == Component::ParentDir)`
-/
def has_parent_dir (p : Path) : Bool :=
  p.any (λ c => c = "..")

/--
  A path is "safe" if it contains no `..` components.
-/
def path_safe (p : Path) : Prop :=
  ∀ c ∈ p, c ≠ ".."

/--
  Lemma: `has_parent_dir` as Bool is equivalent to the existential Prop.
-/
theorem has_parent_dir_iff (p : Path) :
    has_parent_dir p = true ↔ ∃ c ∈ p, c = ".." := by
  simp [has_parent_dir]

-- ============================================================
-- Path resolution (Lean spec)
-- ============================================================

/--
  Lean specification of the Rust function:
  `resolve_redirect_path(command, redirect) -> PathBuf`

  1. redirect has `..` → safe sentinel
  2. cwd available      → cwd ++ redirect
  3. otherwise          → redirect unchanged
-/
def resolve_redirect_path (cwd : Option Path) (redirect : Path) : Path :=
  if has_parent_dir redirect then
    safe_sentinel
  else match cwd with
    | none     => redirect
    | some cwd => cwd ++ redirect

-- ============================================================
-- Lemma 1: the sentinel is safe
-- ============================================================

theorem sentinel_is_safe : path_safe safe_sentinel := by
  unfold path_safe safe_sentinel
  intro c h
  simp at h
  rcases h with (rfl | rfl | rfl)
  · simp
  · simp
  · simp

-- ============================================================
-- Lemma 2: safe paths compose under concatenation
-- ============================================================

theorem path_safe_append (p q : Path) (hp : path_safe p) (hq : path_safe q) :
    path_safe (p ++ q) := by
  unfold path_safe at hp hq ⊢
  intro c hc
  rcases List.mem_append.mp hc with (hcp | hcq)
  · exact hp c hcp
  · exact hq c hcq

-- ============================================================
-- Theorem 1: traversal detection (unconditional)
-- ============================================================

/--
  Any redirect path containing `..` is replaced by the safe sentinel.
-/
theorem traversal_is_caught (cwd : Option Path) (redirect : Path)
    (h : has_parent_dir redirect = true) :
    resolve_redirect_path cwd redirect = safe_sentinel := by
  unfold resolve_redirect_path
  rw [if_pos h]

-- ============================================================
-- Theorem 2: full correctness (safe inputs → safe output)
-- ============================================================

/--
  If both cwd and redirect are safe, the resolved path is safe.

  Main correctness theorem. Guarantees the path resolution function
  preserves the "no `..`" invariant — necessary and sufficient to
  prevent directory traversal.
-/
theorem path_safety_with_safe_cwd (cwd : Option Path) (redirect : Path)
    (hcwd_safe : ∀ p ∈ cwd, path_safe p) :
    path_safe (resolve_redirect_path cwd redirect) := by
  unfold resolve_redirect_path
  by_cases h : has_parent_dir redirect
  · -- Case 1: redirect has `..` → sentinel
    rw [if_pos h]
    exact sentinel_is_safe
  · -- Case 2: redirect is safe
    rw [if_neg h]
    have hsafe : path_safe redirect := by
      unfold path_safe
      intro c hc
      intro heq
      have hmem : has_parent_dir redirect := by
        unfold has_parent_dir
        apply List.any_eq_true.mpr
        -- c = ".." by heq, so decide (c = "..") is true
        have heq_dec : decide (c = "..") = true := by
          simp [heq]
        exact ⟨c, hc, heq_dec⟩
      rw [hmem] at h
      -- h : ¬ true → contradiction
      exact h rfl
    match cwd with
    | none => exact hsafe
    | some cwd_path =>
        have hcwd : path_safe cwd_path := hcwd_safe cwd_path (by simp)
        exact path_safe_append cwd_path redirect hcwd hsafe

end Neve
