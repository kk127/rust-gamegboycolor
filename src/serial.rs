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
    receive_buf: Option<u8>,
    send_buf: Option<u8>,
    tick_timer: u16,
    sc: Sc,
    link_cable: Option<Box<dyn LinkCable>>,

    // For debugging
    rev_count: u16,
    send_count: u16,
    panic_counter: u16,
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

    pub fn write(&mut self, address: u16, value: u8, context: &impl Context) {
        match address {
            0xFF01 => {
                println!("FF01 write: {:#04X}", value);
                self.buf = value;
            }
            0xFF02 => {
                let prev_is_transfer = self.sc.transfer_requested_or_progress();
                println!("FF02 write: {:#04X}", value);
                self.sc = Sc::from_bytes([value]);
                if self.sc.transfer_requested_or_progress() && !prev_is_transfer {
                    self.send_buf = Some(self.buf);
                    self.tick_timer = 128 * 8;
                }
            }
            _ => unreachable!("Unreachable Serial write address: {:#06X}", address),
        }
        println!("----Status----");
        println!("SC: {:#04X}", self.sc.bytes[0]);
        println!("buf: {:#04X}", self.buf);
        println!("send_buf: {:?}", self.send_buf);
        println!("recv_buf: {:?}", self.receive_buf);
        println!("---------------");
    }

    pub fn tick(&mut self, context: &mut impl Context) {
        if !self.sc.transfer_requested_or_progress() || self.link_cable.is_none() {
            return;
        }

        let link_cable = self.link_cable.as_mut().unwrap();
        match self.sc.clock_select() {
            ClockSelect::External => {
                let recv_val = link_cable.try_recv();
                if recv_val.is_some() && self.send_buf.is_some() {
                    self.buf = recv_val.unwrap();
                    self.rev_count += 1;
                    let send_val = self.send_buf.take().unwrap();
                    println!("External Serial receive: {:#04X}", recv_val.unwrap());
                    link_cable.send(send_val);
                    self.send_count += 1;

                    self.sc.set_transfer_requested_or_progress(false);
                    context.set_interrupt_serial(true);
                    println!("******************panic_counter: {}", self.panic_counter);
                    self.panic_counter += 1;
                }
            }
            ClockSelect::Internal => {
                if let Some(send_val) = self.send_buf.take() {
                    println!("Internal Serial send: {:#04X}", send_val);
                    link_cable.send(send_val);
                }

                if let Some(recv_val) = link_cable.try_recv().take() {
                    self.send_count += 1;
                    self.buf = recv_val;
                    self.sc.set_transfer_requested_or_progress(false);
                    context.set_interrupt_serial(true);
                    println!("******************panic_counter: {}", self.panic_counter);
                    self.panic_counter += 1;
                }
            }
        }
    }

    fn get_tick_counter(&self, context: &impl Context) -> u8 {
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
    transfer_requested_or_progress: bool,
}

#[derive(BitfieldSpecifier, Debug, Default, PartialEq, Eq)]
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
