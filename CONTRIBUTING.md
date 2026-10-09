# Contributing to Stitch

Thank you for your interest in contributing to `stitch-rs`.

## Engineering Principles

1. **The Scrooge Systems Mindset (Mechanical Sympathy)**:
   - Design data structures with CPU cache line (64 bytes) awareness and zero unnecessary padding.
   - Core hot-paths must be monomorphic, statically composed, and zero-allocation (`no_std` compatible).
2. **Explicit Failure & Anti-Silent Suppression**:
   - Never swallow or mask errors. Strictly avoid dummy fallbacks, silent truncation, or unlogged ignores.
   - Failures must be typed via domain types (`Result<T, E>` / `FlowControl::Halt`).
3. **Conventional Commits**:
   - All commits must follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:
     `<type>(<optional-scope>): <description in imperative mood>`.
   - Types: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`.
4. **Branching Model**:
   - Active development takes place on the `dev` branch.
   - Feature branches branch from `dev` and merge back via Pull Request.
   - Production releases and tags are cut from `main`.
5. **Zero Emojis**:
   - Emojis are strictly forbidden across code, comments, commit messages, PR titles/bodies, and issue trackers.

## Pre-PR Verification

Before submitting a Pull Request, ensure that:

- `cargo fmt --all -- --check` passes cleanly.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` reports 0 warnings.
- `cargo test --workspace` passes cleanly.
- `cargo test --workspace --no-default-features` passes cleanly.
- `cargo doc --workspace --no-deps --all-features` and `--no-default-features` generate zero warnings.
- All `unsafe` blocks include clear `// SAFETY:` justifications.
