use crate::context;
use modular_bitfield::prelude::*;

use log::debug;

trait Context: context::Bus + context::Interrupt {}
impl<T: context::Bus + context::Interrupt> Context for T {}

#[derive(Debug, Default)]
pub struct Cpu {
    registers: Registers,
    ime: bool,
    halt: bool,

    clock: u64,

    // for debugging
    counter: u64,
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            registers: Registers::default(),
            ime: false,
            halt: false,
            clock: 0,

            counter: 0,
        }
    }

    fn tick(&mut self, context: &mut impl Context) {
        self.clock = self.clock.wrapping_add(1);
        context.tick();
    }
}

impl Cpu {
    pub fn execute_instruction(&mut self, context: &mut impl Context) {
        if self.halt {
            let interrupt_flag = context.interrupt_flag().into_bytes()[0];
            let interrupt_enable = context.interrupt_enable().into_bytes()[0];
            if interrupt_flag & interrupt_enable != 0 {
                self.halt = false;
            }
            self.tick(context);
            return;
        }

        let pc = self.registers.pc;
        let opcode = self.fetch_8(context);

        if self.handle_interrupts(context, pc) {
            return;
        }

        match opcode {
            0x00 => self.nop(),
            0x01 => self.ld_r16_imm16(context, opcode),
            0x02 => self.ld_r16mem_a(context, opcode),
            0x03 => self.inc_r16(context, opcode),
            0x04 => self.inc_r8(context, opcode),
            0x05 => self.dec_r8(context, opcode),
            0x06 => self.ld_r8_imm8(context, opcode),
            0x07 => self.rlca(),

            0x08 => self.ld_ind_imm16_sp(context),
            0x09 => self.add_hl_r16(context, opcode),
            0x0A => self.ld_a_r16mem(context, opcode),
            0x0B => self.dec_r16(context, opcode),
            0x0C => self.inc_r8(context, opcode),
            0x0D => self.dec_r8(context, opcode),
            0x0E => self.ld_r8_imm8(context, opcode),
            0x0F => self.rrca(),

            0x10 => self.stop(),
            0x11 => self.ld_r16_imm16(context, opcode),
            0x12 => self.ld_r16mem_a(context, opcode),
            0x13 => self.inc_r16(context, opcode),
            0x14 => self.inc_r8(context, opcode),
            0x15 => self.dec_r8(context, opcode),
            0x16 => self.ld_r8_imm8(context, opcode),
            0x17 => self.rla(),

            0x18 => self.jr_imm8(context),
            0x19 => self.add_hl_r16(context, opcode),
            0x1A => self.ld_a_r16mem(context, opcode),
            0x1B => self.dec_r16(context, opcode),
            0x1C => self.inc_r8(context, opcode),
            0x1D => self.dec_r8(context, opcode),
            0x1E => self.ld_r8_imm8(context, opcode),
            0x1F => self.rra(),

            0x20 => self.jr_cond_imm8(context, opcode),
            0x21 => self.ld_r16_imm16(context, opcode),
            0x22 => self.ld_r16mem_a(context, opcode),
            0x23 => self.inc_r16(context, opcode),
            0x24 => self.inc_r8(context, opcode),
            0x25 => self.dec_r8(context, opcode),
            0x26 => self.ld_r8_imm8(context, opcode),
            0x27 => self.daa(),

            0x28 => self.jr_cond_imm8(context, opcode),
            0x29 => self.add_hl_r16(context, opcode),
            0x2A => self.ld_a_r16mem(context, opcode),
            0x2B => self.dec_r16(context, opcode),
            0x2C => self.inc_r8(context, opcode),
            0x2D => self.dec_r8(context, opcode),
            0x2E => self.ld_r8_imm8(context, opcode),
            0x2F => self.cpl(),

            0x30 => self.jr_cond_imm8(context, opcode),
            0x31 => self.ld_r16_imm16(context, opcode),
            0x32 => self.ld_r16mem_a(context, opcode),
            0x33 => self.inc_r16(context, opcode),
            0x34 => self.inc_r8(context, opcode),
            0x35 => self.dec_r8(context, opcode),
            0x36 => self.ld_r8_imm8(context, opcode),
            0x37 => self.scf(),

            0x38 => self.jr_cond_imm8(context, opcode),
            0x39 => self.add_hl_r16(context, opcode),
            0x3A => self.ld_a_r16mem(context, opcode),
            0x3B => self.dec_r16(context, opcode),
            0x3C => self.inc_r8(context, opcode),
            0x3D => self.dec_r8(context, opcode),
            0x3E => self.ld_r8_imm8(context, opcode),
            0x3F => self.ccf(),

            0x40..=0x7F => self.ld_r8_r8(context, opcode),

            0x80..=0x87 => self.add_a_r8(context, opcode),
            0x88..=0x8F => self.adc_a_r8(context, opcode),
            0x90..=0x97 => self.sub_a_r8(context, opcode),
            0x98..=0x9F => self.sbc_a_r8(context, opcode),
            0xA0..=0xA7 => self.and_a_r8(context, opcode),
            0xA8..=0xAF => self.xor_a_r8(context, opcode),
            0xB0..=0xB7 => self.or_a_r8(context, opcode),
            0xB8..=0xBF => self.cp_a_r8(context, opcode),

            0xC0 => self.ret_cond(context, opcode),
            0xC1 => self.pop_r16stk(context, opcode),
            0xC2 => self.jp_cond_imm16(context, opcode),
            0xC3 => self.jp_imm16(context),
            0xC4 => self.call_cond_imm16(context, opcode),
            0xC5 => self.push_r16stk(context, opcode),
            0xC6 => self.add_a_imm8(context),
            0xC7 => self.rst_tgt3(context, opcode),

            0xC8 => self.ret_cond(context, opcode),
            0xC9 => self.ret(context),
            0xCA => self.jp_cond_imm16(context, opcode),
            0xCB => self.prefix_cb(context),
            0xCC => self.call_cond_imm16(context, opcode),
            0xCD => self.call_imm16(context),
            0xCE => self.adc_a_imm8(context),
            0xCF => self.rst_tgt3(context, opcode),

            0xD0 => self.ret_cond(context, opcode),
            0xD1 => self.pop_r16stk(context, opcode),
            0xD2 => self.jp_cond_imm16(context, opcode),
            // 0xD3: Invalid opcode
            0xD4 => self.call_cond_imm16(context, opcode),
            0xD5 => self.push_r16stk(context, opcode),
            0xD6 => self.sub_a_imm8(context),
            0xD7 => self.rst_tgt3(context, opcode),

            0xD8 => self.ret_cond(context, opcode),
            0xD9 => self.reti(context),
            0xDA => self.jp_cond_imm16(context, opcode),
            // 0xDB: Invalid opcode
            0xDC => self.call_cond_imm16(context, opcode),
            // 0xDD: Invalid opcode
            0xDE => self.sbc_a_imm8(context),
            0xDF => self.rst_tgt3(context, opcode),

            0xE0 => self.ldh_ind_imm8_a(context),
            0xE1 => self.pop_r16stk(context, opcode),
            0xE2 => self.ldh_ind_c_a(context),
            // 0xE3: Invalid opcode
            // 0xE4: Invalid opcode
            0xE5 => self.push_r16stk(context, opcode),
            0xE6 => self.and_a_imm8(context),
            0xE7 => self.rst_tgt3(context, opcode),

            0xE8 => self.add_sp_imm8(context),
            0xE9 => self.jp_hl(),
            0xEA => self.ld_ind_imm16_a(context),
            // 0xEB: Invalid opcode
            // 0xEC: Invalid opcode
            // 0xED: Invalid opcode
            0xEE => self.xor_a_imm8(context),
            0xEF => self.rst_tgt3(context, opcode),

            0xF0 => self.ldh_a_ind_imm8(context),
            0xF1 => self.pop_r16stk(context, opcode),
            0xF2 => self.ldh_a_ind_c(context),
            0xF3 => self.di(),
            // 0xF4: Invalid opcode
            0xF5 => self.push_r16stk(context, opcode),
            0xF6 => self.or_a_imm8(context),
            0xF7 => self.rst_tgt3(context, opcode),

            0xF8 => self.ld_hl_sp_plus_imm8(context),
            0xF9 => self.ld_sp_hl(context),
            0xFA => self.ld_a_ind_imm16(context),
            0xFB => self.ei(),
            // 0xFC: Invalid opcode
            // 0xFD: Invalid opcode
            0xFE => self.cp_a_imm8(context),
            0xFF => self.rst_tgt3(context, opcode),

            _ => unreachable!("Invalid opcode: {:#04x}", opcode),
        }
        debug!("Count: {:4}, Cycle: {}, IME: {}, PC: {:#06X}, opcode: {:#04X}, sp: {:#06X}, a: {:#04X}, b: {:#04X}, c: {:#04X}, d: {:#04X}, e: {:#04X}, h: {:#04X}, l: {:#04X}, {}{}{}{}", self.counter, self.clock, self.ime, self.registers.pc, opcode, self.registers.sp, self.registers.a, self.registers.b, self.registers.c, self.registers.d, self.registers.e, self.registers.h, self.registers.l, 
        if self.registers.f.zero() { "Z" } else { "z" },
        if self.registers.f.subtract() { "N" } else { "n" },
        if self.registers.f.half_carry() { "H" } else { "h" },
        if self.registers.f.carry() { "C" } else { "c" });
        self.counter += 1;
    }

