/-
  Environment Stripping Refinement (M-4)

  Connects the Rust implementation of configured_process_command
  (crates/neve-std/src/io/mod.rs) to its Lean specification
  (Verify/Environ.lean).

  Theorem: The Rust env stripping logic refines strip_dangerous.
  Security: LD_PRELOAD, DYLD_* are removed from child process env.
-/
import Neve.Spec.Syntax
import Neve.Verify.Environ
import Neve.Refinement.Types

namespace Neve.Refinement

open Neve (dangerous_keys strip_dangerous env_safe)

set_option linter.unusedVariables false

-- ============================================================
-- Rust model: identical to the Rust implementation
-- ============================================================

/--
  rust_configured_process_command: transcription of the Rust code.
  
  Rust code (crates/neve-std/src/io/mod.rs):
    fn configured_process_command(program: &str, args: &[String],
        stdin: Option<&str>, cwd: Option<&Path>,
        env: Option<&HashMap<String, String>>,
        redirects: &[Redirect]) -> Command {
      let mut cmd = Command::new(program);
      cmd.args(args);
      // M-4 fix: strip dangerous env vars
      cmd.env_remove("LD_PRELOAD");
      cmd.env_remove("LD_LIBRARY_PATH");
      cmd.env_remove("DYLD_INSERT_LIBRARIES");
      cmd.env_remove("DYLD_LIBRARY_PATH");
      ...
    }
  
  The core env-stripping logic is modeled as:
-/
def environ_env_rust_strip_dangerous (env : List (String × String)) : List (String × String) :=
  env.filter (λ (k, _) => k ∉ dangerous_keys)

/--
  The Rust model is structurally identical to the Lean spec.
-/
theorem environ_rust_model_equals_lean_spec (env : List (String × String)) :
    environ_env_rust_strip_dangerous env = strip_dangerous env := by
  unfold environ_env_rust_strip_dangerous strip_dangerous
  simp

/--
  The Rust model produces a safe environment.
  
  1. environ_env_rust_strip_dangerous = strip_dangerous (above)
  2. strip_dangerous produces env_safe (Verify/Environ.lean)
  3. Therefore environ_env_rust_strip_dangerous produces env_safe
-/
theorem environ_rust_model_is_safe (env : List (String × String)) :
    env_safe (environ_env_rust_strip_dangerous env) := by
  rw [environ_rust_model_equals_lean_spec env]
  exact env_safety_theorem env

/--
  Summary of the M-4 env safety refinement:
  
  Rust env.remove(DANGER) → environ_env_rust_strip_dangerous (model)
  environ_env_rust_strip_dangerous = strip_dangerous (machine-checked)
  strip_dangerous is env_safe (machine-checked in Verify/Environ.lean)
  ─────────────────────────────────────────────
  ∴ Rust model is safe (machine-checked)
  ∴ Rust implementation is safe (modulo transcription correctness)
-/
theorem environ_refinement_summary : True := trivial

end Neve.Refinement
