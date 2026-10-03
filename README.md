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

```
Request ─────────► [Layer A: on_enter] ──► [Layer B: on_enter] ──► [Terminal Handler]
                                                                          │
Response ◄──────── [Layer A: on_exit]  ◄── [Layer B: on_exit]  ◄──────────┘
                              (The U-Cycle Traverse)
```

## Key Principles

- **Zero-Cost Abstraction**: The execution pipeline unfolds into a static monomorphic chain with full inlining opportunities.
- **Strict U-Cycle Flow**: Every middleware receives an immutable inspection or mutable step during the downward cycle (`on_enter`) and can intercept or transform the result on the upward cycle (`on_exit`).
- **Short-Circuiting**: Middleware can signal `FlowControl::Halt` or `FlowControl::EarlyExit` to abort descending steps while ensuring proper unwinding of outer `on_exit` phases.
- **Clean CQS Separation**: Explicit separation between Queries and Commands across system boundaries.
- **`no_std` Ready**: Compatible with embedded and kernel-adjacent environments without compulsory heap allocation.

## Usage

```rust
use stitch_rs::pipeline::Pipeline;
use stitch_rs::middleware::{Middleware, TerminalHandler};
use stitch_rs::flow::FlowControl;

struct AuthLayer;
impl<Ctx, Req, Res, Err> Middleware<Ctx, Req, Res, Err> for AuthLayer {
    fn on_enter(&self, _ctx: &mut Ctx, req: Req) -> FlowControl<Req, Res, Err> {
        FlowControl::Proceed(req)
    }

    fn on_exit(&self, _ctx: &mut Ctx, _res: &mut Result<Res, Err>) {
        // Post-processing and ascent telemetry
    }
}

struct CoreHandler;
impl<Ctx, Req, Res: Default, Err> TerminalHandler<Ctx, Req, Res, Err> for CoreHandler {
    fn execute(&mut self, _ctx: &mut Ctx, _req: Req) -> Result<Res, Err> {
        Ok(Res::default())
    }
}

fn main() {
    let mut pipeline = Pipeline::on_terminal(CoreHandler)
        .use_middleware(AuthLayer);

    let mut ctx = ();
    let result = pipeline.dispatch(&mut ctx, ());
}
```

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
