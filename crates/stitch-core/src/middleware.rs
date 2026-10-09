//! Composable Middleware Layer contracts for `stitch-rs`.

use crate::flow::FlowControl;

/// A composable layer through which the U-cycle dispatch passes.
///
/// Fully generic across context (`TCtx`), intent (`TIntent`), outcome (`TOutcome`),
/// and failure (`TErr`).
pub trait Layer<TCtx, TIntent = (), TOutcome = (), TErr = ()> {
    /// Phase 1: Descent (Entering the pipeline).
    ///
    /// Evaluates whether to proceed down to deeper layers.
    /// Returns:
    /// - `FlowControl::Proceed(intent)` to continue descending,
    /// - `FlowControl::ShortCircuit(outcome)` to return an immediate success and begin ascent,
    /// - `FlowControl::Halt(reason)` to abort execution with error.
    fn on_enter(&self, ctx: &mut TCtx, intent: TIntent) -> FlowControl<TIntent, TOutcome, TErr>;

    /// Phase 2: Ascent (Exiting the pipeline).
    ///
    /// Observes the outcome emerging from the layer below and tightens the pipeline
    /// (e.g. emits diffs, records telemetry, triggers reactions).
    fn on_exit(&self, ctx: &mut TCtx, outcome: &mut Result<TOutcome, TErr>);
}

/// The bottom-most terminal execution handler that executes against the material.
///
/// This is the "Point of Puncture" (Точка прокола / Дно буквы U).
pub trait Terminal<TCtx, TIntent = (), TOutcome = (), TErr = ()> {
    /// Executes the intent directly and returns the atomic outcome.
    fn execute(&mut self, ctx: &mut TCtx, intent: TIntent) -> Result<TOutcome, TErr>;
}

/// Compatibility alias for `Layer`. Prefer `Layer` in Sewing Machine Architecture.
#[deprecated(since = "0.2.0", note = "Use `Layer` instead")]
pub trait Middleware<TCtx, TIntent = (), TOutcome = (), TErr = ()>:
    Layer<TCtx, TIntent, TOutcome, TErr>
{
}

#[allow(deprecated)]
impl<T, TCtx, TIntent, TOutcome, TErr> Middleware<TCtx, TIntent, TOutcome, TErr> for T where
    T: Layer<TCtx, TIntent, TOutcome, TErr>
{
}

/// Compatibility alias for `Terminal`. Prefer `Terminal` in Sewing Machine Architecture.
#[deprecated(since = "0.2.0", note = "Use `Terminal` instead")]
pub trait TerminalHandler<TCtx, TIntent = (), TOutcome = (), TErr = ()>:
    Terminal<TCtx, TIntent, TOutcome, TErr>
{
}

#[allow(deprecated)]
impl<T, TCtx, TIntent, TOutcome, TErr> TerminalHandler<TCtx, TIntent, TOutcome, TErr> for T where
    T: Terminal<TCtx, TIntent, TOutcome, TErr>
{
}
