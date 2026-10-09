use stitch_core::flow::FlowControl;
use stitch_core::middleware::Layer;
use stitch_macros::layer;

#[repr(C, align(64))]
pub struct CleanContext;
impl stitch_core::Blackboard for CleanContext {}

pub struct BrokenAscentLayer;

#[layer]
impl Layer<CleanContext, (), (), ()> for BrokenAscentLayer {
    fn on_enter(&self, _ctx: &mut CleanContext, intent: ()) -> FlowControl<(), (), ()> {
        FlowControl::Proceed(intent)
    }

    fn on_exit(&self, _ctx: &mut CleanContext, _outcome: &mut Result<(), ()>) {
        panic!("Panic in on_exit violates total execution contract");
    }
}

fn main() {}
