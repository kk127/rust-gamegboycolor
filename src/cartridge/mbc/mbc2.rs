use crate::cartridge::{rom, Mbc};

pub struct Mbc2 {
    rom: rom::Rom,
    rom_bank: u8,
    rom_bank_mask: u8,
    ram: Vec<u8>,
    ram_enable: bool,
}

impl Mbc for Mbc2 {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => self.rom.data()[address as usize],
            0x4000..=0x7FFF => {
                let bank = (self.rom_bank & self.rom_bank_mask) as usize * 0x4000;
                let offset = (address - 0x4000) as usize;
                self.rom.data()[bank + offset]
            }
            0xA000..=0xA1FF => {
                if self.ram_enable {
                    let address = (address & 0x1FF) as usize / 2;
                    let data = self.ram[address];
                    if address % 2 == 0 {
                        data & 0x0F
                    } else {
                        data >> 4
                    }
                } else {
                    0xFF
                }
            }
            _ => unreachable!("Unreachable MBC2 read address: {:#06X}", address),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x3FFF => {
                if address & 0x100 == 0 {
                    self.ram_enable = value & 0x0F == 0x0A;
                } else {
                    self.rom_bank = (value & 0x0F).max(1);
                }
            }
            0xA000..=0xBFFF => {
                if self.ram_enable {
                    let address = (address & 0x1FF) as usize / 2;
                    let data = self.ram[address];
                    if address % 2 == 0 {
                        self.ram[address] = (data & 0xF0) | (value & 0x0F);
                    } else {
                        self.ram[address] = (data & 0x0F) | (value << 4);
                    }
                }
            }
            _ => unreachable!("Unreachable MBC2 write address: {:#06X}", address),
        }
    }
    fn save_data(&self) -> Option<Vec<u8>> {
        if self.rom.have_ram() {
            Some(self.ram.clone())
        } else {
            None
        }
    }
}

impl Mbc2 {
    pub fn new(rom: rom::Rom, backup: Option<Vec<u8>>) -> Self {
        let rom_bank_num = rom.rom_size() / 0x4000;
        let rom_bank_mask = rom_bank_num.saturating_sub(1) as u8;
        let ram = match backup {
            Some(data) => data,
            None => vec![0; 512],
        };

        Self {
            rom,
            rom_bank: 1,
            rom_bank_mask,
            ram,
            ram_enable: false,
        }
    }
}
