//! Architecture-as-Code engine and CLI runner for `stitch` and `sma`.

pub mod check;
pub mod config;
pub mod fix;
pub mod graph;
pub mod health;
pub mod metrics;

use check::CheckRunner;
use config::{RuleSeverity, ScopeFilter, StitchConfig};
use fix::FixEngine;
use graph::GraphExtractor;
use health::HealthEngine;
use metrics::MetricsAuditor;
use std::collections::HashSet;
use std::path::PathBuf;

/// Main entry point for CLI binaries.
pub fn run() -> Result<(), lexopt::Error> {
    use lexopt::prelude::*;

    let mut parser = lexopt::Parser::from_env();
    let mut subcommand = None;
    let mut target_dir = PathBuf::from(".");
    let mut format = String::from("text");
    let mut output_file: Option<PathBuf> = None;
    let mut scope_arg: Option<String> = None;
    let mut categories = HashSet::<String>::new();
    let mut all_categories = false;
    let mut strict = false;
    let mut dry_run = false;

    while let Some(arg) = parser.next()? {
        match arg {
            Value(val) if subcommand.is_none() => {
                subcommand = Some(val.string()?);
            }
            Long("workspace") => {
                // Already defaults to scanning current workspace
            }
            Long("dir") | Short('d') => {
                target_dir = PathBuf::from(parser.value()?.string()?);
            }
            Long("scope") | Short('s') => {
                scope_arg = Some(parser.value()?.string()?);
            }
            Long("category") | Short('c') => {
                let val = parser.value()?.string()?;
                for part in val.split(',') {
                    let trimmed = part.trim().to_lowercase();
                    if !trimmed.is_empty() {
                        categories.insert(trimmed);
                    }
                }
            }
            Long("scrooge") => {
                categories.insert("scrooge".to_string());
            }
            Long("taxo") => {
                categories.insert("taxo".to_string());
            }
            Long("bound") => {
                categories.insert("bound".to_string());
            }
            Long("hotpath") => {
                categories.insert("hotpath".to_string());
            }
            Long("concur") => {
                categories.insert("concur".to_string());
            }
            Long("all") => {
                all_categories = true;
            }
            Long("format") | Short('f') => {
                format = parser.value()?.string()?.to_lowercase();
            }
            Long("output") | Short('o') => {
                output_file = Some(PathBuf::from(parser.value()?.string()?));
            }
            Long("strict") => {
                strict = true;
            }
            Long("dry-run") => {
                dry_run = true;
            }
            Long("help") | Short('h') => {
                print_help();
                return Ok(());
            }
            Long("version") | Short('V') => {
                println!("stitch / sma CLI v{}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            _ => return Err(arg.unexpected()),
        }
    }

    let scope_filter = scope_arg.as_deref().map(ScopeFilter::new);
    let config = StitchConfig::load(&target_dir);

    match subcommand.as_deref() {
        Some("check") => {
            println!(
                "==> Scanning workspace architecture at `{}`...",
                target_dir.display()
            );
            if let Some(scope) = &scope_filter {
                println!("    Scope filter: `{}`", scope.raw);
            }
            let runner = CheckRunner::new_scoped(&config, &target_dir, scope_filter.clone());
            let mut diagnostics = runner.run();

            if !all_categories && !categories.is_empty() {
                diagnostics.retain(|d| {
                    categories
                        .iter()
                        .any(|cat| d.code.to_lowercase().contains(cat))
                });
            }

            let mut error_count = 0;
            let mut warn_count = 0;

            let use_rustc_fmt = format == "rustc";

            for diag in &diagnostics {
                let rendered = if use_rustc_fmt {
                    diag.render_rustc()
                } else {
                    diag.render_miette()
                };

                match diag.severity {
                    RuleSeverity::Deny => {
                        error_count += 1;
                        eprint!("{rendered}");
                    }
                    RuleSeverity::Warn => {
                        warn_count += 1;
                        if strict {
                            error_count += 1;
                            eprint!("{rendered}");
                        } else {
                            println!("{rendered}");
                        }
                    }
                    RuleSeverity::Allow => {}
                }
            }

            if error_count > 0 {
                eprintln!(
                    "\n❌ Architectural check failed: {} error(s), {} warning(s).",
                    error_count, warn_count
                );
                std::process::exit(1);
            } else {
                println!(
                    "\n✅ SMA Architecture Clean: 0 errors, {} warning(s) found.",
                    warn_count
                );
                Ok(())
            }
        }
        Some("health") => {
            println!(
                "==> Evaluating Multidimensional Architecture Health Scorecard at `{}`...",
                target_dir.display()
            );
            let engine = HealthEngine::new_scoped(&config, &target_dir, scope_filter.clone());
            let report = engine.evaluate();

            if format == "json" {
                let json = serde_json::to_string_pretty(&report).unwrap_or_default();
                if let Some(path) = output_file {
                    std::fs::write(&path, &json).expect("failed to write health report");
                    println!("Health report written to `{}`.", path.display());
                } else {
                    println!("{json}");
                }
            } else {
                let text = report.render_terminal();
                if let Some(path) = output_file {
                    std::fs::write(&path, &text).expect("failed to write health report");
                    println!("Health report written to `{}`.", path.display());
                } else {
                    print!("{text}");
                }
            }

            if !report.passed && strict {
                eprintln!(
                    "\n❌ Architecture Health failed threshold: composite {:.1}% (min {:.1}%), hotpath {:.1}% (min {:.1}%).",
                    report.composite_score,
                    config.health.thresholds.min_composite * 100.0,
                    report.dimensions.zero_alloc_hotpath.score_pct,
                    config.health.thresholds.min_hotpath * 100.0
                );
                std::process::exit(1);
            }
            Ok(())
        }
        Some("graph") => {
            let extractor = GraphExtractor::new_scoped(&target_dir, scope_filter.clone());
            let graph = extractor.extract();

            let output_str = match format.as_str() {
                "json" => serde_json::to_string_pretty(&graph).unwrap_or_default(),
                "mermaid" => graph.to_mermaid(),
                "dot" => graph.to_dot(),
                "html" => graph.to_html(),
                _ => graph.to_mermaid(),
            };

            if let Some(path) = output_file {
                if let Err(e) = std::fs::write(&path, &output_str) {
                    eprintln!("Failed to write graph to `{}`: {e}", path.display());
                    std::process::exit(1);
                }
                println!(
                    "✅ Architecture DAG exported to `{}` (format: {}).",
                    path.display(),
                    format
                );
            } else {
                println!("{output_str}");
            }
            Ok(())
        }
        Some("metrics") => {
            println!("==> Running Scrooge Systems Memory & Alignment Audit...");
            if let Some(scope) = &scope_filter {
                println!("    Scope filter: `{}`", scope.raw);
            }
            let auditor = MetricsAuditor::new_scoped(&target_dir, scope_filter.clone());
            let reports = auditor.audit();

            println!("\n{:=<80}", "");
            println!("                    SCROOGE SYSTEMS MEMORY & ALIGNMENT AUDIT");
            println!("{:=<80}", "");

            for r in &reports {
                if r.total_padding > 0 || r.crosses_cache_line {
                    println!("\n[STRUCT] {} ({}:{})", r.name, r.file, r.line);
                    println!("  Declared Size:   {} bytes", r.total_size);
                    println!("  Padding Waste:   {} bytes", r.total_padding);
                    println!(
                        "  Optimal Waste:   {} bytes (via descending alignment order)",
                        r.optimal_padding
                    );
                    if r.crosses_cache_line {
                        println!("  Cache Status:    CROSSES 64-byte L1D boundary!");
                    }

                    for f in &r.fields {
                        if f.padding_before > 0 {
                            println!(
                                "    +{:02} [PADDING HOLE] ({} bytes)",
                                f.offset - f.padding_before,
                                f.padding_before
                            );
                        }
                        println!(
                            "    +{:02} {}: {} ({} B, align {})",
                            f.offset, f.name, f.type_str, f.size, f.align
                        );
                    }
                }
            }
            println!("\n{:=<80}", "");
            println!("Audit completed across {} structs.", reports.len());
            Ok(())
        }
        Some("fix") => {
            let run_scrooge =
                all_categories || categories.is_empty() || categories.contains("scrooge");
            let run_taxo = all_categories || categories.contains("taxo");

            println!(
                "==> Running Automated Architecture Fixer at `{}`...",
                target_dir.display()
            );
            if let Some(scope) = &scope_filter {
                println!("    Scope filter: `{}`", scope.raw);
            }

            if run_scrooge {
                println!("    Active category: `scrooge` (struct alignment optimization)");
                let engine = FixEngine::new_scoped(&target_dir, scope_filter.clone());
                let report = engine
                    .run_scrooge(dry_run)
                    .map_err(|e| lexopt::Error::Custom(Box::new(std::io::Error::other(e))))?;

                if report.changes.is_empty() {
                    println!(
                        "\n✅ All audited structs already adhere to optimal descending alignment. 0 bytes wasted."
                    );
                } else {
                    let action = if dry_run { "PROPOSED" } else { "APPLIED" };
                    println!("\n{:=<80}", "");
                    println!("           AUTOMATED STRUCT ALIGNMENT (SCROOGE) REORDERING REPORT");
                    println!("{:=<80}", "");
                    println!(
                        "  Status: {} fixes across {} structs",
                        action,
                        report.changes.len()
                    );
                    println!(
                        "  Total Padding Eliminated: {} bytes\n",
                        report.total_padding_saved
                    );

                    for ch in &report.changes {
                        println!(
                            "  • [STRUCT] {} ({}:{})",
                            ch.struct_name,
                            ch.file_path.display(),
                            ch.line
                        );
                        println!(
                            "    Declared Size: {} B -> {} B  |  Padding Eliminated: {} B",
                            ch.declared_size_before, ch.declared_size_after, ch.padding_saved
                        );
                    }
                    println!("{:=<80}\n", "");
                }
            }

            if run_taxo {
                println!("    Active category: `taxo` (taxonomy & suffix validation)");
                println!(
                    "ℹ️  Taxonomy naming fixes require architectural confirmation. Running dry-run check..."
                );
                let runner = CheckRunner::new_scoped(&config, &target_dir, scope_filter.clone());
                let taxo_diags: Vec<_> = runner
                    .run()
                    .into_iter()
                    .filter(|d| d.code.starts_with("SMA-TAXO-"))
                    .collect();
                if taxo_diags.is_empty() {
                    println!("✅ All components adhere to strict SMA taxonomy suffixes.");
                } else {
                    println!("⚠️  Found {} taxonomy violations:", taxo_diags.len());
                    for d in &taxo_diags {
                        println!(
                            "  • [{}] {} ({}:{})",
                            d.code,
                            d.message,
                            d.file.display(),
                            d.line
                        );
                    }
                }
            }

            Ok(())
        }
        Some(cmd) => {
            eprintln!("Unknown subcommand `{}`.", cmd);
            print_help();
            std::process::exit(1);
        }
        None => {
            print_help();
            Ok(())
        }
    }
}

fn print_help() {
    println!(
        r#"stitch / sma - Sewing Machine Architecture (SMA) Architecture-as-Code CLI

USAGE:
    stitch <SUBCOMMAND> [OPTIONS]
    sma <SUBCOMMAND> [OPTIONS]

SUBCOMMANDS:
    check       Fast pre-build AST scanner; validates SMA taxonomy, Scrooge rules, and boundaries.
    health      Evaluates multidimensional architecture health scorecard and quality index.
    fix         Automated architecture fixer (struct alignment `--scrooge`, taxonomy `--taxo`, or `--category`).
    graph       Extracts architectural DAG; outputs Mermaid, Graphviz DOT, JSON, or interactive HTML.
    metrics     Audits L1D cache-line alignment, struct padding holes, and field ordering.

OPTIONS:
    -d, --dir <DIR>             Target workspace directory (default: current directory)
    -s, --scope <SCOPE>         Filter by crate, directory, file, or symbol (e.g. `services/*`, `ChatContext`)
    -c, --category <CATEGORIES> Target specific categories (scrooge, taxo, bound, hotpath, concur)
        --scrooge               Target Scrooge memory layout and padding elimination
        --taxo                  Target taxonomy naming and suffix conventions
        --bound                 Target boundary isolation and dependency inversion
        --hotpath               Target zero-allocation hot-path invariants
        --concur                Target thread-safety and receiver immutability
        --all                   Target all categories
    -f, --format <FMT>          Output format for graph/health (text, json, mermaid, dot, html, rustc, miette)
    -o, --output <FILE>         Write output to a specified file
        --dry-run               Simulate changes without writing files to disk
        --strict                Treat warnings as hard errors
    -h, --help                  Print help information
    -V, --version               Print version information
"#
    );
}
