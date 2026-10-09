use stitch_core::blackboard::Blackboard;
use stitch_core::pipeline::Pipeline;

#[repr(C, align(64))]
struct Context;
impl Blackboard for Context {}

struct NotACommand;

fn main() {
    let mut pipe = Pipeline::for_command();
    let mut ctx = Context;
    let _ = pipe.dispatch(&mut ctx, NotACommand);
}
