# GoldSrc.rs & Host Engine Integration Guide

This guide describes how to integrate `stitch-rs` into high-performance host runtimes, game engines, and embedded platforms (with primary reference to `GoldSrc.rs`).

---

## 1. Architectural Philosophy in Game Engines

In realtime game servers (e.g. GoldSrc / ReHLDS tick loop running at 1000 FPS), every microsecond counts. Traditional plugin stacks incur severe performance penalties:
- Dynamic trait objects (`Box<dyn PluginHook>`) trigger indirect branch mispredictions on every engine callback (`server_frame`, `player_spawn`, `cmd_dispatch`).
- Lack of panic boundaries means a single faulty plugin crashes the entire game server.

`stitch-rs` resolves this via **The Sewing Machine Architecture (SMA)**:
1. **Compile-time Monomorphism**: Pipelines are compiled as unrolled, inlined functions with zero vtable dispatch.
2. **Panic Barrier**: Boundary hosts wrap untrusted plugin execution via `Pipeline::dispatch_isolated`.

---

## 2. Implementing Host Pipelines

### Step 1: Define the Cache-Aligned Blackboard Context

The material context holds the tick state, entity tables, and scratchpad memory:

```rust
use stitch_rs::prelude::*;

#[blackboard]
#[repr(C, align(64))]
pub struct ServerFrameContext {
    pub tick_number: u64,
    pub delta_time_ms: f32,
    pub player_count: u32,
}

impl Blackboard for ServerFrameContext {}
```

### Step 2: Implement Monomorphic Layers

Layers handle pre-filtering (descent) and telemetry/audit (ascent):

```rust
pub struct RateLimitLayer;

impl Layer<ServerFrameContext, TickIntent, TickOutcome, ServerError> for RateLimitLayer {
    fn on_enter(
        &self,
        ctx: &mut ServerFrameContext,
        intent: TickIntent,
    ) -> FlowControl<TickIntent, TickOutcome, ServerError> {
        if ctx.player_count == 0 {
            // Short-circuit idle ticks without invoking terminal logic
            FlowControl::ShortCircuit(TickOutcome::Idle)
        } else {
            FlowControl::Proceed(intent)
        }
    }

    fn on_exit(
        &self,
        _ctx: &mut ServerFrameContext,
        _outcome: &mut Result<TickOutcome, ServerError>,
    ) {
        // Collect telemetry or emit profiling events
    }
}
```

### Step 3: Implement Terminal Execution

The terminal marks the bottom of the U-cycle (Point of Puncture):

```rust
pub struct SimulationTerminal;

impl Terminal<ServerFrameContext, TickIntent, TickOutcome, ServerError> for SimulationTerminal {
    fn execute(
        &mut self,
        ctx: &mut ServerFrameContext,
        intent: TickIntent,
    ) -> Result<TickOutcome, ServerError> {
        // Run core physics or entity simulation
        Ok(TickOutcome::Processed)
    }
}
```

### Step 4: Dispatch via Panic Isolation Barrier

When running in host environments with dynamic plugins or FFI boundaries, execute dispatches through `dispatch_isolated`:

```rust
let mut pipeline = Pipeline::on_terminal(SimulationTerminal)
    .wrap(RateLimitLayer);

let result = pipeline.dispatch_isolated(&mut ctx, TickIntent::SimulateFrame);
match result {
    Ok(outcome) => { /* Process outcome */ }
    Err(err) => { /* Handle domain error */ }
}
```

---

## 3. Best Practices for Embedded & `#![no_std]` Runtimes

- Disable default features: `default-features = false`.
- Ensure targets compile with `panic = "unwind"` if host isolation via `dispatch_isolated` is required, or rely on outer domain supervision if `panic = "abort"` is used.
- Annotate blackboard structs with `#[repr(C, align(64))]` to prevent CPU cache bouncing across multiple worker threads.
