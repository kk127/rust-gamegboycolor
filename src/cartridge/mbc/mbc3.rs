use crate::cartridge::{rom, Mbc};
use chrono::{DateTime, Datelike, Timelike, Utc};
use log::warn;

pub struct Mbc3 {
    rom: rom::Rom,
    rom_bank: u8,
    rom_bank_mask: u8,
    ram: Vec<u8>,
    ram_bank_mask: u8,
    ram_rtc_enable: bool,
    rtc_register_select: RegisterSelect,
    prev_latch_data: u8,
    clock: DateTime<Utc>,
    carry_day: bool,
}

impl Mbc for Mbc3 {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => self.rom.data()[address as usize],
            0x4000..=0x7FFF => {
                let bank = (self.rom_bank & self.rom_bank_mask) as usize * 0x4000;
                let offset = (address - 0x4000) as usize;
                self.rom.data()[bank + offset]
            }
            0xA000..=0xBFFF => {
                if self.ram_rtc_enable {
                    match self.rtc_register_select {
                        RegisterSelect::RamBank(bank) => {
                            let bank = (bank & self.ram_bank_mask) as usize * 0x2000;
                            let offset = (address - 0xA000) as usize;
                            self.ram[bank + offset]
                        }
                        RegisterSelect::Rtc(reg) => match reg {
                            0x08 => self.clock.second() as u8,
                            0x09 => self.clock.minute() as u8,
                            0x0A => self.clock.hour() as u8,
                            0x0B => self.clock.day() as u8,
                            0x0C => {
                                let day = (self.clock.day() >> 8) as u8;
                                let carry_day = self.carry_day as u8;
                                carry_day << 7 | day
                            }
                            _ => 0,
                        },
                    }
                } else {
                    0xFF
                }
            }
            _ => unreachable!("Unreachable MBC3 read address: {:#06X}", address),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x1FFF => self.ram_rtc_enable = value & 0x0F == 0x0A,
            0x2000..=0x3FFF => self.rom_bank = (value & 0x7F).max(1),
            0x4000..=0x5FFF => match value {
                0x00..=0x03 => self.rtc_register_select = RegisterSelect::RamBank(value),
                0x08..=0x0C => self.rtc_register_select = RegisterSelect::Rtc(value),
                _ => warn!("Invalid RTC register select: {:#04X}", value),
            },
            0x6000..=0x7FFF => {
                if self.prev_latch_data == 0x00 && value == 0x01 {
                    self.clock = Utc::now();
                    let prev_day = self.clock.day() & 0x1FF;
                    let now_day = self.clock.day() & 0x1FF;
                    self.carry_day = prev_day > now_day;
                }
                self.prev_latch_data = value;
            }
            0xA000..=0xBFFF => {
                if self.ram_rtc_enable {
                    match self.rtc_register_select {
                        RegisterSelect::RamBank(bank) => {
                            let bank = (bank & self.ram_bank_mask) as usize * 0x2000;
                            let offset = (address - 0xA000) as usize;
                            self.ram[bank + offset] = value;
                        }
                        RegisterSelect::Rtc(_) => {
                            warn!("Invalid RTC write address: {:#06X}", address)
                        }
                    }
                }
            }

            _ => unreachable!("Unreachable MBC3 write address: {:#06X}", address),
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

impl Mbc3 {
    pub fn new(rom: rom::Rom, backup: Option<Vec<u8>>) -> Self {
        let rom_bank_num = rom.rom_size() / 0x4000;
        let ram_bank_num = rom.ram_size() / 0x2000;
        let rom_bank_mask = rom_bank_num.saturating_sub(1) as u8;
        let ram_bank_mask = ram_bank_num.saturating_sub(1) as u8;

        let ram = match backup {
            Some(data) => data,
            None => vec![0; rom.ram_size()],
        };

        Self {
            rom,
            rom_bank: 1,
            rom_bank_mask,
            ram,
            ram_bank: 0,
            ram_bank_mask,
            ram_rtc_enable: false,
            rtc_register_select: RegisterSelect::RamBank(0),
            prev_latch_data: 0,
            clock: Utc::now(),
            carry_day: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum RegisterSelect {
    RamBank(u8),
    Rtc(u8),
}
