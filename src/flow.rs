//! Flow control decisions and pipeline error types for `stitch-rs`.

/// Decision made by a middleware during the descent phase: proceed or halt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowControl<TIntent, E> {
    /// Proceed to the next layer in the pipeline, passing the (potentially enriched) intent.
    Proceed(TIntent),

    /// Halt execution immediately with an error/reason. The needle does not descend further.
    Halt(E),
}

/// Alias using Sewing Machine Architecture terminology.
pub type Admission<TIntent, E> = FlowControl<TIntent, E>;
