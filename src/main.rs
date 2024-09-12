use std;
use std::{io, fs, path, env, ffi, thread};
use std::thread::JoinHandle;
use std::time::Duration;
use crate::cpu::{Cpu};

mod cpu;
mod parse;
mod util;

pub const SOURCE_FILE_EXTENSION: &str = ".instr";
pub const COMPILED_FILE_EXTENSION: &str = ".ember";

fn setup(input: Vec<u8>) -> Result<Cpu, String> {
    let mut code = vec![0u16; input.len() >> 1];

    for (i, v) in input.iter().enumerate().step_by(2) {
        code[i >> 1] = ((*v as u16) << 8) | (input[i + 1] as u16);
    }

    Cpu::new(code, cpu::BUILTIN_SUBROUTINES.to_vec())
}

fn main() {
    let args = Vec::from_iter(env::args());
    let args: Vec<&str> = args.iter().map(|v| v.as_ref()).collect::<Vec<_>>();

    // comp <inpath> <outpath>
    // run <path>

    if args.len() < 2 {
        eprintln!("Invalid arguments");
        return;
    }
    match args[1] {
        "norm" => {
            if args.len() < 3 || args.len() > 4 {
                eprintln!("Invalid arguments, correct syntax: norm <inpath> [outpath]");
                return;
            }

            let input = match fs::read_to_string(args[2]) {
                Ok(contents) => contents,
                Err(err) => {
                    eprintln!("Failed to read file {}, error: {}", args[2], err);
                    return;
                }
            };

            match parse::expand_lines(input.as_str(), args[2]) {
                Ok(result) => {
                    let new_path;
                    let output_path = if args.len() == 4 {
                        path::Path::new(args[3])
                    } else {
                        let old_path = path::Path::new(args[2]);
                        let file_stem = old_path.file_stem().unwrap_or(ffi::OsStr::new("")).to_str();
                        let file_stem = match file_stem {
                            Some(some) => some,
                            None => {
                                eprintln!("Failed to parse file name: {}", old_path.to_str().unwrap_or("[UNKNOWN PATH]"));
                                return;
                            },
                        };
                        let file_name = format!("{} (normalized){}", file_stem, SOURCE_FILE_EXTENSION);
                        new_path = old_path.with_file_name(file_name.as_str());
                        path::Path::new(new_path.as_os_str())
                    };

                    let output_string = result.join("\n");

                    match fs::write(output_path, output_string) {
                        Ok(_) => {}
                        Err(err) => {
                            eprintln!("Failed to write to output file {}:\n{}", output_path.to_str().unwrap_or("[UNKNOWN PATH]"), err);
                            return;
                        }
                    }

                    println!("Successfully normalized and output to {}", output_path.to_str().unwrap_or("[UNKNOWN PATH]"))
                }
                Err(err) => {
                    eprintln!("Failed to normalize file {}:\n{}", args[2], err);
                    return;
                }
            }
        }
        "comp" => {
            if args.len() < 3 || args.len() > 4 {
                eprintln!("Invalid arguments, correct syntax: comp <inpath> [outpath]");
                return;
            }

            let input = match fs::read_to_string(args[2]) {
                Ok(contents) => contents,
                Err(err) => {
                    eprintln!("Failed to read file {}, error: {}", args[2], err);
                    return;
                }
            };

            match parse::compile(cpu::PROGRAM_START, input.as_str(), args[2]) {
                Ok(result) => {
                    let new_path;
                    let output_path = if args.len() == 4 {
                        path::Path::new(args[3])
                    } else {
                        let old_path = path::Path::new(args[2]);
                        let file_stem = old_path.file_stem().unwrap_or(ffi::OsStr::new("")).to_str();
                        let file_stem = match file_stem {
                            Some(some) => some,
                            None => {
                                eprintln!("Failed to parse file name: {}", old_path.to_str().unwrap_or("[UNKNOWN PATH]"));
                                return;
                            },
                        };
                        let file_name = format!("{}{}", file_stem, COMPILED_FILE_EXTENSION);
                        new_path = old_path.with_file_name(file_name.as_str());
                        path::Path::new(new_path.as_os_str())
                    };

                    let mut bytes = vec![0u8; result.len() * 2];
                    for (i, v) in result.iter().enumerate() {
                        bytes[i * 2] = (v >> 8) as u8;
                        bytes[i * 2 + 1] = (v & 0xff) as u8;
                    }

                    match fs::write(output_path, bytes) {
                        Ok(_) => {}
                        Err(err) => {
                            eprintln!("Failed to write to output file {}:\n{}", output_path.to_str().unwrap_or("[UNKNOWN PATH]"), err);
                            return;
                        }
                    }

                    println!("Successfully compiled and output to {}", output_path.to_str().unwrap_or("[UNKNOWN PATH]"))
                }
                Err(err) => {
                    eprintln!("Failed to compile file {}:\n{}", args[2], err);
                    return;
                }
            }
        },
        "run" => {
            if args.len() != 3 {
                eprintln!("Invalid arguments, correct syntax: run <path>");
                return;
            }

            if args[2].ends_with(SOURCE_FILE_EXTENSION) {
                let input = match fs::read_to_string(args[2]) {
                    Ok(contents) => contents,
                    Err(err) => {
                        eprintln!("Failed to read file {}:\n{}", args[2], err);
                        return;
                    }
                };

                match parse::compile(cpu::PROGRAM_START, input.as_str(), args[2]) {
                    Ok(result) => {
                        let mut bytes = vec![0u8; result.len() * 2];
                        for (i, v) in result.iter().enumerate() {
                            bytes[i * 2] = (v >> 8) as u8;
                            bytes[i * 2 + 1] = (v & 0xff) as u8;
                        }

                        let cpu = match setup(bytes) {
                            Ok(cpu) => cpu,
                            Err(err) => {
                                eprintln!("Error setting up cpu emulator:\n{}", err);
                                return;
                            },
                        };
                        run_emulator(cpu, args[2]);
                    }
                    Err(err) => {
                        eprintln!("Failed to compile file {}:\n{}", args[2], err);
                        return;
                    }
                }
            } else if args[2].ends_with(COMPILED_FILE_EXTENSION) {
                let input = match fs::read(args[2]) {
                    Ok(contents) => contents,
                    Err(err) => {
                        eprintln!("Failed to read file {}:\n{}", args[2], err);
                        return;
                    },
                };
                if input.len() & 0x1 > 0 {
                    eprintln!("Invalid compiled data in file {}: byte size {} is uneven!", args[2], input.len());
                    return;
                }

                let cpu = match setup(input) {
                    Ok(cpu) => cpu,
                    Err(err) => {
                        eprintln!("Error setting up cpu emulator:\n{}", err);
                        return;
                    },
                };
                run_emulator(cpu, args[2]);
            } else {
                eprintln!("Unknown input file type: {}", args[2]);
                return;
            }
        },
        arg => {
            eprintln!("Invalid argument '{}'", arg);
        },
    }
}

