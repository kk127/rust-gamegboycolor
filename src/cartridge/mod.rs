mod mbc;
pub mod rom;

use mbc::{huc1, mbc1, mbc2, mbc3, mbc5, mbc6, rom_only};
use std::{default, fmt};

pub trait Mbc {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);

    fn save_data(&self) -> Option<Vec<u8>>;
}

#[derive(Default, Debug, Clone, Copy)]
enum MbcType {
    #[default]
    RomOnly,
    Mbc1,
    Mbc2,
    Mbc3,
    Mbc5,
    Mbc6,
    Mbc7,
    Mmm01,
    Huc1,
    Huc3,
}

impl fmt::Display for MbcType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            MbcType::RomOnly => "RomOnly",
            MbcType::Mbc1 => "Mbc1",
            MbcType::Mbc2 => "Mbc2",
            MbcType::Mbc3 => "Mbc3",
            MbcType::Mbc5 => "Mbc5",
            MbcType::Mbc6 => "Mbc6",
            MbcType::Mbc7 => "Mbc7",
            MbcType::Mmm01 => "Mmm01",
            MbcType::Huc1 => "Huc1",
            MbcType::Huc3 => "Huc3",
        };
        write!(f, "{}", s)
    }
}

pub enum Cartridge {
    RomOnly(rom_only::RomOnly),
    Mbc1(mbc1::Mbc1),
    Mbc2(mbc2::Mbc2),
    Mbc3(mbc3::Mbc3),
    Mbc5(mbc5::Mbc5),
    Mbc6(mbc6::Mbc6),
    Huc1(huc1::Huc1),
}

impl Cartridge {
    pub fn new(rom: rom::Rom, backup: Option<Vec<u8>>) -> Self {
        match rom.mbc_type() {
            MbcType::RomOnly => Cartridge::RomOnly(rom_only::RomOnly::new(rom)),
            MbcType::Mbc1 => Cartridge::Mbc1(mbc1::Mbc1::new(rom, backup)),
            MbcType::Mbc2 => Cartridge::Mbc2(mbc2::Mbc2::new(rom, backup)),
            MbcType::Mbc3 => Cartridge::Mbc3(mbc3::Mbc3::new(rom, backup)),
            MbcType::Mbc5 => Cartridge::Mbc5(mbc5::Mbc5::new(rom, backup)),
            MbcType::Mbc6 => Cartridge::Mbc6(mbc6::Mbc6::new(rom, backup)),
            MbcType::Huc1 => Cartridge::Huc1(huc1::Huc1::new(rom, backup)),
            _ => unimplemented!(),
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        match self {
            Cartridge::RomOnly(rom) => rom.read(address),
            Cartridge::Mbc1(mbc) => mbc.read(address),
            Cartridge::Mbc2(mbc) => mbc.read(address),
            Cartridge::Mbc3(mbc) => mbc.read(address),
            Cartridge::Mbc5(mbc) => mbc.read(address),
            Cartridge::Mbc6(mbc) => mbc.read(address),
            Cartridge::Huc1(mbc) => mbc.read(address),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match self {
            Cartridge::RomOnly(rom) => rom.write(address, value),
            Cartridge::Mbc1(mbc) => mbc.write(address, value),
            Cartridge::Mbc2(mbc) => mbc.write(address, value),
            Cartridge::Mbc3(mbc) => mbc.write(address, value),
            Cartridge::Mbc5(mbc) => mbc.write(address, value),
            Cartridge::Mbc6(mbc) => mbc.write(address, value),
            Cartridge::Huc1(mbc) => mbc.write(address, value),
        }
    }

    pub fn save_data(&self) -> Option<Vec<u8>> {
        match self {
            Cartridge::RomOnly(rom) => rom.save_data(),
            Cartridge::Mbc1(mbc) => mbc.save_data(),
            Cartridge::Mbc2(mbc) => mbc.save_data(),
            Cartridge::Mbc3(mbc) => mbc.save_data(),
            Cartridge::Mbc5(mbc) => mbc.save_data(),
            Cartridge::Mbc6(mbc) => mbc.save_data(),
            Cartridge::Huc1(mbc) => mbc.save_data(),
        }
    }
}
