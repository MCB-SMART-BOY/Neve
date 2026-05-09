/-
  Environment Safety: child processes must not receive dangerous env vars.

  Rust reference: crates/neve-std/src/io/mod.rs → configured_process_command
  Security audit: M-4 (LD_PRELOAD / DYLD_* injection via child process env)

  Verified properties:
  1. dangerous_key_is_stripped   — any dangerous key is absent after stripping
  2. strip_dangerous_idempotent  — stripping twice is same as stripping once
  3. env_safety_theorem          — configured_process_command produces safe env
-/

namespace Neve

-- ============================================================
-- Environment model
-- ============================================================

/--
  Dangerous environment variables that can inject code into child processes:
  - LD_PRELOAD: forces loading of a shared library before all others
  - LD_LIBRARY_PATH: overrides shared library search path
  - DYLD_INSERT_LIBRARIES: macOS equivalent of LD_PRELOAD
  - DYLD_LIBRARY_PATH: macOS equivalent of LD_LIBRARY_PATH

  Matches Rust: configured_process_command → cmd.env_remove(...)
-/
def dangerous_keys : List String :=
  ["LD_PRELOAD", "LD_LIBRARY_PATH", "DYLD_INSERT_LIBRARIES", "DYLD_LIBRARY_PATH"]

-- ============================================================
-- Environment predicates
-- ============================================================

/--
  An environment is "safe" if no dangerous key appears in any entry.
-/
def env_safe (e : List (String × String)) : Prop :=
  ∀ (k : String), k ∈ dangerous_keys → ¬ (∃ v, (k, v) ∈ e)

/--
  An environment is "clean" if it contains no entry with key `k`.
-/
def key_absent (e : List (String × String)) (k : String) : Prop :=
  ¬ (∃ v, (k, v) ∈ e)

-- ============================================================
-- Environment stripping (Lean spec of configured_process_command)
-- ============================================================

/--
  Remove all entries whose key is in the dangerous list.

  Lean specification of the Rust code:
  ```
  cmd.env_remove("LD_PRELOAD");
  cmd.env_remove("LD_LIBRARY_PATH");
  cmd.env_remove("DYLD_INSERT_LIBRARIES");
  cmd.env_remove("DYLD_LIBRARY_PATH");
  ```
-/
def strip_dangerous (e : List (String × String)) : List (String × String) :=
  e.filter (λ (k, _) => k ∉ dangerous_keys)

-- ============================================================
-- Lemma: membership after filtering
-- ============================================================

theorem mem_strip_dangerous (e : List (String × String)) (k : String) (v : String) :
    (k, v) ∈ strip_dangerous e ↔ (k, v) ∈ e ∧ k ∉ dangerous_keys := by
  simp [strip_dangerous]

-- ============================================================
-- Theorem 1: individual dangerous key is stripped
-- ============================================================

/--
  After stripping, any specific dangerous key is absent from the environment.
-/
theorem dangerous_key_is_stripped (e : List (String × String)) (k : String)
    (hk : k ∈ dangerous_keys) : key_absent (strip_dangerous e) k := by
  unfold key_absent
  intro h
  rcases h with ⟨v, hv⟩
  rcases (mem_strip_dangerous e k v).mp hv with ⟨_, hk'⟩
  -- hk' says k ∉ dangerous_keys, but hk says k ∈ dangerous_keys
  exact hk' hk

-- ============================================================
-- Theorem 2: idempotence
-- ============================================================

/--
  Stripping twice has the same effect as stripping once.
  This confirms there's no need to call env_remove multiple times
  for the same key — the operation is idempotent.
-/
theorem strip_dangerous_idempotent (e : List (String × String)) :
    strip_dangerous (strip_dangerous e) = strip_dangerous e := by
  simp [strip_dangerous]

-- ============================================================
-- Theorem 3: main safety theorem
-- ============================================================

/--
  For any command environment, after `configured_process_command`
  strips dangerous keys, the resulting child process environment
  is safe: no dangerous key can appear.

  This corresponds to the Rust implementation in
  `crates/neve-std/src/io/mod.rs → configured_process_command`.

  The proof is direct: for each dangerous key k, we show that k
  cannot appear in the stripped environment.
-/
theorem env_safety_theorem (e : List (String × String)) : env_safe (strip_dangerous e) := by
  unfold env_safe
  intro k hk
  exact dangerous_key_is_stripped e k hk

end Neve
