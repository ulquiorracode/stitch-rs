//! Architecture-as-Code engine and CLI runner for `stitch` and `sma`.

pub mod check;
pub mod config;
pub mod fix;
pub mod graph;
pub mod health;
pub mod metrics;

use check::CheckRunner;
use clap::{Parser, Subcommand};
use config::{RuleSeverity, ScopeFilter, StitchConfig};
use fix::FixEngine;
use graph::GraphExtractor;
use health::HealthEngine;
use metrics::MetricsAuditor;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "stitch",
    bin_name = "stitch",
    version,
    about = "Sewing Machine Architecture (SMA) Architecture-as-Code CLI",
    long_about = "Fast pre-build AST scanner, DAG extractor, Scrooge memory layout auditor, and multidimensional health scorecard."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Fast pre-build AST scanner: validates SMA taxonomy, Scrooge rules, and boundaries
    Check {
        /// Target workspace directory (default: current directory)
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
        /// Scope filter by crate, directory, file, or symbol (e.g. `services/*`, `ChatContext`)
        #[arg(short, long)]
        scope: Option<String>,
        /// Filter by category (scrooge, taxo, bound, hotpath, concur)
        #[arg(short, long)]
        category: Option<String>,
        /// Target Scrooge memory layout and padding elimination
        #[arg(long)]
        scrooge: bool,
        /// Target taxonomy naming and suffix conventions
        #[arg(long)]
        taxo: bool,
        /// Target boundary isolation and dependency inversion
        #[arg(long)]
        bound: bool,
        /// Target zero-allocation hot-path invariants
        #[arg(long)]
        hotpath: bool,
        /// Target thread-safety and receiver immutability
        #[arg(long)]
        concur: bool,
        /// Target all categories
        #[arg(long)]
        all: bool,
        /// Diagnostic format (miette, rustc)
        #[arg(short, long, default_value = "miette")]
        format: String,
        /// Treat warnings as hard errors
        #[arg(long)]
        strict: bool,
    },
    /// Evaluates multidimensional architecture health scorecard (Taxonomy, DIP, Scrooge, Hotpath, Concurrency)
    Health {
        /// Target workspace directory (default: current directory)
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
        /// Scope filter by crate, directory, file, or symbol (e.g. `services/*`, `ChatContext`)
        #[arg(short, long)]
        scope: Option<String>,
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
        /// Optional destination file to write output to
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Enforce strict health threshold failure
        #[arg(long)]
        strict: bool,
    },
    /// Automatically fixes and reorders struct fields in descending alignment order (Scrooge)
    Fix {
        /// Target workspace directory (default: current directory)
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
        /// Scope filter by crate, directory, file, or symbol (e.g. `services/*`, `ChatContext`)
        #[arg(short, long)]
        scope: Option<String>,
        /// Target specific category (scrooge, taxo)
        #[arg(short, long)]
        category: Option<String>,
        /// Apply Scrooge struct alignment reordering
        #[arg(long)]
        scrooge: bool,
        /// Apply taxonomy naming check
        #[arg(long)]
        taxo: bool,
        /// Apply all fixers
        #[arg(long)]
        all: bool,
        /// Simulate fixes without writing to disk
        #[arg(long)]
        dry_run: bool,
    },
    /// Extracts architectural DAG: outputs Mermaid, Graphviz DOT, JSON, or interactive HTML
    Graph {
        /// Target workspace directory (default: current directory)
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
        /// Scope filter by crate, directory, file, or symbol (preserves 1-hop boundary context)
        #[arg(short, long)]
        scope: Option<String>,
        /// Output format (text, json, mermaid, dot, html)
        #[arg(short, long, default_value = "mermaid")]
        format: String,
        /// Optional destination file to write output to
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Audits L1D cache-line alignment, struct padding holes, and optimal field ordering
    Metrics {
        /// Target workspace directory (default: current directory)
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
        /// Scope filter by crate, directory, file, or symbol (e.g. `services/*`, `ChatContext`)
        #[arg(short, long)]
        scope: Option<String>,
    },
}

/// Main entry point for CLI binaries.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    execute(cli.command)
}

pub fn execute(command: Commands) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Commands::Check {
            dir,
            scope,
            category,
            scrooge,
            taxo,
            bound,
            hotpath,
            concur,
            all,
            format,
            strict,
        } => run_check(
            &dir,
            CheckArgs {
                scope: scope.as_deref(),
                category: category.as_deref(),
                scrooge,
                taxo,
                bound,
                hotpath,
                concur,
                all,
                format: &format,
                strict,
            },
        ),
        Commands::Health {
            dir,
            scope,
            format,
            output,
            strict,
        } => run_health(&dir, scope.as_deref(), &format, output, strict),
        Commands::Fix {
            dir,
            scope,
            category,
            scrooge,
            taxo,
            all,
            dry_run,
        } => run_fix(
            &dir,
            FixArgs {
                scope: scope.as_deref(),
                category: category.as_deref(),
                scrooge,
                taxo,
                all,
                dry_run,
            },
        ),
        Commands::Graph {
            dir,
            scope,
            format,
            output,
        } => run_graph(&dir, scope.as_deref(), &format, output),
        Commands::Metrics { dir, scope } => run_metrics(&dir, scope.as_deref()),
    }
}

