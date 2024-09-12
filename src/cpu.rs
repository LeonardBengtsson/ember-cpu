use std::io;
use crate::{cpu, parse, util};

pub const CPU_MEMORY_SIZE: usize = 0x10000;
pub const VRAM_START: u16 = 0;
pub const PROGRAM_START: u16 = (CPU_MEMORY_SIZE >> 2) as u16;
pub const STACK_START: u16 = PROGRAM_START + (CPU_MEMORY_SIZE >> 3) as u16;
pub const BUILTIN_START: u16 = STACK_START + (CPU_MEMORY_SIZE >> 4) as u16;
pub const HEAP_META_START: u16 = BUILTIN_START + (CPU_MEMORY_SIZE >> 5) as u16;
pub const HEAP_DATA_START: u16 = (CPU_MEMORY_SIZE >> 1) as u16;

pub const SUCCESS_ERROR_CODE: u16 = 0x0000;
pub const STACK_OVERFLOW_ERROR_CODE: u16 = 0x0010;
pub const HEAP_ALLOC_ERROR_CODE: u16 = 0x0011;
pub const DIV_ZERO_ERROR_CODE: u16 = 0x0020;

pub const BUILTIN_SUBROUTINES: [u16; 0] = [];

pub struct Cpu {
    cycle: u64,
    memory: Vec<u16>,
    a_register: u16,
    b_register: u16,
    c_register: u16,
    instr_register: u16,
    instr_counter: u16,
    stack_counter: u16,
    err_code: u8,
    loop_flag: bool,
    jumped_flag: bool,
    load_const_flag: bool,
    result_zero_flag: bool,
    result_negative_flag: bool,
    result_overflow_flag: bool,
}

impl Cpu {
    pub fn new(code: Vec<u16>, builtin: Vec<u16>) -> Result<Self, String> {
        let mut mem = vec![0u16; CPU_MEMORY_SIZE];

        if code.len() > (STACK_START - PROGRAM_START) as usize {
            return Err(format!("Program size {} exceeds maximum size of {}", code.len(), STACK_START - PROGRAM_START));
        }
        (&mut mem[(PROGRAM_START as usize)..(PROGRAM_START as usize + code.len())]).copy_from_slice(code.as_slice());

        if builtin.len() > (HEAP_META_START - BUILTIN_START) as usize {
            return Err(format!("Built-in program size {} exceeds maximum size of {}", builtin.len(), HEAP_META_START - BUILTIN_START));
        }
        (&mut mem[(BUILTIN_START as usize)..(BUILTIN_START as usize + builtin.len())]).copy_from_slice(&builtin);

        let first_instr = mem[PROGRAM_START as usize];
        Ok(Cpu {
            cycle: 0,
            memory: mem,
            a_register: 0x0000,
            b_register: 0x0000,
            c_register: 0x0000,
            instr_register: first_instr,
            instr_counter: PROGRAM_START,
            stack_counter: STACK_START,
            err_code: 0x00,
            loop_flag: true,
            jumped_flag: false,
            load_const_flag: false,
            result_zero_flag: false,
            result_negative_flag: false,
            result_overflow_flag: false,
        })
    }

    pub fn get_cycle(&self) -> u64 {
        self.cycle
    }

    pub fn get_a(&self) -> u16 {
        self.a_register
    }

    pub fn get_b(&self) -> u16 {
        self.b_register
    }

    pub fn get_c(&self) -> u16 {
        self.c_register
    }

    pub fn set_a(&mut self, val: u16) {
        self.a_register = val
    }

    pub fn set_b(&mut self, val: u16) {
        self.b_register = val
    }

    pub fn set_c(&mut self, val: u16) {
        self.c_register = val
    }

    pub fn get_address(&self, address: u16) -> Result<u16, String> {
        if (address as usize) < self.memory.len() {
            Ok(self.memory[address as usize])
        } else {
            Err(format!("Address {} is out of bounds of memory of size {}", address, self.memory.len()))
        }
    }

