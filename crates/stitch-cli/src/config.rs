//! Configuration file parser and rules schema for `stitch.toml`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

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

        Self {
            workspace: WorkspaceConfig::default(),
            rules,
            boundaries: BoundariesConfig::default(),
            scopes: HashMap::new(),
            scrooge: ScroogeConfig::default(),
        }
    }
}

impl StitchConfig {
    /// Loads configuration starting from `root_dir`, checking `stitch.toml`, `.stitch.toml`, or `Cargo.toml`.
    pub fn load(root_dir: &Path) -> Self {
        let candidate_paths = [root_dir.join("stitch.toml"), root_dir.join(".stitch.toml")];

        for path in &candidate_paths {
            if let Ok(content) = std::fs::read_to_string(path)
                && let Ok(cfg) = toml::from_str::<StitchConfig>(&content)
            {
                return cfg;
            }
        }

        Self::default()
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
            if glob_match(pattern, &path_str) {
                if let Some(strict) = scope.strict_hotpath {
                    return strict;
                }
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
        _ => "SMA-UNKNOWN",
    }
}

fn glob_match(pattern: &str, path: &str) -> bool {
    let norm_path = path.replace('\\', "/");
    let norm_pat = pattern.replace('\\', "/");

    if let Some(rest) = norm_pat.strip_prefix("**/") {
        if let Some(core) = rest.strip_suffix("/**") {
            norm_path.contains(core)
        } else {
            norm_path.ends_with(rest) || norm_path.contains(rest)
        }
    } else if let Some(prefix) = norm_pat.strip_suffix("/**") {
        norm_path.starts_with(prefix) || norm_path.contains(prefix)
    } else {
        norm_path.contains(&norm_pat)
    }
}
