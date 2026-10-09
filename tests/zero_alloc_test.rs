//! Zero-allocation tracking test verifying that U-cycle dispatch performs exactly 0 bytes of heap allocation.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use stitch_rs::prelude::*;

struct TrackingAllocator {
    alloc_count: AtomicUsize,
    allocated_bytes: AtomicUsize,
}

impl TrackingAllocator {
    const fn new() -> Self {
        Self {
            alloc_count: AtomicUsize::new(0),
            allocated_bytes: AtomicUsize::new(0),
        }
    }

    fn reset(&self) {
        self.alloc_count.store(0, Ordering::SeqCst);
        self.allocated_bytes.store(0, Ordering::SeqCst);
    }

    fn counts(&self) -> (usize, usize) {
        (
            self.alloc_count.load(Ordering::SeqCst),
            self.allocated_bytes.load(Ordering::SeqCst),
        )
    }
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_count.fetch_add(1, Ordering::SeqCst);
        self.allocated_bytes.fetch_add(layout.size(), Ordering::SeqCst);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOC: TrackingAllocator = TrackingAllocator::new();

#[repr(C, align(64))]
struct BenchContext {
    val: u64,
}
impl Blackboard for BenchContext {}

#[derive(Clone, Copy)]
struct Intent(u64);

struct LayerA;
impl Layer<BenchContext, Intent, u64, ()> for LayerA {
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

struct LayerB;
impl Layer<BenchContext, Intent, u64, ()> for LayerB {
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

struct BenchTerminal;
impl Terminal<BenchContext, Intent, u64, ()> for BenchTerminal {
    fn execute(&mut self, ctx: &mut BenchContext, intent: Intent) -> Result<u64, ()> {
        Ok(ctx.val.wrapping_add(intent.0))
    }
}


#[test]
fn verify_strictly_zero_heap_allocations_on_u_cycle() {
    let mut pipe = Pipeline::on_terminal(BenchTerminal)
        .wrap(LayerB)
        .wrap(LayerA);
    let mut ctx = BenchContext { val: 0 };

    // Reset tracking allocator before entering U-cycle
    ALLOC.reset();

    // Run 1,000,000 iterations through the monomorphic pipeline
    for i in 0..1_000_000 {
        let res = pipe.dispatch(&mut ctx, Intent(i)).unwrap();
        core::hint::black_box(res);
    }

    let (allocs, bytes) = ALLOC.counts();
    assert_eq!(allocs, 0, "Monomorphic U-cycle violated zero-allocation contract: performed {allocs} heap allocations");
    assert_eq!(bytes, 0, "Monomorphic U-cycle allocated {bytes} bytes of heap memory");
}
