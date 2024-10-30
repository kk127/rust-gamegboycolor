use log::warn;
use modular_bitfield::bitfield;
use modular_bitfield::prelude::*;

pub struct Interrupt {
    interrupt_flag: InterruptFlag,
    interrupt_enable: InterruptEnable,
}

impl Interrupt {
    pub fn new() -> Self {
        Self {
            interrupt_flag: InterruptFlag::new(),
            interrupt_enable: InterruptEnable::new(),
        }
    }

    pub fn interrupt_flag(&self) -> InterruptFlag {
        self.interrupt_flag
    }

    pub fn set_interrupt_enable(&mut self, value: u8) {
        self.interrupt_enable = InterruptEnable::from_bytes([value]);
    }

    pub fn set_interrupt_flag(&mut self, value: u8) {
        self.interrupt_flag = InterruptFlag::from_bytes([value]);
    }

    pub fn interrupt_enable(&self) -> InterruptEnable {
        self.interrupt_enable
    }

    pub fn set_intterupt_vblank(&mut self, flag: bool) {
        self.interrupt_flag.set_vblank(flag);
    }

    pub fn set_interrupt_lcd(&mut self, flag: bool) {
        self.interrupt_flag.set_lcd(flag);
    }

    pub fn set_interrupt_timer(&mut self, flag: bool) {
        self.interrupt_flag.set_timer(flag);
    }

    pub fn set_interrupt_serial(&mut self, flag: bool) {
        self.interrupt_flag.set_serial(flag);
    }

    pub fn set_interrupt_joypad(&mut self, flag: bool) {
        self.interrupt_flag.set_joypad(flag);
    }
}

#[bitfield(bits = 8)]
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
