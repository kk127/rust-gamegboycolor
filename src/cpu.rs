use modular_bitfield::prelude::*;
use crate::context;

trait Context:  context::Bus{}
impl<T: context::Bus> Context for T {}

struct Cpu {
    registers: Registers,
}

impl Cpu {
    fn new() -> Self {
        todo!()
    }

}

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

#[bitfield(bits = 8)]
#[derive(Debug, Default)]
struct Flags {
    __: B4,
    carry: bool,
    half_carry: bool,
    subtract: bool,
    zero: bool,
}

impl Cpu {
    fn read_8(&self, address: u16, context: &mut impl Context) -> u8 {
        context.read(address)
    }

    fn read_16(&self, address: u16, context: &mut impl Context) -> u16 {
        let low = self.read_8(address, context) as u16;
        let high = self.read_8(address + 1, context) as u16;
        (high << 8) | low
    }

    fn write_8(&self, address: u16, value: u8, context: &mut impl Context) {
        context.write(address, value)
    }

    fn write_16(&self, address: u16, value: u16, context: &mut impl Context) {
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

    fn get_af(&self) -> u16 {
        (self.registers.a as u16) << 8 | self.registers.f.bytes[0] as u16
    }

    fn set_af(&mut self, value: u16) {
        self.registers.a = (value >> 8) as u8;
        self.registers.f.bytes[0] = value as u8;
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
}

fn decode_instruction(op: u8) -> Instruction {
    match op {
        0x00 => Instruction::Nop,
        op if op & 0b1100_0000 == 0b0000_0000 => Instruction::LdAImm16,
        // omit
    }
}

enum Instruction {
    Nop,
    LdR16Imm16(Register16),
    LdR16memA(Register16Mem),
    LdAR16mem(Register16Mem),
    LdImm16Sp,
    IncR16(Register16),
    DecR16(Register16),
    AddHLR16(Register16),
    IncR8(Register8),
    DecR8(Register8),
    LdR8Imm8(Register8),
    Rlca,
    Rrca,
    Rla,
    Rra,
    Daa,
    Cpl,
    Scf,
    Ccf,
    JrImm8,
    JrCondImm8(Condition),
    Stop,
    LdR8R8(Register8, Register8),
    Halt,
    AddAR8(Register8),
    AdcAR8(Register8),
    SubAR8(Register8),
    SbcAR8(Register8),
    AndAR8(Register8),
    XorAR8(Register8),
    OrAR8(Register8),
    CpAR8(Register8),
    AddAImm8,
    AdcAImm8,
    SubAImm8,
    SbcAImm8,
    AndAImm8,
    XorAImm8,
    OrAImm8,
    CpAImm8,
    RetCond(Condition),
    Ret,
    Reti,
    JpCondImm16(Condition),
    JpImm16,
    JpHl,
    CallCondImm16(Condition),
    CallImm16,
    Rst(u16),
    PopR16Stk(R16Stk),
    PushR16Stk(R16Stk),
    CbPrefix,
    LdhCA,
    LdhImm8A,
    LdImm16A,
    LdhAC,
    LdhAImm8,
    LdAImm16,
    AddSPImm8,
    LdHlSpImm8,
    LdSpHl,
    Di,
    Ei,
}

enum CbPrefixInstruction {
    Rlc(Register8),
    Rrc(Register8),
    Rl(Register8),
    Rr(Register8),
    Sla(Register8),
    Sra(Register8),
    Swap(Register8),
    Srl(Register8),
    Bit(u8, Register8),
    Res(u8, Register8),
    Set(u8, Register8),    
}

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

enum Register16 {
    BC,
    DE,
    HL,
    SP,
}

enum R16Stk {
    BC,
    DE,
    HL,
    AF,
}

enum Register16Mem {
    BC,
    DE,
    HLPlus,
    HLMinus,
}

enum Condition {
    Nz,
    Z,
    Nc,
    C,
}