    fn handle_interrupts(&mut self, context: &mut impl Context, pc: u16) -> bool {
        if !self.ime {
            return false;
        }

        let interrupt_flag: u8 = context.interrupt_flag().into_bytes()[0];
        let interrupt_enable: u8 = context.interrupt_enable().into_bytes()[0];
        if interrupt_flag & interrupt_enable == 0 {
            return false;
        }

        let interrupt = (interrupt_flag & interrupt_enable).trailing_zeros();

        self.ime = false;
        self.push_16(pc, context);
        context.set_interrupt_flag(interrupt_flag & !(1 << interrupt));
        self.registers.pc = 0x0040 + interrupt as u16 * 0x08;
        match interrupt {
            0 => context.set_interrupt_vblank(false),
            1 => context.set_interrupt_lcd(false),
            2 => context.set_interrupt_timer(false),
            3 => context.set_interrupt_serial(false),
            4 => context.set_interrupt_joypad(false),
            _ => unreachable!("Invalid interrupt: {}", interrupt),
        }
        self.tick(context);
        self.tick(context);
        self.tick(context);

        debug!("Interrupt Occurred: {}", interrupt);
        debug!(
            "IE: {:#04X}, IF: {:#04X} -> {:#04X}",
            interrupt_enable,
            interrupt_flag,
            context.interrupt_flag().into_bytes()[0]
        );
        debug!("Interrupt PC: {:#06X}", self.registers.pc);

        true
    }

