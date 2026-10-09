//! Monomorphic U-Cycle Pipeline Engine.
//!
//! Chains middleware layers recursively using zero-sized static dispatch types.
//! In release mode, the entire U-cycle (`on_enter` -> `terminal` -> `on_exit`)
//! is fully inlined by LLVM into a single flat block of machine code.

use crate::blackboard::Blackboard;
use crate::flow::FlowControl;
use crate::middleware::{Layer, Terminal};
use core::marker::PhantomData;

/// The compile-time monomorphic execution pipeline.
pub struct Pipeline<TCtx: Blackboard, TIntent, TOutcome, TErr, TChain> {
    chain: TChain,
    _phantom: PhantomData<(TCtx, TIntent, TOutcome, TErr)>,
}

/// A leaf terminal node wrapping the terminal handler.
pub struct TerminalNode<T>(pub T);

/// A node in the compile-time stack of middleware layers.
pub struct StackNode<M, Inner> {
    pub layer: M,
    pub inner: Inner,
}

impl<TCtx: Blackboard, TIntent, TOutcome, TErr, TTerm>
    Pipeline<TCtx, TIntent, TOutcome, TErr, TerminalNode<TTerm>>
where
    TTerm: Terminal<TCtx, TIntent, TOutcome, TErr>,
{
    /// Starts constructing a new pipeline ending with the specified terminal handler.
    pub const fn on_terminal(terminal: TTerm) -> Self {
        Self {
            chain: TerminalNode(terminal),
            _phantom: PhantomData,
        }
    }
}

impl<TCtx: Blackboard, TIntent, TOutcome, TErr, TChain>
    Pipeline<TCtx, TIntent, TOutcome, TErr, TChain>
{
    /// Wraps the current pipeline with an additional outer middleware layer.
    pub fn wrap<M>(self, layer: M) -> Pipeline<TCtx, TIntent, TOutcome, TErr, StackNode<M, TChain>>
    where
        M: Layer<TCtx, TIntent, TOutcome, TErr>,
    {
        Pipeline {
            chain: StackNode {
                layer,
                inner: self.chain,
            },
            _phantom: PhantomData,
        }
    }

    /// Compatibility alias for `wrap`.
    #[inline(always)]
    pub fn use_middleware<M>(
        self,
        middleware: M,
    ) -> Pipeline<TCtx, TIntent, TOutcome, TErr, StackNode<M, TChain>>
    where
        M: Layer<TCtx, TIntent, TOutcome, TErr>,
    {
        self.wrap(middleware)
    }
}

use crate::sealed::Sealed;

impl<TTerm> Sealed for TerminalNode<TTerm> {}
impl<M, Inner> Sealed for StackNode<M, Inner> {}

/// Trait implemented by the entire monomorphic stack (both layers and terminal).
///
/// This trait is sealed: only canonical SMA pipeline nodes ([`TerminalNode`] and [`StackNode`])
/// can implement it, mathematically guaranteeing U-cycle descent and ascent invariants.
pub trait PipelineChain<TCtx: Blackboard, TIntent, TOutcome, TErr>: Sealed {
    /// Performs the complete U-Cycle: descent through middleware,
    /// execution at terminal, and ascent back up.
    fn cycle(&mut self, ctx: &mut TCtx, intent: TIntent) -> Result<TOutcome, TErr>;
}

// 1. Base case: TerminalNode (Дно буквы U)
impl<TCtx: Blackboard, TIntent, TOutcome, TErr, TTerm> PipelineChain<TCtx, TIntent, TOutcome, TErr>
    for TerminalNode<TTerm>
where
    TTerm: Terminal<TCtx, TIntent, TOutcome, TErr>,
{
    #[inline(always)]
    fn cycle(&mut self, ctx: &mut TCtx, intent: TIntent) -> Result<TOutcome, TErr> {
        self.0.execute(ctx, intent)
    }
}

// 2. Recursive case: Layer + Inner Stack
impl<TCtx: Blackboard, TIntent, TOutcome, TErr, M, Inner>
    PipelineChain<TCtx, TIntent, TOutcome, TErr> for StackNode<M, Inner>
where
    TCtx: Blackboard,
    M: Layer<TCtx, TIntent, TOutcome, TErr>,
    Inner: PipelineChain<TCtx, TIntent, TOutcome, TErr>,
{
    #[inline(always)]
    fn cycle(&mut self, ctx: &mut TCtx, intent: TIntent) -> Result<TOutcome, TErr> {
        // Phase 1: Descent (Спуск)
        let mut outcome = match self.layer.on_enter(ctx, intent) {
            FlowControl::Proceed(admitted) => self.inner.cycle(ctx, admitted),
            FlowControl::ShortCircuit(early_outcome) => Ok(early_outcome),
            FlowControl::Halt(err) => Err(err),
        };

        // Phase 2: Ascent (Подъем) - halting layer and outer layers participate in ascent
        self.layer.on_exit(ctx, &mut outcome);

        outcome
    }
}

impl<TCtx: Blackboard, TIntent, TOutcome, TErr, TChain>
    Pipeline<TCtx, TIntent, TOutcome, TErr, TChain>
where
    TChain: PipelineChain<TCtx, TIntent, TOutcome, TErr>,
{
    /// Dispatches an intent through the entire pipeline: **Result = Pipeline::dispatch(Material, Intent)**.
    #[inline(always)]
    pub fn dispatch(&mut self, ctx: &mut TCtx, intent: TIntent) -> Result<TOutcome, TErr> {
        let () = TCtx::ASSERT_CACHE_ALIGNED;
        self.chain.cycle(ctx, intent)
    }

    /// Alias using Sewing Machine Architecture terminology.
    #[inline(always)]
    pub fn stitch(&mut self, ctx: &mut TCtx, intent: TIntent) -> Result<TOutcome, TErr> {
        self.dispatch(ctx, intent)
    }
}

/// Compatibility alias for `Pipeline`. Prefer `Pipeline` in Sewing Machine Architecture.
#[deprecated(since = "0.2.0", note = "Use `Pipeline` instead")]
pub type Machine<TCtx, TIntent, TOutcome, TErr, TChain> =
    Pipeline<TCtx, TIntent, TOutcome, TErr, TChain>;
