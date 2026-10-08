use stitch_macros::blackboard;

#[blackboard]
#[repr(C, align(64))]
pub struct LeakyContext {
    pub bad_string: String,
}

fn main() {}
