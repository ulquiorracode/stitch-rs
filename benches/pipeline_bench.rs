//! Pipeline Performance Benchmarks.
//!
//! Evaluates throughput and latency across:
//! 1. `stitch::Pipeline` (Compile-time monomorphic U-cycle)
//! 2. `Box<dyn DynamicLayer>` (Virtual method dynamic dispatch chain)
//! 3. Hand-inlined Baseline (Theoretical zero-cost execution floor)
//! 4. `tower::Service` (Async industry standard using Ready future)
//! 5. Cache-Line Contention (64-byte alignment vs unaligned false sharing)

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use stitch_rs::prelude::*;
use tower::Service;

// --- Context & Types ---

#[repr(C, align(64))]
struct BenchContext {
    val: u64,
}
impl Blackboard for BenchContext {}

struct UnalignedContext {
    val: u64,
}

#[derive(Clone, Copy)]
struct Intent(u64);

// --- 1. Stitch Monomorphic Layers ---

struct LayerA;
impl Layer<BenchContext, Intent, u64, ()> for LayerA {
    #[inline(always)]
    fn on_enter(&self, ctx: &mut BenchContext, intent: Intent) -> FlowControl<Intent, u64, ()> {
        ctx.val = ctx.val.wrapping_add(intent.0);
        FlowControl::Proceed(intent)
    }
    #[inline(always)]
    fn on_exit(&self, ctx: &mut BenchContext, outcome: &mut Result<u64, ()>) {
        if let Ok(val) = outcome {
            ctx.val = ctx.val.wrapping_add(*val);
        }
    }
}

struct LayerB;
impl Layer<BenchContext, Intent, u64, ()> for LayerB {
    #[inline(always)]
    fn on_enter(&self, ctx: &mut BenchContext, intent: Intent) -> FlowControl<Intent, u64, ()> {
        ctx.val = ctx.val.wrapping_add(1);
        FlowControl::Proceed(intent)
    }
    #[inline(always)]
    fn on_exit(&self, _ctx: &mut BenchContext, outcome: &mut Result<u64, ()>) {
        if let Ok(val) = outcome {
            *val = val.wrapping_mul(2);
        }
    }
}

struct BenchTerminal;
impl Terminal<BenchContext, Intent, u64, ()> for BenchTerminal {
    #[inline(always)]
    fn execute(&mut self, ctx: &mut BenchContext, intent: Intent) -> Result<u64, ()> {
        ctx.val = ctx.val.wrapping_add(intent.0);
        Ok(ctx.val)
    }
}

// --- 2. Dynamic Box<dyn Layer> Implementation ---

trait DynLayer {
    fn on_enter(&self, ctx: &mut BenchContext, intent: Intent) -> FlowControl<Intent, u64, ()>;
    fn on_exit(&self, ctx: &mut BenchContext, outcome: &mut Result<u64, ()>);
}

struct LayerADyn;
impl DynLayer for LayerADyn {
    fn on_enter(&self, ctx: &mut BenchContext, intent: Intent) -> FlowControl<Intent, u64, ()> {
        ctx.val = ctx.val.wrapping_add(intent.0);
        FlowControl::Proceed(intent)
    }
    fn on_exit(&self, ctx: &mut BenchContext, outcome: &mut Result<u64, ()>) {
        if let Ok(val) = outcome {
            ctx.val = ctx.val.wrapping_add(*val);
        }
    }
}

struct LayerBDyn;
impl DynLayer for LayerBDyn {
    fn on_enter(&self, ctx: &mut BenchContext, intent: Intent) -> FlowControl<Intent, u64, ()> {
        ctx.val = ctx.val.wrapping_add(1);
        FlowControl::Proceed(intent)
    }
    fn on_exit(&self, _ctx: &mut BenchContext, outcome: &mut Result<u64, ()>) {
        if let Ok(val) = outcome {
            *val = val.wrapping_mul(2);
        }
    }
}

fn run_dyn_pipeline(layers: &[Box<dyn DynLayer>], ctx: &mut BenchContext, intent: Intent) -> u64 {
    for l in layers {
        match l.on_enter(ctx, intent) {
            FlowControl::Proceed(_) => {}
            FlowControl::ShortCircuit(o) => return o,
            FlowControl::Halt(_) => return 0,
        }
    }
    ctx.val = ctx.val.wrapping_add(intent.0);
    let mut outcome = Ok(ctx.val);
    for l in layers.iter().rev() {
        l.on_exit(ctx, &mut outcome);
    }
    outcome.unwrap_or(0)
}

