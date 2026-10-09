use stitch_core::taxonomy::Adapter;

struct RawDatabase; // Does not implement Port

struct MyAdapter;

impl Adapter for MyAdapter {
    type TargetPort = RawDatabase;
}

fn main() {}
