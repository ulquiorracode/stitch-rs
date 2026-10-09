# Changelog

All notable changes to `stitch-rs` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.3.0] - 2026-10-09

### Added

- **Unified CQS Protocol (`stitch-core::cqs`)**:
  - Reintroduced formal, verified `Query<TCtx, TResult, TErr>` trait guaranteeing strictly immutable `&TCtx` context execution.
  - Implemented typed `Pipeline::for_query()` constructor ensuring pipelines dispatch query intents exclusively.
  - Linear witness capability token `CommandPermit<'a>` preventing interior mutability leaks (`RefCell`, `Mutex`) during query execution.
  - Fine-grained property CQS contracts `PropRead<Target>` and `PropWrite<Target>`.
- **Triple-Shield CQS Architecture-as-Code Enforcement**:
  - Procedural macros `#[stitch::query]` and `#[stitch::command]`.
  - Architecture linter rule `SMA-CQS-050` rejecting mutable references `&mut` in query methods across both `stitch-macros` and `stitch-cli`.
  - CLI category filter support: `stitch check --category cqs` or `stitch check --cqs`.
- **Zero-Alloc Dynamic Middleware Alternatives**:
  - `define_layer_enum!` macro: Closed-set enum dispatch achieving **~1.48 ns** (3.3x faster than `Box<dyn Layer>` at ~4.87 ns).
  - `StatelessLayerTable`: Contiguous flat slice of function pointers for dynamically configured plugins in zero-alloc contiguous memory.
- **Peripheral Async Outbox Pattern**:
  - Stack-allocated, fixed-capacity ring buffer `Outbox<TEvent, const CAP: usize>` for staging domain events in synchronous U-cycle terminals without heap allocation.
- **Criterion Benchmark Suite**:
  - Added empirical benchmarks for `stitch_layer_enum` and `stitch_stateless_table`.

## [0.2.0] - 2026-10-09

### Added

- **Core Systems Monomorphism (`stitch-core`)**:
  - `PipelineChain` sealed trait architecture ensuring closed topological composition with open extensible behaviors.
  - Symmetrical U-cycle execution: `FlowControl::Halt` now triggers halting layer's own `on_exit` telemetry before upward ascent.
  - Cache-line alignment compile-time contract (`ASSERT_CACHE_ALIGNED`) for `Blackboard` implementations (`#[repr(C, align(64))]`).
  - Typed command dispatch via `Pipeline::for_command()` enforcing `Command<TCtx, TOutcome, TErr>`.
  - Transparent 64-bit register tokens and IDs (`StitchToken`, `StitchId`, `RawToken`, `RawId`).
  - Perimeter panic isolation barrier (`Pipeline::dispatch_isolated`) under `feature = "std"`.
  - Hexagonal SPI boundaries: `Port`, `Adapter<P>` with statically bounded `type TargetPort: Port + ?Sized`, and `Hub`.
- **Procedural Macro Architecture Linter (`stitch-macros`)**:
  - `#[stitch::layer]`, `#[stitch::terminal]`, `#[stitch::hub]`.
  - `#[stitch::port]`, `#[stitch::adapter]`.
  - `#[stitch::blackboard]`, `#[stitch::token]`, `#[stitch::id]`.
  - Compile-time AST inspection: Suffix enforcement (`SMA-TAXO-*`), recursive heap allocation bans (`SMA-SCROOGE-010`), and hotpath denylist (`SMA-HOTPATH-020`..`022`).
- **Architecture-as-Code CLI (`stitch-cli`)**:
  - Multi-threaded AST syntax scanner (`stitch check`) powered by `syn` and `rayon`.
  - Multidimensional Health Scorecard engine (`stitch health`) evaluating Taxonomy, Scrooge, Hotpath, DIP, and Concurrency.
  - Automated Scrooge struct alignment fixer (`stitch fix --scrooge`) eliminating internal padding waste.
  - Interactive DAG visualizer (`stitch graph`) rendering Mermaid sequence diagrams and flowcharts.
  - Memory padding and cache-line crossing auditor (`stitch metrics`).
- **Examples & Performance Benchmarks**:
  - `01_auth_logging_u_cycle.rs`: Multi-layer authentication, audit logging, and terminal execution.
  - `02_cache_short_circuit.rs`: Zero-allocation cache hit short-circuit traversal.
  - `03_no_std_embedded.rs`: `#![no_std]` bare-metal pipeline showcase with hardware registers (host-executed test).
  - Criterion benchmark suite measuring synchronous pipeline throughput against `Box<dyn Layer>` and hand-inlined baseline, with an asynchronous `tower::Service` reference included for cross-paradigm architectural context.
- **Community & Governance Standards**:
  - `CONTRIBUTING.md`: Scrooge engineering principles, failure explicit rules, and branch guidelines.
  - `SECURITY.md`: Coordinated vulnerability disclosure policy and supported versions.
  - GitHub integration: `PULL_REQUEST_TEMPLATE.md`, `dependabot.yml`, `bug_report.yml`, and `feature_request.yml`.

### Changed

- Encapsulated `TerminalNode` and `StackNode` internal fields and constructors (`pub(crate)`) to prevent bypassing cache-line alignment checks. Code constructing nodes directly should use `Pipeline::on_terminal(terminal)` and `.wrap(layer)`.
- Made `sealed` module private in `stitch-core` to enforce unnameable trait bound.
- Updated license to dual `MIT OR Apache-2.0`.
- Set MSRV to `1.85.0`.

### Removed

- **Strict Backward-Compatibility Purge**:
  - Removed unused and incomplete `Query` trait from `stitch-core`.
  - Removed deprecated aliases `Admission` and `BlackboardFlow` (use `FlowControl`).
  - Removed deprecated traits `Middleware` and `TerminalHandler` (use `Layer` and `Terminal`).
  - Removed deprecated type alias `Machine` (use `Pipeline`).
  - Removed deprecated methods `use_middleware` (use `wrap`) and `stitch` (use `dispatch`).
  - Removed vacuous trait `PortLayer` in favor of direct `Adapter` target port bounding.
  - Removed duplicate CLI binary `sma` (canonical single binary is `stitch`).
  - Removed facade alias `pub use stitch_macros as sma`.

## [0.1.0] - 2026-10-08

### Added

- Initial release of `stitch-rs`.
- Pure monomorphic U-cycle pipeline framework (`Pipeline`, `Machine`).
- Sewing Machine Architecture (SMA) pattern abstraction (`on_enter` descent and `on_exit` ascent phases).
- Zero-cost static generics chain without dynamic heap dispatch (`Box<dyn ..>`).
- Strict Command-Query Separation (`CQS`) primitives (`Query`, `Command`).
- Flow control primitives (`FlowControl::Continue`, `FlowControl::Halt`, `FlowControl::EarlyExit`).
- Core `#![no_std]` compatible design without runtime allocator dependencies.
- GitHub Actions CI matrix with automated clippy, formatting, and unit tests.
