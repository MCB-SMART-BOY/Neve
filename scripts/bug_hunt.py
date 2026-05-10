#!/usr/bin/env python3
"""Bug Hunter: use Lean spec as oracle to find Rust implementation bugs."""

import subprocess, os, sys

NEVE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

def run_rust(source, timeout=30):
    tmp = os.path.join(NEVE_DIR, "tmp_bug.neve")
    with open(tmp, "w") as f: f.write(source)
    neve_bin = os.path.join(NEVE_DIR, "target", "release", "neve")
    if os.path.exists(neve_bin):
        r = subprocess.run([neve_bin, "run", tmp],
            capture_output=True, text=True, cwd=NEVE_DIR, timeout=timeout)
    else:
        r = subprocess.run(["cargo", "run", "-q", "-p", "neve", "--", "run", tmp],
            capture_output=True, text=True, cwd=NEVE_DIR, timeout=timeout)
    os.remove(tmp)
    return r.stdout.strip(), r.stderr.strip(), r.returncode

def check(name, src, expected, desc, bugs):
    stdout, stderr, code = run_rust(src)
    result = stdout.strip().lower()
    exp = str(expected).lower()
    ok = (exp in result) or (result == exp)
    if ok:
        print(f"  [PASS] {name}: {desc}")
    else:
        bugs.append((name, expected, result, desc, stderr[:200]))
        print(f"  [BUG]  {name}: expected='{expected}', got='{result[:60]}'")

def main():
    bugs = []
    print("=" * 55)
    print("  Bug Hunt: stdin limits (H-1)")
    print("=" * 55)
    check("stdin-ok", 'let c=io.commandWith(#{program="cat",args=[],stdin="hi"});let r=io.execCommand(c);println(io.processSuccess(r))', "true", "small stdin works", bugs)
    check("no-stdin", 'let r=io.execCommand(io.command("echo",["hello"]));println(io.processSuccess(r))', "true", "no stdin works", bugs)
    check("pipeline-stdin", 'let c=io.commandWith(#{program="cat",args=[],stdin="pipe-in"});let p=io.pipeline([c,io.command("cat",[])]);let r=io.execPipeline(p);println(io.processStdout(r))', "pipe-in", "pipeline stdin flows", bugs)

    print("\n" + "=" * 55)
    print("  Bug Hunt: output capture (H-2)")
    print("=" * 55)
    check("capture-stdout", 'let r=io.execCommand(io.command("echo",["hello world"]));println(io.processStdout(r))', "hello world", "stdout captured", bugs)
    check("capture-stderr", 'let r=io.execCommand(io.command("sh",["-c","echo err>&2"]));println(io.processStderr(r))', "err", "stderr captured", bugs)
    check("exit-code-0", 'let r=io.execCommand(io.command("true",[]));println(io.processSuccess(r))', "true", "exit 0 is success", bugs)
    check("exit-code-1", 'let r=io.execCommand(io.command("false",[]));println(io.processSuccess(r))', "false", "exit 1 is failure", bugs)

    print("\n" + "=" * 55)
    print("  Bug Hunt: env filtering (M-4)")
    print("=" * 55)
    check("safe-env", 'let c=io.commandWith(#{program="sh",args=["-c","echo $FOO"],env=#{FOO="bar"}});let r=io.execCommand(c);println(io.processStdout(r))', "bar", "safe env passes", bugs)
    # Test LD_PRELOAD stripping: use test -z to check if var is empty
    check("ld-preload-stripped", 'let c=io.commandWith(#{program="sh",args=["-c","test -z $LD_PRELOAD && echo stripped || echo LEAK"],env=#{LD_PRELOAD="/tmp/evil.so"}});let r=io.execCommand(c);println(io.processStdout(r))', "stripped", "LD_PRELOAD stripped from child env", bugs)

    print("\n" + "=" * 55)
    print("  Bug Hunt: spawn/await lifecycle")
    print("=" * 55)
    check("spawn-await", 'let c=io.command("echo",["spawned"]);let t=io.taskCommand(c);let r=io.awaitTask(t);println(io.processStdout(r))', "spawned", "spawn/await works", bugs)

    print(f"\n{'='*55}")
    if bugs:
        print(f"  {len(bugs)} POTENTIAL BUGS FOUND:")
        for name, exp, got, desc, _ in bugs:
            print(f"    {name}: {desc}")
            print(f"    expected={exp}, got={got}")
        print(f"{'='*55}")
        return 1
    else:
        print(f"  All tests passed. No bugs found.")
        print(f"{'='*55}")
        return 0

if __name__ == "__main__":
    sys.exit(main())
