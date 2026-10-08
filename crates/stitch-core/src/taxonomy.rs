//! Rigid Hexagonal Taxonomy contracts for Sewing Machine Architecture (SMA).
//!
//! Enforces clean separation of policy from mechanism:
//! - **`Port`**: Pure abstract inward boundary (SPI). Zero foreign FFI types allowed.
//! - **`Adapter`**: Vendor-specific implementation of a Port. Encapsulates FFI, null guards, and panic catches.
//! - **`Hub`**: Fractal dual-role coordinator (Adapter inward to engine, Port outward to leaves).

/// Suffix Contract: Must end in `Port`.
///
/// Inward-facing SPI trait abstraction boundary.
///
/// Principles:
/// - Defined by the host or core framework.
/// - Pure Rust signatures; must never leak external vendor C-ABI types or pointers.
pub trait Port: Send + Sync {}

/// Suffix Contract: Must end in `Adapter`.
///
/// Platform-specific or vendor-specific implementation of a `Port`.
///
/// Principles:
/// - Bridges external foreign runtimes (Metamod, ReAPI, Wasmtime, POSIX) to a safe Port trait.
/// - Encapsulates all `unsafe` blocks, null pointer checks, and panic boundaries (`catch_unwind`).
/// - Services and domain core logic must never depend directly on an `Adapter`.
pub trait Adapter<P: Port + ?Sized>: Send + Sync {}

/// Suffix Contract: Must end in `Hub`.
///
/// Fractal dual-role node acting as an Adapter inward towards the core runtime,
/// and as a Port outward towards downstream services, blackboards, and pipeline layers.
pub trait Hub: Send + Sync {}

#[cfg(test)]
mod tests {
    use super::*;

    trait MockConsolePort: Port {
        fn print(&self, msg: &str);
    }

    struct MockConsoleAdapter;
    impl Port for MockConsoleAdapter {}
    impl MockConsolePort for MockConsoleAdapter {
        fn print(&self, _msg: &str) {}
    }
    impl Adapter<dyn MockConsolePort> for MockConsoleAdapter {}

    struct SubsystemHub;
    impl Hub for SubsystemHub {}

    #[test]
    fn test_taxonomy_trait_implementations() {
        let adapter = MockConsoleAdapter;
        adapter.print("hello");
        let _hub = SubsystemHub;
    }
}
