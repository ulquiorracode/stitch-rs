//! Command Query Segregation (CQS) abstractions for `stitch-rs`.

/// A Command that mutates material state and produces an optional atomic result.
pub trait Command<TCtx, TOutcome, TErr> {
    /// Executes the mutating command against the material context.
    fn execute(&self, ctx: &mut TCtx) -> Result<TOutcome, TErr>;
}

/// A Query that reads from material state without mutating it.
pub trait Query<TCtx, TResult> {
    /// Evaluates the read-only query against the material context.
    fn evaluate(&self, ctx: &TCtx) -> TResult;
}
