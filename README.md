# stitch-rs

<!-- Project Status & Metrics -->
![Status](https://img.shields.io/badge/status-active%20development-orange?logo=rust) [![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license) [![CI](https://github.com/ulquiorracode/stitch-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/ulquiorracode/stitch-rs/actions/workflows/ci.yml) [![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg?logo=readme)](https://github.com/RichardLitt/standard-readme)  
<!-- Repository & Community -->
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md) [![MSRV: 1.85.0](https://img.shields.io/badge/MSRV-1.85.0-blue.svg?logo=rust)](https://doc.rust-lang.org/edition-guide/rust-2024/) [![Crates.io](https://img.shields.io/crates/v/stitch-rs?style=flat-square)](https://crates.io/crates/stitch-rs) [![Docs.rs](https://img.shields.io/docsrs/stitch-rs?style=flat-square)](./docs/)

> Zero-Cost, Monomorphic U-Cycle Middleware Pipeline Framework implementing The Sewing Machine Architecture (SMA).

**stitch-rs** is a high-performance, `#![no_std]` capable Rust pipeline framework designed for game engines (e.g. [GoldSrc.rs](https://github.com/goldsrc-rs/goldsrc-rs)), bare-metal embedded systems, and high-frequency trading runtimes.

It eliminates dynamic dispatch (`Box<dyn ...>`) and pointer chasing by flattening middleware layers into a compile-time static generic chain, enabling full compiler inlining down to ~1.30 ns per dispatch.

---

## Table of Contents

- [Background](#background)
- [Features](#features)
- [Architecture & Core Taxonomy](#architecture--core-taxonomy)
- [Empirical Benchmarks](#empirical-benchmarks)
- [Install & Prerequisites](#install--prerequisites)
- [Usage](#usage)
- [Tooling & Architecture-as-Code](#tooling--architecture-as-code)
- [Maintainers](#maintainers)
- [Contributing](#contributing)
- [License](#license)

---

## Background

Traditional middleware architectures (like tower, web frameworks, or object-oriented chains-of-responsibility) rely heavily on:

1. Dynamic heap allocations (`Box<dyn Layer>` or pinned futures).
2. Indirect branch mispredictions from vtable lookups.
3. Asynchronous state machines with runtime polling overhead.

In realtime simulation loops (such as 1000 Hz game engine tick loops or network frame dispatchers), these abstractions introduce severe latency penalties (~5–75 ns per call).

**stitch-rs** introduces **The Sewing Machine Architecture (SMA)**:

- **Monomorphic Composition**: Traversal order and layer nesting are resolved entirely at compile time.
- **Symmetric U-Cycle Traversal**: Descent (`on_enter`) descends to the terminal point of puncture, while ascent (`on_exit`) ascends symmetrically through enclosing layers.
- **Scrooge Mechanical Sympathy**: Explicit 64-byte L1D cache line alignment contracts (`ASSERT_CACHE_ALIGNED`) and zero heap allocations on hot paths.

```text
Intent ─────────► [Layer A: on_enter] ──► [Layer B: on_enter] ──► [Terminal: execute]
                                                                         │
Outcome ◄─────── [Layer A: on_exit]  ◄── [Layer B: on_exit]  ◄─────────┘
                               (The U-Cycle Traversal)
```

---

## Features

- **Zero-Cost Monomorphism**: Static compile-time chains enable full LLVM inlining down to **1.30 ns** per dispatch (within 0.22 ns of raw inlined code).
- **Strict U-Cycle Flow Control**: `FlowControl::Proceed`, `FlowControl::ShortCircuit`, and `FlowControl::Halt` with symmetric upward ascent.
- **Closed Topology**: `PipelineChain` sealed trait and encapsulated nodes prevent constructing unverified pipelines.
- **Cache-Line Alignment**: Compile-time assertion (`ASSERT_CACHE_ALIGNED`) enforces `#[repr(C, align(64))]` on blackboard contexts.
- **Panic Isolation Barrier**: Userspace and plugin host perimeter protection via `Pipeline::dispatch_isolated`.
- **`#![no_std]` Native**: Core pipeline runs without heap allocators or standard library dependencies.
- **Architecture-as-Code (AaC)**: Standalone CLI (`stitch check`, `stitch fix`, `stitch metrics`, `stitch health`, `stitch graph`) and procedural macros (`#[stitch::layer]`, `#[stitch::terminal]`, etc.).

---

## Architecture & Core Taxonomy

| Architectural Role | Responsibility | Normative Contract |
| :--- | :--- | :--- |
| **`Blackboard`** | Passive material context passed through pipeline | Cache-aligned (`#[repr(C, align(64))]`) |
| **`Layer`** | Middleware node participating in descent and ascent | Pure, non-panicking `on_exit` |
| **`Terminal`** | Leaf node terminating descent (Point of Puncture) | Executes core domain computation |
| **`Pipeline`** | Monomorphic U-cycle runner | Closed topology via sealed traits |
| **`Port`** | Inward SPI boundary trait | Decouples engine algorithms from adapters |
| **`Adapter`** | Concrete vendor implementation of a Port | Statically bounded target port |
| **`Hub`** | Dual-role coordinator | Fractal port/adapter composition |

For detailed memory layouts, failure semantics, and contract invariants, see [ARCHITECTURE.md](ARCHITECTURE.md).

---

## Empirical Benchmarks

Measurements performed on **11th Gen Intel Core i9-11900H @ 2.50GHz** via Criterion 0.5.1 (2 layers + terminal U-cycle, L1D-resident, ~3.9B iterations):

| Target | Paradigm | Mean Latency | Single-Thread Throughput | Speedup vs. Dynamic |
| :--- | :--- | :--- | :--- | :--- |
| `hand_inlined_baseline` | Raw procedural code | **1.08 ns** | ~925.9 M ops/sec | 4.85x |
| `stitch_monomorphic` | **stitch-rs Monomorphic Pipeline** | **1.30 ns** | **~769.2 M ops/sec** | **4.03x faster** |
| `box_dyn_layers` | Dynamic Polymorphism (`Vec<Box<dyn>>`) | **5.24 ns** | ~190.8 M ops/sec | 1.00x (Baseline) |
| `tower_ready_future`* | Async Service Pipeline (`tower::Service`) | **74.18 ns** | ~13.5 M ops/sec | 14.15x slower |

*\*Note: `tower::Service` is included as the authoritative Rust industry reference for middleware architecture due to the lack of synchronous, `#![no_std]` middleware libraries in embedded/gamedev ecosystems. It illustrates the runtime cost of asynchronous futures machinery in synchronous in-memory domains.*

For comprehensive testbed specifications, exact iteration counts, and reproducible benchmark commands, read [docs/BENCHMARKS.md](docs/BENCHMARKS.md).

---

## Install & Prerequisites

- **Rust**: MSRV `1.85.0` (Edition 2024 recommended).

Add `stitch-rs` to your `Cargo.toml`:

```toml
[dependencies]
stitch-rs = "0.2.0"
```

For bare-metal `#![no_std]` targets:

```toml
[dependencies]
stitch-rs = { version = "0.2.0", default-features = false }
```

---

## Usage

```rust
use stitch_rs::prelude::*;

#[blackboard]
#[repr(C, align(64))]
struct RequestContext {
    user_id: u64,
    is_authenticated: bool,
}

impl Blackboard for RequestContext {}

struct AuthLayer;

impl Layer<RequestContext, &'static str, &'static str, &'static str> for AuthLayer {
    fn on_enter(
        &self,
        ctx: &mut RequestContext,
        intent: &'static str,
    ) -> FlowControl<&'static str, &'static str, &'static str> {
        if !ctx.is_authenticated {
            FlowControl::Halt("Unauthorized")
        } else {
            FlowControl::Proceed(intent)
        }
    }

    fn on_exit(&self, _ctx: &mut RequestContext, _outcome: &mut Result<&'static str, &'static str>) {}
}

struct EchoTerminal;

impl Terminal<RequestContext, &'static str, &'static str, &'static str> for EchoTerminal {
    fn execute(&mut self, _ctx: &mut RequestContext, intent: &'static str) -> Result<&'static str, &'static str> {
        Ok(intent)
    }
}

fn main() {
    let mut pipeline = Pipeline::on_terminal(EchoTerminal)
        .wrap(AuthLayer);

    let mut ctx = RequestContext { user_id: 42, is_authenticated: true };
    let outcome = pipeline.dispatch(&mut ctx, "hello");
    assert_eq!(outcome, Ok("hello"));
}
```

Detailed integration examples:

- [examples/01_auth_logging_u_cycle.rs](examples/01_auth_logging_u_cycle.rs)
- [examples/02_cache_short_circuit.rs](examples/02_cache_short_circuit.rs)
- [examples/03_no_std_embedded.rs](examples/03_no_std_embedded.rs)
- [docs/INTEGRATION_GUIDE.md](docs/INTEGRATION_GUIDE.md)

---

## Tooling & Architecture-as-Code

The workspace includes the `stitch` CLI tool suite:

- `stitch check`: Validates taxonomy rules, dependency boundaries, cache alignment, and hot-path heap allocations.
- `stitch fix`: Automatically reorders struct fields by descending alignment to eliminate preventable padding waste.
- `stitch metrics`: Measures struct sizes, cache line alignment, and memory efficiency.
- `stitch health`: Computes an architectural health score across taxonomy purity, DIP, mechanical sympathy, zero-alloc hot paths, and re-entrancy.
- `stitch graph`: Extracts the architectural DAG and exports to Mermaid, Graphviz DOT, or interactive HTML.

For full CLI options and rules, see [docs/TOOLING_GUIDE.md](docs/TOOLING_GUIDE.md).

---

## Maintainers

- **ulquiorracode** ([@ulquiorracode](https://github.com/ulquiorracode))
- **GoldSrc.rs Systems Architects**

---

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before opening a pull request.

---

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
