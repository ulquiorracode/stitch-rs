# Sewing Machine Architecture (SMA) Technical Reference

**Specification Identifier:** `ARCH-SPEC-SMA-CORE-001`  
**Classification:** Normative Core Systems Specification & Invariant Contracts  
**Status:** Canonical (Single Source of Truth)  

---

## 1. Normative Authority & Contract Hierarchy

This document serves as the **sole normative technical specification** for Sewing Machine Architecture (SMA).

- `ARCHITECTURE.md`: The authoritative reference for memory layouts, byte-level invariants, failure semantics, and concurrency contracts.
- `README.md`: Informative landing page, getting-started examples, and high-level summaries. In any discrepancy, `ARCHITECTURE.md` supersedes `README.md`.

---

## 2. The Monomorphic U-Cycle

The execution pipeline unfolds into a static monomorphic chain with zero dynamic dispatch (`Box<dyn ...>`) and full inlining opportunities.

```text
Intent ─────────► [Layer A: on_enter] ──► [Layer B: on_enter] ──► [Terminal: execute]
                                                                         │
Outcome ◄─────── [Layer A: on_exit]  ◄── [Layer B: on_exit]  ◄─────────┘
                               (The U-Cycle Traversal)
```

### FlowControl Transitions

1. `FlowControl::Proceed(intent)`: Continues descent to the inner layer or terminal.
2. `FlowControl::ShortCircuit(outcome)`: Halts further descent; immediately begins ascent starting at the current layer with `Ok(outcome)`.
3. `FlowControl::Halt(error)`: Halts further descent; immediately begins ascent starting at the current layer with `Err(error)`.

### Closed Topology vs. Open Behavior

The pipeline topology is **statically closed**:

- [`PipelineChain`] is sealed via an unnameable `Sealed` supertrait in a private module to prevent external crates from crafting unverified chain nodes.
- Intermediate chain nodes ([`StackNode`], [`TerminalNode`]) have encapsulated fields and private constructors (`pub(crate)`), ensuring pipelines can only be constructed via [`Pipeline::on_terminal`] and [`.wrap`].
- Both [`Pipeline::dispatch`] and direct [`PipelineChain::cycle`] statically assert L1D cache-line alignment (`ASSERT_CACHE_ALIGNED`) for the context `TCtx`.

Conversely, behavior is **open**: layers and terminals are freely composable across generic contexts, intents, outcomes, and error domains.

---

## 3. Panic & Resilience Policy (Contract Invariant M7)

The SMA core runtime enforces an explicit failure model with verified boundary contracts:

### 3.1 Total Function Contract on Ascent (`on_exit`)

- `on_exit` operates over an already materialized `&mut Result<TOutcome, TErr>`.
- Architectural Invariant: Implementations of `on_exit` **must never panic**.
- **Enforcement Scope & Known Limitations**:
  - Direct invocations of `panic!`, `todo!`, `unimplemented!`, and `unreachable!` inside `on_enter` and `on_exit` are inspected and rejected by the `#[stitch::layer]` procedural macro and `stitch check` (`SMA-HOTPATH-020`).
  - *Opt-in Disclaimer*: Enforcement requires decorating layer implementations with `#[stitch::layer]` or running `stitch check`. A bare, unannotated `impl Layer` is not blocked by `rustc` alone.
  - *Best-Effort Boundary*: Static AST inspection cannot detect indirect panics occurring inside un-inlined external function calls, dynamic dispatch, or panic-equivalent intrinsics (`std::process::abort`, `core::hint::unreachable_unchecked`, `.unwrap()`, `.expect()`).

### 3.2 No ScopeGuards in `#![no_std]` Core

- The core pipeline intentionally avoids RAII scope guards or cleanup destructors during stack unwinding.
- In bare-metal, kernel, and realtime game engine runtimes, a secondary panic inside a destructor unconditionally triggers `abort()`, crashing the host environment.

### 3.3 Context Poisoning on Traversal Abort

- If an unhandled panic occurs in any layer or terminal, traversal aborts immediately and outer ascent phases are skipped.
- The material context (`Blackboard`) is left in an **unspecified/poisoned state**.
- Re-entering the pipeline with the same context without complete reinitialization is an application-level contract violation.

