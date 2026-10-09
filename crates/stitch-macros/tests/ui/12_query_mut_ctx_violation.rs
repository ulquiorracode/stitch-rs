use stitch_macros::query;

struct MaterialContext;

struct FetchStatus;

#[query]
impl FetchStatus {
    pub fn query(&self, _ctx: &mut MaterialContext) -> Result<(), ()> {
        Ok(())
    }
}

fn main() {}
