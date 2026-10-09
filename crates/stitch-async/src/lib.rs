//! # Stitch Async (`stitch-async`)
//!
//! Zero-Cost, Monomorphic Asynchronous U-Cycle Middleware Pipeline & Outbox Streaming Egress.
//!
//! Extends the **Sewing Machine Architecture (SMA)** into the asynchronous execution domain
//! using native Rust Return-Position `impl Trait` in Trait (RPITIT) without compulsory heap allocation
//! (`Box<dyn Future>`).
//!
//! # Core Taxonomical Dimensions
//! - **Async Operational Nodes**: [`AsyncLayer`], [`AsyncTerminal`], [`AsyncPipeline`]
//! - **Streaming Egress Worker**: [`OutboxEgressWorker`], [`OutboxEgressSink`]

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

pub mod egress;
pub mod middleware;
pub mod pipeline;

pub use egress::{OutboxEgressSink, OutboxEgressWorker};
pub use middleware::{AsyncLayer, AsyncTerminal};
pub use pipeline::{AsyncPipeline, AsyncPipelineChain, AsyncStackNode, AsyncTerminalNode};

/// Common imports for `stitch-async`.
pub mod prelude {
    pub use crate::egress::{OutboxEgressSink, OutboxEgressWorker};
    pub use crate::middleware::{AsyncLayer, AsyncTerminal};
    pub use crate::pipeline::{
        AsyncPipeline, AsyncPipelineChain, AsyncStackNode, AsyncTerminalNode,
    };
    pub use stitch_core::prelude::*;
}
