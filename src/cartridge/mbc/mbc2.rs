use crate::cartridge::{Mbc, rom};

pub struct Mbc2 {
    rom: Vec<u8>,
}

impl Mbc for Mbc2 {
    fn read(&self, address: u16) -> u8 {
        todo!()
    }

    fn write(&mut self, _address: u16, _value: u8) {
        todo!()
    }
}

impl Mbc2 {
    pub fn new(rom: rom::Rom) -> Self {
        todo!()
    }
}