    fn nop(&mut self) {
        // Do nothing
    }

    fn ld_r16_imm16(&mut self, context: &mut impl Context, opcode: u8) {
        let value = self.fetch_16(context);
        let dest = (opcode >> 4) & 0b11;
        match Register16::from(dest) {
            Register16::BC => self.set_bc(value),
            Register16::DE => self.set_de(value),
            Register16::HL => self.set_hl(value),
            Register16::SP => self.registers.sp = value,
        }
    }

    fn ld_r16mem_a(&mut self, context: &mut impl Context, opcode: u8) {
        let dest = (opcode >> 4) & 0b11;
        let address = match Register16Mem::from(dest) {
            Register16Mem::BC => self.get_bc(),
            Register16Mem::DE => self.get_de(),
            Register16Mem::HLPlus => {
                let address = self.get_hl();
                self.set_hl(address.wrapping_add(1));
                address
            }
            Register16Mem::HLMinus => {
                let address = self.get_hl();
                self.set_hl(address.wrapping_sub(1));
                address
            }
        };
        self.write_8(address, self.registers.a, context);
    }

    fn ld_a_r16mem(&mut self, context: &mut impl Context, opcode: u8) {
        let src = (opcode >> 4) & 0b11;
        let address = match Register16Mem::from(src) {
            Register16Mem::BC => self.get_bc(),
            Register16Mem::DE => self.get_de(),
            Register16Mem::HLPlus => {
                let address = self.get_hl();
                self.set_hl(address.wrapping_add(1));
                address
            }
            Register16Mem::HLMinus => {
                let address = self.get_hl();
                self.set_hl(address.wrapping_sub(1));
                address
            }
        };
        self.registers.a = self.read_8(address, context);
    }

    fn ld_ind_imm16_sp(&mut self, context: &mut impl Context) {
        let address = self.fetch_16(context);
        self.write_16(address, self.registers.sp, context);
    }

    fn inc_r16(&mut self, context: &mut impl Context, opcode: u8) {
        let operand = (opcode >> 4) & 0b11;
        match Register16::from(operand) {
            Register16::BC => self.set_bc(self.get_bc().wrapping_add(1)),
            Register16::DE => self.set_de(self.get_de().wrapping_add(1)),
            Register16::HL => self.set_hl(self.get_hl().wrapping_add(1)),
            Register16::SP => self.registers.sp = self.registers.sp.wrapping_add(1),
        }
        self.tick(context);
    }

    fn dec_r16(&mut self, context: &mut impl Context, opcode: u8) {
        let operand = (opcode >> 4) & 0b11;
        match Register16::from(operand) {
            Register16::BC => self.set_bc(self.get_bc().wrapping_sub(1)),
            Register16::DE => self.set_de(self.get_de().wrapping_sub(1)),
            Register16::HL => self.set_hl(self.get_hl().wrapping_sub(1)),
            Register16::SP => self.registers.sp = self.registers.sp.wrapping_sub(1),
        }
        self.tick(context);
    }

    fn add_hl_r16(&mut self, context: &mut impl Context, opcode: u8) {
        let operand = (opcode >> 4) & 0b11;
        let value = match Register16::from(operand) {
            Register16::BC => self.get_bc(),
            Register16::DE => self.get_de(),
            Register16::HL => self.get_hl(),
            Register16::SP => self.registers.sp,
        };

        let (res, carry) = self.get_hl().overflowing_add(value);
        let half_carry = (self.get_hl() & 0x0FFF) + (value & 0x0FFF) > 0x0FFF;
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(half_carry);
        self.registers.f.set_carry(carry);
        self.set_hl(res);

        context.tick();
    }

    fn inc_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register8::from(opcode >> 3 & 0b111);
        let value = self.get_register8(context, register);

