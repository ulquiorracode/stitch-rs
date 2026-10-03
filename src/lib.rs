//! # Stitch.rs (`stitch-rs`)
//!
//! Zero-Cost, Monomorphic U-Cycle Middleware Pipeline Framework implementing
//! **The Sewing Machine Architecture (SMA)**.
//!
//! > **Result = Material + Intent + Work**
//!
//! - **Material**: The passive domain context/state (`TCtx`).
//! - **Intent**: The desire entering the pipeline (`TIntent`).
//! - **Work**: The compile-time monomorphic pipeline (`Pipeline`) driving the U-cycle:
//!   - Phase 1: Descent (`on_enter`: Proceed or Halt)
//!   - Point of Puncture: Execution at `TerminalHandler`
//!   - Phase 2: Ascent (`on_exit`: Reactions, Diffs, Telemetry)

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod cqs;
pub mod flow;
pub mod middleware;
pub mod pipeline;

pub use cqs::{Command, Query};
pub use flow::{Admission, FlowControl};
pub use middleware::{Layer, Middleware, Terminal, TerminalHandler};
pub use pipeline::{Machine, Pipeline, PipelineChain, StackNode, TerminalNode};

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;
    use alloc::vec::Vec;

    struct TestContext {
        state: u64,
        audit: Vec<String>,
    }

    enum MathIntent {
        Add(u64),
        Multiply(u64),
    }

    struct LimitGuardMiddleware {
        max_delta: u64,
    }

    impl Middleware<TestContext, MathIntent, u64, &'static str> for LimitGuardMiddleware {
        fn on_enter(
            &self,
            _ctx: &mut TestContext,
            intent: MathIntent,
        ) -> FlowControl<MathIntent, u64, &'static str> {
            match intent {
                MathIntent::Add(val) if val > self.max_delta => {
                    FlowControl::Halt("Addition exceeds allowed limit")
                }
                MathIntent::Multiply(val) if val > self.max_delta => {
                    FlowControl::Halt("Multiplication exceeds allowed limit")
                }
                _ => FlowControl::Proceed(intent),
            }
        }

        fn on_exit(&self, _ctx: &mut TestContext, _outcome: &mut Result<u64, &'static str>) {}
    }

    struct AuditMiddleware;

    impl Middleware<TestContext, MathIntent, u64, &'static str> for AuditMiddleware {
        fn on_enter(
            &self,
            _ctx: &mut TestContext,
            intent: MathIntent,
        ) -> FlowControl<MathIntent, u64, &'static str> {
            FlowControl::Proceed(intent)
        }

        fn on_exit(&self, ctx: &mut TestContext, outcome: &mut Result<u64, &'static str>) {
            match outcome {
                Ok(new_val) => ctx
                    .audit
                    .push(alloc::format!("Success: new_val={}", new_val)),
                Err(err) => ctx.audit.push(alloc::format!("Failed: reason={}", err)),
            }
        }
    }

    struct MathTerminal;

    impl TerminalHandler<TestContext, MathIntent, u64, &'static str> for MathTerminal {
        fn execute(
            &mut self,
            ctx: &mut TestContext,
            intent: MathIntent,
        ) -> Result<u64, &'static str> {
            match intent {
                MathIntent::Add(n) => {
                    ctx.state += n;
                    Ok(ctx.state)
                }
                MathIntent::Multiply(n) => {
                    ctx.state *= n;
                    Ok(ctx.state)
                }
            }
        }
    }

    struct CacheShortCircuitMiddleware {
        cached_result: u64,
    }

    impl Middleware<TestContext, MathIntent, u64, &'static str> for CacheShortCircuitMiddleware {
        fn on_enter(
            &self,
            _ctx: &mut TestContext,
            intent: MathIntent,
        ) -> FlowControl<MathIntent, u64, &'static str> {
            match intent {
                MathIntent::Add(0) => FlowControl::ShortCircuit(self.cached_result),
                _ => FlowControl::Proceed(intent),
            }
        }

        fn on_exit(&self, _ctx: &mut TestContext, _outcome: &mut Result<u64, &'static str>) {}
    }

    #[test]
    fn test_standalone_stitch_pipeline() {
        let mut ctx = TestContext {
            state: 10,
            audit: Vec::new(),
        };

        let mut pipeline = Pipeline::on_terminal(MathTerminal)
            .use_middleware(LimitGuardMiddleware { max_delta: 50 })
            .use_middleware(CacheShortCircuitMiddleware { cached_result: 999 })
            .use_middleware(AuditMiddleware);

        // 1. Success dispatch
        let res = pipeline.dispatch(&mut ctx, MathIntent::Add(25)).unwrap();
        assert_eq!(res, 35);
        assert_eq!(ctx.state, 35);

        // 2. ShortCircuit dispatch (cache hit - terminal bypassed, but ascent audit triggered)
        let res_cached = pipeline.dispatch(&mut ctx, MathIntent::Add(0)).unwrap();
        assert_eq!(res_cached, 999);
        assert_eq!(ctx.state, 35); // State untouched

        // 3. Halted dispatch
        let err = pipeline
            .dispatch(&mut ctx, MathIntent::Add(100))
            .unwrap_err();
        assert_eq!(err, "Addition exceeds allowed limit");
        assert_eq!(ctx.state, 35); // State untouched

        // 4. Multiplication dispatch
        let res_mul = pipeline
            .dispatch(&mut ctx, MathIntent::Multiply(2))
            .unwrap();
        assert_eq!(res_mul, 70);
        assert_eq!(ctx.state, 70);

        // 5. Verify ascent telemetry across all executions
        assert_eq!(ctx.audit.len(), 4);
        assert_eq!(ctx.audit[0], "Success: new_val=35");
        assert_eq!(ctx.audit[1], "Success: new_val=999");
        assert_eq!(
            ctx.audit[2],
            "Failed: reason=Addition exceeds allowed limit"
        );
        assert_eq!(ctx.audit[3], "Success: new_val=70");
    }
}
