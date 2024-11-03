use crate::context;
use crate::context::EmulatorError;
use crate::interface::LinkCable;
use crate::joypad::JoypadKeyState;
use crate::utils;
use crate::DeviceMode;

pub struct GameBoyColor {
    context: context::Context,

    frame_counter: usize,
}

impl GameBoyColor {
    pub fn new(
        data: &[u8],
        device_mode: DeviceMode,
        link_cable: Option<Box<dyn LinkCable>>,
    ) -> Result<Self, EmulatorError> {
        let context = context::Context::new(data, device_mode, link_cable)?;
        Ok(Self {
            context,
            frame_counter: 0,
        })
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

    pub fn set_key(&mut self, key_state: JoypadKeyState) {
        self.context.set_key(key_state);
    }

    pub fn save_data(&self) -> Option<Vec<u8>> {
        self.context.save_data()
    }

    pub fn rom_name(&self) -> &str {
        self.context.rom_name()
    }
}
