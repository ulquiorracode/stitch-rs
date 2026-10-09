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
pub struct TerminalNode<T>(T);

impl<T> TerminalNode<T> {
    /// Wraps a terminal handler in a terminal chain leaf node.
    #[inline(always)]
    pub const fn new(terminal: T) -> Self {
        Self(terminal)
    }

    /// Borrows the underlying terminal handler.
    #[inline(always)]
    pub fn terminal(&self) -> &T {
        &self.0
    }

    /// Mutably borrows the underlying terminal handler.
    #[inline(always)]
    pub fn terminal_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

/// A node in the compile-time stack of middleware layers.
pub struct StackNode<M, Inner> {
    layer: M,
    inner: Inner,
}

impl<M, Inner> StackNode<M, Inner> {
    /// Constructs a new stack node wrapping an inner chain with an outer layer.
    #[inline(always)]
    pub const fn new(layer: M, inner: Inner) -> Self {
        Self { layer, inner }
    }

    /// Borrows the outer layer of this node.
    #[inline(always)]
    pub fn layer(&self) -> &M {
        &self.layer
    }

    /// Borrows the inner chain of this node.
    #[inline(always)]
    pub fn inner(&self) -> &Inner {
        &self.inner
    }

    /// Mutably borrows the inner chain of this node.
    #[inline(always)]
    pub fn inner_mut(&mut self) -> &mut Inner {
        &mut self.inner
    }
}

impl<TCtx: Blackboard, TIntent, TOutcome, TErr, TTerm>
    Pipeline<TCtx, TIntent, TOutcome, TErr, TerminalNode<TTerm>>
where
    TTerm: Terminal<TCtx, TIntent, TOutcome, TErr>,
{
    /// Starts constructing a new pipeline ending with the specified terminal handler.
    pub const fn on_terminal(terminal: TTerm) -> Self {
        Self {
            chain: TerminalNode::new(terminal),
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
            chain: StackNode::new(layer, self.chain),
            _phantom: PhantomData,
        }
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
    ///
    /// # Panic & Resilience Policy (Contract Invariant M7)
    ///
    /// - **No ScopeGuards by Design**: The core U-cycle does **not** employ RAII scope guards
    ///   or abort-prone cleanup destructors during stack unwinding. A secondary panic in a destructor
    ///   would unconditionally abort the entire host process.
    /// - **Unwinding Aborts Traversal**: A panic occurring in any layer or terminal immediately
    ///   bypasses remaining descent and outer ascent phases. The material context `ctx` is left in an
    ///   unspecified/poisoned state.
    /// - **Application Re-entry**: Re-entering the pipeline after an uncaught panic is an application-level
    ///   contract violation.
    /// - **Host Isolation Barrier**: In userspace / host applications (e.g., FFI, plugin hosts, or servers),
    ///   use `dispatch_isolated` (enabled via the `std` feature) to catch unwinds
    ///   at the perimeter boundary and translate them into typed failure states without taking down the process.
    #[inline(always)]
    pub fn dispatch(&mut self, ctx: &mut TCtx, intent: TIntent) -> Result<TOutcome, TErr> {
        let () = TCtx::ASSERT_CACHE_ALIGNED;
        self.chain.cycle(ctx, intent)
    }

    /// Dispatches an intent within an isolated `catch_unwind` error barrier.
    ///
    /// Available strictly under `feature = "std"`.
    ///
    /// Intercepts any panics originating from downstream layers or the terminal,
    /// preventing unwinds from crossing foreign C-ABI / FFI or host boundaries.
    ///
    /// Returns:
    /// - `Ok(Ok(outcome))` on successful U-cycle traversal,
    /// - `Ok(Err(err))` on standard domain / `Halt` error,
    /// - `Err(Box<dyn Any + Send>)` if a layer or terminal panicked.
    #[cfg(feature = "std")]
    pub fn dispatch_isolated(
        &mut self,
        ctx: &mut TCtx,
        intent: TIntent,
    ) -> Result<Result<TOutcome, TErr>, std::boxed::Box<dyn core::any::Any + Send>>
    where
        TCtx: std::panic::UnwindSafe,
        TIntent: std::panic::UnwindSafe,
        Self: std::panic::UnwindSafe,
    {
        let () = TCtx::ASSERT_CACHE_ALIGNED;
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.chain.cycle(ctx, intent)
        }))
    }
}