// --- 3. Hand-Inlined Baseline ---

#[inline(always)]
fn run_hand_inlined(ctx: &mut BenchContext, intent: Intent) -> u64 {
    // LayerA on_enter
    ctx.val = ctx.val.wrapping_add(intent.0);
    // LayerB on_enter
    ctx.val = ctx.val.wrapping_add(1);
    // Terminal
    ctx.val = ctx.val.wrapping_add(intent.0);
    let mut outcome = ctx.val;
    // LayerB on_exit
    outcome = outcome.wrapping_mul(2);
    // LayerA on_exit
    ctx.val = ctx.val.wrapping_add(outcome);
    outcome
}

// --- 4. Tower Service Implementation ---

#[derive(Clone)]
struct TowerLeaf;

impl Service<Intent> for TowerLeaf {
    type Response = u64;
    type Error = ();
    type Future = std::future::Ready<Result<u64, ()>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Intent) -> Self::Future {
        std::future::ready(Ok(req.0.wrapping_mul(2)))
    }
}

#[derive(Clone)]
struct TowerMiddleware<S> {
    inner: S,
}

impl<S> Service<Intent> for TowerMiddleware<S>
where
    S: Service<Intent, Response = u64, Error = ()> + Clone + 'static,
    S::Future: 'static,
{
    type Response = u64;
    type Error = ();
    type Future = Pin<Box<dyn Future<Output = Result<u64, ()>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Intent) -> Self::Future {
        let fut = self.inner.call(req);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res.wrapping_add(1))
        })
    }
}

// --- Benchmarks ---

fn bench_pipeline_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_dispatch");

    // 1. Stitch Monomorphic Pipeline
    let mut stitch_pipe = Pipeline::on_terminal(BenchTerminal)
        .wrap(LayerB)
        .wrap(LayerA);
    let mut stitch_ctx = BenchContext { val: 0 };

    group.bench_function("stitch_monomorphic", |b| {
        b.iter(|| {
            let res = stitch_pipe.dispatch(black_box(&mut stitch_ctx), black_box(Intent(42)));
            black_box(res)
        })
    });

    // 2. Hand-inlined Baseline
    let mut inlined_ctx = BenchContext { val: 0 };
    group.bench_function("hand_inlined_baseline", |b| {
        b.iter(|| {
            let res = run_hand_inlined(black_box(&mut inlined_ctx), black_box(Intent(42)));
            black_box(res)
        })
    });

    // 3. Dynamic Box<dyn Layer>
    let dyn_layers: Vec<Box<dyn DynLayer>> = vec![Box::new(LayerADyn), Box::new(LayerBDyn)];
    let mut dyn_ctx = BenchContext { val: 0 };
    group.bench_function("box_dyn_layers", |b| {
        b.iter(|| {
            let res = run_dyn_pipeline(&dyn_layers, black_box(&mut dyn_ctx), black_box(Intent(42)));
            black_box(res)
        })
    });

    // 4. Tower Service
    let mut tower_service = TowerMiddleware {
        inner: TowerMiddleware { inner: TowerLeaf },
    };
    group.bench_function("tower_ready_future", |b| {
        b.iter(|| {
            let mut fut = tower_service.call(black_box(Intent(42)));
            let waker = std::task::Waker::noop();
            let mut cx = Context::from_waker(waker);
            let res = Pin::new(&mut fut).poll(&mut cx);
            black_box(res)
        })
    });

    group.finish();
}

fn bench_cache_alignment(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_alignment");

    let mut aligned: [BenchContext; 4] = [
        BenchContext { val: 0 },
        BenchContext { val: 0 },
        BenchContext { val: 0 },
        BenchContext { val: 0 },
    ];
    group.bench_function("aligned_64b_cells", |b| {
        b.iter(|| {
            for cell in aligned.iter_mut() {
                cell.val = black_box(cell.val.wrapping_add(1));
            }
        })
    });

    let mut unaligned: [UnalignedContext; 4] = [
        UnalignedContext { val: 0 },
        UnalignedContext { val: 0 },
        UnalignedContext { val: 0 },
        UnalignedContext { val: 0 },
    ];
    group.bench_function("unaligned_packed_cells", |b| {
        b.iter(|| {
            for cell in unaligned.iter_mut() {
                cell.val = black_box(cell.val.wrapping_add(1));
            }
        })
    });

    group.finish();
}

criterion_group!(benches, bench_pipeline_throughput, bench_cache_alignment);
criterion_main!(benches);
