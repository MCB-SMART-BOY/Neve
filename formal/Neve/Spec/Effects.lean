/-
  Effectful evaluation v3 — extends BigStep with complete I/O semantics.

  Blocking:   execCommand, execPipeline
  Deferred:   spawn, awaitTask, awaitTaskTimeout
  Streaming:  execCommandStreaming, execPipelineStreaming,
              execCommandStreamingWithTimeout, execPipelineStreamingWithTimeout,
              readFileLines
  File I/O:   readFile, writeFile
-/
import Neve.Spec.Syntax
import Neve.Spec.Eval

namespace Neve

open Expr Value

-- ============================================================
-- I/O State
-- ============================================================

structure IOState where
  stdin  : String
  stdout : String
  stderr : String
  deriving Repr

def IOState.empty : IOState := { stdin := "", stdout := "", stderr := "" }

-- ============================================================
-- Size limits (synced with Rust: crates/neve-std/src/io/mod.rs)
-- ============================================================

def MAX_STDIN_BYTES : Nat := 10 * 1024 * 1024       -- 10 MB
def MAX_OUTPUT_BYTES : Nat := 50 * 1024 * 1024      -- 50 MB
def MAX_STREAM_LINES : Nat := 100_000               -- 100k lines

-- ============================================================
-- Abstract models
-- ============================================================

structure ProcessOutput where
  code   : Int
  stdout : String
  stderr : String
  deriving Repr

axiom exec_process (program : String) (args : List String) (cwd : Option String)
    (env : List (String × String)) (stdin : String) : ProcessOutput

axiom fileContent (path : String) : String

-- ============================================================
-- Effectful evaluation rules
-- ============================================================

