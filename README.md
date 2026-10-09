# stitch-rs

<p align="center">
  <strong>Zero-Cost, Monomorphic U-Cycle Middleware Pipeline Framework</strong><br>
  <em>The Sewing Machine Architecture (SMA) pattern engine for high-throughput Rust systems</em>
</p>

<p align="center">
  <a href="https://github.com/ulquiorracode/stitch-rs/actions"><img src="https://img.shields.io/github/actions/workflow/status/ulquiorracode/stitch-rs/ci.yml?branch=main&label=CI&logo=github&style=flat-square" alt="CI"></a>
  <a href="https://crates.io/crates/stitch-rs"><img src="https://img.shields.io/crates/v/stitch-rs?style=flat-square" alt="Crates.io"></a>
  <a href="https://docs.rs/stitch-rs"><img src="https://img.shields.io/docsrs/stitch-rs?style=flat-square" alt="Docs.rs"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT%2FApache--2.0-blue?style=flat-square" alt="License"></a>
</p>

---

## Overview

`stitch-rs` implements **The Sewing Machine Architecture (SMA)**: a zero-allocation, monomorphic U-cycle processing pipeline.

Unlike traditional dynamic middleware stacks (e.g. `Box<dyn Middleware>` or vector-based chain-of-responsibility), `stitch-rs` resolves the entire traversal order and state transitions **at compile time** through nested generic composition:

```text
Intent ─────────► [Layer A: on_enter] ──► [Layer B: on_enter] ──► [Terminal: execute]
                                                                         │
Outcome ◄─────── [Layer A: on_exit]  ◄── [Layer B: on_exit]  ◄─────────┘
                               (The U-Cycle Traversal)
```

## Key Principles

- **Zero-Cost Abstraction**: The execution pipeline unfolds into a static monomorphic chain with full inlining opportunities and no dynamic dispatch (`Box<dyn ...>`).
- **Strict U-Cycle Flow**: Every layer inspects or transforms the intent on descent (`on_enter`) and intercepts or annotates the outcome on ascent (`on_exit`).
- **Symmetric Unwinding & Error Guarantees**: Signaling `FlowControl::Halt(err)` or `FlowControl::ShortCircuit(outcome)` halts further descent, while running `on_exit(ctx, &mut outcome)` for the halting layer and all outer enclosing layers during ascent.
- **Scrooge Systems Mindset**: Cache-line alignment (`#[repr(C, align(64))]`), zero heap allocations on hot paths, and padding waste elimination.
- **Clean CQS Separation**: Explicit separation between Queries and Commands across system boundaries.
- **`no_std` Ready**: Core pipeline operates under `#![no_std]` without compulsory heap allocation.

## Usage

```rust
use stitch_core::flow::FlowControl;
use stitch_core::middleware::{Blackboard, Layer, Terminal};
use stitch_core::pipeline::Pipeline;

#[repr(C, align(64))]
struct RequestContext {
    user_id: u64,
    authenticated: bool,
}

impl Blackboard for RequestContext {}

struct AuthLayer;

impl Layer<RequestContext, &'static str, &'static str, &'static str> for AuthLayer {
    fn on_enter(
        &self,
        ctx: &mut RequestContext,
        intent: &'static str,
    ) -> FlowControl<&'static str, &'static str, &'static str> {
        if !ctx.authenticated {
            FlowControl::Halt("Unauthorized")
        } else {
            FlowControl::Proceed(intent)
        }
    }

    fn on_exit(
        &self,
        _ctx: &mut RequestContext,
        outcome: &mut Result<&'static str, &'static str>,
    ) {
        // Runs on both success and error/halt ascent
        if let Err(err) = outcome {
            // Telemetry / audit recording
            let _ = err;
        }
    }
}

struct EchoTerminal;

impl Terminal<RequestContext, &'static str, &'static str, &'static str> for EchoTerminal {
    fn execute(
        &mut self,
        _ctx: &mut RequestContext,
        intent: &'static str,
    ) -> Result<&'static str, &'static str> {
        Ok(intent)
    }
}

fn main() {
    let mut pipeline = Pipeline::on_terminal(EchoTerminal)
        .wrap(AuthLayer);

    let mut ctx = RequestContext {
        user_id: 42,
        authenticated: true,
    };

    let outcome = pipeline.dispatch(&mut ctx, "ping");
    assert_eq!(outcome, Ok("ping"));
}
```

## Tooling & Architecture-as-Code

The workspace includes the `stitch` / `sma` CLI tool suite:

- `stitch check`: Validates taxonomy rules, dependency boundaries, cache alignment, and hot-path heap allocations.
- `stitch fix`: Automatically reorders struct fields by descending alignment to eliminate preventable padding waste.
- `stitch metrics`: Measures struct sizes, cache line alignment, and memory efficiency.
- `stitch health`: Computes an architectural health score across taxonomy purity, DIP, mechanical sympathy, zero-alloc hot paths, and re-entrancy.
- `stitch graph`: Extracts the architectural DAG and exports to Mermaid, Graphviz DOT, or interactive HTML.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)
