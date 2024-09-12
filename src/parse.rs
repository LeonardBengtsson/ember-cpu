use std::collections::HashMap;
use std::{fs, path};
use path_absolutize::*;
use crate::{cpu, SOURCE_FILE_EXTENSION};
use crate::cpu::{AluInstr, CpuInstr, CpuConst, Register, ShiftAmount};

pub const COMMENT_PREFIX: char = '#';
pub const MACRO_PREFIX: char = '.';
pub const JUMP_PREFIX: char = '%';
pub const LABEL_PREFIX: char = ':';
pub const NAMESPACE_SEPARATOR: char = '/';

fn trim_comment(code: &str) -> &str {
    let code = match code.find(COMMENT_PREFIX) {
        Some(index) => &code[..index],
        None => code,
    };
    code.trim()
}

pub fn expand_lines(code: &str, path: &str) -> Result<Vec<String>, String> {
    let lines = code.to_ascii_lowercase();
    let lines: Vec<&str> = lines.lines().map(|l| l.split(';')).flatten().map(|l| l.trim()).collect();

    let mut out = Vec::<String>::new();
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with(MACRO_PREFIX) {
            match expand_macro(trim_comment(line), &mut out, path) {
                Ok(ok) => ok,
                Err(err) => return Err(format!("Line {}:\n  {}", i + 1, err))
            };
        } else if !line.starts_with(COMMENT_PREFIX) && line.len() > 0 {
            out.push(trim_comment(*line).into());
        }
    }
    Ok(out)
}

fn add_namespace_labels(code: &mut Vec<String>, namespace: &str) {
    for line in code.iter_mut() {
        if line.starts_with(LABEL_PREFIX) {
            *line = format!("{}{}{}{}", LABEL_PREFIX, namespace, NAMESPACE_SEPARATOR, &line[1..]);
        } else if line.starts_with(JUMP_PREFIX) {
            let space = trim_comment(line).trim().find(' ');
            *line = if let Some(index) = space {
                format!("{} {}{}{}", &line[..index], namespace, NAMESPACE_SEPARATOR, &line[(index + 1)..])
            } else {
                format!("{}{}{}{}", JUMP_PREFIX, namespace, NAMESPACE_SEPARATOR, &line[1..])
            }
        }
    }
}

