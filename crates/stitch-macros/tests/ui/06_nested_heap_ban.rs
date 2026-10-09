use stitch_macros::layer;

#[layer]
pub struct NestedHeapLayer {
    pub opt_string: Option<String>,
}

fn main() {}