        let res = value.wrapping_add(1);
        self.set_register8(context, register, res);

        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry((value & 0xF) + 1 > 0xF);
    }

    fn dec_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register8::from(opcode >> 3 & 0b111);
        let value = self.get_register8(context, register);

        let res = value.wrapping_sub(1);
        self.set_register8(context, register, res);

        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(true);
        self.registers
            .f
            .set_half_carry((value ^ res) & 0x10 == 0x10);
    }

    fn ld_r8_imm8(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register8::from(opcode >> 3 & 0b111);
        let value = self.fetch_8(context);
        self.set_register8(context, register, value);
    }

    fn rlca(&mut self) {
        let carry = self.registers.a & 0x80 == 0x80;
        self.registers.a = self.registers.a.rotate_left(1);
        self.registers.f.set_zero(false);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(false);
        self.registers.f.set_carry(carry);
    }

    fn rrca(&mut self) {
        let carry = self.registers.a & 0x01 == 0x01;
        self.registers.a = self.registers.a.rotate_right(1);
        self.registers.f.set_zero(false);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(false);
        self.registers.f.set_carry(carry);
    }

    fn rla(&mut self) {
        let carry = self.registers.a & 0x80 == 0x80;
        self.registers.a = (self.registers.a << 1) | (self.registers.f.carry() as u8);
        self.registers.f.set_zero(false);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(false);
        self.registers.f.set_carry(carry);
    }

    fn rra(&mut self) {
        let carry = self.registers.a & 0x01 == 0x01;
        self.registers.a = (self.registers.a >> 1) | (self.registers.f.carry() as u8) << 7;
        self.registers.f.set_zero(false);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(false);
        self.registers.f.set_carry(carry);
    }

    fn jr_imm8(&mut self, context: &mut impl Context) {
        let offset = self.fetch_8(context) as i8 as u16;
        let pc = self.registers.pc.wrapping_add(offset);
        self.registers.pc = pc;
        self.tick(context);
    }

    fn jr_cond_imm8(&mut self, context: &mut impl Context, opcode: u8) {
        let condition = Condition::from(opcode >> 3 & 0b11);
        let should_jump = match condition {
            Condition::Nz => !self.registers.f.zero(),
            Condition::Z => self.registers.f.zero(),
            Condition::Nc => !self.registers.f.carry(),
            Condition::C => self.registers.f.carry(),
        };

        let offset = self.fetch_8(context) as i8 as u16;
        if should_jump {
            let pc = self.registers.pc.wrapping_add(offset);
            self.registers.pc = pc;
            self.tick(context);
        }
    }

    fn stop(&mut self) {
        self.halt = true;
    }

    fn ld_r8_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let dest_register = Register8::from(opcode >> 3 & 0b111);
        let src_register = Register8::from(opcode & 0b111);

        match (dest_register, src_register) {
            (Register8::HLIndirect, Register8::HLIndirect) => self.halt(),
            _ => {
                let value = self.get_register8(context, src_register);
                self.set_register8(context, dest_register, value);
            }
        }
    }

    fn halt(&mut self) {
        self.halt = true;
        debug!("Halt");
    }

    fn add_a_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register8::from(opcode & 0b111);
        let value = self.get_register8(context, register);

        let (res, carry) = self.registers.a.overflowing_add(value);
        let half_carry = (self.registers.a & 0x0F) + (value & 0x0F) > 0x0F;
        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(half_carry);
        self.registers.f.set_carry(carry);
        self.registers.a = res;
    }

    fn adc_a_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register8::from(opcode & 0b111);
        let value = self.get_register8(context, register);

        let c = self.registers.f.carry() as u8;
        let (res, carry1) = self.registers.a.overflowing_add(value);
        let (res, carry2) = res.overflowing_add(c);
        let half_carry = (self.registers.a & 0x0F) + (value & 0x0F) + c > 0x0F;
        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(half_carry);
        self.registers.f.set_carry(carry1 || carry2);
        self.registers.a = res;
    }

    fn sub_a_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register8::from(opcode & 0b111);
        let value = self.get_register8(context, register);

        let (res, carry) = self.registers.a.overflowing_sub(value);
        let half_carry = (self.registers.a & 0x0F) < (value & 0x0F);
        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(true);
        self.registers.f.set_half_carry(half_carry);
        self.registers.f.set_carry(carry);
        self.registers.a = res;
    }

    fn sbc_a_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register8::from(opcode & 0b111);
        let value = self.get_register8(context, register);

        let c = self.registers.f.carry() as u8;
        let (res, carry1) = self.registers.a.overflowing_sub(value);
        let (res, carry2) = res.overflowing_sub(c);
        let half_carry = (self.registers.a & 0x0F) < ((value & 0x0F) + c);
        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(true);
        self.registers.f.set_half_carry(half_carry);
        self.registers.f.set_carry(carry1 || carry2);
        self.registers.a = res;
    }

    fn and_a_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register8::from(opcode & 0b111);
        let value = self.get_register8(context, register);

        let res = self.registers.a & value;
        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(true);
        self.registers.f.set_carry(false);
        self.registers.a = res;
    }

    fn xor_a_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register8::from(opcode & 0b111);
        let value = self.get_register8(context, register);

        let res = self.registers.a ^ value;
        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(false);
        self.registers.f.set_carry(false);
        self.registers.a = res;
    }

    fn or_a_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register8::from(opcode & 0b111);
        let value = self.get_register8(context, register);

        let res = self.registers.a | value;
        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(false);
        self.registers.f.set_carry(false);
        self.registers.a = res;
    }

    fn cp_a_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register8::from(opcode & 0b111);
        let value = self.get_register8(context, register);

        let (res, carry) = self.registers.a.overflowing_sub(value);
        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(true);
        self.registers
            .f
            .set_half_carry((self.registers.a & 0x0F) < (value & 0x0F));
        self.registers.f.set_carry(carry);
    }

    fn add_a_imm8(&mut self, context: &mut impl Context) {
        let value = self.fetch_8(context);

        let (res, carry) = self.registers.a.overflowing_add(value);
        let half_carry = (self.registers.a & 0x0F) + (value & 0x0F) > 0x0F;
        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(half_carry);
        self.registers.f.set_carry(carry);
        self.registers.a = res;
    }

    fn sub_a_imm8(&mut self, context: &mut impl Context) {
        let value = self.fetch_8(context);

        let (res, carry) = self.registers.a.overflowing_sub(value);
        let half_carry = (self.registers.a & 0x0F) < (value & 0x0F);
        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(true);
        self.registers.f.set_half_carry(half_carry);
        self.registers.f.set_carry(carry);
        self.registers.a = res;
    }

    fn and_a_imm8(&mut self, context: &mut impl Context) {
        let value = self.fetch_8(context);

        let res = self.registers.a & value;
        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(true);
        self.registers.f.set_carry(false);
        self.registers.a = res;
    }

    fn or_a_imm8(&mut self, context: &mut impl Context) {
        let value = self.fetch_8(context);

        let res = self.registers.a | value;
        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(false);
        self.registers.f.set_carry(false);
        self.registers.a = res;
    }

    fn adc_a_imm8(&mut self, context: &mut impl Context) {
        let value = self.fetch_8(context);

        let c = self.registers.f.carry() as u8;
        let (res, carry1) = self.registers.a.overflowing_add(value);
        let (res, carry2) = res.overflowing_add(c);
        let half_carry = (self.registers.a & 0x0F) + (value & 0x0F) + c > 0x0F;
        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(half_carry);
        self.registers.f.set_carry(carry1 || carry2);
        self.registers.a = res;
    }

    fn sbc_a_imm8(&mut self, context: &mut impl Context) {
        let value = self.fetch_8(context);

        let c = self.registers.f.carry() as u8;
        let (res, carry1) = self.registers.a.overflowing_sub(value);
        let (res, carry2) = res.overflowing_sub(c);
        let half_carry = (self.registers.a & 0x0F) < ((value & 0x0F) + c);
        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(true);
        self.registers.f.set_half_carry(half_carry);
        self.registers.f.set_carry(carry1 || carry2);
        self.registers.a = res;
    }

    fn xor_a_imm8(&mut self, context: &mut impl Context) {
        let value = self.fetch_8(context);

        let res = self.registers.a ^ value;
        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(false);
        self.registers.f.set_carry(false);
        self.registers.a = res;
    }

    fn cp_a_imm8(&mut self, context: &mut impl Context) {
        let value = self.fetch_8(context);

        let (res, carry) = self.registers.a.overflowing_sub(value);
        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(true);
        self.registers
            .f
            .set_half_carry((self.registers.a & 0x0F) < (value & 0x0F));
        self.registers.f.set_carry(carry);
    }

    fn ret_cond(&mut self, context: &mut impl Context, opcode: u8) {
        self.tick(context);
        let condition = Condition::from(opcode >> 3 & 0b11);
        let should_jump = match condition {
            Condition::Nz => !self.registers.f.zero(),
            Condition::Z => self.registers.f.zero(),
            Condition::Nc => !self.registers.f.carry(),
            Condition::C => self.registers.f.carry(),
        };

        if should_jump {
            let address = self.pop_16(context);
            self.registers.pc = address;
            self.tick(context);
        }
    }

    fn ret(&mut self, context: &mut impl Context) {
        let address = self.pop_16(context);
        self.registers.pc = address;
        self.tick(context);
    }

    fn reti(&mut self, context: &mut impl Context) {
        self.ret(context);
        self.ime = true;
    }

    fn jp_cond_imm16(&mut self, context: &mut impl Context, opcode: u8) {
        let condition = Condition::from(opcode >> 3 & 0b11);
        let should_jump = match condition {
            Condition::Nz => !self.registers.f.zero(),
            Condition::Z => self.registers.f.zero(),
            Condition::Nc => !self.registers.f.carry(),
            Condition::C => self.registers.f.carry(),
        };

        let address = self.fetch_16(context);
        if should_jump {
            self.registers.pc = address;
            self.tick(context);
        }
    }

    fn jp_imm16(&mut self, context: &mut impl Context) {
        let address = self.fetch_16(context);
        self.registers.pc = address;
        self.tick(context);
    }

    fn jp_hl(&mut self) {
        self.registers.pc = self.get_hl();
    }

    fn call_imm16(&mut self, context: &mut impl Context) {
        let address = self.fetch_16(context);
        self.push_16(self.registers.pc, context);
        self.registers.pc = address;
        self.tick(context);
    }

    fn call_cond_imm16(&mut self, context: &mut impl Context, opcode: u8) {
        let condition = Condition::from(opcode >> 3 & 0b11);
        let should_jump = match condition {
            Condition::Nz => !self.registers.f.zero(),
            Condition::Z => self.registers.f.zero(),
            Condition::Nc => !self.registers.f.carry(),
            Condition::C => self.registers.f.carry(),
        };

        let address = self.fetch_16(context);
        if should_jump {
            self.push_16(self.registers.pc, context);
            self.registers.pc = address;
            self.tick(context);
        }
    }

    fn rst_tgt3(&mut self, context: &mut impl Context, opcode: u8) {
        let address = (opcode >> 3 & 0b111) as u16 * 0x08;
        self.push_16(self.registers.pc, context);
        self.registers.pc = address;
        self.tick(context);
    }

    fn pop_r16stk(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register16Stk::from(opcode >> 4 & 0b11);
        let value = self.pop_16(context);
        match register {
            Register16Stk::BC => self.set_bc(value),
            Register16Stk::DE => self.set_de(value),
            Register16Stk::HL => self.set_hl(value),
            Register16Stk::AF => self.set_af(value),
        }
    }

    fn push_r16stk(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register16Stk::from(opcode >> 4 & 0b11);
        let value = match register {
            Register16Stk::BC => self.get_bc(),
            Register16Stk::DE => self.get_de(),
            Register16Stk::HL => self.get_hl(),
            Register16Stk::AF => self.get_af(),
        };
        self.push_16(value, context);
        self.tick(context);
    }

    fn ldh_ind_c_a(&mut self, context: &mut impl Context) {
        let address = 0xFF00 + self.registers.c as u16;
        self.write_8(address, self.registers.a, context);
    }

    fn ldh_ind_imm8_a(&mut self, context: &mut impl Context) {
        let address = 0xFF00 + self.fetch_8(context) as u16;
        self.write_8(address, self.registers.a, context);
    }

    fn ld_ind_imm16_a(&mut self, context: &mut impl Context) {
        let address = self.fetch_16(context);
        self.write_8(address, self.registers.a, context);
    }

    fn ldh_a_ind_c(&mut self, context: &mut impl Context) {
        let address = 0xFF00 + self.registers.c as u16;
        self.registers.a = self.read_8(address, context);
    }

    fn ldh_a_ind_imm8(&mut self, context: &mut impl Context) {
        let address = 0xFF00 + self.fetch_8(context) as u16;
        self.registers.a = self.read_8(address, context);
    }

    fn ld_a_ind_imm16(&mut self, context: &mut impl Context) {
        let address = self.fetch_16(context);
        self.registers.a = self.read_8(address, context);
    }

    fn add_sp_imm8(&mut self, context: &mut impl Context) {
        let offset = self.fetch_8(context) as i8 as u16;
        let sp = self.registers.sp;
        let res = sp.wrapping_add(offset);

        self.registers.f.set_zero(false);
        self.registers.f.set_subtract(false);
        self.registers
            .f
            .set_half_carry((sp & 0x0F) + (offset & 0x0F) > 0x0F);
        self.registers
            .f
            .set_carry((sp & 0xFF) + (offset & 0xFF) > 0xFF);
        self.registers.sp = res;
    }

    fn ld_hl_sp_plus_imm8(&mut self, context: &mut impl Context) {
        let offset = self.fetch_8(context) as i8 as u16;
        let sp = self.registers.sp;
        let res = sp.wrapping_add(offset);

        self.registers.f.set_zero(false);
        self.registers.f.set_subtract(false);
        self.registers
            .f
            .set_half_carry((sp & 0x0F) + (offset & 0x0F) > 0x0F);
        self.registers
            .f
            .set_carry((sp & 0xFF) + (offset & 0xFF) > 0xFF);
        self.set_hl(res);
        self.tick(context);
    }

    fn ld_sp_hl(&mut self, context: &mut impl Context) {
        self.registers.sp = self.get_hl();
        self.tick(context);
    }

    fn prefix_cb(&mut self, context: &mut impl Context) {
        let opcode = self.fetch_8(context);
        match opcode {
            0x00..=0x07 => self.rlc_r8(context, opcode),
            0x08..=0x0F => self.rrc_r8(context, opcode),
            0x10..=0x17 => self.rl_r8(context, opcode),
            0x18..=0x1F => self.rr_r8(context, opcode),
            0x20..=0x27 => self.sla_r8(context, opcode),
            0x28..=0x2F => self.sra_r8(context, opcode),
            0x30..=0x37 => self.swap_r8(context, opcode),
            0x38..=0x3F => self.srl_r8(context, opcode),
            0x40..=0x7F => self.bit_u3_r8(context, opcode),
            0x80..=0xBF => self.res_u3_r8(context, opcode),
            0xC0..=0xFF => self.set_u3_r8(context, opcode),
        }
    }

    fn rlc_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register8::from(opcode & 0b111);
        let value = self.get_register8(context, register);

        let carry = value & 0x80 == 0x80;
        let res = value.rotate_left(1);
        self.set_register8(context, register, res);

        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(false);
        self.registers.f.set_carry(carry);
    }

    fn rrc_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register8::from(opcode & 0b111);
        let value = self.get_register8(context, register);

        let carry = value & 0x01 == 0x01;
        let res = value.rotate_right(1);
        self.set_register8(context, register, res);

        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(false);
        self.registers.f.set_carry(carry);
    }

    fn rl_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register8::from(opcode & 0b111);
        let value = self.get_register8(context, register);

        let carry = value & 0x80 == 0x80;
        let res = (value << 1) | self.registers.f.carry() as u8;
        self.set_register8(context, register, res);

        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(false);
        self.registers.f.set_carry(carry);
    }

    fn rr_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register8::from(opcode & 0b111);
        let value = self.get_register8(context, register);

        let carry = value & 0x01 == 0x01;
        let res = (value >> 1) | (self.registers.f.carry() as u8) << 7;
        self.set_register8(context, register, res);

        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(false);
        self.registers.f.set_carry(carry);
    }

    fn sla_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register8::from(opcode & 0b111);
        let value = self.get_register8(context, register);

        let carry = value & 0x80 == 0x80;
        let res = value << 1;
        self.set_register8(context, register, res);

        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(false);
        self.registers.f.set_carry(carry);
    }

    fn sra_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register8::from(opcode & 0b111);
        let value = self.get_register8(context, register);

        let carry = value & 0x01 == 0x01;
        let res = (value >> 1) | (value & 0x80);
        self.set_register8(context, register, res);

        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(false);
        self.registers.f.set_carry(carry);
    }

    fn swap_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register8::from(opcode & 0b111);
        let value = self.get_register8(context, register);

        let res = value.rotate_left(4);
        self.set_register8(context, register, res);

        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(false);
        self.registers.f.set_carry(false);
    }

    fn srl_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let register = Register8::from(opcode & 0b111);
        let value = self.get_register8(context, register);

        let carry = value & 0x01 == 0x01;
        let res = value >> 1;
        self.set_register8(context, register, res);

        self.registers.f.set_zero(res == 0);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(false);
        self.registers.f.set_carry(carry);
    }

    fn bit_u3_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let bit = (opcode >> 3) & 0b111;
        let register = Register8::from(opcode & 0b111);
        let value = self.get_register8(context, register);

        self.registers.f.set_zero(value & (1 << bit) == 0);
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(true);
    }

    fn res_u3_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let bit = (opcode >> 3) & 0b111;
        let register = Register8::from(opcode & 0b111);
        let value = self.get_register8(context, register);

        let res = value & !(1 << bit);
        self.set_register8(context, register, res);
    }

    fn set_u3_r8(&mut self, context: &mut impl Context, opcode: u8) {
        let bit = (opcode >> 3) & 0b111;
        let register = Register8::from(opcode & 0b111);
        let value = self.get_register8(context, register);

        let res = value | (1 << bit);
        self.set_register8(context, register, res);
    }

    fn di(&mut self) {
        self.ime = false;
    }

    fn ei(&mut self) {
        self.ime = true;
    }

    fn daa(&mut self) {
        let mut a = self.registers.a;
        if self.registers.f.subtract() {
            if self.registers.f.carry() {
                a = a.wrapping_sub(0x60);
            }
            if self.registers.f.half_carry() {
                a = a.wrapping_sub(0x06);
            }
        } else {
            if self.registers.f.carry() || a > 0x99 {
                a = a.wrapping_add(0x60);
                self.registers.f.set_carry(true);
            }
            if self.registers.f.half_carry() || (a & 0x0F) > 0x09 {
                a = a.wrapping_add(0x06);
            }
        }
        self.registers.f.set_zero(a == 0);
        self.registers.f.set_half_carry(false);
        self.registers.a = a;
    }

    fn cpl(&mut self) {
        self.registers.a = !self.registers.a;
        self.registers.f.set_subtract(true);
        self.registers.f.set_half_carry(true);
    }

    fn scf(&mut self) {
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(false);
        self.registers.f.set_carry(true);
    }

    fn ccf(&mut self) {
        self.registers.f.set_subtract(false);
        self.registers.f.set_half_carry(false);
        self.registers.f.set_carry(!self.registers.f.carry());
    }

    fn get_register8(&mut self, context: &mut impl Context, register: Register8) -> u8 {
        match register {
            Register8::B => self.registers.b,
            Register8::C => self.registers.c,
            Register8::D => self.registers.d,
            Register8::E => self.registers.e,
            Register8::H => self.registers.h,
            Register8::L => self.registers.l,
            Register8::HLIndirect => {
                let address = self.get_hl();
                self.read_8(address, context)
            }
            Register8::A => self.registers.a,
        }
    }

    fn set_register8(&mut self, context: &mut impl Context, register: Register8, value: u8) {
        match register {
            Register8::B => self.registers.b = value,
            Register8::C => self.registers.c = value,
            Register8::D => self.registers.d = value,
            Register8::E => self.registers.e = value,
            Register8::H => self.registers.h = value,
            Register8::L => self.registers.l = value,
            Register8::HLIndirect => {
                let address = self.get_hl();
                self.write_8(address, value, context);
            }
            Register8::A => self.registers.a = value,
        }
    }
}