fn expand_macro(line: &str, code: &mut Vec<String>, path: &str) -> Result<(), String> {
    let space = line.find(' ');
    let args = if let Some(index) = space { (line[(index + 1)..]).split(' ').collect::<Vec<&str>>() } else { vec![] };
    match &line[1..(if let Some(p) = space { p } else { line.len() })] {
        "extern" => {
            if args.len() != 1 { return Err(format!("Invalid number of arguments: {}", line)) }
            let cwd = path::Path::new(path);
            let tmp: String;
            let relative_path = if args[0].ends_with(SOURCE_FILE_EXTENSION) { args[0] } else { tmp = format!("{}{}", args[0], SOURCE_FILE_EXTENSION); &tmp[..] };
            let relative_path = path::Path::new(relative_path);
            let new_path = relative_path.absolutize_from(cwd.parent().unwrap()).unwrap();
            let new_path = new_path.as_ref().to_str().unwrap();

            if new_path.ends_with(SOURCE_FILE_EXTENSION) {
                let input = match fs::read_to_string(new_path) {
                    Ok(contents) => contents,
                    Err(err) => {
                        return Err(format!("Failed to read file {}:\n  {}", new_path, err));
                    }
                };
                let mut lines = expand_lines(input.as_str(), new_path)?;
                add_namespace_labels(&mut lines, relative_path.file_stem().unwrap().to_str().unwrap());
                code.append(&mut lines);
            }
        }
        "const" => {
            if args.len() != 1 { return Err(format!("Invalid number of arguments: {}", line)) }
            code.push("const".into());
            code.push(format!("({})", args[0]));
        }
        "read" => {
            if args.len() != 1 { return Err(format!("Invalid number of arguments: {}", line)) }
            expand_macro(&format!(".const {}", args[0]), code, path)?;
            code.push("movab".into());
            code.push("memr".into());
        }
        "write" => {
            if args.len() != 1 { return Err(format!("Invalid number of arguments: {}", line)) }
            code.push("movab".into());
            expand_macro(&format!(".const {}", args[0]), code, path)?;
            code.push("memw".into());
        }
        "err" => {
            if args.len() > 1 { return Err(format!("Invalid number of arguments: {}", line)) }
            let error_code = if args.len() == 1 { args[0] } else { "0xffff" };
            expand_macro(&format!(".const {}", error_code), code, path)?;
            code.push("seterr".into());
            code.push("pause".into());
        }
        "push" => {
            if args.len() > 1 { return Err(format!("Invalid number of arguments: {}", line)) }
            if args.len() == 1 {
                expand_macro(&format!(".const {}", args[0]), code, path)?;
                code.push("movab".into());
                code.push("sctr".into());
                code.push("memw".into());
                code.push("inc".into());
                code.push("msctr".into());
            } else {
                code.push("movab".into());
                code.push("sctr".into());
                code.push("memw".into());
                code.push("inc".into());
                code.push("msctr".into());
            }
        }
        "pop" => {
            if args.len() == 0  {
                code.push("sctr".into());
                code.push("dec".into());
                code.push("msctr".into());
                code.push("movab".into());
                code.push("memr".into());
            } else if args.len() == 1 {
                expand_macro(&format!(".const {}", args[0]), code, path)?;
                code.push("movab".into());
                code.push("sctr".into());
                code.push("sub".into());
                code.push("msctr".into());
            } else {
                return Err(format!("Invalid number of arguments: {}", line))
            }
        }
        "popn" => {
            if args.len() != 0 { return Err(format!("Invalid number of arguments: {}", line)) }
            code.push("sctr".into());
            code.push("dec".into());
            code.push("msctr".into());
        }
        "peek" => {
            if args.len() == 0  {
                code.push("sctr".into());
                code.push("dec".into());
                code.push("movab".into());
                code.push("memr".into());
            } else if args.len() == 1 {
                expand_macro(&format!(".const {}", args[0]), code, path)?;
                code.push("movab".into());
                code.push("sctr".into());
                code.push("dec".into());
                code.push("sub".into());
                code.push("movab".into());
                code.push("memr".into());
            } else {
                return Err(format!("Invalid number of arguments: {}", line))
            }
        }
        "rep" => {
            if args.len() == 0 {
                code.push("sctr".into());
                code.push("dec".into());
                code.push("memw".into());
            } else if args.len() == 1 {
                expand_macro(&format!(".const {}", args[0]), code, path)?;
                code.push("movab".into());
                code.push("sctr".into());
                code.push("sub".into());
                code.push("dec".into());
                code.push("movcb".into());
                code.push("memw".into());
            } else {
                return Err(format!("Invalid number of arguments: {}", line));
            }
        }
        "stackstat" => {
            code.push("sctr".into());
            code.push("movab".into());
            expand_macro(".const BUILTIN", code, path)?;
            code.push("sub".into());
        }
        "call" => {
            if args.len() != 1 { return Err(format!("Invalid number of arguments: {}", line)) }
            code.push("ictr".into());
            code.push("movab".into());
            expand_macro(".const 13", code, path)?;
            code.push("add".into());
            expand_macro(".push", code, path)?;
            code.push(format!("%{}", args[0]));
        }
        "return" => {
            if args.len() == 0 {
                expand_macro(".peek", code, path)?;
                code.push("jmp".into());
            } else if args.len() == 1 {
                expand_macro(&format!(".peek {}", args[0]), code, path)?;
                code.push("jmp".into());
            } else {
                return Err(format!("Invalid number of arguments: {}", line));
            }
        }
        "str" => {
            let s = args.join(" ");
            expand_macro(&format!(".const {}", s.len()), code, path)?;
            code.push("inc".into());
            expand_macro(".push", code, path)?;
            expand_macro(".call std/alloc", code, path)?;
            expand_macro(".pop", code, path)?;
            code.push("movac".into());
            expand_macro(".popn", code, path)?;
            expand_macro(".pop", code, path)?;
            code.push("dec".into());
            code.push("movab".into());
            code.push("movca".into());
            code.push("memw".into());
            for c in s.chars() {
                code.push("inc".into());
                code.push("movac".into());
                expand_macro(&format!(".const {}", c as u8), code, path)?;
                code.push("movab".into());
                code.push("movca".into());
                code.push("memw".into());
            }
            if s.len() > 0 {
                code.push("movca".into());
            }
        }
        "print" => {
            let s = args.join(" ").replace("\\n", "\n");
            for c in s.chars() {
                if c.is_ascii() {
                    expand_macro(&format!(".const {}", c as u8), code, path)?;
                    code.push("movab".into());
                    expand_macro(".const 0x0000", code, path)?;
                    code.push("outp".into());
                }
            }
        }
        m => {
            return Err(format!("Invalid macro '{}'", m));
        },
    }
    Ok(())
}