inductive EffectEval : Env → IOState → Expr → Value → IOState → Prop where

  -- === Pure fragment ===
  | pure (env : Env) (σ : IOState) (e : Expr) (v : Value) :
      BigStep env e v →
      EffectEval env σ e v σ

  -- ================================================================
  -- Blocking process execution
  -- ================================================================

  -- === io.execCommand (blocking) ===
  | execCommand (env : Env) (σ : IOState)
      (program_arg : Expr) (args_arg : Expr) (stdin_arg : Expr)
      (program : String) (args : List String) (stdin_str : String)
      (output : ProcessOutput)
      (hstdin_len : stdin_str.length ≤ MAX_STDIN_BYTES)
      (hout_len : output.stdout.length ≤ MAX_OUTPUT_BYTES)
      (herr_len : output.stderr.length ≤ MAX_OUTPUT_BYTES) :
      BigStep env program_arg (string program) →
      BigStep env args_arg (list (args.map (λ s => string s))) →
      BigStep env stdin_arg (string stdin_str) →
      output = exec_process program args none [] stdin_str →
      EffectEval env σ
        (Expr.builtin "io.execCommand" [program_arg, args_arg, stdin_arg])
        (processResult output.code output.stdout output.stderr)
        { σ with stdout := σ.stdout ++ output.stdout
                 stderr := σ.stderr ++ output.stderr }

  -- === io.execPipeline (blocking) ===
  | execPipeline (env : Env) (σ : IOState)
      (stages : List ProcessOutput) (final : ProcessOutput)
      (hsize : ∀ out ∈ stages, out.stdout.length ≤ MAX_OUTPUT_BYTES
                            ∧ out.stderr.length ≤ MAX_OUTPUT_BYTES)
      (hfinal_out : final.stdout.length ≤ MAX_OUTPUT_BYTES)
      (hfinal_err : final.stderr.length ≤ MAX_OUTPUT_BYTES) :
      EffectEval env σ
        (Expr.builtin "io.execPipeline" [])
        (processResult final.code final.stdout final.stderr)
        { stdout := σ.stdout ++ final.stdout
        , stderr := σ.stderr ++ final.stderr
        , stdin  := σ.stdin }

  -- ================================================================
  -- Deferred execution (spawn/await)
  -- ================================================================

  | spawn (env : Env) (σ : IOState) (body : Expr) :
      EffectEval env σ
        (Expr.builtin "io.spawn" [body])
        (Value.closure "_task" body env)
        σ

  | awaitTask (env : Env) (σ σ' : IOState) (body : Expr) (taskEnv : Env) (v : Value) :
      EffectEval taskEnv σ body v σ' →
      EffectEval env σ
        (Expr.builtin "io.awaitTask" [body])
        v
        σ'

  | awaitTaskTimeout (env : Env) (σ σ' : IOState) (body : Expr) (taskEnv : Env) (timeout : Int) (v : Value) :
      BigStep env (Expr.lit_int timeout) (Value.int timeout) →
      EffectEval taskEnv σ body v σ' →
      EffectEval env σ
        (Expr.builtin "io.awaitTaskWithTimeout" [body, Expr.lit_int timeout])
        (Value.someVal v)
        σ'

  -- === io.awaitTaskWithTimeout (timeout path) ===
  | awaitTaskTimeoutExpired (env : Env) (σ : IOState) (body : Expr) (timeout : Int) :
      BigStep env (Expr.lit_int timeout) (Value.int timeout) →
      EffectEval env σ
        (Expr.builtin "io.awaitTaskWithTimeout" [body, Expr.lit_int timeout])
        Value.noneVal
        σ

  -- ================================================================
  -- Streaming process execution
  -- ================================================================

  -- === io.execCommandStreaming ===
  /--
    Execute a command in streaming mode. stdout is split into lines;
    each line ≤ max_lines total. Size limits enforced as in blocking mode.

    Models: builtin_exec_streaming in crates/neve-eval/src/eval.rs
  -/
  | execCommandStreaming (env : Env) (σ : IOState)
      (program_arg : Expr) (args_arg : Expr) (stdin_arg : Expr)
      (program : String) (args : List String) (stdin_str : String)
      (lines : List String) (output : ProcessOutput)
      (hstdin_len : stdin_str.length ≤ MAX_STDIN_BYTES)
      (hline_count : lines.length ≤ MAX_STREAM_LINES)
      (hout_len : output.stdout.length ≤ MAX_OUTPUT_BYTES)
      (herr_len : output.stderr.length ≤ MAX_OUTPUT_BYTES) :
      BigStep env program_arg (string program) →
      BigStep env args_arg (list (args.map (λ s => string s))) →
      BigStep env stdin_arg (string stdin_str) →
      -- lines are the individual lines from stdout
      -- output.stdout = "\n".intercalate(lines)
      EffectEval env σ
        (Expr.builtin "io.execCommandStreaming" [program_arg, args_arg, stdin_arg])
        (processResult output.code output.stdout output.stderr)
        { σ with stdout := σ.stdout ++ output.stdout
                 stderr := σ.stderr ++ output.stderr }

  -- === io.execPipelineStreaming ===
  /--
    Execute a pipeline in streaming mode. Same limits as blocking pipeline,
    plus line count limit on the final stage output.

    Models: builtin_exec_pipeline_streaming in crates/neve-eval/src/eval.rs
  -/
  | execPipelineStreaming (env : Env) (σ : IOState)
      (stages : List ProcessOutput) (final : ProcessOutput)
      (lines : List String)
      (hsize : ∀ out ∈ stages, out.stdout.length ≤ MAX_OUTPUT_BYTES
                            ∧ out.stderr.length ≤ MAX_OUTPUT_BYTES)
      (hline_count : lines.length ≤ MAX_STREAM_LINES)
      (hfinal_out : final.stdout.length ≤ MAX_OUTPUT_BYTES)
      (hfinal_err : final.stderr.length ≤ MAX_OUTPUT_BYTES) :
      EffectEval env σ
        (Expr.builtin "io.execPipelineStreaming" [])
        (processResult final.code final.stdout final.stderr)
        { stdout := σ.stdout ++ final.stdout
        , stderr := σ.stderr ++ final.stderr
        , stdin  := σ.stdin }

  -- === io.execCommandStreamingWithTimeout ===
  /--
    Streaming command execution with total timeout.
    Returns None (unit) on timeout, Some(result) on completion.

    Models: builtin_exec_streaming_with_timeout in crates/neve-eval/src/eval.rs
  -/
  | execCommandStreamingTimeout (env : Env) (σ : IOState)
      (program_arg : Expr) (args_arg : Expr) (stdin_arg : Expr) (timeout_arg : Expr)
      (program : String) (args : List String) (stdin_str : String) (timeout : Int)
      (lines : List String) (output : ProcessOutput)
      (hstdin_len : stdin_str.length ≤ MAX_STDIN_BYTES)
      (hline_count : lines.length ≤ MAX_STREAM_LINES)
      (hout_len : output.stdout.length ≤ MAX_OUTPUT_BYTES)
      (herr_len : output.stderr.length ≤ MAX_OUTPUT_BYTES) :
      BigStep env program_arg (string program) →
      BigStep env args_arg (list (args.map (λ s => string s))) →
      BigStep env stdin_arg (string stdin_str) →
      BigStep env timeout_arg (Value.int timeout) →
      -- Completed before timeout
      EffectEval env σ
        (Expr.builtin "io.execCommandStreamingWithTimeout" [program_arg, args_arg, stdin_arg, timeout_arg])
        (Value.someVal (processResult output.code output.stdout output.stderr))
        { σ with stdout := σ.stdout ++ output.stdout
                 stderr := σ.stderr ++ output.stderr }

  -- === io.execCommandStreamingWithTimeout (timeout path) ===
  | execCommandStreamingTimeoutExpired (env : Env) (σ : IOState)
      (program_arg : Expr) (args_arg : Expr) (stdin_arg : Expr) (timeout_arg : Expr) (timeout : Int) :
      BigStep env timeout_arg (Value.int timeout) →
      -- Timeout expired before completion: return None
      EffectEval env σ
        (Expr.builtin "io.execCommandStreamingWithTimeout" [program_arg, args_arg, stdin_arg, timeout_arg])
        Value.noneVal
        σ

  -- === io.execPipelineStreamingWithTimeout ===
  | execPipelineStreamingTimeout (env : Env) (σ : IOState)
      (timeout_arg : Expr) (timeout : Int) (final : ProcessOutput)
      (lines : List String)
      (hline_count : lines.length ≤ MAX_STREAM_LINES)
      (hfinal_out : final.stdout.length ≤ MAX_OUTPUT_BYTES)
      (hfinal_err : final.stderr.length ≤ MAX_OUTPUT_BYTES) :
      BigStep env timeout_arg (Value.int timeout) →
      EffectEval env σ
        (Expr.builtin "io.execPipelineStreamingWithTimeout" [timeout_arg])
        (Value.someVal (processResult final.code final.stdout final.stderr))
        { stdout := σ.stdout ++ final.stdout
        , stderr := σ.stderr ++ final.stderr
        , stdin  := σ.stdin }

  -- === io.execPipelineStreamingWithTimeout (timeout path) ===
  | execPipelineStreamingTimeoutExpired (env : Env) (σ : IOState)
      (timeout_arg : Expr) (timeout : Int) :
      BigStep env timeout_arg (Value.int timeout) →
      EffectEval env σ
        (Expr.builtin "io.execPipelineStreamingWithTimeout" [timeout_arg])
        Value.noneVal
        σ

  -- === io.readFileLines ===
  /--
    Read a file line by line, calling a callback for each line.
    Enforces max lines limit.

    Models: builtin_read_file_lines in crates/neve-eval/src/eval.rs
  -/
  | readFileLines (env : Env) (σ : IOState)
      (path_arg : Expr) (path : String) (lines : List String)
      (hline_count : lines.length ≤ MAX_STREAM_LINES) :
      BigStep env path_arg (string path) →
      -- lines = fileContent(path).split("\n")
      EffectEval env σ
        (Expr.builtin "io.readFileLines" [path_arg])
        (list (lines.map (λ s => string s)))
        σ

  -- ================================================================
  -- File I/O
  -- ================================================================

  | readFile (env : Env) (σ : IOState) (path_arg : Expr) (path : String) (content : String) :
      BigStep env path_arg (string path) →
      content = fileContent path →
      EffectEval env σ
        (Expr.builtin "io.readFile" [path_arg])
        (string content)
        σ

  | writeFile (env : Env) (σ : IOState) (path_arg : Expr) (content_arg : Expr) (path content : String) :
      BigStep env path_arg (string path) →
      BigStep env content_arg (string content) →
      EffectEval env σ
        (Expr.builtin "io.writeFile" [path_arg, content_arg])
        Value.unit
        σ

  -- === Bytes I/O ===

  | readFileBytes (env : Env) (σ : IOState) (path_arg : Expr) (path : String) (data : List Nat) :
      BigStep env path_arg (string path) →
      -- data = bytesOfFile(path) (abstract axiom)
      EffectEval env σ
        (Expr.builtin "io.readFileBytes" [path_arg])
        (bytes data)
        σ

  | writeFileBytes (env : Env) (σ : IOState) (path_arg : Expr) (data_arg : Expr) (path : String) (data : List Nat) :
      BigStep env path_arg (string path) →
      BigStep env data_arg (bytes data) →
      EffectEval env σ
        (Expr.builtin "io.writeFileBytes" [path_arg, data_arg])
        Value.unit
        σ

  -- ================================================================
  -- Retry / Ensure (Phase 4 shell capability)
  -- ================================================================

  -- === io.retry ===
  /--
    Retry an effectful function up to maxAttempts times with exponential backoff.
    Returns the first successful result, or an error after all attempts fail.
  -/
  | retry_success (env : Env) (σ σ' : IOState) (func_arg : Expr) (max_arg : Expr) (backoff_arg : Expr)
      (body : Expr) (taskEnv : Env) (v : Value) (n : Int) (ms : Int) :
      BigStep env max_arg (Value.int n) →
      BigStep env backoff_arg (Value.int ms) →
      -- The function (a closure) is evaluated in its captured environment
      BigStep env func_arg (Value.closure "_retry" body taskEnv) →
      -- First attempt succeeds
      EffectEval taskEnv σ body v σ' →
      EffectEval env σ
        (Expr.builtin "io.retry" [func_arg, max_arg, backoff_arg])
        v
        σ'

  | retry_failure (env : Env) (σ : IOState) (func_arg : Expr) (max_arg : Expr) (backoff_arg : Expr)
      (body : Expr) (taskEnv : Env) (n : Int) (ms : Int) :
      BigStep env max_arg (Value.int n) →
      BigStep env backoff_arg (Value.int ms) →
      BigStep env func_arg (Value.closure "_retry" body taskEnv) →
      -- All attempts fail (abstracted: the evaluator loops n times)
      -- Returns unit to indicate exhaustion
      EffectEval env σ
        (Expr.builtin "io.retry" [func_arg, max_arg, backoff_arg])
        Value.unit
        σ

  -- === io.ensure ===
  /--
    Repeatedly check a condition with a timeout.
    Returns true if the condition becomes true within the timeout.
    Returns false if timeout expires.
  -/
  | ensure_success (env : Env) (σ σ' : IOState) (check_arg : Expr) (timeout_arg : Expr) (interval_arg : Expr)
      (body : Expr) (taskEnv : Env) (timeout : Int) (interval : Int) :
      BigStep env timeout_arg (Value.int timeout) →
      BigStep env interval_arg (Value.int interval) →
      BigStep env check_arg (Value.closure "_ensure" body taskEnv) →
      -- Check succeeds (body evaluates to true)
      EffectEval taskEnv σ body (Value.bool true) σ' →
      EffectEval env σ
        (Expr.builtin "io.ensure" [check_arg, timeout_arg, interval_arg])
        (Value.bool true)
        σ'

  | ensure_timeout (env : Env) (σ : IOState) (check_arg : Expr) (timeout_arg : Expr) (interval_arg : Expr)
      (body : Expr) (taskEnv : Env) (timeout : Int) (interval : Int) :
      BigStep env timeout_arg (Value.int timeout) →
      BigStep env interval_arg (Value.int interval) →
      BigStep env check_arg (Value.closure "_ensure" body taskEnv) →
      -- Timeout expires before check succeeds
      EffectEval env σ
        (Expr.builtin "io.ensure" [check_arg, timeout_arg, interval_arg])
        (Value.bool false)
        σ

end Neve
