### Summary

- High-level context and architectural motivation for this change.

### Changes

- **Core**: Structural changes, memory layouts, algorithms (`stitch-core`).
- **Macros**: Compile-time syntax inspection, procedural code generation (`stitch-macros`).
- **CLI**: Architecture-as-Code tooling, validators, scorecard (`stitch-cli`).
- **Tooling / DX**: Tests, benchmarks, CI workflows, documentation.

### Verification

- [ ] `cargo fmt --all -- --check` passes cleanly.
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` reports 0 warnings.
- [ ] `cargo test --workspace` passes cleanly.
- [ ] `cargo test --workspace --no-default-features` passes cleanly.
- [ ] `cargo doc --workspace --no-deps --all-features` and `--no-default-features` pass cleanly.
- [ ] No emojis present in code, commits, or documentation.
- [ ] All unsafe blocks include explicit `// SAFETY:` justifications.
