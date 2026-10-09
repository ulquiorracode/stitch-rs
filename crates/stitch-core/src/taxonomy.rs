//! Rigid Hexagonal Taxonomy contracts for Sewing Machine Architecture (SMA).
//!
//! Enforces clean separation of policy from mechanism:
//! - **`Port`**: Pure abstract inward boundary (SPI). Zero foreign FFI types allowed.
//! - **`Adapter`**: Vendor-specific implementation of a Port. Encapsulates FFI, null guards, and panic catches.
//! - **`Hub`**: Fractal dual-role coordinator (Adapter inward to engine, Port outward to leaves).

use crate::blackboard::Blackboard;
use crate::middleware::Layer;

/// Suffix Contract: Must end in `Port`.
///
/// Inward-facing SPI trait abstraction boundary.
///
/// Principles:
/// - Defined by the host or core framework.
/// - Pure Rust signatures; must never leak external vendor C-ABI types or pointers.
pub trait Port: Send + Sync {}

impl Port for () {}

/// Suffix Contract: Must end in `Adapter`.
///
/// Platform-specific or vendor-specific implementation of a `Port`.
///
/// Principles:
/// - Bridges external foreign runtimes (Metamod, ReAPI, Wasmtime, POSIX) to a safe Port trait.
/// - Encapsulates all `unsafe` blocks, null pointer checks, and panic boundaries (`catch_unwind`).
/// - Services and domain core logic must never depend directly on an `Adapter`.
pub trait Adapter<P: Port + ?Sized = ()>: Send + Sync {
    /// The target port contract implemented or bridged by this adapter.
    type TargetPort: ?Sized;
}

impl Adapter<()> for () {
    type TargetPort = ();
}

/// Architectural Dependency Inversion Contract:
///
/// Component declaring a statically verified requirement for an inward SPI [`Port`].
pub trait RequiresPort<P: Port + ?Sized> {
    /// Binds or accesses the port abstraction.
    fn port(&self) -> &P;
}

/// A composable U-cycle layer that explicitly consumes an inward Port `P`.
///
/// This statically guarantees that the layer CANNOT be constructed with or depend on
/// a foreign unabstracted vendor type; it must receive a pure [`Port`].
pub trait PortLayer<TCtx: Blackboard, TIntent, TOutcome, TErr, P: Port + ?Sized>:
    Layer<TCtx, TIntent, TOutcome, TErr> + RequiresPort<P>
{
}

impl<L, TCtx: Blackboard, TIntent, TOutcome, TErr, P: Port + ?Sized>
    PortLayer<TCtx, TIntent, TOutcome, TErr, P> for L
where
    L: Layer<TCtx, TIntent, TOutcome, TErr> + RequiresPort<P>,
{
}

/// Suffix Contract: Must end in `Hub`.
///
/// Fractal dual-role node acting as an Adapter inward towards the core runtime,
/// and as a Port outward towards downstream services, blackboards, and pipeline layers.
pub trait Hub: Send + Sync {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::FlowControl;

    trait MockConsolePort: Port {
        fn print(&self, msg: &str);
    }

    struct MockConsoleAdapter;
    impl Port for MockConsoleAdapter {}
    impl MockConsolePort for MockConsoleAdapter {
        fn print(&self, _msg: &str) {}
    }
    impl Adapter for MockConsoleAdapter {
        type TargetPort = dyn MockConsolePort;
    }

    #[repr(C, align(64))]
    struct MockBlackboard;
    impl Blackboard for MockBlackboard {}

    struct LoggingLayer<'a, P: MockConsolePort> {
        port: &'a P,
    }

    impl<'a, P: MockConsolePort> RequiresPort<P> for LoggingLayer<'a, P> {
        fn port(&self) -> &P {
            self.port
        }
    }

    impl<'a, P: MockConsolePort> Layer<MockBlackboard, &'static str, (), ()> for LoggingLayer<'a, P> {
        fn on_enter(
            &self,
            _ctx: &mut MockBlackboard,
            intent: &'static str,
        ) -> FlowControl<&'static str, (), ()> {
            self.port().print(intent);
            FlowControl::Proceed(intent)
        }

        fn on_exit(&self, _ctx: &mut MockBlackboard, _outcome: &mut Result<(), ()>) {}
    }

    struct SubsystemHub;
    impl Hub for SubsystemHub {}

    #[test]
    fn test_taxonomy_trait_implementations() {
        let adapter = MockConsoleAdapter;
        adapter.print("hello");

        let logging_layer = LoggingLayer { port: &adapter };
        let mut ctx = MockBlackboard;
        let flow = logging_layer.on_enter(&mut ctx, "test");
        assert!(flow.is_proceed());

        let _hub = SubsystemHub;
    }
}
