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
| **Bench Harness** | Criterion.rs 0.5.1 | 100 samples, 3s warm-up, 5s measurement |

---

## 2. Evaluation Regimes

### 2.1 Monomorphic Inlining vs. Dynamic Dispatch

A primary invariant of **The Sewing Machine Architecture (SMA)** is monomorphic compile-time composition over runtime polymorphism.

1. **`stitch_monomorphic`**: Nested static generic chain (`StackNode<LayerA, StackNode<LayerB, TerminalNode<Terminal>>>`). The compiler flattens the nested structs, analyzes `on_enter` and `on_exit` bodies, and inlines the entire U-cycle descent and ascent into a single branch-free machine block.
2. **`hand_inlined_baseline`**: Manual sequence of operations written directly in a single function without any layer or pipeline abstraction. Serves as the theoretical floor of CPU execution.
3. **`box_dyn_layers`**: Idiomatic object-oriented middleware chain (`Vec<Box<dyn Layer>>`). Incurs heap indirection, pointer chasing, vtable call overhead, and inhibits LLVM inlining passes.
4. **`tower_ready_future`**: Asynchronous `tower::Service` implementation. Operates on `poll_ready`, `call`, and pinned futures. Serves as a cross-paradigm reference measuring async machinery overhead in synchronous in-memory contexts.

### 2.2 Memory Alignment & Cache-Line Straddling

Evaluates L1D cache line effects on blackboard context access:
- **`aligned_64b_cells`**: Context annotated with `#[repr(C, align(64))]`. Fits entirely within a single 64-byte L1D line without crossing cache boundaries.
- **`unaligned_packed_cells`**: Unaligned packed context records.

---

## 3. Empirical Benchmark Matrix

Measured using Criterion 0.5.1 under `cargo bench --bench pipeline_bench`:

```bash
cargo bench --bench pipeline_bench
```

| Benchmark Target | Implementation Paradigm | Mean Latency | 95% Confidence Interval | Relative vs. Baseline | Overhead Analysis |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `hand_inlined_baseline` | Raw procedural code (Zero Abstraction) | **1.08 ns** | [1.07 ns – 1.09 ns] | **1.00x** (Floor) | Theoretical minimum for 2 guard checks + arithmetic |
| `stitch_monomorphic` | SMA Monomorphic U-Cycle (`Pipeline`) | **1.30 ns** | [1.28 ns – 1.31 ns] | **1.20x** | **+0.22 ns** overhead; fully inlined by LLVM |
| `box_dyn_layers` | Dynamic Polymorphism (`Vec<Box<dyn>>`) | **5.24 ns** | [5.20 ns – 5.28 ns] | **4.85x** | **4.03x slower** than `stitch` due to vtable indirection |
| `tower_ready_future` | Async Service Pipeline (`tower::Service`) | **74.18 ns** | [73.38 ns – 75.07 ns] | **68.68x** | **57.06x slower**; wakers, polling state machine, future pin |

### L1D Cache Line Access (Blackboard Contexts)

| Workload | Alignment Specification | Mean Latency | 95% Confidence Interval | Notes |
| :--- | :--- | :--- | :--- | :--- |
| `aligned_64b_cells` | `#[repr(C, align(64))]` | **969.54 ps** | [957.78 ps – 981.61 ps] | Single L1D line, zero boundary crossing |
| `unaligned_packed_cells` | `#[repr(packed)]` | **928.00 ps** | [913.89 ps – 942.83 ps] | L1-resident sequential streaming |

---

## 4. Key Architectural Takeaways

1. **Zero Dynamic Dispatch Overhead**:
   `stitch-rs` delivers **1.30 ns** end-to-end traversal across multi-layer middleware, compared to **5.24 ns** for `Box<dyn Layer>`. In high-frequency game engine tick loops (e.g. 1000 Hz servers processing thousands of entities per tick), eliminating dynamic dispatch saves millions of CPU cycles per second.

2. **Synchronous Engine vs. Async Runtimes**:
   While `tower` is an exceptional framework for network servers and I/O-bound web microservices, its async state machine introduces ~74 ns of latency per dispatch. For bare-metal embedded targets, kernel modules, and realtime simulation engines, `stitch-rs` provides the correct synchronous architectural primitive.