    pub fn set_address(&mut self, address: u16, value: u16) -> Result<(), String> {
        if (address as usize) < self.memory.len() {
            self.memory[address as usize] = value;
            Ok(())
        } else {
            Err(format!("Address {} is out of bounds of memory of size {}", address, self.memory.len()))
        }
    }

    pub fn get_instr_counter(&self) -> u16 {
        self.instr_counter
    }

    pub fn get_const_flag(&self) -> bool {
        self.load_const_flag
    }

    pub fn registers_info(&self) -> String {
        format!(
            "| {:016x}: {:8} | {}{}{} | {}{}{} | {:02x} | {:04x} {:04x} {:04x} | I: {:04x} S: {:04x} |",
            self.cycle,
            parse::parse(self.load_const_flag, self.instr_register).map_or("????".into(), |i| i.get_name()),
            if !self.loop_flag { "H" } else { " " },
            if self.jumped_flag { "J" } else { " " },
            if self.load_const_flag { "C" } else { " " },
            if self.result_zero_flag { "Z" } else { " " },
            if self.result_negative_flag { "N" } else { " " },
            if self.result_overflow_flag { "O" } else { " " },
            self.err_code,
            self.a_register,
            self.b_register,
            self.c_register,
            self.instr_counter,
            self.stack_counter,
        )
        // format!(
        //     "mem size: {} words\n\
        //     a             = {:#06x}\n\
        //     b             = {:#06x}\n\
        //     c             = {:#06x}\n\
        //     instruction   = {:#06x}\n\
        //     instr_counter = {:#06x}\n\
        //     stack_counter = {:#06x}\n\
        //     err_code      = {:#04x}\n\
        //     halted = {}, jumped = {}, load_const = {}\n\
        //     zero = {}, neg = {}, overflow = {}",
        //     self.memory.len(),
        //     self.a_register,
        //     self.b_register,
        //     self.c_register,
        //     self.instr_register,
        //     self.instr_counter,
        //     self.stack_counter,
        //     self.err_code,
        //     !self.loop_flag,
        //     self.jumped_flag,
        //     self.load_const_flag,
        //     self.result_zero_flag,
        //     self.result_negative_flag,
        //     self.result_overflow_flag
        // )
    }

    pub fn mem_dump(&self) -> String {
        let mut info = String::new();
        for i in (0..self.memory.len()).step_by(0x1000) {
            for j in (0..0x1000).step_by(0x200) {
                for k in (0..0x200).step_by(0x10) {
                    info.push_str(&format!("0x{:04x} |", i + j + k));
                    for l in 0..0x10 {
                        let a = i + j + k + l;
                        let val = self.memory[a];
                        if val == 0 && a != 0 && self.memory[a - 1] != CpuInstr::LoadConst.instr_code() {
                            info.push_str(" ......");
                        } else {
                            info.push_str(&format!(" 0x{:04x}", val)[..])
                        }
                    }
                    info.push('\n');
                }
                info.push('\n');
            }
            info.push_str("\n\n\n\n\n\n\n\n");
        }
        let _ = info.split_off(info.len() - 9);
        info
    }

    pub fn partial_mem_dump(&self, section: u16) -> String {
        let mut info = String::new();
        let section_address = section;
        let section = section;

        for i in (0..0x100).step_by(0x10) {
            info.push_str(&format!("0x{:04x} |", section_address + i));
            for j in 0..0x10 {
                let a = (section + i + j) as usize;
                let val = self.memory[a];
                if val == 0 && a != 0 && self.memory[a - 1] != CpuInstr::LoadConst.instr_code() {
                    info.push_str(" ......");
                } else {
                    info.push_str(&format!(" 0x{:04x}", val)[..])
                }
            }
            info.push('\n');
        }
        info
    }