pub fn run_emulator(mut cpu: Cpu, path: &str) {
    let mut run_thread: Option<JoinHandle<()>> = None;
    let mut run_delay = 0u64;
    let mut auto_info = false;
    loop {
        if let Some(thread) = &mut run_thread {
            if thread.is_finished() {
                run_thread = None;
                println!("[i] Stopped running!");
            } else if cpu.is_running() {
                match cpu.cycle() {
                    Ok(_) => {}
                    Err(err) => {
                        eprintln!("[!] Error cycling CPU:\n  {}", err);
                        return;
                    }
                }
                if auto_info { println!("{}", cpu.registers_info()); }
                if run_delay > 0 { thread::sleep(Duration::from_millis(run_delay)); }
            } else {
                println!("[i] CPU paused");
                println!("[i] CPU info:\n{}", cpu.registers_info());
                run_thread = None;
            }
        } else {
            let mut s = String::new();
            match io::stdin().read_line(&mut s) {
                Ok(_) => {}
                Err(err) => {
                    eprintln!("[!] Error while reading user input:\n  {}", err);
                    return;
                }
            }
            let s = s.trim();

            match s {
                "q" => return,
                "help" => println!(
                    "[i] Available commands:\n    \
                    q                      - exits the process\n    \
                    s                      - steps the cpu one cycle\n    \
                    dir                    - prints the working directory of the cpu\n    \
                    i                      - prints cpu info\n    \
                    ti                     - toggle automatically printing info after commands\n    \
                    run [delay]            - run the cpu continuously with an optional delay (in milliseconds) between each cycle (exit by pressing any key)\n    \
                    mem                    - prints the entire emulator memory\n    \
                    sec <section>          - prints a section of the emulator memory\n    \
                    prog                   - prints the contents of the program memory section\n    \
                    stack                  - prints the contents of the stack memory section\n    \
                    get <address>          - gets the value at the specified address\n    \
                    get <register>         - gets the value of the specified register\n    \
                    set <address> <value>  - sets the value at the specified address\n    \
                    set <register> <value> - sets the value of the specified register\n    \
                    do <instruction>       - executes the given instruction"
                ),
                "s" | "" => {
                    if !cpu.is_running() {
                        eprintln!("[!] Failed to step; CPU is halted!");
                    } else {
                        match cpu.cycle() {
                            Ok(_) => {}
                            Err(err) => {
                                eprintln!("[!] Error stepping CPU:\n  {}", err);
                                return;
                            }
                        }
                    }
                    if auto_info { println!("{}", cpu.registers_info()); }
                }
                "i" => {
                    println!("[!] CPU info:\n{}", cpu.registers_info());
                }
                "ti" => {
                    auto_info = !auto_info;
                    println!("[i] Auto info toggled {}", if auto_info { "on" } else { "off" });
                    if auto_info { if auto_info { println!("{}", cpu.registers_info()) } }
                }
                "dir" => {
                    println!("[i] Current working directory: {}", path);
                }
                "disp" => {

                }
                "prog" => {
                    println!("[i] Program memory dump:");
                    for i in (cpu::PROGRAM_START..cpu::STACK_START).step_by(0x0100) {
                        println!("{}", cpu.partial_mem_dump(i))
                    }
                }
                "stack" => {
                    println!("[i] Stack memory dump:");
                    for i in (cpu::STACK_START..cpu::BUILTIN_START).step_by(0x0100) {
                        println!("{}", cpu.partial_mem_dump(i))
                    }
                }
                "mem" => {
                    println!("[i] Full memory dump:");
                    println!("{}", cpu.mem_dump());
                }
                "run" => {
                    run_delay = 0;
                    run_thread = Some(thread::spawn(|| {
                        let _ = io::stdin().read_line(&mut String::new());
                    }));
                }
                _ => {
                    if s.starts_with("run ") {
                        match util::parse_u64(&s[4..]) {
                            Ok(delay) => {
                                run_delay = delay;
                                run_thread = Some(thread::spawn(|| {
                                    let _ = io::stdin().read_line(&mut String::new());
                                }));
                            },
                            Err(err) => eprintln!("[!] Error parsing 'run' command:\n  {}", err)
                        }
                    } else if s.starts_with("sec ") {
                        match util::parse_u16(&s[4..]) {
                            Ok(section) => {
                                println!("[i] Section {:#06x} memory dump:", section);
                                println!("{}", cpu.partial_mem_dump(section))
                            },
                            Err(err) => eprintln!("[!] Error parsing 'sec' command:\n  {}", err)
                        }
                    } else if s.starts_with("get ") {
                        let arg = &s[4..];
                        match arg {
                            "a" => eprintln!("[i] a = {}", cpu.get_a()),
                            "b" => eprintln!("[i] b = {}", cpu.get_b()),
                            "c" => eprintln!("[i] c = {}", cpu.get_c()),
                            arg => match util::parse_u16(arg) {
                                Ok(address) => match cpu.get_address(address) {
                                    Ok(value) => println!("[i] Value of {:#06x} is {:#06x} = {}", address, value, value),
                                    Err(err) => eprintln!("[!] Error parsing 'get' command:\n  {}", err),
                                }
                                Err(err) => eprintln!("[!] Error parsing 'get' command:\n  {}", err)
                            }
                        }
                    } else if s.starts_with("set ") {
                        let arg = &s[4..];
                        if let Some(second_space) = arg.find(' ') {
                            let first = &arg[..second_space];
                            let second = &arg[(second_space + 1)..];
                            match first {
                                "a" => match util::parse_u16(second) {
                                    Ok(value) => cpu.set_a(value),
                                    Err(err) => eprintln!("[!] Error parsing 'set' command:\n  {}", err)
                                }
                                "b" => match util::parse_u16(second) {
                                    Ok(value) => cpu.set_b(value),
                                    Err(err) => eprintln!("[!] Error parsing 'set' command:\n  {}", err)
                                }
                                "c" => match util::parse_u16(second) {
                                    Ok(value) => cpu.set_c(value),
                                    Err(err) => eprintln!("[!] Error parsing 'set' command:\n  {}", err)
                                }
                                first => match util::parse_u16(first) {
                                    Ok(address) => match util::parse_u16(&arg[(second_space + 1)..]) {
                                        Ok(value) => match cpu.set_address(address, value) {
                                            Ok(_) => println!("[i] Set value of {:#06x} to {:#06x} = {}", address, value, value),
                                            Err(err) => eprintln!("[!] Error parsing 'get' command:\n  {}", err),
                                        }
                                        Err(err) => eprintln!("[!] Error parsing 'set' command:\n  {}", err)
                                    },
                                    Err(err) => eprintln!("[!] Error parsing 'set' command:\n  {}", err)
                                }
                            }
                        } else {
                            eprintln!("[!] Too few arguments, correct syntax: set <address|register> <value>")
                        }
                        if auto_info { println!("{}", cpu.registers_info()); }
                    } else if s.starts_with("do ") {
                        let text = &s[3..];
                        match parse::compile(cpu.get_instr_counter(), text, path) {
                            Ok(words) => {
                                for word in words {
                                    match parse::parse(cpu.get_const_flag(), word) {
                                        Some(instr) => match cpu.exec(instr) {
                                            Ok(_) => {}
                                            Err(err) => {
                                                eprintln!("[!] Error running instruction {}:\n  {}", text, err);
                                            }
                                        }
                                        None => {
                                            eprintln!("[!] Error parsing instruction '{}', compiled into {:#06x}", text, word);
                                            return;
                                        }
                                    }
                                }
                                if auto_info { println!("{}", cpu.registers_info()); }
                            }
                            Err(err) => {
                                eprintln!("[!] Error compiling instruction '{}'\n  {}", text, err);
                            }
                        }
                    } else {
                        println!("[!] Unknown command '{}'", s);
                    }
                }
            }
        }
    }
}
