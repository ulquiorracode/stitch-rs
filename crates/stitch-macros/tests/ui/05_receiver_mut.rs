use stitch_core::flow::FlowControl;
use stitch_macros::layer;

#[repr(C, align(64))]
pub struct DummyContext;

pub struct DummyLayer;

#[layer]
impl DummyLayer {
    pub fn on_enter(&mut self, _ctx: &mut DummyContext) -> FlowControl<(), (), ()> {
        FlowControl::Proceed(())
    }

    pub fn on_exit(&mut self, _ctx: &mut DummyContext) {}
}

fn main() {}
