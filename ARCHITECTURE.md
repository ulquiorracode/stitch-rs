# Sewing Machine Architecture (SMA) Technical Reference

**Specification Identifier:** `ARCH-SPEC-SMA-CORE-001`  
**Classification:** Core Systems Specification & Invariant Contracts  

---

## 1. The Monomorphic U-Cycle

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

---

## 2. Panic & Resilience Policy (Contract Invariant M7)

The SMA core runtime enforces a strict, explicit failure model:

1. **Total Function Contract on Ascent (`on_exit`)**:
   - `on_exit` is a total function operating over an already materialized `&mut Result<TOutcome, TErr>`.
   - Implementations of `on_exit` **must never panic** under any circumstances.
   - Banned macros (`panic!`, `todo!`, `unimplemented!`, `unreachable!`) are statically rejected by compiler and AST linters (`SMA-HOTPATH-020`).

2. **No ScopeGuards in `#![no_std]` Core**:
   - The core pipeline does not use RAII scope guards or destructors during stack unwinding.
   - In bare-metal or realtime kernels, a secondary panic inside a destructor unconditionally triggers `abort()`, crashing the entire system.

3. **Poisoning & Traversal Abort**:
   - An unhandled panic in any layer or terminal immediately aborts the traversal. Outer ascent phases are skipped.
   - The material context (`Blackboard`) is considered corrupted/poisoned.
   - Application re-entry into the same context without reinitialization is an application-level contract violation.

4. **Perimeter Isolation Barrier (`std` feature)**:
   - For host runtimes (FFI, WebAssembly, game engines like GoldSrc, dynamic plugins), the pipeline provides `Pipeline::dispatch_isolated`.
   - `dispatch_isolated` wraps traversal in `std::panic::catch_unwind`, translating panics into typed error results (`Result<Result<TOutcome, TErr>, Box<dyn Any + Send>>`) without bringing down the host process.
