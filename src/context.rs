use crate::cartridge::{self, rom};

pub struct Context {
    cartridge: cartridge::Cartridge,
}

impl Context {
    pub fn new(data: &[u8]) -> Self {
        let rom = rom::Rom::new(data).unwrap();
        let cartridge = cartridge::Cartridge::new(rom);
        Self { cartridge }
    }
}

pub trait Bus {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);

    fn tick(&mut self);
}

trait Cartridge {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}
