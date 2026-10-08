//! Domain Intent Accounting contracts for Sewing Machine Architecture (SMA).
//!
//! Provides formal classification of data structures outside operational pipeline nodes:
//! - **`Entity`**: Stateful domain object with a distinct identity.
//! - **`ValueObject`**: Immutable concept defined solely by its attribute values (`Copy`, zero heap).
//! - **`Event`**: Immutable intent or announcement passing through or produced by the pipeline.
//! - **`Data`**: Plain Old Data (POD) / raw memory representation.

use crate::token::StitchId;

/// Represents a stateful domain entity possessing a distinct identity (`Id`).
pub trait Entity: Send + Sync {
    /// The unique identifier type of this entity.
    type Id: StitchId;

    /// Returns the unique identifier of this entity.
    fn id(&self) -> Self::Id;
}

/// Represents an immutable value object defined purely by its structural attributes.
///
/// Invariants:
/// - Must be trivially copyable (`Copy`).
/// - Zero heap allocations (`sizeof <= 64` bytes recommended to fit inside a single cache line).
pub trait ValueObject: Copy + Clone + Eq + PartialEq + Send + Sync + 'static {}

/// Represents an immutable event or intent passing into or emitted from the pipeline.
pub trait Event: Send + Sync + 'static {}

/// Represents plain data / POD / memory buffer without behavior.
pub trait Data: Send + Sync + 'static {}