fn compile_jumps(address_start: u16, lines: &Vec<String>) -> Result<Vec<String>, String> {
    let mut offset = 0i64;

    let mut labels = HashMap::<String, u16>::new();
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with(JUMP_PREFIX) {
            offset += 2;
        } else if line.starts_with(LABEL_PREFIX) {
            let l = &line[1..];
            let l = l.replace(' ', "");
            if l.len() == 0 { return Err(format!("Invalid label '{}'", line)); }
            labels.insert(l, (i as i64 + offset) as u16);
            offset -= 1;
        }
    }

    let mut out = Vec::<String>::new();
    for line in lines.iter() {
        if line.starts_with(JUMP_PREFIX) {
            let l = &line[1..];

            let (jump, label) = match l.find(' ') {
                Some(space) => (&l[..space], &l[(space + 1)..]),
                None => ("", l),
            };

            let instr = match jump {
                "" => "jmp",
                "n" => "jmpn",
                "z" => "jmpz",
                "nz" => "jmpnz",
                "o" => "jmpo",
                val => return Err(format!("Invalid jump instruction: '{}'", val))
            };

            let label = label.replace(' ', "");
            let address = match labels.get(&label) {
                Some(val) => address_start + *val,
                None => return Err(format!("Label '{}' doesn't exist", label)),
            };

            out.push("const".into());
            out.push(format!("({})", address));
            out.push(instr.into());
        } else if !line.starts_with(LABEL_PREFIX) {
            out.push(line.into());
        }
    }
    Ok(out)
}

pub fn compile(address_start: u16, code: &str, path: &str) -> Result<Vec<u16>, String> {
    // MACROS
    let lines = expand_lines(code, path)?;

    // LABELS
    let lines = compile_jumps(address_start, &lines)?;

    // COMPILE
    let mut v = vec![0u16; lines.len()];
    for (i, line) in lines.into_iter().enumerate() {
        v[i] = cpu::CpuInstr::get_instr(i, line.trim())?.instr_code();
    }
    Ok(v)
}

