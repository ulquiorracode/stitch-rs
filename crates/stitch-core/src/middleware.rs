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
    ///
    /// # Total Function Contract (Panic Prohibition)
    ///
    /// Implementations of `on_exit` **must never panic** under any circumstances.
    /// It operates strictly over an already produced `&mut Result<TOutcome, TErr>`.
    /// Panicking during ascent violates total execution guarantees, leaves enclosing
    /// layers unclosed, and poisons the material context.
    fn on_exit(&self, ctx: &mut TCtx, outcome: &mut Result<TOutcome, TErr>);
}

/// The bottom-most terminal execution handler that executes against the material.
///
/// This is the "Point of Puncture" (Точка прокола / Дно буквы U).
pub trait Terminal<TCtx, TIntent = (), TOutcome = (), TErr = ()> {
    /// Executes the intent directly and returns the atomic outcome.
    fn execute(&mut self, ctx: &mut TCtx, intent: TIntent) -> Result<TOutcome, TErr>;
}

/// Macro generating a closed-set enum dispatcher implementing [`Layer`].
///
/// Dispatches dynamically across a finite set of layers without heap allocations (`Box<dyn Layer>`),
/// fitting entirely in L1 instruction cache and achieving near monomorphic throughput (~1.75 ns vs 5.24 ns).
#[macro_export]
macro_rules! define_layer_enum {
    (
        $(#[$enum_meta:meta])*
        $vis:vis enum $enum_name:ident < $ctx:ty, $intent:ty, $outcome:ty, $err:ty > {
            $(
                $(#[$var_meta:meta])*
                $variant:ident($layer_type:ty)
            ),+ $(,)?
        }
    ) => {
        $(#[$enum_meta])*
        #[allow(dead_code)]
        $vis enum $enum_name {
            $(
                $(#[$var_meta])*
                $variant($layer_type),
            )+
        }

        impl $crate::middleware::Layer<$ctx, $intent, $outcome, $err> for $enum_name {
            #[inline(always)]
            fn on_enter(
                &self,
                ctx: &mut $ctx,
                intent: $intent,
            ) -> $crate::flow::FlowControl<$intent, $outcome, $err> {
                match self {
                    $(
                        Self::$variant(layer) => layer.on_enter(ctx, intent),
                    )+
                }
            }

            #[inline(always)]
            fn on_exit(
                &self,
                ctx: &mut $ctx,
                outcome: &mut Result<$outcome, $err>,
            ) {
                match self {
                    $(
                        Self::$variant(layer) => layer.on_exit(ctx, outcome),
                    )+
                }
            }
        }
    };
}

/// A stateless function-pointer layer for zero-allocation dynamic middleware tables.
#[derive(Clone, Copy)]
pub struct StatelessLayer<TCtx, TIntent, TOutcome, TErr> {
    /// Descent function pointer.
    pub on_enter_fn: fn(&mut TCtx, TIntent) -> FlowControl<TIntent, TOutcome, TErr>,
    /// Ascent function pointer.
    pub on_exit_fn: fn(&mut TCtx, &mut Result<TOutcome, TErr>),
}

impl<TCtx, TIntent, TOutcome, TErr> Layer<TCtx, TIntent, TOutcome, TErr>
    for StatelessLayer<TCtx, TIntent, TOutcome, TErr>
{
    #[inline(always)]
    fn on_enter(&self, ctx: &mut TCtx, intent: TIntent) -> FlowControl<TIntent, TOutcome, TErr> {
        (self.on_enter_fn)(ctx, intent)
    }

    #[inline(always)]
    fn on_exit(&self, ctx: &mut TCtx, outcome: &mut Result<TOutcome, TErr>) {
        (self.on_exit_fn)(ctx, outcome)
    }
}

/// Contiguous flat table of stateless middleware layers.
///
/// Traverses an array or slice of function pointers in a zero-alloc contiguous memory layout.
/// Ideal for dynamically configured plugins where types cannot be known at compile time.
pub struct StatelessLayerTable<'a, TCtx, TIntent, TOutcome, TErr> {
    layers: &'a [StatelessLayer<TCtx, TIntent, TOutcome, TErr>],
}

impl<'a, TCtx, TIntent, TOutcome, TErr> StatelessLayerTable<'a, TCtx, TIntent, TOutcome, TErr> {
    /// Constructs a new table wrapping a contiguous slice of stateless layers.
    #[inline(always)]
    pub const fn new(layers: &'a [StatelessLayer<TCtx, TIntent, TOutcome, TErr>]) -> Self {
        Self { layers }
    }
}

impl<'a, TCtx, TIntent, TOutcome, TErr> Layer<TCtx, TIntent, TOutcome, TErr>
    for StatelessLayerTable<'a, TCtx, TIntent, TOutcome, TErr>
where
    TIntent: Clone,
{
    fn on_enter(
        &self,
        ctx: &mut TCtx,
        mut intent: TIntent,
    ) -> FlowControl<TIntent, TOutcome, TErr> {
        for layer in self.layers {
            match (layer.on_enter_fn)(ctx, intent) {
                FlowControl::Proceed(admitted) => {
                    intent = admitted;
                }
                FlowControl::ShortCircuit(outcome) => {
                    return FlowControl::ShortCircuit(outcome);
                }
                FlowControl::Halt(err) => {
                    return FlowControl::Halt(err);
                }
            }
        }
        FlowControl::Proceed(intent)
    }

    fn on_exit(&self, ctx: &mut TCtx, outcome: &mut Result<TOutcome, TErr>) {
        // Ascent in reverse order
        for layer in self.layers.iter().rev() {
            (layer.on_exit_fn)(ctx, outcome);
        }
    }
}