#[derive(Debug, Clone, Default)]
pub struct CheckArgs<'a> {
    pub scope: Option<&'a str>,
    pub category: Option<&'a str>,
    pub format: &'a str,
    pub scrooge: bool,
    pub taxo: bool,
    pub bound: bool,
    pub hotpath: bool,
    pub concur: bool,
    pub all: bool,
    pub strict: bool,
}

pub fn run_check(target_dir: &Path, args: CheckArgs<'_>) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "==> Scanning workspace architecture at `{}`...",
        target_dir.display()
    );
    let scope_filter = args.scope.map(ScopeFilter::new);
    if let Some(s) = &scope_filter {
        println!("    Scope filter: `{}`", s.raw);
    }

    let config = StitchConfig::load(target_dir);
    let runner = CheckRunner::new_scoped(&config, target_dir, scope_filter);
    let mut diagnostics = runner.run();

    let mut cat_set = HashSet::new();
    if let Some(cat) = args.category {
        for part in cat.split(',') {
            let trimmed = part.trim().to_lowercase();
            if !trimmed.is_empty() {
                cat_set.insert(trimmed);
            }
        }
    }
    if args.scrooge {
        cat_set.insert("scrooge".to_string());
    }
    if args.taxo {
        cat_set.insert("taxo".to_string());
    }
    if args.bound {
        cat_set.insert("bound".to_string());
    }
    if args.hotpath {
        cat_set.insert("hotpath".to_string());
    }
    if args.concur {
        cat_set.insert("concur".to_string());
    }

    if !args.all && !cat_set.is_empty() {
        diagnostics.retain(|d| {
            cat_set
                .iter()
                .any(|cat| d.code.to_lowercase().contains(cat))
        });
    }

    let mut error_count = 0;
    let mut warn_count = 0;
    let use_rustc_fmt = args.format == "rustc";

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
                if args.strict {
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

pub fn run_health(
    target_dir: &Path,
    scope: Option<&str>,
    format: &str,
    output_file: Option<PathBuf>,
    strict: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "==> Evaluating Multidimensional Architecture Health Scorecard at `{}`...",
        target_dir.display()
    );
    let scope_filter = scope.map(ScopeFilter::new);
    let config = StitchConfig::load(target_dir);
    let engine = HealthEngine::new_scoped(&config, target_dir, scope_filter);
    let report = engine.evaluate();

    if format == "json" {
        let json = serde_json::to_string_pretty(&report)?;
        if let Some(path) = output_file {
            std::fs::write(&path, &json)?;
            println!("Health report written to `{}`.", path.display());
        } else {
            println!("{json}");
        }
    } else {
        let text = report.render_terminal();
        if let Some(path) = output_file {
            std::fs::write(&path, &text)?;
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

#[derive(Debug, Clone, Default)]
pub struct FixArgs<'a> {
    pub scope: Option<&'a str>,
    pub category: Option<&'a str>,
    pub scrooge: bool,
    pub taxo: bool,
    pub all: bool,
    pub dry_run: bool,
}

pub fn run_fix(target_dir: &Path, args: FixArgs<'_>) -> Result<(), Box<dyn std::error::Error>> {
    let scope_filter = args.scope.map(ScopeFilter::new);
    println!(
        "==> Running Automated Architecture Fixer at `{}`...",
        target_dir.display()
    );
    if let Some(s) = &scope_filter {
        println!("    Scope filter: `{}`", s.raw);
    }

    let mut cat_set = HashSet::new();
    if let Some(cat) = args.category {
        for part in cat.split(',') {
            let trimmed = part.trim().to_lowercase();
            if !trimmed.is_empty() {
                cat_set.insert(trimmed);
            }
        }
    }
    if args.scrooge {
        cat_set.insert("scrooge".to_string());
    }
    if args.taxo {
        cat_set.insert("taxo".to_string());
    }

    let run_scrooge = args.all || cat_set.is_empty() || cat_set.contains("scrooge");
    let run_taxo = args.all || cat_set.contains("taxo");

    if run_scrooge {
        println!("    Active category: `scrooge` (struct alignment optimization)");
        let engine = FixEngine::new_scoped(target_dir, scope_filter.clone());
        let report = engine.run_scrooge(args.dry_run)?;

        if report.changes.is_empty() {
            println!(
                "\n✅ All audited structs already adhere to optimal descending alignment. 0 bytes wasted."
            );
        } else {
            let action = if args.dry_run { "PROPOSED" } else { "APPLIED" };
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
        let config = StitchConfig::load(target_dir);
        let runner = CheckRunner::new_scoped(&config, target_dir, scope_filter);
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

pub fn run_graph(
    target_dir: &Path,
    scope: Option<&str>,
    format: &str,
    output_file: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let scope_filter = scope.map(ScopeFilter::new);
    let extractor = GraphExtractor::new_scoped(target_dir, scope_filter);
    let graph = extractor.extract();

    let output_str = match format {
        "json" => serde_json::to_string_pretty(&graph)?,
        "mermaid" => graph.to_mermaid(),
        "dot" => graph.to_dot(),
        "html" => graph.to_html(),
        _ => graph.to_mermaid(),
    };

    if let Some(path) = output_file {
        std::fs::write(&path, &output_str)?;
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

pub fn run_metrics(
    target_dir: &Path,
    scope: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("==> Running Scrooge Systems Memory & Alignment Audit...");
    let scope_filter = scope.map(ScopeFilter::new);
    if let Some(s) = &scope_filter {
        println!("    Scope filter: `{}`", s.raw);
    }
    let auditor = MetricsAuditor::new_scoped(target_dir, scope_filter);
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
