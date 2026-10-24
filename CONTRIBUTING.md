# Contributing to Stitch (`stitch-rs`)

Thank you for your interest in contributing to **`stitch-rs`**! We welcome contributions of all kinds: pipeline stages, performance optimizations, bug fixes, architecture improvements, documentation, and ecosystem integrations.

Please take a moment to review these guidelines before submitting code.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Branching Strategy](#branching-strategy)
- [Conventional Commits](#conventional-commits)
- [Development Setup](#development-setup)
- [Coding Standards & Core Invariants](#coding-standards--core-invariants)
- [Pull Request Workflow](#pull-request-workflow)

---

## Code of Conduct

This project and everyone participating in it is governed by our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

---

## Branching Strategy

Our repository strictly adheres to the standard GitFlow-inspired branching strategy:

- **`main`**: Production/stable releases only. Protected branch; direct commits are strictly prohibited.
- **`dev`**: Active development integration branch. Protected branch; all feature, fix, and dependency PRs must target `dev`.
- **`feature/<name>`**: New features and enhancements, branched from `dev` and merged back into `dev` via PR.
- **`fix/<name>`**: Bug fixes, branched from `dev` (or `main` for critical production hotfixes).

```text
main   ───────────────────────────● (v0.3.0)
                                 /
dev    ──────●──────────●───────●─── (Active Development)
              \        /
feature/layer  ●──────●             (Feature Branch)
```

---

## Conventional Commits

Commit messages must follow the [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification:

```text
<type>(<optional-scope>): <description in imperative mood>

[optional body]

[optional footer(s)]
```

### Allowed Types

| Type | Description | Example |
| :--- | :--- | :--- |
| `feat` | A new feature or capability | `feat(arch): purge legacy Hub in favor of Port-Adapter-Provider triad` |
| `fix` | A bug fix | `fix(pipeline): correct terminal handler abort propagation` |
| `perf` | A code change that improves performance | `perf(dispatch): monomorphize layer descent for zero-cost execution` |
| `refactor` | Code change that neither fixes a bug nor adds a feature | `refactor(taxonomy): align provider traits to canonical triad` |
| `docs` | Documentation changes | `docs(architecture): add U-cycle sequence diagram` |
| `test` | Adding or updating tests | `test(alloc): verify 0 heap allocations across dispatch loop` |
| `build` | Changes to build configuration or dependencies | `build(deps): bump tokio from 1.53.2 to 1.53.3` |
| `ci` | Changes to CI/CD pipeline | `ci: add deny audit step` |
| `chore` | Routine maintenance tasks | `chore: clean up cargo manifest` |

---

## Development Setup

### Prerequisites

- **Rust**: Rust 2024 Edition (MSRV: `1.85.0` or newer stable).
- **Cargo Tools**:

  ```bash
  rustup component add clippy rustfmt
  ```

### Local Verification Suite

Run the full local verification pipeline before submitting any PR:

```bash
# 1. Format check
cargo fmt --all -- --check

# 2. Strict Clippy check with all targets and features
cargo clippy --workspace --all-targets --all-features -- -D warnings

# 3. Comprehensive tests
cargo test --workspace --verbose
cargo test --workspace --no-default-features --verbose

# 4. Benchmark harness compilation check
cargo bench --no-run

# 5. Documentation build check
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
```

---

## Coding Standards & Core Invariants (The SMA Principles)

1. **The Sewing Machine Architecture (SMA)**: Monomorphic compile-time composition over dynamic dispatch (`Box<dyn ...>`).
2. **Strictly Bounded U-Cycle Traversal**: `on_enter` descent, terminal handler invocation, `on_exit` ascent.
3. **Zero-Allocation Discipline**: The core pipeline loop must be strictly zero-allocation on the stack.
4. **Clean CQS Boundaries**: State mutations and command queries must be decoupled cleanly.
5. **Strict `#![no_std]` Compliance**: `stitch-core` must compile cleanly without the standard library.

---

## Pull Request Workflow

1. Fork or branch from `dev`.
2. Implement your changes surgically, preserving performance and memory layout invariants.
3. Verify all checks pass locally using the commands listed in [Local Verification Suite](#local-verification-suite).
4. Open a Pull Request targeting `dev` with a clear description of changes and test evidence.
