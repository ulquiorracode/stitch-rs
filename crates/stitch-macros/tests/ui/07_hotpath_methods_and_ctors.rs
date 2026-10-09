use stitch_macros::layer;

pub struct CtorLayer;

#[layer]
impl CtorLayer {
    pub fn on_enter(&self, _ctx: &mut ()) -> stitch_core::flow::FlowControl<(), (), ()> {
        let _ = Box::new(123);
        stitch_core::flow::FlowControl::Proceed(())
    }
}

fn main() {}
