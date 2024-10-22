use crate::cartridge::{Mbc, rom};

pub struct Mbc1 {
    rom: Vec<u8>,
}

impl Mbc for Mbc1 {
    fn read(&self, address: u16) -> u8 {
        todo!()
    }

    fn write(&mut self, _address: u16, _value: u8) {
        todo!()
    }
}

impl Mbc1 {
    pub fn new(rom: rom::Rom) -> Self {
        todo!()
    }
}