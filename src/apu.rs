use log::warn;

pub struct Apu {}

impl Apu {
    pub fn new() -> Self {
        warn!("Apu not implemented");
        Self {}
    }

    pub fn read(&self, address: u16) -> u8 {
        todo!()
    }

    pub fn write(&mut self, address: u16, value: u8) {
        todo!()
    }

    pub fn tick(&mut self) {
        todo!()
    }

    pub fn audio_buffer(&self) -> &[u8] {
        todo!()
    }
    
}