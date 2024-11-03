use crate::config::{DeviceMode, Speed};
use crate::context;
use crate::interface::LinkCable;
use log::debug;

use modular_bitfield::bitfield;
use modular_bitfield::prelude::*;

trait Context: context::Interrupt + context::Config {}
impl<T> Context for T where T: context::Interrupt + context::Config {}

#[derive(Default)]
pub struct Serial {
    buf: u8,
    recv_buf: Option<u8>,
    tick_counter: u8,
    transfer_pos: u8,
    sc: Sc,
    link_cable: Option<Box<dyn LinkCable>>,
}

impl Serial {
    pub fn new(link_cable: Option<Box<dyn LinkCable>>) -> Self {
        Self {
            link_cable,
            ..Default::default()
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        match address {
            0xFF01 => self.buf,
            0xFF02 => self.sc.into(),
            _ => unreachable!("Unreachable Serial read address: {:#06X}", address),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0xFF01 => self.buf = value,
            0xFF02 => {
                self.sc = Sc::from(value);
                if self.sc.transfer_requestted_or_progress() {
                    if let Some(link_cable) = &mut self.link_cable {
                        link_cable.send(self.buf);
                    }

                    self.tick_counter = 0;
                    self.transfer_pos = 0;
                    self.recv_buf = None;
                }
            }
            _ => unreachable!("Unreachable Serial write address: {:#06X}", address),
        }
    }

    pub fn tick(&mut self, context: &mut impl Context) {
        debug!("Serial tick");
        if !self.sc.transfer_requestted_or_progress() {
            return;
        }

        if let Some(link_cable) = &mut self.link_cable {
            if self.recv_buf.is_none() {
                self.recv_buf = link_cable.try_recv();
            }
            // self.recv_buf = link_cable.try_recv();
        }

        let mut transfer_complete = false;

        match self.sc.clock_select() {
            ClockSelect::External => {
                if self.recv_buf.is_some() {
                    transfer_complete = true;
                }
            }
            ClockSelect::Internal => {
                self.tick_counter += 1;
                // let tick_threshold = self.get_tick_threashold(context);
                // if self.tick_counter >= tick_threshold {
                if self.tick_counter >= 128 {
                    self.tick_counter = 0;
                    self.transfer_pos += 1;
                    if self.transfer_pos >= 8 {
                        self.transfer_pos = 0;
                        transfer_complete = true;
                    }
                }
            }
        }

        if transfer_complete {
            self.buf = self.recv_buf.unwrap_or(0xFF);
            self.recv_buf = None;
            self.transfer_pos = 0;
            self.sc.set_transfer_requestted_or_progress(false);
            context.set_interrupt_serial(true);
        }
    }

    fn get_tick_threashold(&self, context: &impl Context) -> u8 {
        match context.device_mode() {
            DeviceMode::GameBoy => 128,
            DeviceMode::GameBoyColor => match (self.sc.clock_speed(), context.current_speed()) {
                (ClockSpeed::Normal, Speed::Normal) => 128,
                (ClockSpeed::Normal, Speed::Double) => 64,
                (ClockSpeed::Double, Speed::Normal) => 4,
                (ClockSpeed::Double, Speed::Double) => 2,
            },
        }
    }
}

#[bitfield(bits = 8)]
#[repr(u8)]
#[derive(Debug, Clone, Copy, Default)]
struct Sc {
    clock_select: ClockSelect,
    clock_speed: ClockSpeed,
    #[skip]
    __: B5,
    transfer_requestted_or_progress: bool,
}

#[derive(BitfieldSpecifier, Debug, Default)]
#[bits = 1]
enum ClockSelect {
    #[default]
    External = 0,
    Internal = 1,
}

#[derive(BitfieldSpecifier, Debug, Default)]
#[bits = 1]
enum ClockSpeed {
    #[default]
    Normal = 0,
    Double = 1,
}
