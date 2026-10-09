//! Example 02: Cache Short-Circuit Traversal
//!
//! Demonstrates short-circuiting:
//! - When a cache hit occurs, `FlowControl::ShortCircuit` halts further descent.
//! - The expensive terminal handler is completely bypassed.
//! - Outer layers still participate in the ascent phase (`on_exit`) to record telemetry.

use stitch_rs::prelude::*;

#[blackboard]
#[repr(C, align(64))]
struct QueryContext {
    cached_hits: usize,
    terminal_executions: usize,
}

impl Blackboard for QueryContext {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct QueryIntent {
    key: u64,
}

struct CacheLayer {
    cached_key: u64,
    cached_value: u64,
}

impl Layer<QueryContext, QueryIntent, u64, &'static str> for CacheLayer {
    fn on_enter(
        &self,
        ctx: &mut QueryContext,
        intent: QueryIntent,
    ) -> FlowControl<QueryIntent, u64, &'static str> {
        if intent.key == self.cached_key {
            ctx.cached_hits += 1;
            // Short-circuit: halt descent and return cached outcome immediately!
            FlowControl::ShortCircuit(self.cached_value)
        } else {
            FlowControl::Proceed(intent)
        }
    }

    fn on_exit(&self, _ctx: &mut QueryContext, _outcome: &mut Result<u64, &'static str>) {}
}

struct MetricsLayer;

impl Layer<QueryContext, QueryIntent, u64, &'static str> for MetricsLayer {
    fn on_enter(
        &self,
        _ctx: &mut QueryContext,
        intent: QueryIntent,
    ) -> FlowControl<QueryIntent, u64, &'static str> {
        FlowControl::Proceed(intent)
    }

    fn on_exit(&self, _ctx: &mut QueryContext, outcome: &mut Result<u64, &'static str>) {
        if let Ok(val) = outcome {
            println!("[METRICS] Response delivered: value={val}");
        }
    }
}

struct HeavyComputeTerminal;

impl Terminal<QueryContext, QueryIntent, u64, &'static str> for HeavyComputeTerminal {
    fn execute(
        &mut self,
        ctx: &mut QueryContext,
        intent: QueryIntent,
    ) -> Result<u64, &'static str> {
        ctx.terminal_executions += 1;
        println!(
            "[TERMINAL] Performing heavy computation for key {}",
            intent.key
        );
        Ok(intent.key * 42)
    }
}

fn main() {
    let mut pipeline = Pipeline::on_terminal(HeavyComputeTerminal)
        .wrap(CacheLayer {
            cached_key: 100,
            cached_value: 4200,
        })
        .wrap(MetricsLayer);

    let mut ctx = QueryContext {
        cached_hits: 0,
        terminal_executions: 0,
    };

    println!("--- Query 1: Cache Miss (Triggers Terminal) ---");
    let res1 = pipeline.dispatch(&mut ctx, QueryIntent { key: 5 });
    assert_eq!(res1, Ok(210));
    assert_eq!(ctx.terminal_executions, 1);
    assert_eq!(ctx.cached_hits, 0);

    println!("\n--- Query 2: Cache Hit (Short-Circuits Terminal) ---");
    let res2 = pipeline.dispatch(&mut ctx, QueryIntent { key: 100 });
    assert_eq!(res2, Ok(4200));
    // Terminal executions remained 1, cache hit incremented to 1
    assert_eq!(ctx.terminal_executions, 1);
    assert_eq!(ctx.cached_hits, 1);

    println!("\nShort-circuit traversal successfully verified.");
}
