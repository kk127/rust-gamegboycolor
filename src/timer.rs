use crate::config::Speed;
use crate::context;

trait Context: context::Interrupt + context::Config {}
impl<T> Context for T where T: context::Interrupt + context::Config {}

pub struct Timer {
    div: u16, // 0xFF04: Divider Register (R/W)
    tima: u8, // 0xFF05: Timer Counter (R/W)
    tma: u8,  // 0xFF06: Timer Modulo (R/W)
    tac: u8,  // 0xFF07: Timer Control (R/W)
    div_counter: u16,
    tima_counter: u16,
    tima_enable: bool,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            div_counter: 0,
            tima_counter: 0,
            tima_enable: false,
        }
    }
}

impl Timer {
    pub fn read(&self, address: u16) -> u8 {
        match address {
            0xFF04 => (self.div >> 8) as u8,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => (self.tima_enable as u8) << 2 | self.tac,
            _ => unreachable!("Unreachable Timer read address: {:#06X}", address),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        match address {
            0xFF04 => self.div = 0,
            0xFF05 => self.tima = value,
            0xFF06 => self.tma = value,
            0xFF07 => {
                self.tac = value & 0x03;
                self.tima_enable = (value >> 2) & 0x01 == 1;
            }
            _ => unreachable!("Unreachable Timer write address: {:#06X}", address),
        }
    }

    pub fn tick(&mut self, context: &mut impl Context) {
        self.tick_div();
        self.tick_tima(context);
    }

    fn tick_div(&mut self) {
        self.div_counter += 1;
        if self.div_counter == 64 {
            self.div_counter = 0;
            self.div = self.div.wrapping_add(1);
        }
    }

    fn tick_tima(&mut self, context: &mut impl Context) {
        if !self.tima_enable {
            return;
        }

        let mut tac_threshold = match self.tac & 0x03 {
            0 => 256,
            1 => 4,
            2 => 16,
            3 => 64,
            _ => unreachable!("Unreachable TAC threshold: {:#04X}", self.tac),
        };
        if context.current_speed() == Speed::Double {
            tac_threshold /= 2;
        }

        // self.tima_counter += 1;
        self.tima_counter = self.tima_counter.wrapping_add(1);
        if self.tima_counter == tac_threshold {
            self.tima_counter = 0;

            let (new_tima, overflow) = self.tima.overflowing_add(1);
            self.tima = new_tima;
            if overflow {
                self.tima = self.tma;
                context.set_interrupt_timer(true);
            }
        }
    }
}