#[derive(Debug)]
struct Registers {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    f: Flags,
    pc: u16,
    sp: u16,
}

impl Default for Registers {
    fn default() -> Self {
        // TODO This is initial state DMG after boot ROM execution.
        // This should be configurable.
        Self {
            a: 0x01,
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            f: Flags::new(),
            pc: 0x100,
            sp: 0xFFFE,
        }
    }
}

#[bitfield(bits = 8)]
#[derive(Debug, Default)]
struct Flags {
    #[skip]
    __: B4,
    carry: bool,
    half_carry: bool,
    subtract: bool,
    zero: bool,
}

impl Cpu {
    fn read_8(&mut self, address: u16, context: &mut impl Context) -> u8 {
        self.tick(context);
        context.read(address)
    }

    fn read_16(&mut self, address: u16, context: &mut impl Context) -> u16 {
        let low = self.read_8(address, context) as u16;
        let high = self.read_8(address + 1, context) as u16;
        (high << 8) | low
    }

    fn write_8(&mut self, address: u16, value: u8, context: &mut impl Context) {
        self.tick(context);
        context.write(address, value)
    }

    fn write_16(&mut self, address: u16, value: u16, context: &mut impl Context) {
        let low = value as u8;
        let high = (value >> 8) as u8;
        self.write_8(address, low, context);
        self.write_8(address + 1, high, context);
    }

