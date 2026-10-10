//! Asynchronous Middleware Layer contracts for `stitch-rs`.
//!
//! Provides zero-cost RPITIT (Rust 2024 / 1.85+) async contracts for U-cycle traversal,
//! avoiding boxed futures or dynamic dispatch on hot execution paths.

use core::future::Future;
use stitch_core::flow::FlowControl;

/// An asynchronous composable layer through which the U-cycle dispatch passes.
///
/// Fully generic across context (`TCtx`), intent (`TIntent`), outcome (`TOutcome`),
/// and failure (`TErr`).
pub trait AsyncLayer<TCtx, TIntent = (), TOutcome = (), TErr = ()>: Send + Sync {
    /// Phase 1: Descent (Entering the pipeline asynchronously).
    ///
    /// Evaluates whether to proceed down to deeper layers.
    /// Returns:
    /// - `FlowControl::Proceed(intent)` to continue descending,
    /// - `FlowControl::ShortCircuit(outcome)` to return an immediate success and begin ascent,
    /// - `FlowControl::Halt(reason)` to abort execution with error.
    fn on_enter(
        &self,
        ctx: &mut TCtx,
        intent: TIntent,
    ) -> impl Future<Output = FlowControl<TIntent, TOutcome, TErr>> + Send;

    /// Phase 2: Ascent (Exiting the pipeline asynchronously).
    ///
    /// Observes the outcome emerging from the layer below and tightens the pipeline
    /// (e.g. emits diffs, records telemetry, triggers reactions).
    ///
    /// # Total Function Contract (Panic Prohibition)
    ///
    /// Implementations of `on_exit` **must never panic** under any circumstances.
    /// It operates strictly over an already produced `&mut Result<TOutcome, TErr>`.
    fn on_exit(
        &self,
        ctx: &mut TCtx,
        outcome: &mut Result<TOutcome, TErr>,
    ) -> impl Future<Output = ()> + Send;
}

/// The bottom-most asynchronous terminal execution handler that executes against the material.
///
/// This is the "Point of Puncture" (Точка прокола / Дно буквы U).
pub trait AsyncTerminal<TCtx, TIntent = (), TOutcome = (), TErr = ()>: Send + Sync {
    /// Executes the intent directly and returns the atomic outcome asynchronously.
    fn execute(
        &mut self,
        ctx: &mut TCtx,
        intent: TIntent,
    ) -> impl Future<Output = Result<TOutcome, TErr>> + Send;
}
