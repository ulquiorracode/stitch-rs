//! Context Blackboard contract and Scrooge cache-line alignment contracts.

/// Suffix Contract: Must end in `Context` or `Blackboard`.
///
/// A `Blackboard` represents the mutable scratchpad / material context passed down and up
/// through the Sewing Machine Architecture U-cycle traversal.
///
/// Mechanical Sympathy Invariants:
/// - Must align to a 64-byte boundary (`#[repr(C, align(64))]`) to match L1D cache-line size
///   and eliminate false sharing between concurrent worker threads.
/// - Must avoid dynamic heap allocations (`String`, `Vec`, `Box`) on hot paths.
pub trait Blackboard: Send + Sync + Sized {
    /// Asserts at compile-time that the implementor satisfies the 64-byte alignment requirement.
    const ASSERT_CACHE_ALIGNED: () = {
        assert!(
            core::mem::align_of::<Self>() >= 64,
            "Blackboard types must be aligned to at least 64 bytes (L1D cache line). Add `#[repr(C, align(64))]`."
        );
    };
}

/// Helper macro for asserting that a type satisfies Blackboard cache line alignment.
#[macro_export]
macro_rules! assert_blackboard_aligned {
    ($ty:ty) => {
        const _: () = {
            assert!(
                ::core::mem::align_of::<$ty>() >= 64,
                concat!(
                    "SMA-SCROOGE-012: Blackboard type `",
                    stringify!($ty),
                    "` must be aligned to at least 64 bytes (L1D cache line). Add `#[repr(C, align(64))]`."
                )
            );
        };
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[repr(C, align(64))]
    struct AlignedContext {
        pub counter: u64,
        pub flags: u32,
    }

    impl Blackboard for AlignedContext {}

    assert_blackboard_aligned!(AlignedContext);

    #[test]
    fn test_blackboard_alignment() {
        assert!(core::mem::align_of::<AlignedContext>() >= 64);
        let () = <AlignedContext as Blackboard>::ASSERT_CACHE_ALIGNED;
    }
}
