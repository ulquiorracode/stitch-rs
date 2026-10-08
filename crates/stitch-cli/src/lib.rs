//! Architecture-as-Code engine and CLI runner for `stitch` and `sma`.

pub mod check;
pub mod config;
pub mod graph;
pub mod metrics;

use check::CheckRunner;
use config::{RuleSeverity, StitchConfig};
use graph::GraphExtractor;
use metrics::MetricsAuditor;
use std::path::PathBuf;

/// Main entry point for CLI binaries.
pub fn run() -> Result<(), lexopt::Error> {
    use lexopt::prelude::*;

    let mut parser = lexopt::Parser::from_env();
    let mut subcommand = None;
    let mut target_dir = PathBuf::from(".");
    let mut format = String::from("text");
    let mut output_file: Option<PathBuf> = None;
    let mut strict = false;

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
            Long("format") | Short('f') => {
                format = parser.value()?.string()?.to_lowercase();
            }
            Long("output") | Short('o') => {
                output_file = Some(PathBuf::from(parser.value()?.string()?));
            }
            Long("strict") => {
                strict = true;
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

    let config = StitchConfig::load(&target_dir);

    match subcommand.as_deref() {
        Some("check") => {
            println!(
                "==> Scanning workspace architecture at `{}`...",
                target_dir.display()
            );
            let runner = CheckRunner::new(&config, &target_dir);
            let diagnostics = runner.run();

            let mut error_count = 0;
            let mut warn_count = 0;

            for diag in &diagnostics {
                match diag.severity {
                    RuleSeverity::Deny => {
                        error_count += 1;
                        eprint!("{}", diag.render_rustc());
                    }
                    RuleSeverity::Warn => {
                        warn_count += 1;
                        if strict {
                            error_count += 1;
                            eprint!("{}", diag.render_rustc());
                        } else {
                            println!("{}", diag.render_rustc());
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
        Some("graph") => {
            let extractor = GraphExtractor::new(&target_dir);
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
            let auditor = MetricsAuditor::new(&target_dir);
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
    graph       Extracts architectural DAG; outputs Mermaid, Graphviz DOT, JSON, or interactive HTML.
    metrics     Audits L1D cache-line alignment, struct padding holes, and field ordering.

OPTIONS:
    -d, --dir <DIR>          Target workspace directory (default: current directory)
    -f, --format <FMT>       Output format for graph (text, json, mermaid, dot, html)
    -o, --output <FILE>      Write graph/metrics output to a specified file
        --strict             Treat warnings as hard errors
    -h, --help               Print help information
    -V, --version            Print version information
"#
    );
}
