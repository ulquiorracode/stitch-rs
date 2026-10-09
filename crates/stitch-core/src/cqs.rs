//! Command abstractions for `stitch-rs`.

/// A Command that mutates material state and produces an optional atomic result.
pub trait Command<TCtx, TOutcome, TErr> {
    /// Executes the mutating command against the material context.
    fn execute(&self, ctx: &mut TCtx) -> Result<TOutcome, TErr>;
}

/// A Query that reads state strictly without mutating the material context.
///
/// Guaranteed to receive an immutable `&TCtx`.
pub trait Query<TCtx, TResult, TErr> {
    /// Executes the non-mutating query against the material context.
    fn query(&self, ctx: &TCtx) -> Result<TResult, TErr>;
}

/// A linear capability token required to perform mutating operations.
///
/// Under CQS, queries are executed without access to a `CommandPermit`.
/// If a context or entity uses interior mutability (`RefCell`, `Mutex`),
/// mutation methods can demand `&mut CommandPermit` as a linear witness proof,
/// making illegal interior mutations during queries impossible at compile time.
pub struct CommandPermit<'a, TCtx: ?Sized> {
    ctx: &'a mut TCtx,
}

impl<'a, TCtx: ?Sized> CommandPermit<'a, TCtx> {
    /// Constructs a linear permit proving mutable execution context authority.
    ///
    /// Borrows the mutable context `&'a mut TCtx`, ensuring at most one active permit
    /// exists and preventing overlapping permits under Rust's exclusive borrow rules.
    #[inline(always)]
    pub fn new(ctx: &'a mut TCtx) -> Self {
        Self { ctx }
    }

    /// Provides safe access to the mutable context through the permit witness.
    #[inline(always)]
    pub fn ctx_mut(&mut self) -> &mut TCtx {
        self.ctx
    }
}

/// Granular property read contract (fine-grained CQS).
pub trait PropRead<Target> {
    /// Reads a property from the target without side-effects.
    fn read(target: &Target) -> Self;
}

/// Granular property write contract (fine-grained CQS).
pub trait PropWrite<Target: ?Sized> {
    /// Mutates the target with the given property value through an authorized permit.
    fn write(self, permit: &mut CommandPermit<'_, Target>);
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

/// Terminal executor bridging any [`Query`] to terminate a monomorphic U-cycle pipeline.
#[derive(Debug, Clone, Copy, Default)]
pub struct QueryExecutor;

impl<TCtx, Q, TResult, TErr> Terminal<TCtx, Q, TResult, TErr> for QueryExecutor
where
    Q: Query<TCtx, TResult, TErr>,
{
    #[inline(always)]
    fn execute(&mut self, ctx: &mut TCtx, query: Q) -> Result<TResult, TErr> {
        query.query(ctx)
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

impl<TCtx: Blackboard, Q: Query<TCtx, TResult, TErr>, TResult, TErr>
    Pipeline<TCtx, Q, TResult, TErr, TerminalNode<QueryExecutor>>
{
    /// Constructs a typed U-cycle pipeline strictly terminated by a [`QueryExecutor`].
    ///
    /// This establishes a type-level contract ensuring the pipeline ONLY dispatches
    /// intent types implementing [`Query<TCtx, TResult, TErr>`].
    #[inline(always)]
    pub const fn for_query() -> Self {
        Pipeline::on_terminal(QueryExecutor)
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

    struct ReadCounterQuery;
    impl Query<MaterialContext, u32, ()> for ReadCounterQuery {
        fn query(&self, ctx: &MaterialContext) -> Result<u32, ()> {
            Ok(ctx.counter)
        }
    }

    struct QueryLogLayer;
    impl Layer<MaterialContext, ReadCounterQuery, u32, ()> for QueryLogLayer {
        fn on_enter(
            &self,
            _ctx: &mut MaterialContext,
            intent: ReadCounterQuery,
        ) -> FlowControl<ReadCounterQuery, u32, ()> {
            FlowControl::Proceed(intent)
        }

        fn on_exit(&self, _ctx: &mut MaterialContext, _outcome: &mut Result<u32, ()>) {}
    }

    #[test]
    fn test_pipeline_for_query_type_enforcement() {
        let mut pipeline = Pipeline::for_query().wrap(QueryLogLayer);
        let mut ctx = MaterialContext { counter: 42 };

        let res = pipeline.dispatch(&mut ctx, ReadCounterQuery);
        assert_eq!(res, Ok(42));
    }

    struct GuardedContext {
        val: u32,
    }

    impl GuardedContext {
        pub fn get_val(&self) -> u32 {
            self.val
        }

        pub fn set_val(&mut self, new_val: u32) {
            self.val = new_val;
        }
    }

    #[test]
    fn test_command_permit_witness() {
        let mut ctx = GuardedContext { val: 100 };
        assert_eq!(ctx.get_val(), 100);

        {
            let mut permit = CommandPermit::new(&mut ctx);
            permit.ctx_mut().set_val(200);
        }
        assert_eq!(ctx.get_val(), 200);
    }

    struct CounterProp(pub u32);

    impl PropRead<MaterialContext> for CounterProp {
        fn read(target: &MaterialContext) -> Self {
            CounterProp(target.counter)
        }
    }

    impl PropWrite<MaterialContext> for CounterProp {
        fn write(self, permit: &mut CommandPermit<'_, MaterialContext>) {
            permit.ctx_mut().counter = self.0;
        }
    }

    #[test]
    fn test_prop_read_write_contracts() {
        let mut ctx = MaterialContext { counter: 50 };
        let read = CounterProp::read(&ctx);
        assert_eq!(read.0, 50);

        {
            let mut permit = CommandPermit::new(&mut ctx);
            CounterProp(99).write(&mut permit);
        }
        assert_eq!(ctx.counter, 99);
    }
}
