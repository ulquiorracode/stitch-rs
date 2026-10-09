//! # Stitch Core (`stitch-core`)
//!
//! Zero-Cost, Monomorphic U-Cycle Middleware Pipeline Framework implementing
//! **The Sewing Machine Architecture (SMA)**.
//!
//! # Core Taxonomical Dimensions
//! - **Operational Nodes**: [`Layer`], [`Terminal`], [`Pipeline`]
//! - **Hexagonal Abstraction Boundaries**: [`Port`], [`Adapter`], [`Hub`]
//! - **Memory & Scratchpads**: [`Blackboard`]
//! - **Registration & Identification**: [`StitchToken`], [`StitchId`], [`RawToken`], [`RawId`]
//! - **Domain Intent Accounting**: [`Entity`], [`ValueObject`], [`Event`], [`Data`]
//! - **Flow Control**: [`FlowControl`], [`Admission`]

#![cfg_attr(not(feature = "std"), no_std)]

pub mod blackboard;
pub mod cqs;
pub mod data;
pub mod flow;
pub mod hash;
pub mod middleware;
pub mod pipeline;
pub mod taxonomy;
pub mod token;

pub use blackboard::Blackboard;
pub use cqs::{Command, CommandExecutor, Query};
pub use data::{Data, Entity, Event, ValueObject};
pub use flow::FlowControl;
#[allow(deprecated)]
pub use flow::{Admission, BlackboardFlow};
pub use hash::{fnv1a_32, fnv1a_64};
pub use middleware::{Layer, Terminal};
#[allow(deprecated)]
pub use middleware::{Middleware, TerminalHandler};
#[allow(deprecated)]
pub use pipeline::Machine;
pub use pipeline::{Pipeline, PipelineChain, StackNode, TerminalNode};
pub use taxonomy::{Adapter, Hub, Port};
pub use token::{RawId, RawToken, StitchId, StitchToken};

/// Common imports and contracts for Sewing Machine Architecture (SMA).
pub mod prelude {
    pub use crate::assert_blackboard_aligned;
    pub use crate::blackboard::Blackboard;
    pub use crate::cqs::{Command, CommandExecutor, Query};
    pub use crate::data::{Data, Entity, Event, ValueObject};
    pub use crate::flow::FlowControl;
    #[allow(deprecated)]
    pub use crate::flow::{Admission, BlackboardFlow};
    pub use crate::hash::{fnv1a_32, fnv1a_64};
    pub use crate::middleware::{Layer, Terminal};
    #[allow(deprecated)]
    pub use crate::middleware::{Middleware, TerminalHandler};
    #[allow(deprecated)]
    pub use crate::pipeline::Machine;
    pub use crate::pipeline::{Pipeline, PipelineChain, StackNode, TerminalNode};
    pub use crate::taxonomy::{Adapter, Hub, Port};
    pub use crate::token::{RawId, RawToken, StitchId, StitchToken};
}

#[cfg(test)]
mod tests {
    use super::prelude::*;
    #[repr(C, align(64))]
    struct TestBlackboard {
        pub state: u64,
        pub audit_len: usize,
        pub audit: [&'static str; 4],
    }

    impl Blackboard for TestBlackboard {}

    enum MathIntent {
        Add(u64),
        Multiply(u64),
    }

    struct LimitGuardLayer {
        max_delta: u64,
    }

    impl Layer<TestBlackboard, MathIntent, u64, &'static str> for LimitGuardLayer {
        fn on_enter(
            &self,
            _ctx: &mut TestBlackboard,
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

        fn on_exit(&self, _ctx: &mut TestBlackboard, _outcome: &mut Result<u64, &'static str>) {}
    }

    struct AuditLayer;

    impl Layer<TestBlackboard, MathIntent, u64, &'static str> for AuditLayer {
        fn on_enter(
            &self,
            _ctx: &mut TestBlackboard,
            intent: MathIntent,
        ) -> FlowControl<MathIntent, u64, &'static str> {
            FlowControl::Proceed(intent)
        }

        fn on_exit(&self, ctx: &mut TestBlackboard, outcome: &mut Result<u64, &'static str>) {
            if ctx.audit_len < ctx.audit.len() {
                ctx.audit[ctx.audit_len] = match outcome {
                    Ok(35) => "Success: 35",
                    Ok(999) => "Success: 999",
                    Ok(70) => "Success: 70",
                    Ok(_) => "Success: other",
                    Err(err) => *err,
                };
                ctx.audit_len += 1;
            }
        }
    }