    pub fn run_instr(&mut self, instr: CpuInstr) -> Result<(), String> {
        if self.load_const_flag {
            self.a_register = instr.instr_code();
            self.load_const_flag = false;
        } else {
            self.exec(instr)?;
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.loop_flag
    }

    pub fn cycle(&mut self) -> Result<(), String> {
        if let Some(instr) = parse::parse(self.load_const_flag, self.instr_register) {
            self.jumped_flag = false;
            self.run_instr(instr)?;
            if !self.jumped_flag { self.instr_counter += 1; }
            self.cycle += 1;
            self.instr_register = self.memory[self.instr_counter as usize];
            Ok(())
        } else {
            Err(format!("Failed to parse instruction {}", self.instr_register))
        }
    }

    pub fn exec(&mut self, instr: CpuInstr) -> Result<(), String> {
        match instr {
            CpuInstr::Wait => {}
            CpuInstr::Halt => {
                self.loop_flag = false;
                self.instr_counter = PROGRAM_START;
                self.result_zero_flag = false;
                self.result_negative_flag = false;
                self.result_overflow_flag = false;
                self.err_code = 0;
            }
            CpuInstr::Pause => {
                self.loop_flag = false;
            }
            CpuInstr::Resume => {
                self.loop_flag = true;
            }
            CpuInstr::SetError => {
                self.err_code = self.a_register as u8;
            }
            CpuInstr::Move { from, to } => {
                match (from, to) {
                    (Register::A, Register::B) => { self.b_register = self.a_register; }
                    (Register::B, Register::A) => { self.a_register = self.b_register; }
                    (Register::A, Register::C) => { self.c_register = self.a_register; }
                    (Register::C, Register::A) => { self.a_register = self.c_register; }
                    (Register::B, Register::C) => { self.c_register = self.b_register; }
                    (Register::C, Register::B) => { self.b_register = self.c_register; }
                    (Register::A, Register::A) => { return Err(String::from("Can't move a register to itself")); }
                    (Register::B, Register::B) => { return Err(String::from("Can't move a register to itself")); }
                    (Register::C, Register::C) => { return Err(String::from("Can't move a register to itself")); }
                }
            }
            CpuInstr::LoadConst => {
                self.load_const_flag = true;
            }
            CpuInstr::Const(val) => {
                if self.load_const_flag {
                    self.a_register = val;
                } else {
                    return Err(String::from("Tried to load a constant without Load Constant Flag being set"));
                }
                self.load_const_flag = false;
            }
            CpuInstr::CpuConst(cpu_const) => {
                self.a_register = cpu_const.value();
            }
            CpuInstr::InstrCounter => {
                self.a_register = self.instr_counter;
            }
            CpuInstr::StackCounter => {
                self.a_register = self.stack_counter;
            }
            CpuInstr::MoveToStackCounter => {
                self.stack_counter = self.a_register;
            }
            CpuInstr::Input => {
                match self.a_register {
                    0 => {
                        let mut s = String::new();
                        if let Ok(_) = io::stdin().read_line(&mut s) {
                            if let Some(byte) = s.bytes().next() {
                                self.b_register = byte as u16;
                            }
                        }
                    }
                    _ => {}
                }
            }
            CpuInstr::Output => {
                match self.a_register {
                    0 => {
                        print!("{}", self.b_register as u8 as char);
                    }
                    _ => {}
                }
            }
            CpuInstr::MemRead => {
                if self.b_register as usize > self.memory.len() {
                    return Err(format!("Memory address {} out of range of memory size {}", self.b_register, self.memory.len()));
                }
                self.a_register = self.memory[self.b_register as usize];
            }
            CpuInstr::MemWrite => {
                if self.a_register as usize > self.memory.len() {
                    return Err(format!("Memory address {} out of range of memory size {}", self.b_register, self.memory.len()));
                }
                self.memory[self.a_register as usize] = self.b_register;
            }
            CpuInstr::Jump => {
                self.instr_counter = self.a_register;
                self.jumped_flag = true;
            }
            CpuInstr::JumpIfZero => {
                if self.result_zero_flag {
                    self.instr_counter = self.a_register;
                    self.jumped_flag = true;
                }
            }
            CpuInstr::JumpIfNeg => {
                if self.result_negative_flag {
                    self.instr_counter = self.a_register;
                    self.jumped_flag = true;
                }
            }
            CpuInstr::JumpIfNegOrZero => {
                if self.result_negative_flag || self.result_zero_flag {
                    self.instr_counter = self.a_register;
                    self.jumped_flag = true;
                }
            }
            CpuInstr::JumpIfOverflow => {
                if self.result_overflow_flag {
                    self.instr_counter = self.a_register;
                    self.jumped_flag = true;
                }
            }
            CpuInstr::AluInstr { instr, pass } => {
                let pass = pass || instr == AluInstr::NoOp;
                let result = match instr {
                    AluInstr::NoOp => self.a_register as i32,
                    AluInstr::Increment => self.a_register as i32 + 1,
                    AluInstr::Decrement => self.a_register as i32 - 1,
                    AluInstr::Not => (!self.a_register) as i32,
                    AluInstr::Or => (self.a_register | self.b_register) as i32,
                    AluInstr::And => (self.a_register & self.b_register) as i32,
                    AluInstr::Xor => (self.a_register ^ self.b_register) as i32,
                    AluInstr::Add => (self.a_register as i32) + (self.b_register as i32),
                    AluInstr::Subtract => (self.a_register as i32) - (self.b_register as i32),
                    AluInstr::Multiply => (self.a_register as i32) * (self.b_register as i32),
                    AluInstr::Random => rand::random::<u16>() as i32,
                    AluInstr::ShiftLeftVar => (self.a_register as i32) << (self.b_register & 0x000f),
                    AluInstr::ShiftLeft(shift) => (self.a_register as i32) << shift.value(),
                    AluInstr::ShiftRightVar => (self.a_register as i32) >> (self.b_register & 0x000f),
                    AluInstr::ShiftRight(shift) => (self.a_register as i32) >> shift.value(),
                };

                self.result_negative_flag = result < 0;
                self.result_zero_flag = result == 0;
                self.result_overflow_flag = result > 0xffff;

                if !pass { self.a_register = if result < 0 { 0 } else { if result > 0xffff { 0xffff} else { result } } as u16; }
            }
        }
        Ok(())
    }
}

pub enum Register {
    A, B, C
}

pub enum CpuConst {
    X0000,
    X0001,
    X000E,
    X000F,
    X0010,
}

impl CpuConst {
    pub fn value(&self) -> u16 {
        match self {
            CpuConst::X0000 => 0x0000,
            CpuConst::X0001 => 0x0001,
            CpuConst::X000E => 0x000e,
            CpuConst::X000F => 0x000f,
            CpuConst::X0010 => 0x0010,
        }
    }
}

#[derive(PartialEq)]
pub enum AluInstr {
    NoOp,
    Increment,
    Decrement,
    Not,
    Or,
    And,
    Xor,
    Add,
    Subtract,
    Multiply,
    Random,
    ShiftLeftVar,
    ShiftLeft(ShiftAmount),
    ShiftRightVar,
    ShiftRight(ShiftAmount),
}

#[derive(PartialEq)]
pub enum ShiftAmount {
    S1, S2, S3, S4, S5, S6, S7, S8, S9, S10, S11, S12, S13, S14, S15
}

impl ShiftAmount {
    pub fn value(&self) -> u16 {
        match self {
            ShiftAmount::S1 => 1,
            ShiftAmount::S2 => 2,
            ShiftAmount::S3 => 3,
            ShiftAmount::S4 => 4,
            ShiftAmount::S5 => 5,
            ShiftAmount::S6 => 6,
            ShiftAmount::S7 => 7,
            ShiftAmount::S8 => 8,
            ShiftAmount::S9 => 9,
            ShiftAmount::S10 => 10,
            ShiftAmount::S11 => 11,
            ShiftAmount::S12 => 12,
            ShiftAmount::S13 => 13,
            ShiftAmount::S14 => 14,
            ShiftAmount::S15 => 15,
        }
    }
}

pub enum CpuInstr {
    Halt,
    Wait,
    Pause,
    Resume,
    SetError,
    Move { from: Register, to: Register},
    LoadConst,
    Const(u16),
    CpuConst(CpuConst),
    InstrCounter,
    StackCounter,
    MoveToStackCounter,
    Input,
    Output,
    MemRead,
    MemWrite,
    Jump,
    JumpIfZero,
    JumpIfNeg,
    JumpIfNegOrZero,
    JumpIfOverflow,
    AluInstr { instr: AluInstr, pass: bool },
}

impl CpuInstr {
    pub fn get_instr(line: usize, s: &str) -> Result<CpuInstr, String> {
        match s {
            "wait" => Ok(CpuInstr::Wait),
            "halt" => Ok(CpuInstr::Halt),
            "pause" => Ok(CpuInstr::Pause),
            "resume" => Ok(CpuInstr::Resume),
            "seterr" => Ok(CpuInstr::SetError),
            "movab" => Ok(CpuInstr::Move { from: Register::A, to: Register::B }),
            "movba" => Ok(CpuInstr::Move { from: Register::B, to: Register::A }),
            "movac" => Ok(CpuInstr::Move { from: Register::A, to: Register::C }),
            "movca" => Ok(CpuInstr::Move { from: Register::C, to: Register::A }),
            "movbc" => Ok(CpuInstr::Move { from: Register::B, to: Register::C }),
            "movcb" => Ok(CpuInstr::Move { from: Register::C, to: Register::B }),
            "const" => Ok(CpuInstr::LoadConst),
            "ictr" => Ok(CpuInstr::InstrCounter),
            "sctr" => Ok(CpuInstr::StackCounter),
            "msctr" => Ok(CpuInstr::MoveToStackCounter),
            "inp" => Ok(CpuInstr::Input),
            "outp" => Ok(CpuInstr::Output),
            "memr" => Ok(CpuInstr::MemRead),
            "memw" => Ok(CpuInstr::MemWrite),
            "jmp" => Ok(CpuInstr::Jump),
            "jmpz" => Ok(CpuInstr::JumpIfZero),
            "jmpn" => Ok(CpuInstr::JumpIfNeg),
            "jmpnz" => Ok(CpuInstr::JumpIfNegOrZero),
            "jmpo" => Ok(CpuInstr::JumpIfOverflow),
            "noop" => Ok(CpuInstr::AluInstr { instr: AluInstr::NoOp, pass: true }),
            "inc" => Ok(CpuInstr::AluInstr { instr: AluInstr::Increment, pass: false }),
            "incp" => Ok(CpuInstr::AluInstr { instr: AluInstr::Increment, pass: true }),
            "dec" => Ok(CpuInstr::AluInstr { instr: AluInstr::Decrement, pass: false }),
            "decp" => Ok(CpuInstr::AluInstr { instr: AluInstr::Decrement, pass: true }),
            "not" => Ok(CpuInstr::AluInstr { instr: AluInstr::Not, pass: false }),
            "notp" => Ok(CpuInstr::AluInstr { instr: AluInstr::Not, pass: true }),
            "or" => Ok(CpuInstr::AluInstr { instr: AluInstr::Or, pass: false }),
            "orp" => Ok(CpuInstr::AluInstr { instr: AluInstr::Or, pass: true }),
            "and" => Ok(CpuInstr::AluInstr { instr: AluInstr::And, pass: false }),
            "andp" => Ok(CpuInstr::AluInstr { instr: AluInstr::And, pass: true }),
            "xor" => Ok(CpuInstr::AluInstr { instr: AluInstr::Xor, pass: false }),
            "xorp" => Ok(CpuInstr::AluInstr { instr: AluInstr::Xor, pass: true }),
            "add" => Ok(CpuInstr::AluInstr { instr: AluInstr::Add, pass: false }),
            "addp" => Ok(CpuInstr::AluInstr { instr: AluInstr::Add, pass: true }),
            "sub" => Ok(CpuInstr::AluInstr { instr: AluInstr::Subtract, pass: false }),
            "subp" => Ok(CpuInstr::AluInstr { instr: AluInstr::Subtract, pass: true }),
            "mult" => Ok(CpuInstr::AluInstr { instr: AluInstr::Multiply, pass: false }),
            "multp" => Ok(CpuInstr::AluInstr { instr: AluInstr::Multiply, pass: true }),
            "rand" => Ok(CpuInstr::AluInstr { instr: AluInstr::Random, pass: false }),
            "shl" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeftVar, pass: false }),
            "shlp" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeftVar, pass: true }),
            "shr" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRightVar, pass: false }),
            "shrp" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRightVar, pass: true }),
            "shl1" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S1), pass: false }),
            "shl2" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S2), pass: false }),
            "shl3" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S3), pass: false }),
            "shl4" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S4), pass: false }),
            "shl5" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S5), pass: false }),
            "shl6" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S6), pass: false }),
            "shl7" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S7), pass: false }),
            "shl8" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S8), pass: false }),
            "shl9" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S9), pass: false }),
            "shla" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S10), pass: false }),
            "shlb" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S11), pass: false }),
            "shlc" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S12), pass: false }),
            "shld" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S13), pass: false }),
            "shle" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S14), pass: false }),
            "shlf" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S15), pass: false }),
            "shl1p" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S1), pass: true }),
            "shl2p" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S2), pass: true }),
            "shl3p" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S3), pass: true }),
            "shl4p" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S4), pass: true }),
            "shl5p" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S5), pass: true }),
            "shl6p" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S6), pass: true }),
            "shl7p" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S7), pass: true }),
            "shl8p" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S8), pass: true }),
            "shl9p" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S9), pass: true }),
            "shlap" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S10), pass: true }),
            "shlbp" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S11), pass: true }),
            "shlcp" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S12), pass: true }),
            "shldp" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S13), pass: true }),
            "shlep" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S14), pass: true }),
            "shlfp" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S15), pass: true }),
            "shr1" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S1), pass: false }),
            "shr2" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S2), pass: false }),
            "shr3" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S3), pass: false }),
            "shr4" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S4), pass: false }),
            "shr5" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S5), pass: false }),
            "shr6" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S6), pass: false }),
            "shr7" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S7), pass: false }),
            "shr8" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S8), pass: false }),
            "shr9" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S9), pass: false }),
            "shra" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S10), pass: false }),
            "shrb" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S11), pass: false }),
            "shrc" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S12), pass: false }),
            "shrd" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S13), pass: false }),
            "shre" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S14), pass: false }),
            "shrf" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S15), pass: false }),
            "shr1p" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S1), pass: true }),
            "shr2p" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S2), pass: true }),
            "shr3p" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S3), pass: true }),
            "shr4p" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S4), pass: true }),
            "shr5p" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S5), pass: true }),
            "shr6p" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S6), pass: true }),
            "shr7p" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S7), pass: true }),
            "shr8p" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S8), pass: true }),
            "shr9p" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S9), pass: true }),
            "shrap" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S10), pass: true }),
            "shrbp" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S11), pass: true }),
            "shrcp" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S12), pass: true }),
            "shrdp" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S13), pass: true }),
            "shrep" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S14), pass: true }),
            "shrfp" => Ok(CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S15), pass: true }),
            "set0x0000" => Ok(CpuInstr::CpuConst(CpuConst::X0000)),
            "set0x0001" => Ok(CpuInstr::CpuConst(CpuConst::X0001)),
            "set0x000e" => Ok(CpuInstr::CpuConst(CpuConst::X000E)),
            "set0x000f" => Ok(CpuInstr::CpuConst(CpuConst::X000F)),
            "set0x0010" => Ok(CpuInstr::CpuConst(CpuConst::X0010)),
            l => {
                if l.starts_with('(') && l.ends_with(')') {
                    let middle = &l[1..(l.len() - 1)];
                    match middle.to_ascii_lowercase().as_str() {
                        "vram" => Ok(CpuInstr::Const(cpu::VRAM_START)),
                        "program" => Ok(CpuInstr::Const(cpu::PROGRAM_START)),
                        "stack" => Ok(CpuInstr::Const(cpu::STACK_START)),
                        "builtin" => Ok(CpuInstr::Const(cpu::BUILTIN_START)),
                        "heap_meta" => Ok(CpuInstr::Const(cpu::HEAP_META_START)),
                        "heap_data" => Ok(CpuInstr::Const(cpu::HEAP_DATA_START)),
                        "success_error" => Ok(CpuInstr::Const(cpu::SUCCESS_ERROR_CODE)),
                        "stack_error" => Ok(CpuInstr::Const(cpu::STACK_OVERFLOW_ERROR_CODE)),
                        "heap_alloc_error" => Ok(CpuInstr::Const(cpu::HEAP_ALLOC_ERROR_CODE)),
                        "div_0_error" => Ok(CpuInstr::Const(cpu::DIV_ZERO_ERROR_CODE)),
                        _ => match util::parse_u16(middle) {
                            Ok(val) => Ok(CpuInstr::Const(val)),
                            Err(_) => return Err(format!("Line {}:\n  Invalid constant: {}", line + 1, middle)),
                        }
                    }
                } else {
                    return Err(format!("Line {}:\n  Invalid instruction: '{}'", line + 1, l))
                }
            }
        }
    }

    pub fn get_name(&self) -> String {
        match self {
            CpuInstr::Halt => "halt".into(),
            CpuInstr::Wait => "wait".into(),
            CpuInstr::Pause => "pause".into(),
            CpuInstr::Resume => "resume".into(),
            CpuInstr::SetError => "seterr".into(),
            CpuInstr::Move { from, to } => match (from, to) {
                (Register::A, Register::B) => "movab".into(),
                (Register::B, Register::A) => "movba".into(),
                (Register::A, Register::C) => "movac".into(),
                (Register::C, Register::A) => "movca".into(),
                (Register::B, Register::C) => "movbc".into(),
                (Register::C, Register::B) => "movcb".into(),
                _ => "????".into()
            }
            CpuInstr::LoadConst => "const".into(),
            CpuInstr::Const(v) => format!("({:#06x})", v),
            CpuInstr::CpuConst(v) => format!("({:#06x})", v.value()),
            CpuInstr::InstrCounter => "ictr".into(),
            CpuInstr::StackCounter => "sctr".into(),
            CpuInstr::MoveToStackCounter => "msctr".into(),
            CpuInstr::Input => "inp".into(),
            CpuInstr::Output => "outp".into(),
            CpuInstr::MemRead => "memr".into(),
            CpuInstr::MemWrite => "memw".into(),
            CpuInstr::Jump => "jmp".into(),
            CpuInstr::JumpIfZero => "jmpz".into(),
            CpuInstr::JumpIfNeg => "jmpn".into(),
            CpuInstr::JumpIfNegOrZero => "jmpnz".into(),
            CpuInstr::JumpIfOverflow => "jmpo".into(),
            CpuInstr::AluInstr { instr, pass } => {
                let pass = *pass;
                match instr {
                    AluInstr::NoOp => "noop".into(),
                    AluInstr::Increment => if pass { "incp" } else { "inc" } .into(),
                    AluInstr::Decrement => if pass { "decp" } else { "dec" } .into(),
                    AluInstr::Not => if pass { "notp" } else { "not" } .into(),
                    AluInstr::Or => if pass { "orp" } else { "or" } .into(),
                    AluInstr::And => if pass { "andp" } else { "and" } .into(),
                    AluInstr::Xor => if pass { "xorp" } else { "xor" } .into(),
                    AluInstr::Add => if pass { "addp" } else { "add" } .into(),
                    AluInstr::Subtract => if pass { "subp" } else { "sub" } .into(),
                    AluInstr::Multiply => if pass { "multp" } else { "mult" } .into(),
                    AluInstr::Random => "rand" .into(),
                    AluInstr::ShiftLeftVar => if pass { "shlp" } else { "shl" } .into(),
                    AluInstr::ShiftLeft(a) => format!("shl{:#01x}{}", a.value(), if pass { "p" } else { "" }),
                    AluInstr::ShiftRightVar => if pass { "shrp" } else { "shr" } .into(),
                    AluInstr::ShiftRight(a) => format!("shr{:#01x}{}", a.value(), if pass { "p" } else { "" }),
                }
            }
        }
    }

    pub fn instr_code(self) -> u16 {
        match self {
            CpuInstr::Halt => 0x0000,
            CpuInstr::Wait => 0x0001,
            CpuInstr::Pause => 0x0002,
            CpuInstr::Resume => 0x0003,
            CpuInstr::SetError => 0x0004,
            CpuInstr::Move { from, to } => match (from, to) {
                (Register::A, Register::B) => 0x0010,
                (Register::B, Register::A) => 0x0011,
                (Register::A, Register::C) => 0x0012,
                (Register::C, Register::A) => 0x0013,
                (Register::B, Register::C) => 0x0014,
                (Register::C, Register::B) => 0x0015,
                (Register::A, Register::A) => CpuInstr::Wait.instr_code(),
                (Register::B, Register::B) => CpuInstr::Wait.instr_code(),
                (Register::C, Register::C) => CpuInstr::Wait.instr_code(),
            }
            CpuInstr::LoadConst => 0x0005,
            CpuInstr::Const(val) => val,
            CpuInstr::CpuConst(c) => match c {
                CpuConst::X0000 => 0x0020,
                CpuConst::X0001 => 0x0021,
                CpuConst::X000E => 0x0022,
                CpuConst::X000F => 0x0023,
                CpuConst::X0010 => 0x0024,
            }
            CpuInstr::InstrCounter => 0x0006,
            CpuInstr::StackCounter => 0x0007,
            CpuInstr::MoveToStackCounter => 0x0008,
            CpuInstr::Input => 0x0009,
            CpuInstr::Output => 0x000a,
            CpuInstr::MemRead => 0x000b,
            CpuInstr::MemWrite => 0x000c,
            CpuInstr::Jump => 0x0030,
            CpuInstr::JumpIfZero => 0x0031,
            CpuInstr::JumpIfNeg => 0x0032,
            CpuInstr::JumpIfNegOrZero => 0x0033,
            CpuInstr::JumpIfOverflow => 0x0034,
            CpuInstr::AluInstr { instr, pass } => {
                let p: u16 = if pass { 0 } else { 1 };
                match instr {
                    AluInstr::NoOp => 0x0040,
                    AluInstr::Increment => 0x0042 | p,
                    AluInstr::Decrement => 0x0044 | p,
                    AluInstr::Not => 0x0046 | p,
                    AluInstr::Or => 0x0048 | p,
                    AluInstr::And => 0x004a | p,
                    AluInstr::Xor => 0x004c | p,
                    AluInstr::Add => 0x004e | p,
                    AluInstr::Subtract => 0x0050 | p,
                    AluInstr::Multiply => 0x0052 | p,
                    AluInstr::Random => 0x0054 | p,
                    AluInstr::ShiftLeftVar => 0x0056 | p,
                    AluInstr::ShiftRightVar => 0x0058 | p,
                    AluInstr::ShiftLeft(shift) => (0x0060 + (shift.value() << 1)) | p,
                    AluInstr::ShiftRight(shift) => (0x0080 + (shift.value() << 1)) | p,
                }
            }
        }
    }
}
