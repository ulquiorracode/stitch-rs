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
//! - **Flow Control**: [`FlowControl`]

#![cfg_attr(not(feature = "std"), no_std)]

pub mod blackboard;
pub mod cqs;
pub mod data;
pub mod flow;
pub mod hash;
pub mod middleware;
pub mod outbox;
pub mod pipeline;
mod sealed;
pub mod taxonomy;
pub mod token;

pub use blackboard::Blackboard;
pub use cqs::{Command, CommandExecutor, CommandPermit, PropRead, PropWrite, Query, QueryExecutor};
pub use data::{Data, Entity, Event, ValueObject};
pub use flow::FlowControl;
pub use hash::{fnv1a_32, fnv1a_64};
pub use middleware::{Layer, StatelessLayer, StatelessLayerTable, Terminal};
pub use outbox::Outbox;
pub use pipeline::{Pipeline, PipelineChain, StackNode, TerminalNode};
pub use taxonomy::{Adapter, Hub, Port};
pub use token::{RawId, RawToken, StitchId, StitchToken};

/// Common imports and contracts for Sewing Machine Architecture (SMA).
pub mod prelude {
    pub use crate::assert_blackboard_aligned;
    pub use crate::blackboard::Blackboard;
    pub use crate::cqs::{
        Command, CommandExecutor, CommandPermit, PropRead, PropWrite, Query, QueryExecutor,
    };
    pub use crate::data::{Data, Entity, Event, ValueObject};
    pub use crate::define_layer_enum;
    pub use crate::flow::FlowControl;
    pub use crate::hash::{fnv1a_32, fnv1a_64};
    pub use crate::middleware::{Layer, StatelessLayer, StatelessLayerTable, Terminal};
    pub use crate::outbox::Outbox;
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

    #[derive(Clone, Copy)]
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

    #[cfg(feature = "std")]
    struct PanickingLayer;

    #[cfg(feature = "std")]
    impl Layer<TestBlackboard, MathIntent, u64, &'static str> for PanickingLayer {
        fn on_enter(
            &self,
            _ctx: &mut TestBlackboard,
            _intent: MathIntent,
        ) -> FlowControl<MathIntent, u64, &'static str> {
            panic!("Deliberate layer panic to test isolation barrier");
        }

        fn on_exit(&self, _ctx: &mut TestBlackboard, _outcome: &mut Result<u64, &'static str>) {}
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_dispatch_isolated_catches_unwind_barrier() {
        let mut ctx = TestBlackboard {
            state: 0,
            audit_len: 0,
            audit: [""; 4],
        };

        let mut pipeline = Pipeline::on_terminal(MathTerminal).wrap(PanickingLayer);

        let isolated_res = pipeline.dispatch_isolated(&mut ctx, MathIntent::Add(42));
        assert!(
            isolated_res.is_err(),
            "dispatch_isolated must catch unwind without process abort"
        );
    }

    #[test]
    #[cfg(feature = "std")]
    fn test_poisoned_context_state_after_panic_and_safe_reinit() {
        struct PoisoningLayer;
        impl Layer<TestBlackboard, MathIntent, u64, &'static str> for PoisoningLayer {
            fn on_enter(
                &self,
                ctx: &mut TestBlackboard,
                _intent: MathIntent,
            ) -> FlowControl<MathIntent, u64, &'static str> {
                // Mutate context half-way before panicking (simulating interrupted transaction)
                ctx.state = 999;
                ctx.audit[0] = "halfway_dirty_state";
                ctx.audit_len = 1;
                panic!("Catastrophic worker fault inside layer");
            }
            fn on_exit(&self, _ctx: &mut TestBlackboard, _outcome: &mut Result<u64, &'static str>) {
            }
        }

        let mut ctx = TestBlackboard {
            state: 10,
            audit_len: 0,
            audit: [""; 4],
        };

        let mut failing_pipe = Pipeline::on_terminal(MathTerminal).wrap(PoisoningLayer);
        let res = failing_pipe.dispatch_isolated(&mut ctx, MathIntent::Add(5));
        assert!(res.is_err(), "Must intercept unwind barrier");

        // Contract Invariant M7 verification:
        // Traversal was aborted immediately; context was left in mutated/poisoned state
        assert_eq!(ctx.state, 999, "State reflects dirty partial mutation");
        assert_eq!(ctx.audit[0], "halfway_dirty_state");

        // Safe operational lifecycle: re-entering requires explicit context reinitialization / sanitization
        ctx.state = 0;
        ctx.audit_len = 0;
        ctx.audit = [""; 4];

        let mut clean_pipe = Pipeline::on_terminal(MathTerminal);
        let clean_res = clean_pipe.dispatch(&mut ctx, MathIntent::Add(15));
        assert_eq!(clean_res, Ok(15));
        assert_eq!(ctx.state, 15);
    }

    struct MultiplierLayer {
        factor: u64,
    }

    impl Layer<TestBlackboard, MathIntent, u64, &'static str> for MultiplierLayer {
        fn on_enter(
            &self,
            _ctx: &mut TestBlackboard,
            intent: MathIntent,
        ) -> FlowControl<MathIntent, u64, &'static str> {
            match intent {
                MathIntent::Add(v) => FlowControl::Proceed(MathIntent::Add(v * self.factor)),
                other => FlowControl::Proceed(other),
            }
        }
        fn on_exit(&self, _ctx: &mut TestBlackboard, _outcome: &mut Result<u64, &'static str>) {}
    }

    crate::define_layer_enum! {
        enum DynamicMathLayer<TestBlackboard, MathIntent, u64, &'static str> {
            Guard(LimitGuardLayer),
            Multiplier(MultiplierLayer),
        }
    }

    #[test]
    fn test_define_layer_enum_dispatch() {
        let mut ctx = TestBlackboard {
            state: 10,
            audit_len: 0,
            audit: [""; 4],
        };

        let dynamic_layer = DynamicMathLayer::Multiplier(MultiplierLayer { factor: 2 });
        let mut pipeline = Pipeline::on_terminal(MathTerminal).wrap(dynamic_layer);

        let res = pipeline.dispatch(&mut ctx, MathIntent::Add(5)).unwrap();
        assert_eq!(res, 20); // 10 + (5 * 2) = 20
        assert_eq!(ctx.state, 20);
    }

    #[test]
    fn test_stateless_layer_table_dispatch() {
        let mut ctx = TestBlackboard {
            state: 10,
            audit_len: 0,
            audit: [""; 4],
        };

        fn filter_negative(
            _ctx: &mut TestBlackboard,
            intent: MathIntent,
        ) -> FlowControl<MathIntent, u64, &'static str> {
            FlowControl::Proceed(intent)
        }

        fn log_exit(_ctx: &mut TestBlackboard, _outcome: &mut Result<u64, &'static str>) {}

        let table_layers = [StatelessLayer {
            on_enter_fn: filter_negative,
            on_exit_fn: log_exit,
        }];

        let table = StatelessLayerTable::new(&table_layers);
        let mut pipeline = Pipeline::on_terminal(MathTerminal).wrap(table);

        let res = pipeline.dispatch(&mut ctx, MathIntent::Add(7)).unwrap();
        assert_eq!(res, 17);
        assert_eq!(ctx.state, 17);
    }
}
