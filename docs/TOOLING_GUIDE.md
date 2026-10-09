# Architecture-as-Code (AaC) Tooling Guide

This document describes the design, rule catalog, and usage of the `stitch` CLI tooling suite.

`stitch-cli` operates as an AST-level syntax and structural analyzer powered by `syn` and `rayon`. It enforces **The Sewing Machine Architecture (SMA)** rules before compilation, catching architectural regressions that `rustc` alone cannot detect.

---

## 1. Tool Suite Overview

The `stitch` binary provides five primary subcommands:

| Subcommand | Functionality | Primary Application |
| :--- | :--- | :--- |
| `stitch check` | Multi-threaded AST architectural rule validation | Pre-commit hook & CI pipeline gate |
| `stitch fix` | Automated Scrooge memory alignment restructuring | Batch elimination of struct padding holes |
| `stitch metrics` | Memory size, padding, and cache alignment auditor | Memory footprint profiling |
| `stitch health` | Architectural Health Scorecard calculation | Codebase quality assessment |
| `stitch graph` | Static architectural DAG extractor & visualizer | Mermaid, Graphviz, and HTML export |

---

## 2. Rule Catalog

### 2.1 Taxonomy & Suffix Rules (`SMA-TAXO-*`)

Enforces rigid domain naming conventions across hexagonal boundaries:

| Rule Code | Target Element | Contract Requirement | Severity |
| :--- | :--- | :--- | :--- |
| `SMA-TAXO-001` | Struct decorated with `#[stitch::layer]` | Type name must end with `Layer` | Error |
| `SMA-TAXO-002` | Struct decorated with `#[stitch::terminal]` | Type name must end with `Terminal` | Error |
| `SMA-TAXO-003` | Trait decorated with `#[stitch::port]` | Trait name must end with `Port` | Error |
| `SMA-TAXO-004` | Struct decorated with `#[stitch::adapter]` | Type name must end with `Adapter` | Error |
| `SMA-TAXO-005` | Struct decorated with `#[stitch::hub]` | Type name must end with `Hub` | Error |
| `SMA-TAXO-006` | Struct decorated with `#[stitch::token]` | Type name must end with `Token` | Error |
| `SMA-TAXO-007` | Struct decorated with `#[stitch::id]` | Type name must end with `Id` | Error |

### 2.2 Scrooge Memory Layout Rules (`SMA-SCROOGE-*`)

Enforces cache-friendly memory layouts and eliminates padding waste:

| Rule Code | Target Element | Contract Requirement | Severity |
| :--- | :--- | :--- | :--- |
| `SMA-SCROOGE-001` | Context marked with `#[stitch::blackboard]` | Must declare `#[repr(C, align(64))]` | Error |
| `SMA-SCROOGE-002` | Domain tokens / IDs | Must declare `#[repr(transparent)]` | Error |
| `SMA-SCROOGE-010` | Hot-path structs | Prohibits heap types (`String`, `Vec`, `Box`, `HashMap`) | Error |

### 2.3 Hot-Path Anti-Pattern Denylist (`SMA-HOTPATH-*`)

Inspects pipeline traversal methods (`on_enter`, `on_exit`, `execute`):

| Rule Code | Anti-Pattern | Reason |
| :--- | :--- | :--- |
| `SMA-HOTPATH-020` | `format!`, `vec!`, `Box::new` | Heap allocations forbidden inside high-frequency dispatch |
| `SMA-HOTPATH-021` | `.clone()`, `.to_string()`, `.to_vec()` | Implicit heap copies on hot execution paths |
| `SMA-HOTPATH-022` | `panic!`, `todo!`, `unimplemented!`, `unreachable!` | Violation of total function contract on U-cycle ascent |

---

## 3. CLI Usage Examples

### Running Architectural Verification

```bash
# Check all crates in workspace
stitch check

# Scoped check for specific package
stitch check -p stitch-core

# Filter by category
stitch check --category scrooge
stitch check --category taxonomy
```

### Automated Struct Field Alignment Optimization

```bash
# Reorder struct fields in descending alignment order to eliminate padding
stitch fix --scrooge
```

### Measuring Architecture Health

```bash
# Output architectural scorecard
stitch health
```

### Exporting Architectural Graphs

```bash
# Generate Mermaid flowchart of pipeline topology
stitch graph --format mermaid

# Generate interactive SVG/HTML diagram
stitch graph --format html -o target/arch_graph.html
```
