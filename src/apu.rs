use log::warn;

pub struct Apu {}

impl Apu {
    pub fn new() -> Self {
        warn!("Apu not implemented");
        Self {}
    }

    pub fn read(&self, address: u16) -> u8 {
        warn!("Apu read not implemented: {:#06X}", address);
        0xFF
    }

    pub fn write(&mut self, address: u16, value: u8) {
        warn!(
            "Apu write not implemented: {:#06X} <- {:#04X}",
            address, value
        );
    }

    pub fn tick(&mut self) {
        todo!()
    }

    pub fn audio_buffer(&self) -> &[u8] {
        todo!()
    }
}
