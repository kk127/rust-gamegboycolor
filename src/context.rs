use core::error;

use crate::cartridge::rom::{self, CgbFlag};
use crate::config::DeviceMode;
use crate::interface::LinkCable;
use crate::joypad::JoypadKeyState;
use crate::utils;
use crate::{apu, bus, cartridge, config, cpu, interrupt, joypad, ppu, serial, timer};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmulatorError {
    #[error("Unsupported mode: {0}")]
    UnsupportedMode(String),

    #[error("Error loading save data: {0}")]
    SaveDataError(#[from] std::io::Error),
}

pub struct Context {
    cpu: cpu::Cpu,
    inner1: Inner1,

    rom_name: String,
}

impl Context {
    pub fn new(
        data: &[u8],
        device_mode: DeviceMode,
        link_cable: Option<Box<dyn LinkCable>>,
    ) -> Result<Self, EmulatorError> {
        let rom = rom::Rom::new(data).unwrap();
        if rom.cgb_flag() == CgbFlag::CgbOnly && device_mode == DeviceMode::GameBoy {
            return Err(EmulatorError::UnsupportedMode(
                "GameBoy Color only game cannot be run in GameBoy mode".to_string(),
            ));
        }

        let rom_name = rom.title().to_string();
        let backup = utils::load_save_data(&rom_name)?;

        let cartridge = cartridge::Cartridge::new(rom, backup);
        Ok(Self {
            cpu: cpu::Cpu::new(device_mode),
            inner1: Inner1 {
                bus: bus::Bus::new(device_mode),
                inner2: Inner2 {
                    cartridge,
                    ppu: ppu::Ppu::new(device_mode),
                    apu: apu::Apu::new(),
                    joypad: joypad::Joypad::new(),
                    timer: timer::Timer::new(),
                    serial: serial::Serial::new(link_cable),
                    inner3: Inner3 {
                        interrupt: interrupt::Interrupt::new(),
                        config: config::Config::new(device_mode),
                    },
                },
            },
            rom_name,
        })
    }

    pub fn execute_instruction(&mut self) {
        self.cpu.execute_instruction(&mut self.inner1);
    }

    pub fn execute_frame(&mut self) {
        let frame = self.inner1.frame();
        while self.inner1.frame() == frame {
            self.execute_instruction();
        }
    }

    pub fn set_key(&mut self, key_state: JoypadKeyState) {
        self.inner1.inner2.set_key(key_state);
    }

    pub fn frame_buffer(&self) -> &[(u8, u8, u8)] {
        self.inner1.frame_buffer()
    }

    pub fn save_data(&self) -> Option<Vec<u8>> {
        self.inner1.save_data()
    }

    pub fn rom_name(&self) -> &str {
        &self.rom_name
    }

    pub fn get_audio_buffer(&self) -> &Vec<[i16; 2]> {
        self.inner1.inner2.apu.get_audio_buffer()
    }

    pub fn clear_audio_buffer(&mut self) {
        self.inner1.inner2.apu.clear_audio_buffer();
    }
}

pub trait Bus {
    fn read(&mut self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);

    fn tick(&mut self);
}

pub trait Cartridge {
    fn cartridge_read(&self, address: u16) -> u8;
    fn cartridge_write(&mut self, address: u16, value: u8);

    fn save_data(&self) -> Option<Vec<u8>>;
}

pub trait Ppu {
    fn ppu_read(&mut self, address: u16) -> u8;
    fn ppu_write(&mut self, address: u16, value: u8);

    fn ppu_tick(&mut self);
    fn frame_buffer(&self) -> &[(u8, u8, u8)];
    fn frame(&self) -> u64;
    fn ppu_mode(&self) -> ppu::PpuMode;
}

pub trait Apu {
    fn apu_read(&mut self, address: u16) -> u8;
    fn apu_write(&mut self, address: u16, value: u8);

    fn apu_tick(&mut self);
    fn audio_buffer(&self) -> &Vec<[i16; 2]>;
    fn clear_audio_buffer(&mut self);
}

pub trait Timer {
    fn timer_read(&self, address: u16) -> u8;
    fn timer_write(&mut self, address: u16, value: u8);

    fn timer_tick(&mut self);
}

pub trait Joypad {
    fn joypad_read(&self) -> u8;
    fn joypad_write(&mut self, value: u8);
    fn set_key(&mut self, key_state: JoypadKeyState);
}

pub trait Serial {
    fn serial_read(&self, address: u16) -> u8;
    fn serial_write(&mut self, address: u16, value: u8);
    fn serial_tick(&mut self);
}

pub trait Interrupt {
    fn interrupt_enable(&self) -> interrupt::InterruptEnable;
    fn interrupt_flag(&self) -> interrupt::InterruptFlag;

    fn set_interrupt_enable(&mut self, value: u8);
    fn set_interrupt_flag(&mut self, value: u8);

    fn set_interrupt_vblank(&mut self, value: bool);
    fn set_interrupt_lcd(&mut self, value: bool);
    fn set_interrupt_timer(&mut self, value: bool);
    fn set_interrupt_serial(&mut self, value: bool);
    fn set_interrupt_joypad(&mut self, value: bool);
}

pub trait Config {
    fn device_mode(&self) -> DeviceMode;

    fn set_speed_switch(&mut self, value: u8);
    fn get_speed_switch(&self) -> u8;
    fn current_speed(&self) -> config::Speed;
}

struct Inner1 {
    bus: bus::Bus,
    inner2: Inner2,
}

impl Bus for Inner1 {
    fn read(&mut self, address: u16) -> u8 {
        self.bus.read(&mut self.inner2, address)
    }

    fn write(&mut self, address: u16, value: u8) {
        self.bus.write(&mut self.inner2, address, value);
    }

    fn tick(&mut self) {
        self.bus.tick(&mut self.inner2);
        self.inner2.ppu_tick();
        self.inner2.apu_tick();
        self.inner2.timer_tick();
        self.inner2.serial_tick();
    }
}

impl Cartridge for Inner1 {
    fn cartridge_read(&self, address: u16) -> u8 {
        self.inner2.cartridge_read(address)
    }

    fn cartridge_write(&mut self, address: u16, value: u8) {
        self.inner2.cartridge_write(address, value);
    }

    fn save_data(&self) -> Option<Vec<u8>> {
        self.inner2.save_data()
    }
}

impl Ppu for Inner1 {
    fn ppu_read(&mut self, address: u16) -> u8 {
        self.inner2.ppu_read(address)
    }

    fn ppu_write(&mut self, address: u16, value: u8) {
        self.inner2.ppu_write(address, value);
    }

    fn ppu_tick(&mut self) {
        self.inner2.ppu_tick();
    }

    fn frame_buffer(&self) -> &[(u8, u8, u8)] {
        self.inner2.frame_buffer()
    }

    fn frame(&self) -> u64 {
        self.inner2.frame()
    }

    fn ppu_mode(&self) -> ppu::PpuMode {
        self.inner2.ppu_mode()
    }
}

impl Apu for Inner1 {
    fn apu_read(&mut self, address: u16) -> u8 {
        self.inner2.apu_read(address)
    }

    fn apu_write(&mut self, address: u16, value: u8) {
        self.inner2.apu_write(address, value);
    }

    fn apu_tick(&mut self) {
        self.inner2.apu_tick();
    }

    fn audio_buffer(&self) -> &Vec<[i16; 2]> {
        self.inner2.audio_buffer()
    }

    fn clear_audio_buffer(&mut self) {
        self.inner2.clear_audio_buffer();
    }
}

impl Interrupt for Inner1 {
    fn interrupt_flag(&self) -> interrupt::InterruptFlag {
        self.inner2.interrupt_flag()
    }

    fn interrupt_enable(&self) -> interrupt::InterruptEnable {
        self.inner2.interrupt_enable()
    }

    fn set_interrupt_enable(&mut self, value: u8) {
        self.inner2.set_interrupt_enable(value);
    }

    fn set_interrupt_flag(&mut self, value: u8) {
        self.inner2.set_interrupt_flag(value);
    }

    fn set_interrupt_vblank(&mut self, value: bool) {
        self.inner2.set_interrupt_vblank(value);
    }

    fn set_interrupt_lcd(&mut self, value: bool) {
        self.inner2.set_interrupt_lcd(value);
    }

    fn set_interrupt_timer(&mut self, value: bool) {
        self.inner2.set_interrupt_timer(value);
    }

    fn set_interrupt_serial(&mut self, value: bool) {
        self.inner2.set_interrupt_serial(value);
    }

    fn set_interrupt_joypad(&mut self, value: bool) {
        self.inner2.set_interrupt_joypad(value);
    }
}

impl Config for Inner1 {
    fn device_mode(&self) -> DeviceMode {
        self.inner2.device_mode()
    }

    fn set_speed_switch(&mut self, value: u8) {
        self.inner2.set_speed_switch(value);
    }

    fn get_speed_switch(&self) -> u8 {
        self.inner2.get_speed_switch()
    }

    fn current_speed(&self) -> config::Speed {
        self.inner2.current_speed()
    }
}

struct Inner2 {
    cartridge: cartridge::Cartridge,
    ppu: ppu::Ppu,
    apu: apu::Apu,
    joypad: joypad::Joypad,
    timer: timer::Timer,
    serial: serial::Serial,
    inner3: Inner3,
}

impl Cartridge for Inner2 {
    fn cartridge_read(&self, address: u16) -> u8 {
        self.cartridge.read(address)
    }

    fn cartridge_write(&mut self, address: u16, value: u8) {
        self.cartridge.write(address, value);
    }

    fn save_data(&self) -> Option<Vec<u8>> {
        self.cartridge.save_data()
    }
}

impl Ppu for Inner2 {
    fn ppu_read(&mut self, address: u16) -> u8 {
        self.ppu.read(&mut self.inner3, address)
    }

    fn ppu_write(&mut self, address: u16, value: u8) {
        self.ppu.write(&mut self.inner3, address, value);
    }

    fn ppu_tick(&mut self) {
        self.ppu.tick(&mut self.inner3);
    }

    fn frame_buffer(&self) -> &[(u8, u8, u8)] {
        self.ppu.frame_buffer()
    }

    fn frame(&self) -> u64 {
        self.ppu.frame()
    }

    fn ppu_mode(&self) -> ppu::PpuMode {
        self.ppu.ppu_mode()
    }
}

impl Apu for Inner2 {
    fn apu_read(&mut self, address: u16) -> u8 {
        self.apu.read(address)
    }

    fn apu_write(&mut self, address: u16, value: u8) {
        self.apu.write(address, value);
    }

    fn apu_tick(&mut self) {
        self.apu.tick(&self.inner3);
    }

    fn audio_buffer(&self) -> &Vec<[i16; 2]> {
        self.apu.get_audio_buffer()
    }

    fn clear_audio_buffer(&mut self) {
        self.apu.clear_audio_buffer();
    }
}

impl Joypad for Inner2 {
    fn joypad_read(&self) -> u8 {
        self.joypad.read()
    }

    fn joypad_write(&mut self, value: u8) {
        self.joypad.write(value);
    }

    fn set_key(&mut self, key_state: JoypadKeyState) {
        self.joypad.set_key(&mut self.inner3, key_state);
    }
}

impl Timer for Inner2 {
    fn timer_read(&self, address: u16) -> u8 {
        self.timer.read(address)
    }

    fn timer_write(&mut self, address: u16, value: u8) {
        self.timer.write(address, value);
    }

    fn timer_tick(&mut self) {
        self.timer.tick(&mut self.inner3);
    }
}

impl Serial for Inner2 {
    fn serial_read(&self, address: u16) -> u8 {
        self.serial.read(address)
    }

    fn serial_write(&mut self, address: u16, value: u8) {
        self.serial.write(address, value, &mut self.inner3);
    }

    fn serial_tick(&mut self) {
        self.serial.tick(&mut self.inner3);
    }
}

impl Interrupt for Inner2 {
    fn interrupt_flag(&self) -> interrupt::InterruptFlag {
        self.inner3.interrupt_flag()
    }

    fn interrupt_enable(&self) -> interrupt::InterruptEnable {
        self.inner3.interrupt_enable()
    }

    fn set_interrupt_enable(&mut self, value: u8) {
        self.inner3.set_interrupt_enable(value);
    }

    fn set_interrupt_flag(&mut self, value: u8) {
        self.inner3.set_interrupt_flag(value);
    }

    fn set_interrupt_vblank(&mut self, value: bool) {
        self.inner3.set_interrupt_vblank(value);
    }

    fn set_interrupt_lcd(&mut self, value: bool) {
        self.inner3.set_interrupt_lcd(value);
    }

    fn set_interrupt_timer(&mut self, value: bool) {
        self.inner3.set_interrupt_timer(value);
    }

    fn set_interrupt_serial(&mut self, value: bool) {
        self.inner3.set_interrupt_serial(value);
    }

    fn set_interrupt_joypad(&mut self, value: bool) {
        self.inner3.set_interrupt_joypad(value);
    }
}

impl Config for Inner2 {
    fn device_mode(&self) -> DeviceMode {
        self.inner3.device_mode()
    }

    fn set_speed_switch(&mut self, value: u8) {
        self.inner3.set_speed_switch(value);
    }

    fn get_speed_switch(&self) -> u8 {
        self.inner3.get_speed_switch()
    }

    fn current_speed(&self) -> config::Speed {
        self.inner3.current_speed()
    }
}

struct Inner3 {
    interrupt: interrupt::Interrupt,
    config: config::Config,
}

impl Interrupt for Inner3 {
    fn interrupt_flag(&self) -> interrupt::InterruptFlag {
        self.interrupt.interrupt_flag()
    }

    fn interrupt_enable(&self) -> interrupt::InterruptEnable {
        self.interrupt.interrupt_enable()
    }

    fn set_interrupt_enable(&mut self, value: u8) {
        self.interrupt.set_interrupt_enable(value);
    }

    fn set_interrupt_flag(&mut self, value: u8) {
        self.interrupt.set_interrupt_flag(value);
    }

    fn set_interrupt_vblank(&mut self, value: bool) {
        self.interrupt.set_intterupt_vblank(value);
    }

    fn set_interrupt_lcd(&mut self, value: bool) {
        self.interrupt.set_interrupt_lcd(value);
    }

    fn set_interrupt_timer(&mut self, value: bool) {
        self.interrupt.set_interrupt_timer(value);
    }

    fn set_interrupt_serial(&mut self, value: bool) {
        self.interrupt.set_interrupt_serial(value);
    }

    fn set_interrupt_joypad(&mut self, value: bool) {
        self.interrupt.set_interrupt_joypad(value);
    }
}

impl Config for Inner3 {
    fn device_mode(&self) -> DeviceMode {
        self.config.device_mode()
    }

    fn set_speed_switch(&mut self, value: u8) {
        self.config.set_speed_switch(value);
    }

    fn get_speed_switch(&self) -> u8 {
        self.config.get_speed_switch()
    }

    fn current_speed(&self) -> config::Speed {
        self.config.current_speed()
    }
}
