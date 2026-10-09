# Stitch-rs Microbenchmark & Empirical Analysis

This document provides a rigorous, empirical, and transparent breakdown of `stitch-rs` pipeline latency, monomorphic inlining efficiency, dynamic dispatch overheads, and cross-paradigm service comparisons.

Following the methodological standards established by Andrew Gallant ([BurntSushi/rebar](https://github.com/BurntSushi/rebar/blob/master/METHODOLOGY.md)) for low-level systems engineering, all claims are paired with exact hardware characteristics, cache line sizing, compiler versions, and reproducible harness commands.

---

## 1. Testbed Specification

All measurements reported herein were executed on the following dedicated testbed:

| Parameter | Specification | Notes |
| :--- | :--- | :--- |
| **CPU** | 11th Gen Intel(R) Core(TM) i9-11900H @ 2.50GHz (8C/16T) | AVX2, BMI2, CLFLUSHOPT enabled |
| **L1d Cache** | 32 KB per core | 8-way associative, 64-byte lines |
| **L2 Cache** | 512 KB per core | Private per-core cache |
| **L3 Cache** | 24 MB shared | Intel Smart Cache |
| **System Memory** | DDR4 Dual-Channel | ~50–60 GB/s bandwidth |
| **OS** | Windows 11 Pro 64-bit | Page size 4096 bytes (4 KB) |
| **Rust Toolchain** | `rustc 1.85.0` (Stable) | Edition 2024, `opt-level = 3`, LTO |
| **Bench Harness** | Criterion.rs 0.5.1 | 100 samples, 3s warm-up, 5s measurement per target |

---

## 2. Evaluation Regimes & Workload Profile

### 2.1 The Execution Workload Contract

To isolate the structural overhead of dispatching across layers from arbitrary payload computation, every benchmark evaluates an identical, representative middleware workload:

- **Context (`BenchContext`)**: 64-byte aligned blackboard state (`#[repr(C, align(64))]`) holding an integer accumulator `val: u64`.
- **Descent Phase (`on_enter`)**:
  - `LayerA::on_enter`: Mutates material state `ctx.val = ctx.val.wrapping_add(intent.0)` and signals `FlowControl::Proceed(intent)`.
  - `LayerB::on_enter`: Increments state `ctx.val = ctx.val.wrapping_add(1)` and signals `FlowControl::Proceed(intent)`.
- **Point of Puncture (`Terminal`)**:
  - `BenchTerminal::execute`: Terminal handler computing `intent.0.wrapping_mul(2)` and returning `Ok(result)`.
- **Ascent Phase (`on_exit`)**:
  - `LayerB::on_exit`: Intercepts outcome and doubles output `*val = val.wrapping_mul(2)`.
  - `LayerA::on_exit`: Accumulates final outcome back into context `ctx.val = ctx.val.wrapping_add(*val)`.

This workload validates the complete symmetric U-cycle (descent inspection, terminal calculation, ascent outcome interception, and context mutation).

### 2.2 Why Tower? The Embedded & GameDev Middleware Vacuum

A frequent question when reviewing these numbers is: *Why compare a synchronous, `#![no_std]` in-memory framework against `tower::Service`?*

1. **The Ecosystem Vacuum in Embedded, GameDev & Kernels**:
   In the Rust ecosystem, there are virtually **no established synchronous, zero-cost middleware libraries** supporting `#![no_std]` and compile-time U-cycle ascent.
   - Traditional web/microservice frameworks (Axum, Tonic, Hyper, Linkerd) standardized universally on `tower::Service`.
   - In realtime game engines (e.g. 1000 FPS tick loops in [GoldSrc.rs](https://github.com/goldsrc-rs/goldsrc-rs)), bare-metal microcontrollers, and network packet dispatchers, developers historically had to abandon middleware abstractions entirely and write monolithic procedural functions (`if/else` ladders) to avoid runtime penalties.
2. **Tower as the Industry Reference**:
   `tower` is included not as a direct peer for embedded execution, but as the **authoritative industry benchmark of the middleware pattern in Rust**.
   Comparing against Tower empirically measures the **cost of asynchronous abstraction** (`poll_ready`, `Poll`, `Pin`, `Waker`, heap future allocation) when applied to synchronous in-memory domains.
3. **Different Workload Paradigm**:
   Tower is engineered for I/O-bound concurrency across threads and network sockets. `stitch-rs` is engineered for CPU-bound, sub-nanosecond, cache-resident domain pipelines.

---

## 3. Empirical Benchmark Matrix

Measured using Criterion 0.5.1 under `cargo bench --bench pipeline_bench`:

```bash
cargo bench --bench pipeline_bench
```

| Benchmark Target | Implementation Paradigm | Mean Latency | 95% Confidence Interval | Single-Thread Throughput | Total Iterations (5s Sample) | Speedup vs. Dynamic |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `hand_inlined_baseline` | Raw procedural code (Zero Abstraction) | **1.08 ns** | [1.07 ns – 1.09 ns] | **~925.9 M ops/sec** | ~4,600,000,000 | 4.85x |
| `stitch_monomorphic` | SMA Monomorphic U-Cycle (`Pipeline`) | **1.30 ns** | [1.28 ns – 1.31 ns] | **~769.2 M ops/sec** | ~3,900,000,000 | **4.03x faster** |
| `box_dyn_layers` | Dynamic Polymorphism (`Vec<Box<dyn>>`) | **5.24 ns** | [5.20 ns – 5.28 ns] | **~190.8 M ops/sec** | ~947,000,000 | 1.00x (Baseline) |
| `tower_ready_future` | Async Service Pipeline (`tower::Service`) | **74.18 ns** | [73.38 ns – 75.07 ns] | **~13.5 M ops/sec** | ~67,000,000 | 14.15x slower |

### L1D Cache Line Access (Blackboard Contexts)

Measures memory throughput on single L1D cache line residency:

| Workload | Alignment Specification | Mean Latency | 95% Confidence Interval | Iterations Sampled | Notes |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `aligned_64b_cells` | `#[repr(C, align(64))]` | **969.54 ps** | [957.78 ps – 981.61 ps] | ~5,000,000,000 | Single L1D line, zero boundary crossing |
| `unaligned_packed_cells` | `#[repr(packed)]` | **928.00 ps** | [913.89 ps – 942.83 ps] | ~5,300,000,000 | Sequential streaming inside L1D cache |

---

## 4. Key Architectural Takeaways

1. **Zero Dynamic Dispatch Overhead**:
   `stitch-rs` delivers **1.30 ns** end-to-end traversal across multi-layer middleware, compared to **5.24 ns** for `Box<dyn Layer>`. In high-frequency game engine tick loops (e.g. 1000 Hz servers processing thousands of entities per tick), eliminating dynamic dispatch saves hundreds of millions of CPU cycles per second.

2. **The Cost of Future State Machines**:
   Even with an immediately resolved `std::future::Ready`, Tower requires **74.18 ns** per dispatch due to virtual call indirection, waker management, and future pinning. For synchronous simulation loops with 1 ms frame budgets, `stitch-rs` provides the only viable zero-cost pipeline abstraction.