    fn fetch_8(&mut self, context: &mut impl Context) -> u8 {
        let data = self.read_8(self.registers.pc, context);
        self.registers.pc += 1;
        data
    }

    fn fetch_16(&mut self, context: &mut impl Context) -> u16 {
        let data = self.read_16(self.registers.pc, context);
        self.registers.pc += 2;
        data
    }

    fn pop_8(&mut self, context: &mut impl Context) -> u8 {
        let data = self.read_8(self.registers.sp, context);
        self.registers.sp += 1;
        data
    }

    fn pop_16(&mut self, context: &mut impl Context) -> u16 {
        let lo = self.pop_8(context) as u16;
        let hi = self.pop_8(context) as u16;
        (hi << 8) | lo
    }

    fn push_8(&mut self, value: u8, context: &mut impl Context) {
        self.registers.sp -= 1;
        self.write_8(self.registers.sp, value, context);
    }

    fn push_16(&mut self, value: u16, context: &mut impl Context) {
        let lo = value as u8;
        let hi = (value >> 8) as u8;
        self.push_8(hi, context);
        self.push_8(lo, context);
    }

    fn get_af(&self) -> u16 {
        (self.registers.a as u16) << 8 | self.registers.f.bytes[0] as u16
    }

    fn set_af(&mut self, value: u16) {
        self.registers.a = (value >> 8) as u8;
        self.registers.f.bytes[0] = value as u8 & 0xF0;
    }

