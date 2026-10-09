//! Monomorphic Asynchronous U-Cycle Pipeline Engine.
//!
//! Chains middleware layers recursively using zero-sized static dispatch types.
//! Traversal operates entirely via compiler-synthesized state machines (RPITIT)
//! without `Box<dyn Future>` heap allocation.

use crate::middleware::{AsyncLayer, AsyncTerminal};
use core::future::Future;
use core::marker::PhantomData;
use stitch_core::blackboard::Blackboard;
use stitch_core::flow::FlowControl;

/// The compile-time monomorphic asynchronous execution pipeline.
pub struct AsyncPipeline<TCtx: Blackboard, TIntent, TOutcome, TErr, TChain> {
    chain: TChain,
    _phantom: PhantomData<(TCtx, TIntent, TOutcome, TErr)>,
}

/// A leaf terminal node wrapping the asynchronous terminal handler.
pub struct AsyncTerminalNode<T>(T);

impl<T> AsyncTerminalNode<T> {
    /// Wraps an async terminal handler in a terminal chain leaf node.
    #[inline(always)]
    pub(crate) const fn new(terminal: T) -> Self {
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

/// A node in the compile-time stack of asynchronous middleware layers.
pub struct AsyncStackNode<M, Inner> {
    layer: M,
    inner: Inner,
}

impl<M, Inner> AsyncStackNode<M, Inner> {
    /// Constructs a new async stack node wrapping an inner chain with an outer layer.
    #[inline(always)]
    pub(crate) const fn new(layer: M, inner: Inner) -> Self {
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
    AsyncPipeline<TCtx, TIntent, TOutcome, TErr, AsyncTerminalNode<TTerm>>
where
    TTerm: AsyncTerminal<TCtx, TIntent, TOutcome, TErr>,
{
    /// Starts constructing a new async pipeline ending with the specified terminal handler.
    pub const fn on_terminal(terminal: TTerm) -> Self {
        Self {
            chain: AsyncTerminalNode::new(terminal),
            _phantom: PhantomData,
        }
    }
}

impl<TCtx: Blackboard, TIntent, TOutcome, TErr, TChain>
    AsyncPipeline<TCtx, TIntent, TOutcome, TErr, TChain>
{
    /// Wraps the current async pipeline with an additional outer middleware layer.
    pub fn wrap<M>(
        self,
        layer: M,
    ) -> AsyncPipeline<TCtx, TIntent, TOutcome, TErr, AsyncStackNode<M, TChain>>
    where
        M: AsyncLayer<TCtx, TIntent, TOutcome, TErr>,
    {
        AsyncPipeline {
            chain: AsyncStackNode::new(layer, self.chain),
            _phantom: PhantomData,
        }
    }
}

/// Trait implemented by the entire monomorphic async stack (both layers and terminal).
pub trait AsyncPipelineChain<TCtx: Blackboard, TIntent, TOutcome, TErr>: Send + Sync {
    /// Performs the complete asynchronous U-Cycle: descent through middleware,
    /// execution at terminal, and ascent back up.
    fn cycle(
        &mut self,
        ctx: &mut TCtx,
        intent: TIntent,
    ) -> impl Future<Output = Result<TOutcome, TErr>> + Send;
}

// 1. Base case: AsyncTerminalNode (Bottom of the U)
impl<TCtx: Blackboard, TIntent: Send, TOutcome: Send, TErr: Send, TTerm>
    AsyncPipelineChain<TCtx, TIntent, TOutcome, TErr> for AsyncTerminalNode<TTerm>
where
    TTerm: AsyncTerminal<TCtx, TIntent, TOutcome, TErr>,
{
    #[inline(always)]
    async fn cycle(&mut self, ctx: &mut TCtx, intent: TIntent) -> Result<TOutcome, TErr> {
        let () = TCtx::ASSERT_CACHE_ALIGNED;
        self.0.execute(ctx, intent).await
    }
}

// 2. Recursive case: AsyncStackNode (Layer + Inner Stack)
impl<TCtx: Blackboard, TIntent: Send, TOutcome: Send, TErr: Send, M, Inner>
    AsyncPipelineChain<TCtx, TIntent, TOutcome, TErr> for AsyncStackNode<M, Inner>
where
    M: AsyncLayer<TCtx, TIntent, TOutcome, TErr>,
    Inner: AsyncPipelineChain<TCtx, TIntent, TOutcome, TErr>,
{
    #[inline(always)]
    async fn cycle(&mut self, ctx: &mut TCtx, intent: TIntent) -> Result<TOutcome, TErr> {
        let () = TCtx::ASSERT_CACHE_ALIGNED;
        // Phase 1: Descent (Спуск)
        let mut outcome = match self.layer.on_enter(ctx, intent).await {
            FlowControl::Proceed(admitted) => self.inner.cycle(ctx, admitted).await,
            FlowControl::ShortCircuit(early_outcome) => Ok(early_outcome),
            FlowControl::Halt(err) => Err(err),
        };

        // Phase 2: Ascent (Подъем)
        self.layer.on_exit(ctx, &mut outcome).await;

        outcome
    }
}

impl<TCtx: Blackboard, TIntent, TOutcome, TErr, TChain>
    AsyncPipeline<TCtx, TIntent, TOutcome, TErr, TChain>
where
    TChain: AsyncPipelineChain<TCtx, TIntent, TOutcome, TErr>,
{
    /// Dispatches an intent asynchronously through the entire pipeline:
    /// **Result = pipeline.dispatch(ctx, intent).await**.
    #[inline(always)]
    pub async fn dispatch(&mut self, ctx: &mut TCtx, intent: TIntent) -> Result<TOutcome, TErr> {
        let () = TCtx::ASSERT_CACHE_ALIGNED;
        self.chain.cycle(ctx, intent).await
    }
}
