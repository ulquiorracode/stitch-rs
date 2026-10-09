//! Configuration file parser and rules schema for `stitch.toml`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StitchConfig {
    #[serde(default)]
    pub workspace: WorkspaceConfig,
    #[serde(default)]
    pub rules: HashMap<String, RuleSeverity>,
    #[serde(default)]
    pub boundaries: BoundariesConfig,
    #[serde(default)]
    pub scopes: HashMap<String, ScopeConfig>,
    #[serde(default)]
    pub scrooge: ScroogeConfig,
    #[serde(default)]
    pub health: HealthConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    #[serde(default = "default_schema_version")]
    pub schema_version: String,
    #[serde(default = "default_root")]
    pub root: String,
    #[serde(default = "default_topology")]
    pub topology: String,
}

fn default_schema_version() -> String {
    "1.0.0".to_string()
}
fn default_root() -> String {
    ".".to_string()
}
fn default_topology() -> String {
    "hexagonal".to_string()
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            schema_version: default_schema_version(),
            root: default_root(),
            topology: default_topology(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleSeverity {
    #[default]
    Deny,
    Warn,
    Allow,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BoundariesConfig {
    #[serde(default)]
    pub core_crates: Vec<String>,
    #[serde(default)]
    pub port_crates: Vec<String>,
    #[serde(default)]
    pub adapter_crates: Vec<String>,
    #[serde(default)]
    pub service_crates: Vec<String>,
    #[serde(default)]
    pub forbidden_dependencies: Vec<ForbiddenDependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForbiddenDependency {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScopeConfig {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub strict_hotpath: Option<bool>,
    #[serde(default)]
    pub allow_heap: Option<bool>,
    #[serde(default)]
    pub allow_unsafe: Option<bool>,
    #[serde(default)]
    pub allow_ffi: Option<bool>,
    #[serde(default)]
    pub rules: HashMap<String, RuleSeverity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScroogeConfig {
    #[serde(default = "default_cache_line_bytes")]
    pub cache_line_bytes: usize,
    #[serde(default = "default_max_padding_waste_pct")]
    pub max_padding_waste_pct: f64,
    #[serde(default = "default_forbidden_heap_types")]
    pub forbid_heap_types: Vec<String>,
}

fn default_cache_line_bytes() -> usize {
    64
}
fn default_max_padding_waste_pct() -> f64 {
    15.0
}
fn default_forbidden_heap_types() -> Vec<String> {
    vec![
        "String".to_string(),
        "Vec".to_string(),
        "Box".to_string(),
        "Arc".to_string(),
        "Rc".to_string(),
        "BTreeMap".to_string(),
        "HashMap".to_string(),
        "HashSet".to_string(),
        "LinkedList".to_string(),
        "VecDeque".to_string(),
        "CString".to_string(),
        "OsString".to_string(),
        "PathBuf".to_string(),
    ]
}

impl Default for ScroogeConfig {
    fn default() -> Self {
        Self {
            cache_line_bytes: default_cache_line_bytes(),
            max_padding_waste_pct: default_max_padding_waste_pct(),
            forbid_heap_types: default_forbidden_heap_types(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HealthConfig {
    #[serde(default)]
    pub weights: HealthWeights,
    #[serde(default)]
    pub thresholds: HealthThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthWeights {
    #[serde(default = "default_weight_taxo")]
    pub taxo: f64,
    #[serde(default = "default_weight_dip")]
    pub dip: f64,
    #[serde(default = "default_weight_scrooge")]
    pub scrooge: f64,
    #[serde(default = "default_weight_hotpath")]
    pub hotpath: f64,
    #[serde(default = "default_weight_concur")]
    pub concur: f64,
}

fn default_weight_taxo() -> f64 {
    1.5
}
fn default_weight_dip() -> f64 {
    2.5
}
fn default_weight_scrooge() -> f64 {
    2.5
}
fn default_weight_hotpath() -> f64 {
    3.0
}
fn default_weight_concur() -> f64 {
    1.5
}

impl Default for HealthWeights {
    fn default() -> Self {
        Self {
            taxo: default_weight_taxo(),
            dip: default_weight_dip(),
            scrooge: default_weight_scrooge(),
            hotpath: default_weight_hotpath(),
            concur: default_weight_concur(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthThresholds {
    #[serde(default = "default_min_composite")]
    pub min_composite: f64,
    #[serde(default = "default_min_hotpath")]
    pub min_hotpath: f64,
}

fn default_min_composite() -> f64 {
    0.90
}
fn default_min_hotpath() -> f64 {
    1.00
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            min_composite: default_min_composite(),
            min_hotpath: default_min_hotpath(),
        }
    }
}

impl Default for StitchConfig {
    fn default() -> Self {
        let mut rules = HashMap::new();
        // Defaults matching SMA specification
        rules.insert("TAXO-SUFFIX-LAYER".to_string(), RuleSeverity::Deny);
        rules.insert("TAXO-SUFFIX-PORT".to_string(), RuleSeverity::Deny);
        rules.insert("TAXO-SUFFIX-ADAPTER".to_string(), RuleSeverity::Deny);
        rules.insert("TAXO-SUFFIX-TERMINAL".to_string(), RuleSeverity::Deny);
        rules.insert("TAXO-SUFFIX-TOKEN".to_string(), RuleSeverity::Deny);
        rules.insert("TAXO-SUFFIX-ID".to_string(), RuleSeverity::Deny);
        rules.insert("TAXO-SUFFIX-BLACKBOARD".to_string(), RuleSeverity::Deny);

        rules.insert("SCROOGE-HEAP-HOTPATH".to_string(), RuleSeverity::Deny);
        rules.insert("SCROOGE-DYNAMIC-DISP".to_string(), RuleSeverity::Deny);
        rules.insert("SCROOGE-CACHE-ALIGN".to_string(), RuleSeverity::Deny);
        rules.insert("SCROOGE-TOKEN-SIZE".to_string(), RuleSeverity::Deny);
        rules.insert("SCROOGE-PADDING-HOLE".to_string(), RuleSeverity::Warn);

        rules.insert("HOTPATH-ALLOC-MACRO".to_string(), RuleSeverity::Deny);
        rules.insert("HOTPATH-ALLOC-METHOD".to_string(), RuleSeverity::Deny);
        rules.insert("HOTPATH-PANIC".to_string(), RuleSeverity::Deny);

        rules.insert("BOUND-INVERSION".to_string(), RuleSeverity::Deny);
        rules.insert("BOUND-DIRECT-ADAPTER".to_string(), RuleSeverity::Deny);
        rules.insert("BOUND-FFI-LEAK".to_string(), RuleSeverity::Deny);
        rules.insert("BOUND-COLOCATION".to_string(), RuleSeverity::Deny);

        rules.insert("CONCUR-RECEIVER-MUT".to_string(), RuleSeverity::Deny);

        rules.insert("SMA-IO-001".to_string(), RuleSeverity::Deny);
        rules.insert("SMA-IO-002".to_string(), RuleSeverity::Deny);
        rules.insert("SMA-PARSE-001".to_string(), RuleSeverity::Deny);
        rules.insert("SMA-PARSE-002".to_string(), RuleSeverity::Deny);

        Self {
            workspace: WorkspaceConfig::default(),
            rules,
            boundaries: BoundariesConfig::default(),
            scopes: HashMap::new(),
            scrooge: ScroogeConfig::default(),
            health: HealthConfig::default(),
        }
    }
}

/// Errors occurring during configuration file discovery, reading, or parsing.
#[derive(Debug)]
pub enum ConfigError {
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: Box<toml::de::Error>,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(
                    f,
                    "Failed to read configuration file `{}`: {source}",
                    path.display()
                )
            }
            Self::Parse { path, source } => {
                write!(
                    f,
                    "Failed to parse configuration file `{}`: {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source.as_ref()),
        }
    }
}

impl StitchConfig {
    /// Loads configuration starting from `root_dir`, traversing parent directories for `stitch.toml` or `.stitch.toml`.
    ///
    /// If a candidate configuration file is found, it is parsed strictly. If parsing or I/O fails,
    /// a [`ConfigError`] is returned immediately (Strict Fail-Fast / Anti-Silent Error Suppression).
    /// If no candidate file exists anywhere in the directory hierarchy, `Ok(Self::default())` is returned.
    pub fn load(root_dir: &Path) -> Result<Self, ConfigError> {
        let mut curr = Some(root_dir);
        while let Some(dir) = curr {
            let candidate_paths = [dir.join("stitch.toml"), dir.join(".stitch.toml")];

            for path in &candidate_paths {
                if path.exists() {
                    let content = std::fs::read_to_string(path).map_err(|e| ConfigError::Io {
                        path: path.clone(),
                        source: e,
                    })?;
                    let cfg = toml::from_str::<StitchConfig>(&content).map_err(|e| {
                        ConfigError::Parse {
                            path: path.clone(),
                            source: Box::new(e),
                        }
                    })?;
                    return Ok(cfg);
                }
            }
            curr = dir.parent();
        }

        Ok(Self::default())
    }

    /// Evaluates effective severity of a given rule code.
    pub fn severity_for(&self, rule_code: &str) -> RuleSeverity {
        let canonical = canonical_rule_key(rule_code);
        if let Some(sev) = self
            .rules
            .get(rule_code)
            .or_else(|| self.rules.get(canonical))
        {
            return *sev;
        }
        RuleSeverity::Deny
    }

    /// Evaluates effective severity for a specific file path and rule code, applying any scope overrides.
    pub fn severity_for_path(&self, path: &Path, rule_code: &str) -> RuleSeverity {
        let path_str = path.to_string_lossy();
        let canonical = canonical_rule_key(rule_code);

        for (pattern, scope) in &self.scopes {
            if glob_match(pattern, &path_str) {
                if let Some(sev) = scope
                    .rules
                    .get(rule_code)
                    .or_else(|| scope.rules.get(canonical))
                    .or_else(|| {
                        // Reverse lookup from canonical to user key
                        scope.rules.iter().find_map(|(k, v)| {
                            if canonical_rule_key(k) == canonical {
                                Some(v)
                            } else {
                                None
                            }
                        })
                    })
                {
                    return *sev;
                }
                if scope.allow_heap.unwrap_or(false)
                    && (rule_code.starts_with("SMA-SCROOGE-")
                        || canonical.starts_with("SMA-SCROOGE-"))
                {
                    return RuleSeverity::Allow;
                }
            }
        }

        self.severity_for(rule_code)
    }

    /// Determines if a file path belongs to a scope marked as `strict_hotpath`.
    pub fn is_strict_hotpath(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        for (pattern, scope) in &self.scopes {
            if glob_match(pattern, &path_str)
                && let Some(strict) = scope.strict_hotpath
            {
                return strict;
            }
        }
        false
    }
}

pub fn canonical_rule_key(key: &str) -> &'static str {
    match key {
        "SMA-TAXO-001" | "TAXO-SUFFIX-LAYER" => "SMA-TAXO-001",
        "SMA-TAXO-002" | "TAXO-SUFFIX-TERMINAL" => "SMA-TAXO-002",
        "SMA-TAXO-003" | "TAXO-SUFFIX-PORT" => "SMA-TAXO-003",
        "SMA-TAXO-004" | "TAXO-SUFFIX-ADAPTER" => "SMA-TAXO-004",
        "SMA-TAXO-005" | "TAXO-SUFFIX-HUB" => "SMA-TAXO-005",
        "SMA-TAXO-006" | "TAXO-SUFFIX-TOKEN" => "SMA-TAXO-006",
        "SMA-TAXO-007" | "TAXO-SUFFIX-ID" => "SMA-TAXO-007",
        "SMA-TAXO-008" | "TAXO-SUFFIX-BLACKBOARD" => "SMA-TAXO-008",
        "SMA-TAXO-009" | "TAXO-UNCLASSIFIED-PUB" => "SMA-TAXO-009",
        "SMA-SCROOGE-010" | "SCROOGE-HEAP-HOTPATH" => "SMA-SCROOGE-010",
        "SMA-SCROOGE-011" | "SCROOGE-DYNAMIC-DISP" => "SMA-SCROOGE-011",
        "SMA-SCROOGE-012" | "SCROOGE-CACHE-ALIGN" => "SMA-SCROOGE-012",
        "SMA-SCROOGE-013" | "SCROOGE-TOKEN-SIZE" => "SMA-SCROOGE-013",
        "SMA-SCROOGE-014" | "SCROOGE-PADDING-HOLE" => "SMA-SCROOGE-014",
        "SMA-SCROOGE-015" | "SCROOGE-CACHE-CROSS" => "SMA-SCROOGE-015",
        "SMA-HOTPATH-020" | "HOTPATH-ALLOC-MACRO" => "SMA-HOTPATH-020",
        "SMA-HOTPATH-021" | "HOTPATH-ALLOC-METHOD" => "SMA-HOTPATH-021",
        "SMA-HOTPATH-022" | "HOTPATH-ALLOC-CTOR" => "SMA-HOTPATH-022",
        "SMA-HOTPATH-023" | "HOTPATH-BLOCKING-IO" => "SMA-HOTPATH-023",
        "SMA-HOTPATH-024" | "HOTPATH-FLOW-RETURN" => "SMA-HOTPATH-024",
        "SMA-BOUND-030" | "BOUND-INVERSION" => "SMA-BOUND-030",
        "SMA-BOUND-031" | "BOUND-DIRECT-ADAPTER" => "SMA-BOUND-031",
        "SMA-BOUND-032" | "BOUND-FFI-LEAK" => "SMA-BOUND-032",
        "SMA-BOUND-033" | "BOUND-COLOCATION" => "SMA-BOUND-033",
        "SMA-CONCUR-040" | "CONCUR-RECEIVER-MUT" => "SMA-CONCUR-040",
        "SMA-CONCUR-041" | "CONCUR-STATIC-MUT" => "SMA-CONCUR-041",
        "SMA-IO-001" | "IO-READ-ERROR" => "SMA-IO-001",
        "SMA-IO-002" | "IO-MANIFEST-ERROR" => "SMA-IO-002",
        "SMA-PARSE-001" | "PARSE-SYNTAX-ERROR" => "SMA-PARSE-001",
        "SMA-PARSE-002" | "PARSE-MANIFEST-ERROR" => "SMA-PARSE-002",
        _ => "SMA-UNKNOWN",
    }
}

pub fn glob_match(pattern: &str, path: &str) -> bool {
    let norm_pat = pattern.replace('\\', "/");
    let mut norm_path = path.replace('\\', "/");
    if norm_path.starts_with("./") {
        norm_path = norm_path[2..].to_string();
    }

    if norm_pat.contains('*') || norm_pat.contains('?') {
        glob_match_recursive(norm_pat.as_bytes(), norm_path.as_bytes())
    } else {
        norm_path.contains(&norm_pat)
    }
}

fn glob_match_recursive(pat: &[u8], path: &[u8]) -> bool {
    if pat.is_empty() {
        return path.is_empty();
    }

    if pat.starts_with(b"**/") {
        let rest_pat = &pat[3..];
        if glob_match_recursive(rest_pat, path) {
            return true;
        }
        for i in 0..path.len() {
            if path[i] == b'/' && glob_match_recursive(rest_pat, &path[i + 1..]) {
                return true;
            }
        }
        return false;
    }

    if pat == b"**" {
        return true;
    }

    if pat[0] == b'*' {
        let rest_pat = &pat[1..];
        for i in 0..=path.len() {
            if i > 0 && path[i - 1] == b'/' {
                break;
            }
            if glob_match_recursive(rest_pat, &path[i..]) {
                return true;
            }
        }
        return false;
    }

    if pat[0] == b'?' {
        if path.is_empty() || path[0] == b'/' {
            return false;
        }
        return glob_match_recursive(&pat[1..], &path[1..]);
    }

    if path.is_empty() {
        return false;
    }

    if pat[0] == path[0] {
        return glob_match_recursive(&pat[1..], &path[1..]);
    }

    false
}

/// Dynamic scope filter passed via CLI (`--scope`) to restrict analysis, graph, or fixes.
#[derive(Debug, Clone, Default)]
pub struct ScopeFilter {
    pub raw: String,
    pub patterns: Vec<String>,
}

impl ScopeFilter {
    pub fn new(raw: &str) -> Self {
        let patterns = raw
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Self {
            raw: raw.to_string(),
            patterns,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    pub fn matches_path(&self, path: &Path) -> bool {
        if self.patterns.is_empty() {
            return true;
        }
        let path_str = path.to_string_lossy();
        self.patterns.iter().any(|pat| glob_match(pat, &path_str))
    }

    pub fn matches_name(&self, name: &str) -> bool {
        if self.patterns.is_empty() {
            return true;
        }
        self.patterns
            .iter()
            .any(|pat| pat == name || glob_match(pat, name))
    }

    /// Checks if a file could potentially be relevant for this scope.
    pub fn is_file_relevant(&self, path: &Path) -> bool {
        if self.patterns.is_empty() {
            return true;
        }
        if self.matches_path(path) {
            return true;
        }
        self.patterns.iter().all(|pat| {
            !pat.contains('/') && !pat.contains('\\') && !pat.ends_with(".rs") && !pat.contains('*')
        })
    }
}

/// Reusable iterator discovering all relevant Rust source files in a workspace,
/// filtering out build artifacts, tests/ui compile-fail suites, and applying the optional scope filter.
pub fn collect_rust_files<'a>(
    root_dir: &'a Path,
    scope: Option<&'a ScopeFilter>,
) -> impl Iterator<Item = PathBuf> + 'a {
    walkdir::WalkDir::new(root_dir)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            let path_str = entry.path().to_string_lossy();
            entry.depth() == 0
                || (name != "target"
                    && name != ".git"
                    && !name.starts_with('.')
                    && !path_str.contains("tests/ui")
                    && !path_str.contains("tests\\ui")
                    && !path_str.contains("goldsrc-sys")
                    && !name.ends_with("bindings_pregenerated.rs"))
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
        .filter(move |e| {
            if let Some(scope) = scope {
                scope.is_file_relevant(e.path())
            } else {
                true
            }
        })
        .map(|e| e.into_path())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glob_match_wildcards() {
        assert!(glob_match("services/*", "services/chat"));
        assert!(!glob_match("services/*", "services/chat/src/lib.rs"));
        assert!(glob_match("services/**", "services/chat/src/lib.rs"));
        assert!(glob_match("**/*.rs", "crates/stitch-cli/src/lib.rs"));
        assert!(!glob_match("**/*.rs", "crates/stitch-cli/Cargo.toml"));
        assert!(glob_match(
            "crates/stitch-core",
            "D:/Repo/stitch-rs/crates/stitch-core/src/lib.rs"
        ));
    }

    #[test]
    fn test_config_load_invalid_toml_fails_loudly() {
        let temp_dir =
            std::env::temp_dir().join(format!("stitch_test_invalid_toml_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let toml_path = temp_dir.join("stitch.toml");
        std::fs::write(&toml_path, "[[rules = corrupt_syntax_error").unwrap();

        let result = StitchConfig::load(&temp_dir);
        let _ = std::fs::remove_dir_all(&temp_dir);

        assert!(
            result.is_err(),
            "Invalid stitch.toml must return Err, never silent default!"
        );
        let err = result.unwrap_err();
        assert!(matches!(err, ConfigError::Parse { .. }));
    }

    #[test]
    fn test_config_load_missing_file_returns_default() {
        let temp_dir =
            std::env::temp_dir().join(format!("stitch_test_missing_toml_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let result = StitchConfig::load(&temp_dir);
        let _ = std::fs::remove_dir_all(&temp_dir);

        assert!(result.is_ok());
        let cfg = result.unwrap();
        assert_eq!(cfg.severity_for("SMA-TAXO-001"), RuleSeverity::Deny);
    }
}
