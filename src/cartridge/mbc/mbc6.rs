use crate::cartridge::{rom, Mbc};

pub struct Mbc6 {
    rom: Vec<u8>,
}

impl Mbc for Mbc6 {
    fn read(&self, address: u16) -> u8 {
        todo!()
    }

    fn write(&mut self, _address: u16, _value: u8) {
        todo!()
    }

    fn save_data(&self) -> Option<Vec<u8>> {
        todo!()
    }
}

impl Mbc6 {
    pub fn new(rom: rom::Rom, backup: Option<Vec<u8>>) -> Self {
        todo!()
    }
}
