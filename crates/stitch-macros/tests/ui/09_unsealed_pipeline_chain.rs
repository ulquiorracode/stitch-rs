use stitch_core::blackboard::Blackboard;
use stitch_core::pipeline::PipelineChain;

#[repr(C, align(64))]
struct MyCtx;
impl Blackboard for MyCtx {}

struct KnowingAttacker;

// An adversary attempts to bypass the seal by importing the sealed trait directly
impl stitch_core::sealed::Sealed for KnowingAttacker {}

impl PipelineChain<MyCtx, (), (), ()> for KnowingAttacker {
    fn cycle(&mut self, _ctx: &mut MyCtx, _intent: ()) -> Result<(), ()> {
        Ok(())
    }
}

fn main() {}
