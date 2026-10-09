//! Strongly-typed, register-bound Token and Id contracts for SMA.

use crate::hash::fnv1a_64;

/// Suffix Contract: Implementors represent compile-time or runtime opaque 64-bit keys.
///
/// Invariants:
/// - Must be pass-by-value in CPU registers.
/// - Memory footprint is strictly 8 bytes (`sizeof == 8`, `alignof == 8`).
/// - Zero heap allocation.
pub trait StitchToken:
    Copy + Clone + Eq + PartialEq + Ord + PartialOrd + core::hash::Hash + Send + Sync + 'static
{
    /// Asserts at compile time that this token type strictly satisfies the 8-byte register layout.
    const ASSERT_REGISTER_LAYOUT: () = {
        assert!(
            core::mem::size_of::<Self>() == 8,
            "SMA-SCROOGE-013: Token type must be strictly 8 bytes (64 bits)."
        );
        assert!(
            core::mem::align_of::<Self>() == 8,
            "SMA-SCROOGE-013: Token type must have 8-byte alignment."
        );
    };

    /// Returns the underlying raw 64-bit integer.
    fn raw_u64(&self) -> u64;

    /// Equivalent alias for `raw_u64()`.
    #[inline(always)]
    fn as_u64(&self) -> u64 {
        let () = Self::ASSERT_REGISTER_LAYOUT;
        self.raw_u64()
    }
}

/// Suffix Contract: Implementors represent strongly-typed newtype projections over a Token.
///
/// Provides compile-time domain separation, preventing heterogeneous identifier mix-ups.
pub trait StitchId:
    Copy + Clone + Eq + PartialEq + Ord + PartialOrd + core::hash::Hash + Send + Sync + 'static
{
    /// The inner Token type backing this domain Id.
    type Token: StitchToken;

    /// Asserts at compile time that this Id type strictly satisfies the 8-byte register layout.
    const ASSERT_REGISTER_LAYOUT: () = {
        assert!(
            core::mem::size_of::<Self>() == 8,
            "SMA-SCROOGE-013: Id type must be strictly 8 bytes (64 bits)."
        );
        assert!(
            core::mem::align_of::<Self>() == 8,
            "SMA-SCROOGE-013: Id type must have 8-byte alignment."
        );
    };

    /// Returns the inner domain token.
    fn token(&self) -> Self::Token;

    /// Returns the raw 64-bit value of the underlying token.
    #[inline(always)]
    fn raw_u64(&self) -> u64 {
        let () = Self::ASSERT_REGISTER_LAYOUT;
        self.token().raw_u64()
    }
}

/// Standard 8-byte transparent opaque token implementation.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RawToken(pub u64);

impl RawToken {
    /// Const constructor from a raw 64-bit unsigned integer.
    #[inline(always)]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Evaluates a 64-bit FNV-1a hash at compile time from a string literal.
    #[inline(always)]
    pub const fn from_str_hash(name: &str) -> Self {
        Self(fnv1a_64(name.as_bytes()))
    }
}

impl StitchToken for RawToken {
    #[inline(always)]
    fn raw_u64(&self) -> u64 {
        self.0
    }
}

/// Generic transparent newtype wrapper projecting any domain identifier over a `StitchToken`.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RawId<T: StitchToken>(pub T);

impl<T: StitchToken> RawId<T> {
    /// Constructs a new domain Id from its backing Token.
    #[inline(always)]
    pub const fn new(token: T) -> Self {
        Self(token)
    }
}

impl<T: StitchToken> StitchId for RawId<T> {
    type Token = T;

    #[inline(always)]
    fn token(&self) -> Self::Token {
        self.0
    }
}

// Compile-time static assertions verifying register-fit invariants
const _: () = {
    assert!(core::mem::size_of::<RawToken>() == 8);
    assert!(core::mem::align_of::<RawToken>() == 8);
    assert!(core::mem::size_of::<RawId<RawToken>>() == 8);
    assert!(core::mem::align_of::<RawId<RawToken>>() == 8);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_token_const_evaluation() {
        const CHANNEL_GLOBAL: RawToken = RawToken::from_str_hash("channel:global");
        assert_eq!(CHANNEL_GLOBAL.raw_u64(), fnv1a_64(b"channel:global"));
        assert_eq!(core::mem::size_of::<RawToken>(), 8);
    }

    #[test]
    fn test_raw_id_transmutation() {
        let token = RawToken::from_raw(12345);
        let id = RawId::new(token);
        assert_eq!(id.token(), token);
        assert_eq!(id.raw_u64(), 12345);
    }
}
