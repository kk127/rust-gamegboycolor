use crate::config::Speed;
use crate::context;
use crate::DeviceMode;
use log::{debug, warn};

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
    line_info: Vec<Option<PixelInfo>>,

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
    window_line_counter: u8,

    bg_color_palette: ColorPalette,
    obj_color_palette: ColorPalette,

    scan_line_obj_x: Vec<u8>,

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
        let line_info = vec![None; 160];
        Self {
            vram,
            oam,
            frame_buffer,
            line_info,

            scan_line_obj_x: vec![u8::MAX; 160],

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
                    0xFF
                } else {
                    self.bg_palette.into()
                }
            }
            0xFF48 | 0xFF49 => {
                if context.device_mode() == DeviceMode::GameBoyColor {
                    warn!("Attempted to read from FF48 or FF49 in CGB mode");
                    0xFF
                } else {
                    self.obj_palette[(address - 0xFF48) as usize].into()
                }
            }
            0xFF4A => self.window_y,
            0xFF4B => self.window_x,
            0xFF4F => {
                if context.device_mode() == DeviceMode::GameBoyColor {
                    0b1111_1110 | self.vram_bank
                } else {
                    warn!("Attempted to read from FF4F in DMG mode");
                    0xFF
                }
            }
            // BG Color Palette
            0xFF68 | 0xFF69 => self.bg_color_palette.read(address - 0xFF68),
            // OBJ Color Palette
            0xFF6A | 0xFF6B => self.obj_color_palette.read(address - 0xFF6A),
            _ => unreachable!("Unreachable PPU read address: {:#06X}", address),
        }
    }

    pub fn write(&mut self, context: &mut impl Context, address: u16, value: u8) {
        debug!("PPU write: {:#06X} = {:#04X}", address, value);
        match address {
            0x8000..=0x9FFF => self.vram[(address - 0x8000) as usize] = value,
            0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize] = value,
            0xFF40 => {
                let new_lcdc = Lcdc::from(value);
                if !self.lcdc.lcd_enable() && new_lcdc.lcd_enable() {
                    self.lx = 0;
                    self.ly = 0;
                    self.frame += 1;
                }
                self.lcdc = new_lcdc;
            }
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
            0xFF4F => {
                if context.device_mode() == DeviceMode::GameBoyColor {
                    self.vram_bank = value & 0x01;
                } else {
                    warn!("Attempted to write to FF4F in DMG mode");
                }
            }
            // BG Color Palette
            0xFF68 | 0xFF69 => self.bg_color_palette.write(address - 0xFF68, value),
            // OBJ Color Palette
            0xFF6A | 0xFF6B => self.obj_color_palette.write(address - 0xFF6A, value),
            // _ => unreachable!("Unreachable PPU write address: {:#06X}", address),
            _ => warn!("Invalid PPU write address: {:#06X}", address),
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
        debug!(
            "Frame: {}, LX: {}, LY: {}, Mode: {:?}",
            self.frame, self.lx, self.ly, self.mode
        );

        self.update_lx_ly();

        if !self.lcdc.lcd_enable() {
            self.mode = PpuMode::HBlank;
            return;
        }

        self.update_mode(context);
        self.update_interrupt(context);
    }

    pub fn ppu_mode(&self) -> PpuMode {
        self.mode
    }

    fn update_lx_ly(&mut self) {
        self.lx += 1;
        if self.lx == 456 {
            self.lx = 0;
            self.ly += 1;
            if self.ly == 154 {
                self.ly = 0;
                self.frame += 1;
            }
        }
    }

    fn update_mode(&mut self, context: &mut impl Context) {
        if (0..144).contains(&self.ly) {
            if self.lx < 80 {
                self.set_mode(PpuMode::OamSearch, context);
            } else if self.lx < 252 {
                self.set_mode(PpuMode::DataTransfer, context);
            } else {
                self.set_mode(PpuMode::HBlank, context);
            }
        } else {
            self.set_mode(PpuMode::VBlank, context);
        }
    }

    fn set_mode(&mut self, mode: PpuMode, context: &mut impl Context) {
        if self.mode != mode {
            if mode == PpuMode::VBlank {
                context.set_interrupt_vblank(true);
            } else if mode == PpuMode::DataTransfer {
                self.render_scanline();
            }
        }

        self.mode = mode;
    }

    fn render_scanline(&mut self) {
        self.render_background();
        if self.lcdc.obj_enable() {
            self.render_obj();
        }

        for x in 0..160 {
            let pixel_index = (self.ly as usize) * 160 + x as usize;
            if self.line_info[x as usize].is_none() {
                self.frame_buffer[pixel_index] = 0xFF;
                continue;
            }

            let pixel_info = self.line_info[x as usize].unwrap();

            let color = match pixel_info.layer {
                Layer::Bg_Win => match pixel_info.color_id {
                    0 => self.bg_palette.ID0(),
                    1 => self.bg_palette.ID1(),
                    2 => self.bg_palette.ID2(),
                    3 => self.bg_palette.ID3(),
                    _ => unreachable!("Invalid pixel data id: {}", pixel_info.color_id),
                },
                Layer::Obj_0 => match pixel_info.color_id {
                    0 => self.obj_palette[0].ID0(),
                    1 => self.obj_palette[0].ID1(),
                    2 => self.obj_palette[0].ID2(),
                    3 => self.obj_palette[0].ID3(),
                    _ => unreachable!("Invalid pixel data id: {}", pixel_info.color_id),
                },
                Layer::Obj_1 => match pixel_info.color_id {
                    0 => self.obj_palette[1].ID0(),
                    1 => self.obj_palette[1].ID1(),
                    2 => self.obj_palette[1].ID2(),
                    3 => self.obj_palette[1].ID3(),
                    _ => unreachable!("Invalid pixel data id: {}", pixel_info.color_id),
                },
            };

            self.frame_buffer[pixel_index] = match color {
                Color::White => 0xFF,
                Color::LightGray => 0xAA,
                Color::DarkGray => 0x55,
                Color::Black => 0x00,
            };
        }
    }

    fn render_background(&mut self) {
        let is_in_window_y = self.window_y <= self.ly;
        if self.ly == self.window_y {
            self.window_line_counter = 0;
        }
        let mut increment_window_line_counter = false;
        for x in 0..160 {
            if !self.lcdc.bg_and_window_enable() {
                continue;
            }

            let is_in_window_x = self.window_x <= x + 7;
            let render_window = self.lcdc.window_enable() && is_in_window_y && is_in_window_x;

            // println!("x: {}, y: {}, render_window: {}", x, self.ly, render_window);

            let (tile_map_x, tile_map_y, tile_map_base_address) = if render_window {
                let window_x = x + 7 - self.window_x;
                let window_y = self.window_line_counter;
                increment_window_line_counter = true;
                let tile_map_base_address = if self.lcdc.window_tile_map_display_select() {
                    0x1C00
                } else {
                    0x1800
                };
                (window_x as usize, window_y as usize, tile_map_base_address)
            } else {
                let screen_x = self.scx.wrapping_add(x);
                let screen_y = self.scy.wrapping_add(self.ly);
                let tile_map_base_address = if self.lcdc.bg_tile_map_display_select() {
                    0x1C00
                } else {
                    0x1800
                };
                (screen_x as usize, screen_y as usize, tile_map_base_address)
            };

            let tile_x = tile_map_x / 8;
            let tile_y = tile_map_y / 8;
            let pixel_x = tile_map_x % 8;
            let pixel_y = tile_map_y % 8;

            let tile_number = tile_x + tile_y * 32;
            let tile_map_address = tile_map_base_address + tile_number;
            let tile_index = self.vram[tile_map_address] as usize;
            let tile_address = match self.lcdc.bg_window_tile_data_select() {
                true => tile_index * 16,
                false => (0x1000_i16).wrapping_add((tile_index as i8 as i16) * 16) as usize,
            };

            let pixel_address = tile_address + pixel_y * 2;
            let pixel_data_low = (self.vram[pixel_address] >> (7 - pixel_x)) & 1;
            let pixel_data_high = (self.vram[pixel_address + 1] >> (7 - pixel_x)) & 1;
            let pixel_data_id = (pixel_data_high << 1) | pixel_data_low;

            self.line_info[x as usize] = Some(PixelInfo {
                layer: Layer::Bg_Win,
                color_id: pixel_data_id,
            });
        }
        if increment_window_line_counter {
            self.window_line_counter += 1;
        }
    }

    fn render_obj(&mut self) {
        let mut scanline_obj_count = 0;
        for i in 0..40 {
            let obj_attr_address = i * 4;
            let obj_attr = ObjAttr::from_bytes(
                self.oam[obj_attr_address..obj_attr_address + 4]
                    .try_into()
                    .unwrap(),
            );

            let obj_y_length = if self.lcdc.obj_size() == ObjSize::EightBySixteen {
                16
            } else {
                8
            };

            let upper_y = obj_attr.y().wrapping_sub(16);
            if !(upper_y..(upper_y.wrapping_add(obj_y_length))).contains(&self.ly) {
                continue;
            }

            scanline_obj_count += 1;
            if scanline_obj_count > 10 {
                break;
            }

            let offset_y = self.ly.wrapping_sub(obj_attr.y().wrapping_sub(16));
            for offset_x in 0..8 {
                let screen_x = obj_attr.x().wrapping_sub(8).wrapping_add(offset_x);

                if screen_x >= 160 {
                    continue;
                }

                if let Some(pixel_info) = self.line_info[screen_x as usize] {
                    if obj_attr.bg_window_priority_is_high() && pixel_info.color_id != 0 {
                        continue;
                    }
                }

                if obj_attr.x() >= self.scan_line_obj_x[screen_x as usize] {
                    continue;
                }

                let pixel_x = if obj_attr.x_flip() {
                    7 - offset_x
                } else {
                    offset_x
                };

                let pixel_y = if obj_attr.y_flip() {
                    obj_y_length - 1 - offset_y
                } else {
                    offset_y
                };

                let tile_address = if self.lcdc.obj_size() == ObjSize::EightBySixteen {
                    (obj_attr.tile_number() & 0xFE) as usize * 16
                } else {
                    obj_attr.tile_number() as usize * 16
                };

                // println!("Tile address: {:#06X}", tile_address);
                let pixel_address = tile_address + pixel_y as usize * 2;
                let pixel_data_low = (self.vram[pixel_address] >> (7 - pixel_x)) & 1;
                let pixel_data_high = (self.vram[pixel_address + 1] >> (7 - pixel_x)) & 1;
                let pixel_data_id = (pixel_data_high << 1) | pixel_data_low;

                if pixel_data_id == 0 {
                    continue;
                }

                let layer = match obj_attr.dmg_palette_number() {
                    0 => Layer::Obj_0,
                    1 => Layer::Obj_1,
                    _ => unreachable!(
                        "Invalid DMG palette number: {}",
                        obj_attr.dmg_palette_number()
                    ),
                };

                self.line_info[screen_x as usize] = Some(PixelInfo {
                    layer,
                    color_id: pixel_data_id,
                });
            }
        }
    }

    fn update_interrupt(&mut self, context: &mut impl Context) {
        let mut cur_interrupt = match self.mode {
            PpuMode::HBlank => self.stat.hblank_interrupt(),
            PpuMode::VBlank => self.stat.vblank_interrupt(),
            PpuMode::OamSearch => self.stat.oam_interrupt(),
            PpuMode::DataTransfer => false,
        };
        cur_interrupt |= self.stat.lyc_ly_coincidence_interrupt() && (self.ly == self.lyc);

        if !self.prev_interrupt && cur_interrupt {
            debug!("Ppu Stat interrupt");
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

#[derive(BitfieldSpecifier, Debug, Clone, Copy, Default, PartialEq, Eq)]
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

#[derive(BitfieldSpecifier, Debug, Clone, Copy, Default, Eq, PartialEq)]
#[bits = 2]
pub enum PpuMode {
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

#[bitfield(bits = 32)]
#[derive(Debug, Clone, Copy, Default)]
struct ObjAttr {
    y: u8,
    x: u8,
    tile_number: u8,
    cgb_palette_number: B3,
    cgb_bank: B1,
    dmg_palette_number: B1,
    x_flip: bool,
    y_flip: bool,
    bg_window_priority_is_high: bool,
}

#[derive(Debug, Clone, Copy)]
struct PixelInfo {
    layer: Layer,
    color_id: u8,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Layer {
    Bg_Win,
    Obj_0,
    Obj_1,
}

#[derive(Debug)]
struct ColorPalette {
    color_palette: Vec<u8>,
    color_palette_index: u8,
    enable_palette_index_auto_increment: bool,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            color_palette: vec![0; 64],
            color_palette_index: 0,
            enable_palette_index_auto_increment: false,
        }
    }
}

impl ColorPalette {
    fn read(&self, offset: u16) -> u8 {
        match offset {
            0 => (self.enable_palette_index_auto_increment as u8) << 7 | self.color_palette_index,
            1 => self.color_palette[self.color_palette_index as usize],
            _ => unreachable!("Invalid color palette offset: {:#06X}", offset),
        }
    }

    fn write(&mut self, offset: u16, value: u8) {
        match offset {
            0 => {
                self.color_palette_index = value & 0x3F;
                self.enable_palette_index_auto_increment = value & 0x80 == 0x80;
            }
            1 => {
                self.color_palette[self.color_palette_index as usize] = value;
                if self.enable_palette_index_auto_increment {
                    self.color_palette_index = (self.color_palette_index + 1) % 64;
                }
            }
            _ => unreachable!("Invalid color palette offset: {:#06X}", offset),
        }
    }
}
