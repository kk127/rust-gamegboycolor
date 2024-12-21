use log::{debug, warn};

use crate::config::Config;
use crate::{context, ppu, DeviceMode};

trait Context:
    context::Cartridge
    + context::Ppu
    + context::Apu
    + context::Config
    + context::Interrupt
    + context::Joypad
    + context::Timer
    + context::Serial
{
}
impl<T> Context for T where
    T: context::Cartridge
        + context::Ppu
        + context::Apu
        + context::Config
        + context::Interrupt
        + context::Joypad
        + context::Timer
        + context::Serial
{
}

#[derive(Debug)]
pub struct Bus {
    wram: Vec<u8>,
    wram_bank: u8,
    hram: [u8; 0x7F],

    dma: Dma,
    hdma: Hdma,

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

            dma: Dma::default(),
            hdma: Hdma::default(),

            ff72: 0,
            ff73: 0,
            ff74: 0,
            ff75: 0,
        }
    }

    pub fn read(&mut self, context: &mut impl Context, address: u16) -> u8 {
        let data = match address {
            0x0000..=0x7FFF => context.cartridge_read(address),
            0x8000..=0x9FFF => context.ppu_read(address),
            0xA000..=0xBFFF => context.cartridge_read(address),
            0xC000..=0xFDFF => {
                let bank = address & 0x1000;
                self.wram[((address & 0x0FFF) + bank * self.wram_bank as u16) as usize]
            }
            0xFE00..=0xFE9F => context.ppu_read(address),
            0xFEA0..=0xFEFF => {
                warn!("Invalid Bus Address: {:#06X}", address);
                0xFF
            }
            0xFF00 => context.joypad_read(),
            0xFF01..=0xFF02 => context.serial_read(address),
            0xFF04..=0xFF07 => context.timer_read(address),
            0xFF0F => 0xE0 | context.interrupt_flag().into_bytes()[0],
            0xFF10..=0xFF3F => context.apu_read(address),
            0xFF40..=0xFF45 => context.ppu_read(address),
            0xFF46 => self.dma.read(),
            0xFF47..=0xFF4B => context.ppu_read(address),
            0xFF4C => 0xFF, // KEY0
            0xFF4D => {
                if context.device_mode() == DeviceMode::GameBoy {
                    warn!("Read from FF4D in DMG mode");
                    0xFF
                } else {
                    context.get_speed_switch()
                }
            }
            0xFF4F => context.ppu_read(address),
            0xFF50 => {
                warn!("Boot ROM");
                0xFF
            }
            0xFF51..=0xFF55 => {
                if context.device_mode() == DeviceMode::GameBoy {
                    warn!("Read from HDMA register in DMG mode");
                    0xFF
                } else {
                    self.hdma.read(address)
                }
            }
            0xFF68..=0xFF6B => context.ppu_read(address),
            0xFF70 => {
                if context.device_mode() == DeviceMode::GameBoyColor {
                    0xF8 | self.wram_bank
                } else {
                    warn!("Read from FF70 in DMG mode");
                    0xFF
                }
            }
            0xFF72 => {
                if context.device_mode() == DeviceMode::GameBoy {
                    warn!("Read CGB Undocumented Register : FF72");
                    0xFF
                } else {
                    self.ff72
                }
            }
            0xFF73 => {
                if context.device_mode() == DeviceMode::GameBoy {
                    warn!("Read CGB Undocumented Register: FF73");
                    0xFF
                } else {
                    self.ff73
                }
            }
            0xFF74 => {
                if context.device_mode() == DeviceMode::GameBoy {
                    warn!("Read CGB Undocumented Register: FF74");
                    0xFF
                } else {
                    self.ff74
                }
            }
            0xFF75 => {
                if context.device_mode() == DeviceMode::GameBoy {
                    warn!("Read CGB Undocumented Register: FF75");
                    0xFF
                } else {
                    self.ff75
                }
            }
            0xFF76..=0xFF7F => context.apu_read(address),
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize],
            0xFFFF => context.interrupt_enable().into_bytes()[0],
            _ => {
                // warn!("Invalid Bus Address: {:#06X}", address);
                println!("Invalid Bus Address: {:#06X}", address);
                0x00
            }
        };
        debug!("Bus read: {:#06X} = {:#04X}", address, data);
        // println!("Bus read: {:#06X} = {:#04X}", address, data);
        data
    }

    pub fn write(&mut self, context: &mut impl Context, address: u16, value: u8) {
        debug!("Bus write: {:#06X} = {:#04X}", address, value);
        match address {
            0x0000..=0x7FFF => context.cartridge_write(address, value),
            0x8000..=0x9FFF => context.ppu_write(address, value),
            0xA000..=0xBFFF => context.cartridge_write(address, value),
            0xC000..=0xFDFF => {
                let bank = address & 0x1000;
                let wram_address = (address & 0x0FFF) + bank * self.wram_bank as u16;
                self.wram[wram_address as usize] = value;
            }
            0xFE00..=0xFE9F => {
                context.ppu_write(address, value);
            }
            0xFEA0..=0xFEFF => {
                warn!("Invalid Bus Address: {:#06X}", address);
            }
            0xFF00 => context.joypad_write(value),
            0xFF01..=0xFF02 => context.serial_write(address, value),
            0xFF04..=0xFF07 => context.timer_write(address, value),
            0xFF0F => context.set_interrupt_flag(value),
            0xFF10..=0xFF3F => context.apu_write(address, value),
            0xFF40..=0xFF45 => context.ppu_write(address, value),
            0xFF46 => self.dma.write(value),
            0xFF47..=0xFF4B => context.ppu_write(address, value),
            0xFF4D => {
                if context.device_mode() == DeviceMode::GameBoy {
                    warn!("Write to FF4D in DMG mode");
                }
                context.set_speed_switch(value);
            }
            0xFF4F => context.ppu_write(address, value),
            0xFF50 => warn!("Boot ROM not implemented"),
            0xFF51..=0xFF55 => {
                if context.device_mode() == DeviceMode::GameBoy {
                    warn!("Write to HDMA register in DMG mode");
                } else {
                    self.hdma.write(address, value);
                }
            }
            // 0xFF56 => {
            //     if context.device_mode() == DeviceMode::GameBoy {
            //         warn!("Write to FF56 in DMG mode");
            //     } else {
            //         todo!("Write to FF56 in CGB mode");
            //     }
            // }
            0xFF68..=0xFF6C => context.ppu_write(address, value),
            0xFF70 => {
                if context.device_mode() == DeviceMode::GameBoyColor {
                    self.wram_bank = (value & 0x07).max(1);
                } else {
                    warn!("Write to FF70 in DMG mode");
                }
            }
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
            0xFF76..=0xFF7F => context.apu_write(address, value),
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize] = value,
            0xFFFF => {
                context.set_interrupt_enable(value);
                debug!(
                    "IE Set After: {:#04X}",
                    context.interrupt_enable().into_bytes()[0]
                );
                debug!("IE Set After: {:?}", context.interrupt_enable());
            }
            _ => warn!("Invalid Bus Address: {:#06X}", address),
        }
    }

    pub fn tick(&mut self, context: &mut impl Context) {
        self.process_dma(context);
        self.process_hdma(context);
    }

    fn process_dma(&mut self, context: &mut impl Context) {
        if !self.dma.enable {
            return;
        }

        let source_address = (self.dma.upper_source_address as u16) << 8 | self.dma.counter as u16;
        let destination_address = 0xFE00 + self.dma.counter as u16;
        let data = self.read(context, source_address);
        debug!(
            "DMA Source: {:#04X} -> {:#04X}: {:#04X}",
            source_address, destination_address, data
        );
        self.write(context, destination_address, data);

        self.dma.counter = self.dma.counter.wrapping_add(1);
        if self.dma.counter == 0xA0 {
            self.dma.enable = false;
        }
    }

    fn process_hdma(&mut self, context: &mut impl Context) {
        assert!(!(self.hdma.enable_gdma && self.hdma.enable_hdma));

        let is_hblank = context.ppu_mode() == ppu::PpuMode::HBlank;
        let enter_hblank = is_hblank && !self.hdma.is_prev_hblank;
        self.hdma.is_prev_hblank = is_hblank;

        if self.hdma.enable_gdma || (self.hdma.enable_hdma && enter_hblank) {
            println!("HDMA: {:#?}", self.hdma);
            for i in 0..16 {
                let source_address = self.hdma.source_address + i;
                let destination_address = 0x8000 | (self.hdma.destination_address + i);
                let value = self.read(context, source_address);
                self.write(context, destination_address, value);
            }

            self.hdma.source_address = self.hdma.source_address.wrapping_add(16);
            self.hdma.destination_address = self.hdma.destination_address.wrapping_add(16);

            let (length, ovf) = self.hdma.length.overflowing_sub(1);
            self.hdma.length = length;
            if ovf || self.hdma.destination_address >= 0x2000 {
                self.hdma.enable_gdma = false;
                self.hdma.enable_hdma = false;
                self.hdma.destination_address &= 0x1FFF;
            }
        }
    }
}

