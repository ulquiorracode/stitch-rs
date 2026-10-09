//! Compile-time const hashing primitives for Sewing Machine Architecture (SMA).

/// 64-bit FNV-1a offset basis.
pub const FNV1A_64_OFFSET_BASIS: u64 = 0xcbf29ce484222325;

/// 64-bit FNV-1a prime.
pub const FNV1A_64_PRIME: u64 = 0x100000001b3;

/// 32-bit FNV-1a offset basis.
pub const FNV1A_32_OFFSET_BASIS: u32 = 0x811c9dc5;

/// 32-bit FNV-1a prime.
pub const FNV1A_32_PRIME: u32 = 0x01000193;

/// Computes a 64-bit FNV-1a hash over a byte slice at compile time.
#[inline(always)]
pub const fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash = FNV1A_64_OFFSET_BASIS;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(FNV1A_64_PRIME);
        i += 1;
    }
    hash
}

/// Computes a 32-bit FNV-1a hash over a byte slice at compile time.
#[inline(always)]
pub const fn fnv1a_32(bytes: &[u8]) -> u32 {
    let mut hash = FNV1A_32_OFFSET_BASIS;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u32;
        hash = hash.wrapping_mul(FNV1A_32_PRIME);
        i += 1;
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_fnv1a_64_stability() {
        const HASH_EMPTY: u64 = fnv1a_64(b"");
        assert_eq!(HASH_EMPTY, FNV1A_64_OFFSET_BASIS);

        const HASH_FOO: u64 = fnv1a_64(b"foo");
        assert_eq!(HASH_FOO, fnv1a_64(b"foo"));
        assert_ne!(HASH_FOO, fnv1a_64(b"bar"));
    }

    #[test]
    fn test_const_fnv1a_32_stability() {
        const HASH_EMPTY: u32 = fnv1a_32(b"");
        assert_eq!(HASH_EMPTY, FNV1A_32_OFFSET_BASIS);

        const HASH_TEST: u32 = fnv1a_32(b"test");
        assert_eq!(HASH_TEST, fnv1a_32(b"test"));
    }
}
