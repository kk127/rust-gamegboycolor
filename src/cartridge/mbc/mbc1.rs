use crate::cartridge::{rom, Mbc};

pub struct Mbc1 {
    rom: rom::Rom,
    ram: Vec<u8>,
    ram_enable: bool,
    rom_bank: u8,
    ram_bank_or_upper_rom_bank: u8,
    rom_bank_mask: u8,
    ram_bank_mask: u8,
    banking_mode: bool,
}

impl Mbc for Mbc1 {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => {
                let rom_bank = if self.banking_mode {
                    ((self.ram_bank_or_upper_rom_bank << 5) & self.rom_bank_mask) as usize
                } else {
                    0
                };
                self.rom.data()[rom_bank * 0x4000 + address as usize]
            }
            0x4000..=0x7FFF => {
                let rom_bank = ((self.ram_bank_or_upper_rom_bank << 5 | self.rom_bank)
                    & self.rom_bank_mask) as usize;
                self.rom.data()[rom_bank * 0x4000 + (address & 0x3FFF) as usize]
            }
            0xA000..=0xBFFF => {
                if self.ram_enable {
                    let ram_bank = if self.banking_mode {
                        (self.ram_bank_or_upper_rom_bank & self.ram_bank_mask) as usize
                    } else {
                        0
                    };
                    self.ram[ram_bank * 0x2000 + (address & 0x1FFF) as usize]
                } else {
                    0xFF
                }
            }
            _ => unreachable!("Unreachable MBC1 read address: {:#06X}", address),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x1FFF => self.ram_enable = (value & 0x0F) == 0x0A,
            0x2000..=0x3FFF => self.rom_bank = (value & 0x1F).max(1),
            0x4000..=0x5FFF => self.ram_bank_or_upper_rom_bank = value & 0x03,
            0x6000..=0x7FFF => self.banking_mode = value & 0x01 == 0x01,
            0xA000..=0xBFFF => {
                if self.ram_enable {
                    let ram_bank = if self.banking_mode {
                        (self.ram_bank_or_upper_rom_bank & self.ram_bank_mask) as usize
                    } else {
                        0
                    };
                    self.ram[ram_bank * 0x2000 + (address & 0x1FFF) as usize] = value;
                }
            }
            _ => unreachable!("Unreachable MBC1 write address: {:#06X}", address),
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

impl Mbc1 {
    pub fn new(rom: rom::Rom, backup: Option<Vec<u8>>) -> Self {
        let ram = match backup {
            Some(data) => data,
            None => vec![0; rom.ram_size()],
        };

        let rom_bank_mask = (rom.rom_size() / 0x4000).saturating_sub(1) as u8;
        let ram_bank_mask = (rom.ram_size() / 0x2000).saturating_sub(1) as u8;

        // println!(
        //     "ROM: size: {}, banks: {}, mask: {:b}",
        //     rom.rom_size(),
        //     rom.rom_size() / 0x4000,
        //     rom_bank_mask
        // );
        // println!(
        //     "RAM: size: {}, banks: {}, mask: {:b}",
        //     rom.ram_size(),
        //     rom.ram_size() / 0x2000,
        //     ram_bank_mask
        // );

        Self {
            rom,
            ram,
            ram_enable: false,
            rom_bank: 1,
            ram_bank_or_upper_rom_bank: 1,
            rom_bank_mask,
            ram_bank_mask,
            banking_mode: false,
        }
    }
}
