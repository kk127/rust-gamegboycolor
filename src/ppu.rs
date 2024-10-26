use crate::config::Speed;
use crate::context;
use crate::DeviceMode;
use log::{debug, warn};
use std::io::Write;

use modular_bitfield::bitfield;
use modular_bitfield::prelude::*;

trait Context: context::Interrupt + context::Config {}
impl<T> Context for T where T: context::Interrupt + context::Config {}

#[derive(Default)]
pub struct Ppu {
    vram: Vec<u8>,
    vram_bank: u8,
    oam: Vec<u8>,
    frame_buffer: Vec<u8>,

    lx: u16,
    mode: PpuMode,
    prev_interrupt: bool,

    lcdc: Lcdc,                // FF40
    stat: Stat,                // FF41
    scy: u8,                   // FF42
    scx: u8,                   // FF43
    ly: u8,                    // FF44
    lyc: u8,                   // FF45
    bg_palette: Palette,       // FF47 Non-CGB Mode Only
    obj_palette: [Palette; 2], // FF48, FF49 Non-CGB Mode Only
    window_y: u8,              // FF4A
    window_x: u8,              // FF4B

    frame: u64,
}

impl Ppu {
    pub fn new(device_mode: DeviceMode) -> Self {
        let vram = match device_mode {
            DeviceMode::GameBoy => vec![0; 0x2000],
            DeviceMode::GameBoyColor => vec![0; 0x4000],
        };
        let oam = vec![0; 0xA0];
        let frame_buffer = vec![0; 160 * 144];
        Self {
            vram,
            oam,
            frame_buffer,

            ..Default::default()
        }
    }

    pub fn read(&mut self, context: &mut impl Context, address: u16) -> u8 {
        match address {
            0x8000..=0x9FFF => self.vram[(address - 0x8000) as usize],
            0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize],
            0xFF40 => self.lcdc.into(),
            0xFF41 => {
                self.stat.set_lyc_ly_coincidence(self.ly == self.lyc);
                self.stat.into()
            }
            0xFF42 => self.scy,
            0xFF43 => self.scx,
            0xFF44 => self.ly,
            0xFF45 => self.lyc,
            // FF46 DMA transfer
            0xFF47 => {
                if context.device_mode() == DeviceMode::GameBoyColor {
                    warn!("Attempted to read from FF47 in CGB mode");
                }
                self.bg_palette.into()
            }
            0xFF48 | 0xFF49 => {
                if context.device_mode() == DeviceMode::GameBoyColor {
                    warn!("Attempted to read from FF48 or FF49 in CGB mode");
                }
                self.obj_palette[(address - 0xFF48) as usize].into()
            }
            0xFF4A => self.window_y,
            0xFF4B => self.window_x,
            _ => unreachable!("Unreachable PPU read address: {:#06X}", address),
        }
    }

    pub fn write(&mut self, context: &mut impl Context, address: u16, value: u8) {
        debug!("PPU write: {:#06X} = {:#04X}", address, value);
        match address {
            0x8000..=0x9FFF => self.vram[(address - 0x8000) as usize] = value,
            0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize] = value,
            0xFF40 => self.lcdc = Lcdc::from(value),
            0xFF41 => self.stat = Stat::from(value & 0b0111_1100),
            0xFF42 => self.scy = value,
            0xFF43 => self.scx = value,
            // ly 0xFF44 is read only
            0xFF45 => self.lyc = value,
            // FF46 DMA transfer
            0xFF47 => {
                if context.device_mode() == DeviceMode::GameBoyColor {
                    warn!("Attempted to write to FF47 in CGB mode");
                }
                self.bg_palette = Palette::from(value);
            }
            0xFF48 | 0xFF49 => {
                if context.device_mode() == DeviceMode::GameBoyColor {
                    warn!("Attempted to write to FF48 or FF49 in CGB mode");
                }
                self.obj_palette[(address - 0xFF48) as usize] = Palette::from(value);
            }
            0xFF4A => self.window_y = value,
            0xFF4B => self.window_x = value,
            _ => unreachable!("Unreachable PPU write address: {:#06X}", address),
        }
    }

    pub fn tick(&mut self, context: &mut impl Context) {
        let tick_count = match context.current_speed() {
            Speed::Normal => 4,
            Speed::Double => 2,
        };
        for _ in 0..tick_count {
            self.tick_pixel(context);
        }
    }

    fn tick_pixel(&mut self, context: &mut impl Context) {
        self.lx += 1;
        if (0..144).contains(&self.ly) {
            if self.lx == 80 {
                self.mode = PpuMode::DataTransfer;
                self.render_scanline();
            } else if self.lx == 252 {
                self.mode = PpuMode::HBlank;
            } else if self.lx == 456 {
                self.lx = 0;
                self.ly += 1;
                if self.ly == 144 {
                    self.mode = PpuMode::VBlank;
                    context.set_intterupt_vblank(true);
                } else {
                    self.mode = PpuMode::OamSearch;
                }
            }
        } else {
            if self.lx == 456 {
                self.lx = 0;
                self.ly += 1;
                if self.ly == 154 {
                    self.ly = 0;
                    self.mode = PpuMode::OamSearch;
                    self.frame += 1;

                    // log/vram.logにvramの内容を出力
                    // let path = "./log/vram.log";
                    // let mut file = std::fs::File::create(path).unwrap();
                    // file.write_all(&self.vram).unwrap();

                    // println!("ly: {}, scx: {}, scy: {}", self.ly, self.scx, self.scy);
                    // println!(
                    //     "lcdc: {:#04X}, stat: {:#04X}",
                    //     self.lcdc.bytes[0], self.stat.bytes[0],
                    // );
                    // println!("bg_palette: {:#04X}", self.bg_palette.bytes[0]);
                }
            }
        }
        self.update_interrupt(context);
    }

    fn render_scanline(&mut self) {
        self.render_background();
    }

    fn render_background(&mut self) {
        for x in 0..160 {
            let screen_x = self.scx.wrapping_add(x) as usize;
            let screen_y = self.scy.wrapping_add(self.ly) as usize;
            let tile_x = screen_x / 8;
            let tile_y = screen_y / 8;
            let pixel_x = screen_x % 8;
            let pixel_y = screen_y % 8;

            let tile_number = tile_x + tile_y * 32;
            let tile_map_address_base = if self.lcdc.bg_tile_map_display_select() {
                0x1C00
            } else {
                0x1800
            };

            println!("x: {}, y: {}, tile_number: {}", x, self.ly, tile_number);
            println!("tile_x: {}, tile_y: {}", tile_x, tile_y);
            println!("tile_map_address_base: {:#06X}", tile_map_address_base);
            let tile_map_address = tile_map_address_base + tile_number;
            println!("tile_map_address: {:#06X}", tile_map_address);

            let tile_index = self.vram[tile_map_address] as usize;
            let tile_address = match self.lcdc.bg_window_tile_data_select() {
                true => tile_index * 16,
                false => (0x1000_i16).wrapping_add((tile_index as i8 as i16) * 16) as usize,
            };
            println!(
                "bg_window_tile_data_select: {}",
                self.lcdc.bg_window_tile_data_select()
            );
            println!(
                "tile_index: {:#04X}, tile_address: {:#06X}",
                tile_index, tile_address
            );

            let pixel_address = tile_address + pixel_y * 2;
            let pixel_data_low = (self.vram[pixel_address] >> (7 - pixel_x)) & 1;
            let pixel_data_high = (self.vram[pixel_address + 1] >> (7 - pixel_x)) & 1;
            let pixel_data_id = (pixel_data_high << 1) | pixel_data_low;

            let color = match pixel_data_id {
                0 => self.bg_palette.ID0(),
                1 => self.bg_palette.ID1(),
                2 => self.bg_palette.ID2(),
                3 => self.bg_palette.ID3(),
                _ => unreachable!("Invalid pixel data id: {}", pixel_data_id),
            };

            let pixel_index = (self.ly as usize) * 160 + x as usize;
            self.frame_buffer[pixel_index] = match color {
                Color::White => 0xFF,
                Color::LightGray => 0xAA,
                Color::DarkGray => 0x55,
                Color::Black => 0x00,
            };
            println!(
                "frame_buffer[{}]: {:#04X}",
                pixel_index, self.frame_buffer[pixel_index]
            );
        }
    }

    fn update_interrupt(&mut self, context: &mut impl Context) {
        let cur_interrupt = match self.mode {
            PpuMode::HBlank => self.stat.hblank_interrupt(),
            PpuMode::VBlank => self.stat.vblank_interrupt(),
            PpuMode::OamSearch => self.stat.oam_interrupt(),
            PpuMode::DataTransfer => false,
        };

        if !self.prev_interrupt && cur_interrupt {
            context.set_interrupt_lcd(true);
        }
        self.prev_interrupt = cur_interrupt;
    }

    pub fn frame_buffer(&self) -> &[u8] {
        &self.frame_buffer
    }

    pub fn frame(&self) -> u64 {
        self.frame
    }
}

