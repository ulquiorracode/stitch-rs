# Stitch-rs Roadmap

## v0.1.0 — Foundation & Pure Monomorphic U-Cycle ✅

**Goal:** Establish the monomorphic compile-time U-cycle middleware pattern and baseline repository structure.

- [x] Initial release of `stitch-rs`.
- [x] Static compile-time pipeline composition (`Pipeline`).
- [x] Symmetrical U-cycle execution (`on_enter` descent, `on_exit` ascent).
- [x] Core `#![no_std]` compliance without compulsory heap allocators.
- [x] Baseline GitHub Actions CI matrix with automated clippy, formatting, and unit tests.

---

## v0.2.0 — The Sewing Machine Architecture (SMA) & Architecture-as-Code ✅

**Goal:** Deliver complete industrial-grade architectural contracts, sealed topological boundaries, AST macro linters, and the standalone `stitch` AaC CLI.

- [x] **Core Monomorphism & Safety**:
  - [x] Closed topological pipeline composition via unnameable sealed trait `PipelineChain`.
  - [x] Encapsulate `TerminalNode` and `StackNode` constructors and fields (`pub(crate)`).
  - [x] Symmetrical U-cycle error handling: `FlowControl::Halt` executes halting layer's own `on_exit` telemetry.
  - [x] Compile-time L1D cache-line alignment contract (`ASSERT_CACHE_ALIGNED`) for `Blackboard` implementations (`#[repr(C, align(64))]`).
  - [x] Contract Invariant M7 panic policy & perimeter `dispatch_isolated` error barrier under `feature = "std"`.
  - [x] Hexagonal SPI boundaries: `Port`, `Adapter<P>` with statically bounded `type TargetPort`, and `Hub`.
  - [x] Transparent 64-bit register identity tokens (`StitchToken`, `StitchId`, `RawToken`, `RawId`).
- [x] **Procedural Macros & Architecture-as-Code**:
  - [x] Suffix enforcement (`SMA-TAXO-*` for `Layer`, `Terminal`, `Port`, `Adapter`, `Hub`, `Token`, `Id`).
  - [x] Recursive heap allocation ban (`SMA-SCROOGE-010`) in hot-path structs.
  - [x] Traversal hot-path denylist (`SMA-HOTPATH-020`..`022` for allocations and panics in `on_enter`/`on_exit`).
- [x] **CLI Tool Suite (`stitch`)**:
  - [x] Multi-threaded AST syntax validator (`stitch check`).
  - [x] Scrooge struct alignment fixer (`stitch fix --scrooge`).
  - [x] Cache line crossing and padding auditor (`stitch metrics`).
  - [x] Multidimensional Health Scorecard engine (`stitch health`).
  - [x] Pipeline DAG visualization generator (`stitch graph`).
- [x] **Documentation & Community Standards**:
  - [x] Empirical Criterion benchmark matrix comparing against raw inlining, `Box<dyn Layer>`, and `tower::Service`.
  - [x] Full `docs/` suite: `docs/BENCHMARKS.md`, `docs/TOOLING_GUIDE.md`, `docs/INTEGRATION_GUIDE.md`.
  - [x] Community governance: `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `.editorconfig`, `.gitattributes`, issue and PR templates.

---

## v0.3.0 — Unified CQS Protocol & Zero-Alloc Dynamic Middleware ✅

**Goal:** Bridge macro and micro-level CQS with GoldSrc.rs, establish triple-shield Architecture-as-Code CQS enforcement, and introduce high-performance zero-alloc dynamic dispatch alternatives.

- [x] **Unified CQS Protocol (`stitch-core::cqs`)**:
  - [x] Reintroduce formal, verified `Query<TCtx, TResult, TErr>` with strict `&TCtx` immutable semantics.
  - [x] Implement `CommandPermit` linear capability token to prevent interior mutability bypasses (`RefCell`, `Mutex`) during queries.
  - [x] Universal property-level CQS traits (`PropRead<Target>`, `PropWrite<Target>`) bridging `GoldSrc.rs` `PropGet`/`PropSet`.
- [x] **Triple-Shield CQS Enforcement (`stitch-macros` & `stitch-cli`)**:
  - [x] `#[stitch::query]` macro: Rejects mutable parameters, interior mutability locks, and assignment expressions at compile time.
  - [x] `#[stitch::command]` macro: Validates mutable execution semantics and command naming conventions.
  - [x] Rule `SMA-CQS-050`: AST linter checks prohibiting illegal state mutations in queries repository-wide.
  - [x] Rule `SMA-CQS-051`: Static check prohibiting hybrid types implementing both `Query` and `Command`.
- [x] **Zero-Alloc Dynamic Middleware Alternatives**:
  - [x] Implement `define_layer_enum!` macro: Closed-set enum dispatch achieving **~1.48 ns** (3.3x faster than `Box<dyn Layer>`).
  - [x] Implement `StatelessLayerTable`: Contiguous slice dispatcher for stateless plugins.
- [x] **Peripheral Async Preparation (`stitch-core::outbox`)**:
  - [x] Fixed-capacity stack-allocated `Outbox<TEvent, CAP>` buffer for zero-alloc event draining from synchronous terminals.

## Phase 4: Async Pipelines & Autonomous Egress (v0.4.0)

- [ ] **Dedicated `stitch-async` Crate**:
  - [ ] Implement `AsyncPipeline` with zero heap allocation per tick using RPITIT / `async fn in traits` (Rust 1.85+).
  - [ ] Zero-cost bridge between synchronous `#![no_std]` U-cycles and async runtimes (Tokio / embassy / smol).
  - [ ] Event-driven egress worker consuming `Outbox` batches via channel streaming without blocking hot simulation frames.
- [ ] **Cross-Platform Benchmarking Suite**:
  - [ ] Integrate Hyperfine, cargo-flamegraph, and DHAT profiling harnesses across benchmarks.
  - [ ] Rigorous cache miss, branch misprediction, and heap allocation ledger reporting.

