use crate::context;

pub struct GameBoyColor {
    ctx: context::Context,
}

impl GameBoyColor {
    pub fn new(data: &[u8]) -> Self {
        let ctx = context::Context::new(data);
        Self { ctx }
    }
}