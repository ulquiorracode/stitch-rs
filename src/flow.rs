//! Flow control decisions and pipeline error types for `stitch-rs`.

/// Decision made by a middleware during the descent phase: proceed, short-circuit, or halt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowControl<TIntent, TOutcome, E> {
    /// Proceed to the next layer in the pipeline, passing the (potentially enriched) intent.
    Proceed(TIntent),

    /// Short-circuit the descent with an early successful outcome (e.g. cache hit).
    ///
    /// The needle does not puncture deeper layers, but the outcome immediately ascends
    /// back through outer layers via `on_exit` for auditing and telemetry.
    ShortCircuit(TOutcome),

    /// Halt execution immediately with an error/reason. The needle does not descend further.
    Halt(E),
}

/// Alias using Sewing Machine Architecture terminology.
pub type Admission<TIntent, TOutcome, E> = FlowControl<TIntent, TOutcome, E>;
