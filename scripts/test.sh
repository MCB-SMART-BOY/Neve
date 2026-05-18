#!/bin/bash
# Neve test runner — uses cargo examples (official Rust toolchain)
# Flags: --clippy, --hunt, --diff, --all, --neve

for arg in "$@"; do
    case "$arg" in
        --neve) exec cargo run -q -p neve -- run scripts/test.neve "$@" 2>/dev/null ;;
        --hunt) exec cargo run -q -p neve --example bug_hunt ;;
        --diff) exec cargo run -q -p neve --example gen_diff_test -- -n 50 -d 3 ;;
        --all)
            cargo run -q -p neve --example test_runner -- --clippy
            echo; cargo run -q -p neve --example bug_hunt
            echo; cargo run -q -p neve --example gen_diff_test -- -n 100 -d 4
            exit $? ;;
    esac
done

# Default: cargo test runner
cargo run -q -p neve --example test_runner "$@"
