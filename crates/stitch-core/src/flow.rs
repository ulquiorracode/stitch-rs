//! Flow control decisions and pipeline error types for `stitch-rs`.

/// Decision made by a middleware during the descent phase: proceed, short-circuit, or halt.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowControl<TIntent = (), TOutcome = (), E = ()> {
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

/// Convenience alias for Blackboard-centric pipelines where intent and outcome are unit `()`.
pub type BlackboardFlow<E = ()> = FlowControl<(), (), E>;

impl<TIntent, TOutcome, E> FlowControl<TIntent, TOutcome, E> {
    /// Returns true if this decision is `Proceed`.
    #[inline(always)]
    pub const fn is_proceed(&self) -> bool {
        matches!(self, Self::Proceed(_))
    }

    /// Returns true if this decision is `ShortCircuit`.
    #[inline(always)]
    pub const fn is_short_circuit(&self) -> bool {
        matches!(self, Self::ShortCircuit(_))
    }

    /// Returns true if this decision is `Halt`.
    #[inline(always)]
    pub const fn is_halt(&self) -> bool {
        matches!(self, Self::Halt(_))
    }
}
