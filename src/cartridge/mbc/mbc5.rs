use crate::cartridge::{rom, Mbc};

pub struct Mbc5 {
    rom: rom::Rom,
    ram: Vec<u8>,
    ram_enable: bool,
    rom_bank: u16,
    rom_bank_mask: u16,
    ram_bank: u8,
    ram_bank_mask: u8,
}

impl Mbc for Mbc5 {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => self.rom.data()[address as usize],
            0x4000..=0x7FFF => {
                let bank = (self.rom_bank & self.rom_bank_mask) as usize * 0x4000;
                let offset = (address - 0x4000) as usize;
                self.rom.data()[bank + offset]
            }
            0xA000..=0xBFFF => {
                if self.ram_enable {
                    let bank = (self.ram_bank & self.ram_bank_mask) as usize * 0x2000;
                    let offset = (address - 0xA000) as usize;
                    self.ram[bank + offset]
                } else {
                    0xFF
                }
            }
            _ => unreachable!("Unreachable MBC5 read address: {:#06X}", address),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x1FFF => self.ram_enable = value & 0x0F == 0x0A,
            0x2000..=0x2FFF => self.rom_bank = (self.rom_bank & 0x100) | value as u16,
            0x3000..=0x3FFF => {
                self.rom_bank = (self.rom_bank & 0xFF) | ((value as u16 & 0x01) << 8)
            }
            0x4000..=0x5FFF => self.ram_bank = value & 0x0F,
            0xA000..=0xBFFF => {
                if self.ram_enable {
                    let bank = (self.ram_bank & self.ram_bank_mask) as usize * 0x2000;
                    let offset = (address - 0xA000) as usize;
                    self.ram[bank as usize + offset] = value;
                }
            }
            _ => unreachable!("Unreachable MBC5 write address: {:#06X}", address),
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

impl Mbc5 {
    pub fn new(rom: rom::Rom, backup: Option<Vec<u8>>) -> Self {
        let ram = match backup {
            Some(data) => data,
            None => vec![0; rom.ram_size()],
        };

        let rom_bank_num = rom.rom_size() / 0x4000;
        let ram_bank_num = rom.ram_size() / 0x2000;

        let rom_bank_mask = rom_bank_num.saturating_sub(1) as u16;
        let ram_bank_mask = ram_bank_num.saturating_sub(1) as u8;

        Self {
            rom,
            ram,
            ram_enable: false,
            rom_bank: 1,
            ram_bank: 0,
            rom_bank_mask,
            ram_bank_mask,
        }
    }
}
