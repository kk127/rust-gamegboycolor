use crate::cartridge::{Mbc, rom};

pub struct RomOnly {
    rom: Vec<u8>,
}


impl Mbc for RomOnly {
    fn read(&self, address: u16) -> u8 {
        self.rom[address as usize]
    }

    fn write(&mut self, _address: u16, _value: u8) {
        // Do nothing
    }
}

impl RomOnly {
    pub fn new(rom: rom::Rom) -> Self {
        Self { rom: rom.data }
    }
}