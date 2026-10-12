//! Rigid Hexagonal Taxonomy contracts for Sewing Machine Architecture (SMA).
//!
//! Enforces clean separation of policy from mechanism through the canonical Triad:
//!
//! ```text
//! [Port (Inward SPI)] <---> [Adapter (Bridge/Translator)] <---> [Provider (Engine/Runtime)]
//! ```
//!
//! - **`Port`**: Pure abstract inward boundary (SPI). Zero foreign FFI types allowed.
//! - **`Provider`**: External service, operating system resource, database, or native storage engine.
//! - **`Adapter`**: Bidirectional bridge translating a `Port` to/from a `Provider`.
//! - **`Hub`**: Composite or routing adapter multiplexing multiple child adapters across a shared port.

/// Suffix Contract: Must end in `Port`.
///
/// Inward-facing SPI trait abstraction boundary.
///
/// Principles:
/// - Defined by the host or core framework.
/// - Pure Rust signatures; must never leak external vendor C-ABI types or pointers.
pub trait Port: Send + Sync {}

impl Port for () {}

/// Role/Suffix Contract: Represents an external source, service, or engine of truth.
///
/// Principles:
/// - Represents physical state, an external library, an OS subsystem, or an I/O driver.
/// - Accessed exclusively through an [`Adapter`] implementing an inward [`Port`].
pub trait Provider: Send + Sync {}

impl Provider for () {}

/// Suffix Contract: Must end in `Adapter`.
///
/// Bidirectional translation bridge connecting a [`Port`] to an external [`Provider`].
///
/// Principles:
/// - Bridges external foreign runtimes (Metamod, ReAPI, Wasmtime, POSIX, SQLite, external configs) to a safe Port trait.
/// - Encapsulates all `unsafe` blocks, null pointer checks, and panic boundaries (`catch_unwind`).
/// - Replaces the legacy "Bridge" concept: any bridge is fundamentally an adapter to a provider.
/// - Services and domain core logic must never depend directly on an `Adapter`.
pub trait Adapter<P: Port + ?Sized = ()>: Send + Sync {
    /// The target port contract implemented or bridged by this adapter.
    type TargetPort: Port + ?Sized;
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

/// Composite or Routing Adapter Contract (formerly Hub).
///
/// Acts as a composite adapter multiplexing multiple child adapters, routing calls,
/// or fanning out pipeline events across downstream layers.
pub trait Hub: Send + Sync {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blackboard::Blackboard;
    use crate::flow::FlowControl;
    use crate::middleware::Layer;

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
