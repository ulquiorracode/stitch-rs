use stitch_core::blackboard::Blackboard;
use stitch_core::pipeline::Pipeline;

#[repr(C, align(64))]
struct Context;
impl Blackboard for Context {}

struct NotACommand;

fn main() {
    let _pipe = Pipeline::<Context, NotACommand, (), (), _>::for_command();
}
