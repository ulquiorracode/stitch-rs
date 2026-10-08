//! Diagnostic Error Codes and Rule Registry for Sewing Machine Architecture (SMA).

pub const SMA_TAXO_001: &str = "SMA-TAXO-001";
pub const SMA_TAXO_002: &str = "SMA-TAXO-002";
pub const SMA_TAXO_003: &str = "SMA-TAXO-003";
pub const SMA_TAXO_004: &str = "SMA-TAXO-004";
pub const SMA_TAXO_005: &str = "SMA-TAXO-005";
pub const SMA_TAXO_006: &str = "SMA-TAXO-006";
pub const SMA_TAXO_007: &str = "SMA-TAXO-007";
pub const SMA_TAXO_008: &str = "SMA-TAXO-008";

pub const SMA_SCROOGE_010: &str = "SMA-SCROOGE-010";
pub const SMA_SCROOGE_011: &str = "SMA-SCROOGE-011";
pub const SMA_SCROOGE_012: &str = "SMA-SCROOGE-012";
pub const SMA_SCROOGE_013: &str = "SMA-SCROOGE-013";

pub const SMA_HOTPATH_020: &str = "SMA-HOTPATH-020";
pub const SMA_HOTPATH_021: &str = "SMA-HOTPATH-021";
pub const SMA_HOTPATH_022: &str = "SMA-HOTPATH-022";
pub const SMA_HOTPATH_024: &str = "SMA-HOTPATH-024";

pub const SMA_CONCUR_040: &str = "SMA-CONCUR-040";

pub const FORBIDDEN_HEAP_TYPES: &[&str] = &[
    "String",
    "Vec",
    "Box",
    "Arc",
    "Rc",
    "BTreeMap",
    "HashMap",
    "HashSet",
    "LinkedList",
    "VecDeque",
    "CString",
    "OsString",
    "PathBuf",
];
