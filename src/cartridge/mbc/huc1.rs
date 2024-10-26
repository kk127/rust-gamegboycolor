use crate::cartridge::{Mbc, rom};

pub struct Huc1 {
    rom: Vec<u8>,
}

impl Mbc for Huc1 {
    fn read(&self, address: u16) -> u8 {
        todo!()
    }

    fn write(&mut self, _address: u16, _value: u8) {
        todo!()
    }
}

impl Huc1 {
    pub fn new(rom: rom::Rom, backup: Option<&[u8]>) -> Self {
        todo!()
    }
}
