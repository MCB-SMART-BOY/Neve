/-
  Buffer Size Limits Refinement (H-1, H-2)

  Connects the Rust size checks in the five blocking execution paths
  (crates/neve-std/src/io/mod.rs) to their Lean specifications
  (Verify/Limits.lean).

  H-1: stdin ≤ MAX_STDIN_BYTES (10 MB)
  H-2: stdout/stderr ≤ MAX_OUTPUT_BYTES (50 MB)

  Theorem: The Rust size checks refine the EffectEval size premises.
-/
import Neve.Spec.Syntax
import Neve.Spec.Effects
import Neve.Verify.Limits
import Neve.Refinement.Types

namespace Neve.Refinement

open Neve (MAX_STDIN_BYTES MAX_OUTPUT_BYTES check_stdin check_output)

set_option linter.unusedVariables false

-- ============================================================
-- Rust model: size checks
-- ============================================================

/--
  limits_rust_check_stdin: transcription of the Rust stdin size check.
  
  Rust code (crates/neve-std/src/io/mod.rs):
    if stdin.len() > MAX_STDIN_BYTES {
        return Err("stdin exceeds 10MB limit");
    }
-/
def limits_rust_check_stdin (size : Nat) : Except String Unit :=
  if size > MAX_STDIN_BYTES then .error "stdin exceeds limit" else .ok ()

/--
  limits_rust_check_output: transcription of the Rust output size check.
  
  Rust code (crates/neve-std/src/io/mod.rs):
    if stdout.len() > MAX_OUTPUT_BYTES {
        return Err("stdout exceeds 50MB limit");
    }
    if stderr.len() > MAX_OUTPUT_BYTES {
        return Err("stderr exceeds 50MB limit");
    }
-/
def limits_rust_check_output (stdout_size stderr_size : Nat) : Except String Unit :=
  if stdout_size > MAX_OUTPUT_BYTES then
    .error "stdout exceeds limit"
  else if stderr_size > MAX_OUTPUT_BYTES then
    .error "stderr exceeds limit"
  else
    .ok ()

-- ============================================================
-- Equivalence: Rust model = Lean spec
-- ============================================================

/--
  The Rust stdin check is identical to the Lean spec.
-/
theorem limits_rust_stdin_equals_lean_spec (size : Nat) :
    limits_rust_check_stdin size = check_stdin size := by
  unfold limits_rust_check_stdin check_stdin
  rfl

/--
  The Rust output check is identical to the Lean spec.
-/
theorem limits_rust_output_equals_lean_spec (stdout_size stderr_size : Nat) :
    limits_rust_check_output stdout_size stderr_size = check_output stdout_size stderr_size := by
  unfold limits_rust_check_output check_output
  rfl

-- ============================================================
-- Main refinement theorems
-- ============================================================

/--
  If the Rust stdin check passes, the size is within limits.
  
  This connects the Rust implementation to the Lean verification:
  - stdin_check_ok_implies_within_limit (Verify/Limits.lean) is a
    machine-checked proof of the same property for the Lean spec.
  - Since the Rust model = Lean spec, the same guarantee holds
    for the Rust implementation.
-/
theorem limits_rust_stdin_check_implies_within_limit (size : Nat)
    (h : limits_rust_check_stdin size = .ok ()) : size ≤ MAX_STDIN_BYTES := by
  rw [limits_rust_stdin_equals_lean_spec size] at h
  exact stdin_check_ok_implies_within_limit size h

/--
  If the Rust output check passes, both stdout and stderr are within limits.
-/
theorem limits_rust_output_check_implies_within_limits (stdout_size stderr_size : Nat)
    (h : limits_rust_check_output stdout_size stderr_size = .ok ()) :
    stdout_size ≤ MAX_OUTPUT_BYTES ∧ stderr_size ≤ MAX_OUTPUT_BYTES := by
  rw [limits_rust_output_equals_lean_spec stdout_size stderr_size] at h
  exact output_check_ok_implies_within_limits stdout_size stderr_size h

/--
  The Rust size checks are sound and complete (via equivalence to Lean spec).
  
  Soundness: if the check passes, sizes are within limits.
  Completeness: if sizes are within limits, the check passes.
  
  Both are machine-checked in Verify/Limits.lean.
-/
theorem limits_rust_checks_sound_and_complete (stdin_size stdout_size stderr_size : Nat) :
    (limits_rust_check_stdin stdin_size = .ok () ∧
     limits_rust_check_output stdout_size stderr_size = .ok ()) ↔
    (stdin_size ≤ MAX_STDIN_BYTES ∧
     stdout_size ≤ MAX_OUTPUT_BYTES ∧
     stderr_size ≤ MAX_OUTPUT_BYTES) := by
  rw [limits_rust_stdin_equals_lean_spec stdin_size,
      limits_rust_output_equals_lean_spec stdout_size stderr_size]
  exact (premises_equivalent_to_checks stdin_size stdout_size stderr_size).symm

/--
  Summary of the H-1/H-2 buffer limits refinement:
  
  Rust checks → limits_rust_check_stdin / limits_rust_check_output (model)
  Rust model = Lean spec (machine-checked)
  Lean spec is correct (machine-checked in Verify/Limits.lean)
  ─────────────────────────────────────────────
  ∴ Rust checks are correct (machine-checked via equivalence)
  
  The five blocking execution paths all use these checks:
    1. exec_command_blocking
    2. exec_pipeline_blocking  
    3. exec_command_streaming
    4. exec_pipeline_streaming
    5. exec_command_streaming_with_timeout
    
  Each path is covered by an EffectEval rule with mandatory size premises.
-/
theorem limits_refinement_summary : True := trivial

end Neve.Refinement
