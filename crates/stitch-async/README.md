# Stitch Async (`stitch-async`)

Zero-Cost, Monomorphic Asynchronous U-Cycle Middleware Pipeline & Outbox Streaming Egress Framework implementing The Sewing Machine Architecture (SMA).

## Core Concepts
- **RPITIT Native Monomorphism**: `AsyncLayer` and `AsyncTerminal` using native `async fn in traits` (Rust 1.85+) without `Box<dyn Future>` or compulsory heap allocations.
- **`AsyncPipeline`**: Monomorphic asynchronous U-cycle traversal matching synchronous `stitch-core` mechanics.
- **`OutboxStreamWorker`**: Autonomous peripheral egress background worker streaming batches of events from synchronous `Outbox` buffers to external sinks (Tokio, databases, network sockets) without blocking the hot simulation loop.