pub fn parse(const_flag: bool, val :u16) -> Option<CpuInstr> {
    if const_flag {
        return Some(CpuInstr::Const(val));
    }
    Some(match val {
        0x0000 => CpuInstr::Halt,
        0x0001 => CpuInstr::Wait,
        0x0002 => CpuInstr::Pause,
        0x0003 => CpuInstr::Resume,
        0x0004 => CpuInstr::SetError,
        0x0005 => CpuInstr::LoadConst,
        0x0006 => CpuInstr::InstrCounter,
        0x0007 => CpuInstr::StackCounter,
        0x0008 => CpuInstr::MoveToStackCounter,
        0x0009 => CpuInstr::Input,
        0x000a => CpuInstr::Output,
        0x000b => CpuInstr::MemRead,
        0x000c => CpuInstr::MemWrite,
        0x0010 => CpuInstr::Move { from: Register::A, to: Register::B },
        0x0011 => CpuInstr::Move { from: Register::B, to: Register::A },
        0x0012 => CpuInstr::Move { from: Register::A, to: Register::C },
        0x0013 => CpuInstr::Move { from: Register::C, to: Register::A },
        0x0014 => CpuInstr::Move { from: Register::B, to: Register::C },
        0x0015 => CpuInstr::Move { from: Register::C, to: Register::B },
        0x0020 => CpuInstr::CpuConst(CpuConst::X0000),
        0x0021 => CpuInstr::CpuConst(CpuConst::X0001),
        0x0022 => CpuInstr::CpuConst(CpuConst::X000E),
        0x0023 => CpuInstr::CpuConst(CpuConst::X000F),
        0x0024 => CpuInstr::CpuConst(CpuConst::X0010),
        0x0030 => CpuInstr::Jump,
        0x0031 => CpuInstr::JumpIfZero,
        0x0032 => CpuInstr::JumpIfNeg,
        0x0033 => CpuInstr::JumpIfNegOrZero,
        0x0034 => CpuInstr::JumpIfOverflow,
        0x0040 | 0x0041 => CpuInstr::AluInstr { instr: AluInstr::NoOp, pass: true },
        0x0042 => CpuInstr::AluInstr { instr: AluInstr::Increment, pass: true },
        0x0043 => CpuInstr::AluInstr { instr: AluInstr::Increment, pass: false },
        0x0044 => CpuInstr::AluInstr { instr: AluInstr::Decrement, pass: true },
        0x0045 => CpuInstr::AluInstr { instr: AluInstr::Decrement, pass: false },
        0x0046 => CpuInstr::AluInstr { instr: AluInstr::Not, pass: true },
        0x0047 => CpuInstr::AluInstr { instr: AluInstr::Not, pass: false },
        0x0048 => CpuInstr::AluInstr { instr: AluInstr::Or, pass: true },
        0x0049 => CpuInstr::AluInstr { instr: AluInstr::Or, pass: false },
        0x004a => CpuInstr::AluInstr { instr: AluInstr::And, pass: true },
        0x004b => CpuInstr::AluInstr { instr: AluInstr::And, pass: false },
        0x004c => CpuInstr::AluInstr { instr: AluInstr::Xor, pass: true },
        0x004d => CpuInstr::AluInstr { instr: AluInstr::Xor, pass: false },
        0x004e => CpuInstr::AluInstr { instr: AluInstr::Add, pass: true },
        0x004f => CpuInstr::AluInstr { instr: AluInstr::Add, pass: false },
        0x0050 => CpuInstr::AluInstr { instr: AluInstr::Subtract, pass: true },
        0x0051 => CpuInstr::AluInstr { instr: AluInstr::Subtract, pass: false },
        0x0052 => CpuInstr::AluInstr { instr: AluInstr::Multiply, pass: true },
        0x0053 => CpuInstr::AluInstr { instr: AluInstr::Multiply, pass: false },
        0x0054 | 0x0055 => CpuInstr::AluInstr { instr: AluInstr::Random, pass: false },
        0x0056 => CpuInstr::AluInstr { instr: AluInstr::ShiftLeftVar, pass: true },
        0x0057 => CpuInstr::AluInstr { instr: AluInstr::ShiftLeftVar, pass: false },
        0x0058 => CpuInstr::AluInstr { instr: AluInstr::ShiftRightVar, pass: true },
        0x0059 => CpuInstr::AluInstr { instr: AluInstr::ShiftRightVar, pass: false },
        0x0062 => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S1), pass: true },
        0x0063 => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S1), pass: false },
        0x0064 => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S2), pass: true },
        0x0065 => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S2), pass: false },
        0x0066 => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S3), pass: true },
        0x0067 => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S3), pass: false },
        0x0068 => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S4), pass: true },
        0x0069 => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S4), pass: false },
        0x006a => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S5), pass: true },
        0x006b => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S5), pass: false },
        0x006c => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S6), pass: true },
        0x006d => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S6), pass: false },
        0x006e => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S7), pass: true },
        0x006f => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S7), pass: false },
        0x0070 => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S8), pass: true },
        0x0071 => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S8), pass: false },
        0x0072 => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S9), pass: true },
        0x0073 => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S9), pass: false },
        0x0074 => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S10), pass: true },
        0x0075 => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S10), pass: false },
        0x0076 => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S11), pass: true },
        0x0077 => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S11), pass: false },
        0x0078 => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S12), pass: true },
        0x0079 => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S12), pass: false },
        0x007a => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S13), pass: true },
        0x007b => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S13), pass: false },
        0x007c => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S14), pass: true },
        0x007d => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S14), pass: false },
        0x007e => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S15), pass: true },
        0x007f => CpuInstr::AluInstr { instr: AluInstr::ShiftLeft(ShiftAmount::S15), pass: false },
        0x0082 => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S1), pass: true },
        0x0083 => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S1), pass: false },
        0x0084 => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S2), pass: true },
        0x0085 => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S2), pass: false },
        0x0086 => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S3), pass: true },
        0x0087 => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S3), pass: false },
        0x0088 => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S4), pass: true },
        0x0089 => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S4), pass: false },
        0x008a => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S5), pass: true },
        0x008b => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S5), pass: false },
        0x008c => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S6), pass: true },
        0x008d => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S6), pass: false },
        0x008e => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S7), pass: true },
        0x008f => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S7), pass: false },
        0x0090 => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S8), pass: true },
        0x0091 => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S8), pass: false },
        0x0092 => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S9), pass: true },
        0x0093 => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S9), pass: false },
        0x0094 => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S10), pass: true },
        0x0095 => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S10), pass: false },
        0x0096 => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S11), pass: true },
        0x0097 => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S11), pass: false },
        0x0098 => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S12), pass: true },
        0x0099 => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S12), pass: false },
        0x009a => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S13), pass: true },
        0x009b => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S13), pass: false },
        0x009c => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S14), pass: true },
        0x009d => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S14), pass: false },
        0x009e => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S15), pass: true },
        0x009f => CpuInstr::AluInstr { instr: AluInstr::ShiftRight(ShiftAmount::S15), pass: false },
        _ => return None,
    })
}