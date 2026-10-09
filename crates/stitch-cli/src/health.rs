//! Multidimensional Architecture Health & Mechanical Sympathy Engine.
//!
//! Formulates a sound, mathematically calibrated health vector across orthogonal dimensions:
//! - Taxonomy Purity ($S_{\text{TAXO}}$)
//! - Dependency Inversion & Boundary Isolation ($S_{\text{DIP}}$)
//! - Mechanical Sympathy & Memory Efficiency ($S_{\text{SCROOGE}}$)
//! - Zero-Allocation Hot-Path Invariant ($S_{\text{HOTPATH}}$)
//! - Re-entrancy & Receiver Safety ($S_{\text{CONCUR}}$)

use crate::check::CheckRunner;
use crate::config::{ScopeFilter, StitchConfig};
use crate::metrics::MetricsAuditor;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub composite_score: f64,
    pub grade: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    pub dimensions: HealthDimensions,
    pub penalties: Vec<HealthPenalty>,
    pub recommendations: Vec<String>,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthDimensions {
    pub taxonomy_purity: DimensionScore,
    pub dependency_inversion: DimensionScore,
    pub mechanical_sympathy: DimensionScore,
    pub zero_alloc_hotpath: DimensionScore,
    pub reentrancy_safety: DimensionScore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionScore {
    pub name: String,
    pub score_pct: f64,
    pub weight: f64,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthPenalty {
    pub category: String,
    pub impact_pct: f64,
    pub description: String,
}

pub struct HealthEngine<'a> {
    config: &'a StitchConfig,
    root_dir: &'a Path,
    scope: Option<ScopeFilter>,
}

impl<'a> HealthEngine<'a> {
    pub fn new(config: &'a StitchConfig, root_dir: &'a Path) -> Self {
        Self {
            config,
            root_dir,
            scope: None,
        }
    }

    pub fn new_scoped(
        config: &'a StitchConfig,
        root_dir: &'a Path,
        scope: Option<ScopeFilter>,
    ) -> Self {
        Self {
            config,
            root_dir,
            scope,
        }
    }

    pub fn evaluate(&self) -> HealthReport {
        let check_runner = CheckRunner::new_scoped(self.config, self.root_dir, self.scope.clone());
        let diagnostics = check_runner.run();

        let metrics_auditor = MetricsAuditor::new_scoped(self.root_dir, self.scope.clone());
        let struct_metrics = metrics_auditor.audit();

        let mut penalties = Vec::new();
        let mut recommendations = Vec::new();

        // 1. TAXONOMY PURITY
        let taxo_violations = diagnostics
            .iter()
            .filter(|d| d.code.starts_with("SMA-TAXO-"))
            .count();
        let s_taxo = if taxo_violations == 0 {
            1.0
        } else {
            let p = (taxo_violations as f64 * 0.05).min(0.5);
            penalties.push(HealthPenalty {
                category: "TAXO".to_string(),
                impact_pct: p * 100.0,
                description: format!(
                    "{taxo_violations} taxonomy or suffix convention violations detected"
                ),
            });
            recommendations.push("Ensure all ports end with `Port`, adapters with `Adapter`, and layers with `Layer`.".to_string());
            (1.0 - p).max(0.0)
        };

        // 2. DEPENDENCY INVERSION (DIP)
        let bound_violations = diagnostics
            .iter()
            .filter(|d| d.code.starts_with("SMA-BOUND-"))
            .count();
        let s_dip = if bound_violations == 0 {
            1.0
        } else {
            let p = (bound_violations as f64 * 0.25).min(1.0);
            penalties.push(HealthPenalty {
                category: "DIP".to_string(),
                impact_pct: p * 100.0,
                description: format!("{bound_violations} boundary inversion violations detected"),
            });
            recommendations.push(
                "Decouple core and SPI crates from vendor adapter crates via dependency inversion."
                    .to_string(),
            );
            (1.0 - p).max(0.0)
        };

        // 3. MECHANICAL SYMPATHY (SCROOGE)
        let total_struct_bytes: usize = struct_metrics.iter().map(|s| s.total_size).sum();
        let total_waste_bytes: usize = struct_metrics.iter().map(|s| s.preventable_padding).sum();
        let waste_ratio = if total_struct_bytes > 0 {
            total_waste_bytes as f64 / total_struct_bytes as f64
        } else {
            0.0
        };

        let scrooge_violations = diagnostics
            .iter()
            .filter(|d| d.code.starts_with("SMA-SCROOGE-"))
            .count();

        let s_scrooge = {
            let waste_penalty = waste_ratio.min(0.20);
            let rule_penalty = (scrooge_violations as f64 * 0.05).min(0.40);
            let total_p: f64 = waste_penalty + rule_penalty;
            if total_waste_bytes > 0 {
                penalties.push(HealthPenalty {
                    category: "SCROOGE".to_string(),
                    impact_pct: waste_penalty * 100.0,
                    description: format!(
                        "Preventable padding waste: {total_waste_bytes} B across {} audited structs ({:.1}% memory overhead)",
                        struct_metrics.len(),
                        waste_ratio * 100.0
                    ),
                });
                recommendations.push("Reorder struct fields by descending alignment (align 8 -> align 4 -> align 2 -> align 1) or run `stitch fix --scrooge`.".to_string());
            }
            if scrooge_violations > 0 {
                penalties.push(HealthPenalty {
                    category: "SCROOGE".to_string(),
                    impact_pct: rule_penalty * 100.0,
                    description: format!(
                        "{scrooge_violations} cache alignment or register size violations"
                    ),
                });
            }
            (1.0f64 - total_p).max(0.0f64)
        };

        // 4. ZERO-ALLOC HOTPATH
        let hotpath_violations = diagnostics
            .iter()
            .filter(|d| d.code.starts_with("SMA-HOTPATH-"))
            .count();
        let s_hotpath = if hotpath_violations == 0 {
            1.0
        } else {
            let s = (-0.5 * hotpath_violations as f64).exp();
            let p = 1.0 - s;
            penalties.push(HealthPenalty {
                category: "HOTPATH".to_string(),
                impact_pct: p * 100.0,
                description: format!(
                    "{hotpath_violations} heap allocations or panics in hot-path traversal methods"
                ),
            });
            recommendations.push("Replace `vec!`, `format!`, and heap constructors with inline fixed arrays `[u8; N]` in U-cycle methods.".to_string());
            s
        };

        // 5. RE-ENTRANCY & CONCURRENCY
        let concur_violations = diagnostics
            .iter()
            .filter(|d| d.code.starts_with("SMA-CONCUR-"))
            .count();
        let s_concur = if concur_violations == 0 {
            1.0
        } else {
            let p = (concur_violations as f64 * 0.15).min(0.5);
            penalties.push(HealthPenalty {
                category: "CONCUR".to_string(),
                impact_pct: p * 100.0,
                description: format!(
                    "{concur_violations} mutable receiver &mut self or static mutable violations"
                ),
            });
            recommendations.push("Use immutable `&self` receivers for middleware layers to guarantee thread-safe re-entrancy.".to_string());
            (1.0 - p).max(0.0)
        };

        let w = &self.config.health.weights;
        let total_weight = w.taxo + w.dip + w.scrooge + w.hotpath + w.concur;
        let composite = if total_weight > 0.0 {
            (w.taxo * s_taxo
                + w.dip * s_dip
                + w.scrooge * s_scrooge
                + w.hotpath * s_hotpath
                + w.concur * s_concur)
                / total_weight
        } else {
            1.0
        };

        let composite_pct = composite * 100.0;
        let grade = if composite_pct >= 97.0 {
            "A+".to_string()
        } else if composite_pct >= 90.0 {
            "A".to_string()
        } else if composite_pct >= 80.0 {
            "B".to_string()
        } else if composite_pct >= 70.0 {
            "C".to_string()
        } else {
            "F".to_string()
        };

        let t = &self.config.health.thresholds;
        let passed = composite >= t.min_composite && s_hotpath >= t.min_hotpath;

        HealthReport {
            composite_score: composite_pct,
            grade,
            passed,
            scope: self.scope.as_ref().map(|s| s.raw.clone()),
            dimensions: HealthDimensions {
                taxonomy_purity: DimensionScore {
                    name: "Taxonomy Purity".to_string(),
                    score_pct: s_taxo * 100.0,
                    weight: w.taxo,
                    description: "Naming conventions and explicit architectural intent".to_string(),
                },
                dependency_inversion: DimensionScore {
                    name: "Dependency Inversion".to_string(),
                    score_pct: s_dip * 100.0,
                    weight: w.dip,
                    description: "Strict isolation of Core and Port contracts from vendor adapters"
                        .to_string(),
                },
                mechanical_sympathy: DimensionScore {
                    name: "Mechanical Sympathy".to_string(),
                    score_pct: s_scrooge * 100.0,
                    weight: w.scrooge,
                    description:
                        "L1D cache alignment (64B), 8B register passing, and padding waste ratio"
                            .to_string(),
                },
                zero_alloc_hotpath: DimensionScore {
                    name: "Zero-Alloc Hotpath".to_string(),
                    score_pct: s_hotpath * 100.0,
                    weight: w.hotpath,
                    description:
                        "Zero heap allocations, string formatting, or panics on traversal paths"
                            .to_string(),
                },
                reentrancy_safety: DimensionScore {
                    name: "Re-entrancy Safety".to_string(),
                    score_pct: s_concur * 100.0,
                    weight: w.concur,
                    description: "Immutable `&self` receivers on pipeline layers".to_string(),
                },
            },
            penalties,
            recommendations,
        }
    }
}

impl HealthReport {
    pub fn render_terminal(&self) -> String {
        let mut out = String::new();
        out.push_str(
            "================================================================================\n",
        );
        out.push_str("           SEWING MACHINE ARCHITECTURE (SMA) HEALTH SCORECARD\n");
        out.push_str(
            "================================================================================\n",
        );
        let status = if self.passed { "PASSED" } else { "FAILED" };
        out.push_str(&format!(
            "  Overall Score: {:.1}% [Grade {}]  •  Status: {}\n",
            self.composite_score, self.grade, status
        ));
        if let Some(scope) = &self.scope {
            out.push_str(&format!("  Active Scope:  `{}`\n", scope));
        }
        out.push('\n');

        out.push_str("  DIMENSIONS:\n");
        let dims = [
            &self.dimensions.taxonomy_purity,
            &self.dimensions.dependency_inversion,
            &self.dimensions.mechanical_sympathy,
            &self.dimensions.zero_alloc_hotpath,
            &self.dimensions.reentrancy_safety,
        ];

        for d in dims {
            let bar = render_bar(d.score_pct);
            out.push_str(&format!(
                "  • {:<24} {} {:>5.1}%  (weight: {:.1})\n",
                d.name, bar, d.score_pct, d.weight
            ));
        }

        if !self.penalties.is_empty() {
            out.push_str("\n  PENALTIES:\n");
            for p in &self.penalties {
                out.push_str(&format!(
                    "  - [{}] -{:.1}%: {}\n",
                    p.category, p.impact_pct, p.description
                ));
            }
        }

        if !self.recommendations.is_empty() {
            out.push_str("\n  RECOMMENDATIONS:\n");
            for r in &self.recommendations {
                out.push_str(&format!("  • {}\n", r));
            }
        }

        out.push_str(
            "================================================================================\n",
        );
        out
    }
}

fn render_bar(pct: f64) -> String {
    let filled = ((pct / 100.0) * 20.0).round() as usize;
    let filled = filled.min(20);
    let empty = 20 - filled;
    format!("[{}{}]", "█".repeat(filled), "░".repeat(empty))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_evaluation_clean_workspace() {
        let temp_dir =
            std::env::temp_dir().join(format!("stitch_test_health_clean_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let valid_file = temp_dir.join("clean.rs");
        std::fs::write(
            &valid_file,
            "#[repr(C, align(64))]\npub struct CleanContext;\n",
        )
        .unwrap();

        let cfg = StitchConfig::default();
        let engine = HealthEngine::new(&cfg, &temp_dir);
        let report = engine.evaluate();
        let _ = std::fs::remove_dir_all(&temp_dir);

        assert!(report.passed);
        assert_eq!(report.grade, "A+");
        assert_eq!(report.composite_score, 100.0);
        assert_eq!(report.penalties.len(), 0);
    }

    #[test]
    fn test_render_bar() {
        assert_eq!(render_bar(100.0), "[████████████████████]");
        assert_eq!(render_bar(0.0), "[░░░░░░░░░░░░░░░░░░░░]");
        assert_eq!(render_bar(50.0), "[██████████░░░░░░░░░░]");
    }
}
