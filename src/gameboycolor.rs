use crate::context;
use crate::context::EmulatorError;
use crate::DeviceMode;

pub struct GameBoyColor {
    context: context::Context,
}

impl GameBoyColor {
    pub fn new(data: &[u8], device_mode: DeviceMode) -> Result<Self, EmulatorError> {
        let context = context::Context::new(data, device_mode)?;
        Ok(Self { context })
    }

    pub fn execute_instruction(&mut self) {
        self.context.execute_instruction();
    }

    pub fn execute_frame(&mut self) {
        self.context.execute_frame();
    }

    pub fn frame_buffer(&self) -> &[u8] {
        self.context.frame_buffer()
    }
}
