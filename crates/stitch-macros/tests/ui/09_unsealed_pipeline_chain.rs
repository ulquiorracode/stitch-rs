use stitch_core::blackboard::Blackboard;
use stitch_core::pipeline::PipelineChain;

#[repr(C, align(64))]
struct MyCtx;
impl Blackboard for MyCtx {}

struct RogueNode;

impl PipelineChain<MyCtx, (), (), ()> for RogueNode {
    fn cycle(&mut self, _ctx: &mut MyCtx, _intent: ()) -> Result<(), ()> {
        Ok(())
    }
}

fn main() {}
