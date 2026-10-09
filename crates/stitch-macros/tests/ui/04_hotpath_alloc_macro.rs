use stitch_core::flow::FlowControl;
use stitch_core::middleware::Layer;
use stitch_macros::layer;

#[repr(C, align(64))]
pub struct DummyContext;

pub struct DummyLayer;

#[layer]
impl Layer<DummyContext, (), (), ()> for DummyLayer {
    fn on_enter(&self, _ctx: &mut DummyContext, _intent: ()) -> FlowControl<(), (), ()> {
        let _s = format!("forbidden alloc");
        FlowControl::Proceed(())
    }

    fn on_exit(&self, _ctx: &mut DummyContext, _outcome: &mut Result<(), ()>) {}
}

fn main() {}
