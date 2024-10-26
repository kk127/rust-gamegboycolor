use log::warn;

use crate::config::Config;
use crate::{context, DeviceMode};

trait Context:
    context::Cartridge + context::Ppu + context::Apu + context::Config + context::Interrupt
{
}
impl<T> Context for T where
    T: context::Cartridge + context::Ppu + context::Apu + context::Config + context::Interrupt
{
}

#[derive(Debug)]
pub struct Bus {
    wram: Vec<u8>,
    wram_bank: u8,
    hram: [u8; 0x7F],

    dma_enable: bool,

    // CGB undocumented registers
    ff72: u8,
    ff73: u8,
    ff74: u8,
    ff75: u8,
}

impl Bus {
    pub fn new(device_mode: DeviceMode) -> Self {
        let wram = match device_mode {
            DeviceMode::GameBoy => vec![0; 0x2000],
            DeviceMode::GameBoyColor => vec![0; 0x8000],
        };
        Self {
            wram,
            wram_bank: 1,
            hram: [0; 0x7F],

            dma_enable: false,

            ff72: 0,
            ff73: 0,
            ff74: 0,
            ff75: 0,
        }
    }

    pub fn read(&mut self, context: &mut impl Context, address: u16) -> u8 {
        match address {
            0x0000..=0x7FFF => context.cartridge_read(address),
            0x8000..=0x9FFF => context.ppu_read(address),
            0xA000..=0xBFFF => context.cartridge_read(address),
            0xC000..=0xFDFF => {
                let bank = address & 0x1000;
                self.wram[((address & 0x0FFF) + bank * self.wram_bank as u16) as usize]
            }
            0xFE00..=0xFE9F => {
                if self.dma_enable {
                    0xFF
                } else {
                    context.ppu_read(address)
                }
            }
            0xFEA0..=0xFEFF => {
                warn!("Invalid Bus Address: {:#06X}", address);
                0xFF
            }
            0xFF00 => todo!("Joypad"),
            0xFF01..=0xFF02 => todo!("Serial"),
            0xFF04..=0xFF07 => todo!("Timer"),
            0xFF0F => context.interrupt_flag().into_bytes()[0],
            0xFF10..=0xFF3F => todo!("APU"),
            0xFF40..=0xFF4B => context.ppu_read(address),
            0xFF4D => {
                if context.device_mode() == DeviceMode::GameBoy {
                    warn!("Read from FF4D in DMG mode");
                }
                context.get_speed_switch()
            }
            0xFF4F => context.ppu_read(address),
            0xFF50 => todo!("Boot ROM"),
            0xFF51..=0xFF55 => todo!("HDMA"),
            0xFF68..=0xFF6B => context.ppu_read(address),
            0xFF70 => self.wram_bank,
            0xFF72 => {
                if context.device_mode() == DeviceMode::GameBoy {
                    warn!("Read CGB Undocumented Register : FF72");
                }
                self.ff72
            }
            0xFF73 => {
                if context.device_mode() == DeviceMode::GameBoy {
                    warn!("Read CGB Undocumented Register: FF73");
                }
                self.ff73
            }
            0xFF74 => {
                if context.device_mode() == DeviceMode::GameBoy {
                    warn!("Read CGB Undocumented Register: FF74");
                }
                self.ff74
            }
            0xFF75 => {
                if context.device_mode() == DeviceMode::GameBoy {
                    warn!("Read CGB Undocumented Register: FF75");
                }
                self.ff75
            }
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize],
            0xFFFF => context.interrupt_enable().into_bytes()[0],
            _ => unreachable!("Invalid Bus Address: {:#06X}", address),
        }
    }

    pub fn write(&mut self, context: &mut impl Context, address: u16, value: u8) {
        match address {
            0x0000..=0x7FFF => context.cartridge_write(address, value),
            0x8000..=0x9FFF => context.ppu_write(address, value),
            0xA000..=0xBFFF => context.cartridge_write(address, value),
            0xC000..=0xFDFF => {
                let bank = address & 0x1000;
                let wram_address = (address & 0x0FFF) + bank * self.wram_bank as u16;
                // self.wram[((address & 0x0FFF) + bank * self.wram_bank as u16) as usize] = value;
                println!("Write to WRAM: {:#06X} = {:#04X}", wram_address, value);
                self.wram[wram_address as usize] = value;
            }
            0xFE00..=0xFE9F => {
                if !self.dma_enable {
                    context.ppu_write(address, value);
                }
            }
            0xFEA0..=0xFEFF => {
                warn!("Invalid Bus Address: {:#06X}", address);
            }
            0xFF00 => todo!("Joypad"),
            0xFF01..=0xFF02 => todo!("Serial"),
            0xFF04..=0xFF07 => todo!("Timer"),
            0xFF0F => context.interrupt_flag().set_byte(value),
            0xFF10..=0xFF3F => todo!("APU"),
            0xFF40..=0xFF4B => context.ppu_write(address, value),
            0xFF4D => {
                if context.device_mode() == DeviceMode::GameBoy {
                    warn!("Write to FF4D in DMG mode");
                }
                context.set_speed_switch(value);
            }
            0xFF4F => context.ppu_write(address, value),
            0xFF50 => todo!("Boot ROM"),
            0xFF51..=0xFF55 => todo!("HDMA"),
            0xFF56 => todo!("RP"),
            0xFF68..=0xFF6C => context.ppu_write(address, value),
            0xFF70 => self.wram_bank = (value & 0x07).max(1),
            0xFF72 => {
                if context.device_mode() == DeviceMode::GameBoy {
                    warn!("Write CGB Undocumented Register: FF72");
                }
                self.ff72 = value;
            }
            0xFF73 => {
                if context.device_mode() == DeviceMode::GameBoy {
                    warn!("Write CGB Undocumented Register: FF73");
                }
                self.ff73 = value;
            }
            0xFF74 => {
                if context.device_mode() == DeviceMode::GameBoy {
                    warn!("Write CGB Undocumented Register: FF74");
                } else {
                    self.ff74 = value;
                }
            }
            0xFF75 => {
                if context.device_mode() == DeviceMode::GameBoy {
                    warn!("Write CGB Undocumented Register: FF75");
                } else {
                    self.ff75 = value & 0x70;
                }
            }
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize] = value,
            0xFFFF => context.interrupt_enable().set_byte(value),
            _ => unreachable!("Invalid Bus Address: {:#06X}", address),
        }
    }

    pub fn tick(&mut self, context: &mut impl Context) {
        warn!("Bus tick not implemented");
    }
}
