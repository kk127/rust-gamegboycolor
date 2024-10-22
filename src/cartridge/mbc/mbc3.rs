use crate::cartridge::{Mbc, rom};

pub struct Mbc3 {
    rom: Vec<u8>,
}

impl Mbc for Mbc3 {
    fn read(&self, address: u16) -> u8 {
        todo!()
    }

    fn write(&mut self, address: u16, _value: u8) {
        todo!()
    }
}

impl Mbc3 {
    pub fn new(rom: rom::Rom) -> Self {
        todo!()
    }
}
