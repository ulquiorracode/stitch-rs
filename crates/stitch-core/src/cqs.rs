//! Command abstractions for `stitch-rs`.

/// A Command that mutates material state and produces an optional atomic result.
pub trait Command<TCtx, TOutcome, TErr> {
    /// Executes the mutating command against the material context.
    fn execute(&self, ctx: &mut TCtx) -> Result<TOutcome, TErr>;
}

use crate::blackboard::Blackboard;
use crate::middleware::Terminal;
use crate::pipeline::{Pipeline, TerminalNode};

/// Terminal executor bridging any [`Command`] to terminate a monomorphic U-cycle pipeline.
#[derive(Debug, Clone, Copy, Default)]
pub struct CommandExecutor;

impl<TCtx, C, TOutcome, TErr> Terminal<TCtx, C, TOutcome, TErr> for CommandExecutor
where
    C: Command<TCtx, TOutcome, TErr>,
{
    #[inline(always)]
    fn execute(&mut self, ctx: &mut TCtx, command: C) -> Result<TOutcome, TErr> {
        command.execute(ctx)
    }
}

impl<TCtx: Blackboard, C: Command<TCtx, TOutcome, TErr>, TOutcome, TErr>
    Pipeline<TCtx, C, TOutcome, TErr, TerminalNode<CommandExecutor>>
{
    /// Constructs a typed U-cycle pipeline strictly terminated by a [`CommandExecutor`].
    ///
    /// This establishes a type-level contract ensuring the pipeline ONLY dispatches
    /// intent types implementing [`Command<TCtx, TOutcome, TErr>`].
    #[inline(always)]
    pub const fn for_command() -> Self {
        Pipeline::on_terminal(CommandExecutor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::FlowControl;
    use crate::middleware::Layer;

    #[repr(C, align(64))]
    struct MaterialContext {
        pub counter: u32,
    }
    impl Blackboard for MaterialContext {}

    struct IncrementCommand {
        pub delta: u32,
    }

    impl Command<MaterialContext, u32, ()> for IncrementCommand {
        fn execute(&self, ctx: &mut MaterialContext) -> Result<u32, ()> {
            ctx.counter += self.delta;
            Ok(ctx.counter)
        }
    }

    struct GuardLayer;
    impl Layer<MaterialContext, IncrementCommand, u32, ()> for GuardLayer {
        fn on_enter(
            &self,
            _ctx: &mut MaterialContext,
            intent: IncrementCommand,
        ) -> FlowControl<IncrementCommand, u32, ()> {
            if intent.delta == 0 {
                FlowControl::Halt(())
            } else {
                FlowControl::Proceed(intent)
            }
        }

        fn on_exit(&self, _ctx: &mut MaterialContext, _outcome: &mut Result<u32, ()>) {}
    }

    #[test]
    fn test_pipeline_for_command_type_enforcement() {
        let mut pipeline = Pipeline::for_command().wrap(GuardLayer);
        let mut ctx = MaterialContext { counter: 10 };

        let res = pipeline.dispatch(&mut ctx, IncrementCommand { delta: 5 });
        assert_eq!(res, Ok(15));
        assert_eq!(ctx.counter, 15);

        let err = pipeline.dispatch(&mut ctx, IncrementCommand { delta: 0 });
        assert_eq!(err, Err(()));
        assert_eq!(ctx.counter, 15);
    }
}