    struct MathTerminal;

    impl Terminal<TestBlackboard, MathIntent, u64, &'static str> for MathTerminal {
        fn execute(
            &mut self,
            ctx: &mut TestBlackboard,
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

    struct CacheShortCircuitLayer {
        cached_result: u64,
    }

    impl Layer<TestBlackboard, MathIntent, u64, &'static str> for CacheShortCircuitLayer {
        fn on_enter(
            &self,
            _ctx: &mut TestBlackboard,
            intent: MathIntent,
        ) -> FlowControl<MathIntent, u64, &'static str> {
            match intent {
                MathIntent::Add(0) => FlowControl::ShortCircuit(self.cached_result),
                _ => FlowControl::Proceed(intent),
            }
        }

        fn on_exit(&self, _ctx: &mut TestBlackboard, _outcome: &mut Result<u64, &'static str>) {}
    }

    #[test]
    fn test_stitch_core_u_cycle() {
        let mut ctx = TestBlackboard {
            state: 10,
            audit_len: 0,
            audit: [""; 4],
        };

        let mut pipeline = Pipeline::on_terminal(MathTerminal)
            .wrap(LimitGuardLayer { max_delta: 50 })
            .wrap(CacheShortCircuitLayer { cached_result: 999 })
            .wrap(AuditLayer);

        // 1. Success dispatch
        let res = pipeline.dispatch(&mut ctx, MathIntent::Add(25)).unwrap();
        assert_eq!(res, 35);
        assert_eq!(ctx.state, 35);

        // 2. ShortCircuit dispatch (cache hit - terminal bypassed, but ascent audit triggered)
        let res_cached = pipeline.dispatch(&mut ctx, MathIntent::Add(0)).unwrap();
        assert_eq!(res_cached, 999);
        assert_eq!(ctx.state, 35);

        // 3. Halted dispatch
        let err = pipeline
            .dispatch(&mut ctx, MathIntent::Add(100))
            .unwrap_err();
        assert_eq!(err, "Addition exceeds allowed limit");
        assert_eq!(ctx.state, 35);

        // 4. Multiplication dispatch
        let res_mul = pipeline
            .dispatch(&mut ctx, MathIntent::Multiply(2))
            .unwrap();
        assert_eq!(res_mul, 70);
        assert_eq!(ctx.state, 70);

        // 5. Verify ascent telemetry across all executions
        assert_eq!(ctx.audit_len, 4);
        assert_eq!(ctx.audit[0], "Success: 35");
        assert_eq!(ctx.audit[1], "Success: 999");
        assert_eq!(ctx.audit[2], "Addition exceeds allowed limit");
        assert_eq!(ctx.audit[3], "Success: 70");
    }

    struct SelfAuditingHaltLayer;
    impl Layer<TestBlackboard, MathIntent, u64, &'static str> for SelfAuditingHaltLayer {
        fn on_enter(
            &self,
            _ctx: &mut TestBlackboard,
            _intent: MathIntent,
        ) -> FlowControl<MathIntent, u64, &'static str> {
            FlowControl::Halt("Denied by SelfAuditingHaltLayer")
        }

        fn on_exit(&self, ctx: &mut TestBlackboard, outcome: &mut Result<u64, &'static str>) {
            if let Err(err) = outcome {
                ctx.audit[0] = *err;
                ctx.audit_len = 1;
            }
        }
    }

    #[test]
    fn test_outermost_halt_runs_own_on_exit() {
        let mut ctx = TestBlackboard {
            state: 0,
            audit_len: 0,
            audit: [""; 4],
        };

        let mut pipeline = Pipeline::on_terminal(MathTerminal).wrap(SelfAuditingHaltLayer);

        let err = pipeline.dispatch(&mut ctx, MathIntent::Add(1)).unwrap_err();
        assert_eq!(err, "Denied by SelfAuditingHaltLayer");
        assert_eq!(ctx.audit_len, 1);
        assert_eq!(ctx.audit[0], "Denied by SelfAuditingHaltLayer");
    }
}