    fn get_bc(&self) -> u16 {
        (self.registers.b as u16) << 8 | self.registers.c as u16
    }

    fn set_bc(&mut self, value: u16) {
        self.registers.b = (value >> 8) as u8;
        self.registers.c = value as u8;
    }

    fn get_de(&self) -> u16 {
        (self.registers.d as u16) << 8 | self.registers.e as u16
    }

    fn set_de(&mut self, value: u16) {
        self.registers.d = (value >> 8) as u8;
        self.registers.e = value as u8;
    }

    fn get_hl(&self) -> u16 {
        (self.registers.h as u16) << 8 | self.registers.l as u16
    }

    fn set_hl(&mut self, value: u16) {
        self.registers.h = (value >> 8) as u8;
        self.registers.l = value as u8;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Register8 {
    B,
    C,
    D,
    E,
    H,
    L,
    HLIndirect,
    A,
}

impl From<u8> for Register8 {
    fn from(value: u8) -> Self {
        match value {
            0b000 => Register8::B,
            0b001 => Register8::C,
            0b010 => Register8::D,
            0b011 => Register8::E,
            0b100 => Register8::H,
            0b101 => Register8::L,
            0b110 => Register8::HLIndirect,
            0b111 => Register8::A,
            _ => unreachable!("Invalid register 8 value: {:#04x}", value),
        }
    }
}

#[repr(u8)]
enum Register16 {
    BC,
    DE,
    HL,
    SP,
}

impl From<u8> for Register16 {
    fn from(value: u8) -> Self {
        match value {
            0b00 => Register16::BC,
            0b01 => Register16::DE,
            0b10 => Register16::HL,
            0b11 => Register16::SP,
            _ => unreachable!("Invalid register 16 value: {:#04x}", value),
        }
    }
}

enum Register16Stk {
    BC,
    DE,
    HL,
    AF,
}

impl From<u8> for Register16Stk {
    fn from(value: u8) -> Self {
        match value {
            0b00 => Register16Stk::BC,
            0b01 => Register16Stk::DE,
            0b10 => Register16Stk::HL,
            0b11 => Register16Stk::AF,
            _ => unreachable!("Invalid register 16 stack value: {:#04x}", value),
        }
    }
}

enum Register16Mem {
    BC,
    DE,
    HLPlus,
    HLMinus,
}

impl From<u8> for Register16Mem {
    fn from(value: u8) -> Self {
        match value {
            0b00 => Register16Mem::BC,
            0b01 => Register16Mem::DE,
            0b10 => Register16Mem::HLPlus,
            0b11 => Register16Mem::HLMinus,
            _ => unreachable!("Invalid register 16 mem value: {:#04x}", value),
        }
    }
}

enum Condition {
    Nz,
    Z,
    Nc,
    C,
}

impl From<u8> for Condition {
    fn from(value: u8) -> Self {
        match value {
            0b00 => Condition::Nz,
            0b01 => Condition::Z,
            0b10 => Condition::Nc,
            0b11 => Condition::C,
            _ => unreachable!("Invalid condition value: {:#04x}", value),
        }
    }
}