### 3.4 Perimeter Isolation Barrier (`std` Feature)

- For userspace host runtimes (FFI, WebAssembly, game engines like GoldSrc, dynamic plugins), `Pipeline::dispatch_isolated` provides a perimeter panic barrier.
- **Important `panic = "abort"` Caveat**:
  - `dispatch_isolated` relies on `std::panic::catch_unwind`.
  - When the target or profile is compiled with `panic = "abort"` (common in embedded, WebAssembly, and optimized release builds), `catch_unwind` is a no-op; any panic unconditionally aborts the entire process without unwinding.
  - Hosts requiring panic containment must ensure their build target uses the `panic = "unwind"` runtime strategy.

---

## 4. Command-Query Segregation (CQS) Contract

The CQS subsystem partitions intent dispatch into pure reads and state-mutating transactions:

### 4.1 Scope of `for_query` vs Layer Execution

- `Pipeline::for_query()` statically constrains the **terminal node** to implementations of `Query<TCtx, TResult, TErr>`, guaranteeing the terminal receives an immutable `&TCtx`.
- **Surrounding Layer Scope Contract**:
  - Middleware layers wrapping a query pipeline retain `&mut TCtx` in their `on_enter` / `on_exit` signatures solely for operational telemetry (e.g. rate-limiting counters, query audit logging, timing blackboards).
  - Layers MUST NOT mutate domain entity state during query traversal.
  - In scenarios where domain entities employ interior mutability (`RefCell`, `Mutex`), mutation methods MUST require a `CommandPermit` witness.

### 4.2 Linear Witness Token (`CommandPermit<'a, TCtx>`)

- `CommandPermit<'a, TCtx>` acts as a linear capability token required by `PropWrite<Target>::write` or guarded domain setters.
- A `CommandPermit` can only be constructed by exclusively borrowing `&'a mut TCtx`:

  ```rust
  let mut permit = CommandPermit::new(&mut ctx);
  permit.ctx_mut().set_val(42);
  ```

- Because a query terminal only receives `&TCtx`, it is structurally impossible for terminal query code to fabricate a `CommandPermit`, preventing illegal interior mutations at compile time.

---

## 5. Dynamic Middleware & Dispatch Semantics

Where compile-time monomorphism cannot be used (e.g., dynamic plugins loaded at runtime), `stitch-rs` provides two zero-allocation alternatives to `Box<dyn Layer>`:

### 5.1 Closed-Set Enum Dispatch (`define_layer_enum!`)

- Generated enums implement `Layer` directly with compiler jump tables (`match`).
- Benchmarked mean latency: **~1.48 ns** (vs. **5.24 ns** for `Box<dyn Layer>`).

### 5.2 `StatelessLayerTable`: U-Cycle Parity & Short-Circuit Semantics

- Dispatches across a flat slice of function pointer pairs `&[StatelessLayerFn<TCtx, TIntent, TOutcome, TErr>]`.
- **U-Cycle Parity Contract**:
  - `StatelessLayerTable` tracks the exact number of layers that successfully returned `FlowControl::Proceed` during `on_enter`.
  - If a layer short-circuits (`ShortCircuit`) or halts (`Halt`), `on_exit` is called in reverse order **only for the layers that completed `on_enter`**, exactly preserving the U-cycle invariant of static monomorphic pipelines.
  - Unreached inner layers never receive `on_exit`.
- **Intent Cloning Requirement**:
  - In dynamic function-pointer traversal, advancing `intent` across layer boundaries requires `TIntent: Clone`.

---

## 6. Peripheral Async Outbox Pattern

- `Outbox<TEvent: Event, const CAP: usize>` provides a stack-allocated, fixed-capacity linear staging buffer.
- Terminals push domain events to the outbox synchronously during `execute`.
- If capacity is exceeded, `push` safely returns `Err(event)` without data loss.
- `drain()` employs panic-safe iteration, clearing elements as they are yielded to prevent stale retention if a consumer panics.
