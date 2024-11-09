use modular_bitfield::bitfield;
use modular_bitfield::prelude::*;

pub struct Config {
    device_mode: DeviceMode,
    speed_switch: PrepareSpeedSwitch,
}

impl Config {
    pub fn new(device_mode: DeviceMode) -> Self {
        let speed_switch = PrepareSpeedSwitch::default();
        Self {
            device_mode,
            speed_switch,
        }
    }

    pub fn device_mode(&self) -> DeviceMode {
        self.device_mode
    }

    pub fn set_speed_switch(&mut self, value: u8) {
        self.speed_switch = PrepareSpeedSwitch::from(value & 0x01);
    }

    pub fn get_speed_switch(&self) -> u8 {
        let mut ret = self.speed_switch.into();
        ret |= 0b0111_1110;
        ret
    }

    pub fn current_speed(&self) -> Speed {
        self.speed_switch.speed()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DeviceMode {
    GameBoy,
    GameBoyColor,
}

#[bitfield(bits = 8)]
#[repr(u8)]
#[derive(Debug, Clone, Copy, Default)]
struct PrepareSpeedSwitch {
    armed: bool,
    #[skip]
    __: B6,
    speed: Speed,
}

#[derive(BitfieldSpecifier, Debug, Clone, Copy, Default, Eq, PartialEq)]
#[bits = 1]
pub enum Speed {
    #[default]
    Normal = 0,
    Double = 1,
}