#[bitfield(bits = 8)]
#[repr(u8)]
#[derive(Debug, Clone, Copy, Default)]
struct Lcdc {
    bg_and_window_enable: bool,
    obj_enable: bool,
    obj_size: ObjSize,
    bg_tile_map_display_select: bool,
    bg_window_tile_data_select: bool,
    window_enable: bool,
    window_tile_map_display_select: bool,
    lcd_enable: bool,
}

#[derive(BitfieldSpecifier, Debug, Clone, Copy, Default)]
#[bits = 1]
enum ObjSize {
    #[default]
    EightByEight = 0,
    EightBySixteen = 1,
}

#[bitfield(bits = 8)]
#[repr(u8)]
#[derive(Debug, Clone, Copy, Default)]
struct Stat {
    ppu_mode: PpuMode,
    lyc_ly_coincidence: bool,
    hblank_interrupt: bool,
    vblank_interrupt: bool,
    oam_interrupt: bool,
    lyc_ly_coincidence_interrupt: bool,
    #[skip]
    __: B1,
}

#[derive(BitfieldSpecifier, Debug, Clone, Copy, Default)]
#[bits = 2]
enum PpuMode {
    HBlank = 0,
    VBlank = 1,
    #[default]
    OamSearch = 2,
    DataTransfer = 3,
}

#[bitfield(bits = 8)]
#[repr(u8)]
#[derive(Debug, Clone, Copy, Default)]
struct Palette {
    ID0: Color,
    ID1: Color,
    ID2: Color,
    ID3: Color,
}

#[derive(BitfieldSpecifier, Debug, Clone, Copy, Default)]
#[bits = 2]
enum Color {
    #[default]
    White = 0,
    LightGray = 1,
    DarkGray = 2,
    Black = 3,
}