#[derive(Debug, Default)]
struct Dma {
    upper_source_address: u8,
    counter: u8,
    enable: bool,
}

impl Dma {
    fn write(&mut self, value: u8) {
        self.upper_source_address = value;
        self.counter = 0;
        self.enable = true;
    }

    fn read(&self) -> u8 {
        self.upper_source_address
    }
}

#[derive(Debug, Default)]
struct Hdma {
    source_address: u16,
    destination_address: u16,
    length: u8,
    enable_gdma: bool,
    enable_hdma: bool,
    is_prev_hblank: bool,
}

impl Hdma {
    fn read(&self, address: u16) -> u8 {
        match address {
            0xFF51..=0xFF54 => {
                warn!("Load Invalid HDMA register: {:#06X}", address);
                0xFF
            }
            0xFF55 => (!self.enable_hdma as u8) << 7 | self.length,
            _ => unreachable!("Invalid HDMA register: {:#06X}", address),
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            0xFF51 => self.source_address = (value as u16) << 8 | (self.source_address & 0x00FF),
            0xFF52 => self.source_address = (self.source_address & 0xFF00) | (value & 0xF0) as u16,
            0xFF53 => {
                self.destination_address =
                    ((value & 0x1F) as u16) << 8 | (self.destination_address & 0x00FF)
            }
            0xFF54 => {
                self.destination_address =
                    (self.destination_address & 0xFF00) | (value & 0xF0) as u16
            }
            0xFF55 => {
                if self.enable_hdma {
                    self.enable_hdma = false;
                } else if (value >> 7) & 0x01 == 1 {
                    self.enable_hdma = true;
                    self.length = value & 0x7F;
                } else {
                    self.enable_gdma = true;
                    self.length = value & 0x7F;
                }
            }
            _ => unreachable!("Invalid HDMA register: {:#06X}", address),
        }
    }
}
