use stitch_core::blackboard::Blackboard;
use stitch_core::flow::FlowControl;
use stitch_core::middleware::Layer;
use stitch_core::taxonomy::{Port, PortLayer, RequiresPort};

trait MockConsolePort: Port {}

struct RawDatabase;

#[repr(C, align(64))]
struct MyCtx;
impl Blackboard for MyCtx {}

struct MyPortLayer<'a> {
    db: &'a RawDatabase,
}

impl<'a> RequiresPort<RawDatabase> for MyPortLayer<'a> {
    fn port(&self) -> &RawDatabase {
        self.db
    }
}

impl<'a> Layer<MyCtx, (), (), ()> for MyPortLayer<'a> {
    fn on_enter(&self, _ctx: &mut MyCtx, intent: ()) -> FlowControl<(), (), ()> {
        FlowControl::Proceed(intent)
    }
    fn on_exit(&self, _ctx: &mut MyCtx, _outcome: &mut Result<(), ()>) {}
}

fn assert_port_layer<'a, L: PortLayer<MyCtx, (), (), (), dyn MockConsolePort>>(_l: &'a L) {}

fn main() {
    let raw = RawDatabase;
    let layer = MyPortLayer { db: &raw };
    assert_port_layer(&layer);
}
