use log::warn;
use modular_bitfield::bitfield;
use modular_bitfield::prelude::*;

pub struct Interrupt {
    interrupt_flag: InterruptFlag,
    interrupt_enable: InterruptEnable,
}

impl Interrupt {
    pub fn new() -> Self {
        warn!("Interrupts not implemented");
        Self {
            interrupt_flag: InterruptFlag::new(),
            interrupt_enable: InterruptEnable::new(),
        }
    }

    pub fn interrupt_flag(&self) -> InterruptFlag {
        self.interrupt_flag
    }

    pub fn interrupt_enable(&self) -> InterruptEnable {
        self.interrupt_enable
    }

    pub fn set_intterupt_vblank(&mut self, value: bool) {
        self.interrupt_flag.set_vblank(value);
    }

    pub fn set_interrupt_lcd(&mut self, value: bool) {
        self.interrupt_flag.set_lcd(value);
    }

    pub fn set_interrupt_timer(&mut self, value: bool) {
        self.interrupt_flag.set_timer(value);
    }

    pub fn set_interrupt_serial(&mut self, value: bool) {
        self.interrupt_flag.set_serial(value);
    }

    pub fn set_interrupt_joypad(&mut self, value: bool) {
        self.interrupt_flag.set_joypad(value);
    }
}

#[bitfield(bits = 8)]
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub struct InterruptFlag {
    vblank: bool,
    lcd: bool,
    timer: bool,
    serial: bool,
    joypad: bool,
    #[skip]
    __: B3,
}

impl InterruptFlag {
    pub fn set_byte(&mut self, value: u8) {
        *self = InterruptFlag::from_bytes([value]);
    }
}

#[bitfield(bits = 8)]
#[derive(Debug, Clone, Copy)]
pub struct InterruptEnable {
    vblank: bool,
    lcd: bool,
    timer: bool,
    serial: bool,
    joypad: bool,
    #[skip]
    __: B3,
}

impl InterruptEnable {
    pub fn set_byte(&mut self, value: u8) {
        *self = InterruptEnable::from_bytes([value]);
    }
}
