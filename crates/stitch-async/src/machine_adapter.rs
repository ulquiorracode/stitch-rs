//! Machine-RS FSM and Algebraic Effect Adapters for `stitch-async`.
//!
//! Provides zero-cost bridges between the Sewing Machine Architecture (SMA)
//! U-cycle pipeline traversal and `machine-core` Finite State Machines.
//!
//! # Architectural Roles:
//! 1. [`MachineTerminal`]: Terminal leaf node in an SMA pipeline that steps a `machine_core::Machine`
//!    at the U-cycle puncture point, converting intent into resumption signals.
//! 2. [`PipelineStepMachine`]: Wraps a synchronous or asynchronous SMA pipeline execution
//!    as an atomic step transition inside an outer `machine_core::Machine`.

use core::marker::PhantomData;
use machine_core::{Machine, Step};
use stitch_core::blackboard::Blackboard;
use stitch_core::pipeline::Pipeline;

/// An SMA Terminal leaf node that drives a `machine_core::Machine` synchronously or asynchronously.
///
/// Converts U-cycle intents into FSM resumption transitions without dynamic allocation.
pub struct MachineTerminal<M, TCtx, TIntent, TOutcome, TErr> {
    machine: M,
    _phantom: PhantomData<(TCtx, TIntent, TOutcome, TErr)>,
}

impl<M, TCtx, TIntent, TOutcome, TErr> MachineTerminal<M, TCtx, TIntent, TOutcome, TErr> {
    /// Constructs a new `MachineTerminal` wrapping an FSM.
    #[inline(always)]
    pub const fn new(machine: M) -> Self {
        Self {
            machine,
            _phantom: PhantomData,
        }
    }

    /// Borrows the underlying state machine.
    #[inline(always)]
    pub fn machine(&self) -> &M {
        &self.machine
    }

    /// Mutably borrows the underlying state machine.
    #[inline(always)]
    pub fn machine_mut(&mut self) -> &mut M {
        &mut self.machine
    }

    /// Consumes the terminal, returning the inner state machine.
    #[inline(always)]
    pub fn into_inner(self) -> M {
        self.machine
    }
}

// Synchronous stitch-core Terminal implementation
impl<M, TCtx, TIntent> stitch_core::middleware::Terminal<TCtx, TIntent, M::Return, M::Yield>
    for MachineTerminal<M, TCtx, TIntent, M::Return, M::Yield>
where
    M: Machine<Resume = TIntent>,
{
    #[inline(always)]
    fn execute(&mut self, _ctx: &mut TCtx, intent: TIntent) -> Result<M::Return, M::Yield> {
        match self.machine.step(intent) {
            Step::Done(ret) => Ok(ret),
            Step::Yielded(yielded) => Err(yielded),
        }
    }
}

// Asynchronous stitch-async AsyncTerminal implementation
impl<M, TCtx, TIntent> crate::middleware::AsyncTerminal<TCtx, TIntent, M::Return, M::Yield>
    for MachineTerminal<M, TCtx, TIntent, M::Return, M::Yield>
where
    M: Machine<Resume = TIntent> + Send + Sync,
    TCtx: Send + Sync,
    TIntent: Send + Sync,
    M::Return: Send + Sync,
    M::Yield: Send + Sync,
{
    #[inline(always)]
    async fn execute(&mut self, _ctx: &mut TCtx, intent: TIntent) -> Result<M::Return, M::Yield> {
        match self.machine.step(intent) {
            Step::Done(ret) => Ok(ret),
            Step::Yielded(yielded) => Err(yielded),
        }
    }
}

/// An FSM adapter wrapping a `stitch-core::Pipeline` as a single-step or multi-step `Machine`.
///
/// Allows driving a complete SMA U-cycle from within an algebraic effect machine or generator loop.
pub struct PipelineStepMachine<TCtx, TIntent, TOutcome, TErr, TChain>
where
    TCtx: Blackboard,
{
    pipeline: Pipeline<TCtx, TIntent, TOutcome, TErr, TChain>,
    context: TCtx,
    is_done: bool,
}

impl<TCtx, TIntent, TOutcome, TErr, TChain>
    PipelineStepMachine<TCtx, TIntent, TOutcome, TErr, TChain>
where
    TCtx: Blackboard,
{
    /// Constructs a new machine adapter wrapping an SMA pipeline and its root blackboard context.
    #[inline(always)]
    pub const fn new(
        pipeline: Pipeline<TCtx, TIntent, TOutcome, TErr, TChain>,
        context: TCtx,
    ) -> Self {
        Self {
            pipeline,
            context,
            is_done: false,
        }
    }

    /// Accesses the underlying blackboard context.
    #[inline(always)]
    pub fn context(&self) -> &TCtx {
        &self.context
    }

    /// Mutably accesses the underlying blackboard context.
    #[inline(always)]
    pub fn context_mut(&mut self) -> &mut TCtx {
        &mut self.context
    }
}

impl<TCtx, TIntent, TOutcome, TErr, TChain> Machine
    for PipelineStepMachine<TCtx, TIntent, TOutcome, TErr, TChain>
where
    TCtx: Blackboard,
    TChain: stitch_core::pipeline::PipelineChain<TCtx, TIntent, TOutcome, TErr>,
{
    type Yield = TErr;
    type Resume = TIntent;
    type Return = TOutcome;

    #[inline(always)]
    fn step(&mut self, resume: Self::Resume) -> Step<Self::Yield, Self::Return> {
        if self.is_done {
            panic!("Contract violation: PipelineStepMachine stepped after completion");
        }

        match self.pipeline.dispatch(&mut self.context, resume) {
            Ok(outcome) => {
                self.is_done = true;
                Step::Done(outcome)
            }
            Err(err) => Step::Yielded(err),
        }
    }
}
