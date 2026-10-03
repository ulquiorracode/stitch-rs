# Changelog

All notable changes to `stitch-rs` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Pure monomorphic U-cycle pipeline framework (`Pipeline`, `Machine`).
- Sewing Machine Architecture (SMA) pattern abstraction (`on_enter` descent and `on_exit` ascent phases).
- Zero-cost static generics chain without dynamic heap dispatch (`Box<dyn ..>`).
- Strict Command-Query Separation (`CQS`) primitives (`Query`, `Command`).
- Flow control primitives (`FlowControl::Continue`, `FlowControl::Halt`, `FlowControl::EarlyExit`).
- Fully `no_std` compatible design without runtime allocator dependencies.
- GitHub Actions CI matrix with automated clippy, formatting, and unit tests.
