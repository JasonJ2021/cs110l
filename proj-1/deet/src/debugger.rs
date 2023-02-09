use crate::debugger_command::DebuggerCommand;
use crate::dwarf_data::{DwarfData, Error as DwarfError};
use crate::inferior::{self, Inferior};
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::collections::HashMap;
use std::fmt::format;

#[derive(Clone)]
struct Breakpoint {
    addr: usize,
    orig_byte: u8,
}

pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<()>,
    inferior: Option<Inferior>,
    debug_data: DwarfData,
    breakpoints: HashMap<usize, Option<u8>>,
}

impl Debugger {
    /// Initializes the debugger.
    pub fn new(target: &str) -> Debugger {
        // TODO (milestone 3): initialize the DwarfData
        let debug_data = match DwarfData::from_file(target) {
            Ok(val) => val,
            Err(DwarfError::ErrorOpeningFile) => {
                println!("Could not open file {}", target);
                std::process::exit(1);
            }
            Err(DwarfError::DwarfFormatError(err)) => {
                println!("Could not debugging symbols from {}: {:?}", target, err);
                std::process::exit(1);
            }
        };
        let history_path = format!("{}/.deet_history", std::env::var("HOME").unwrap());
        let mut readline = Editor::<()>::new().unwrap();
        // Attempt to load history from ~/.deet_history if it exists
        let _ = readline.load_history(&history_path);
        debug_data.print();
        Debugger {
            target: target.to_string(),
            history_path,
            readline,
            inferior: None,
            debug_data: debug_data,
            breakpoints: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        loop {
            match self.get_next_command() {
                DebuggerCommand::Run(args) => {
                    if let Some(inferior) = Inferior::new(&self.target, &args) {
                        // Create the inferior
                        if self.inferior.is_some() {
                            let prev_proc = self.inferior.as_mut().unwrap();
                            prev_proc.try_kill();
                            self.inferior = None;
                        }
                        self.inferior = Some(inferior);
                        // TODO (milestone 1): make the inferior run
                        // You may use self.inferior.as_mut().unwrap() to get a mutable reference
                        // to the Inferior object
                        let inferior = self.inferior.as_mut().unwrap();
                        // inject breakpoints
                        for (addr, prev_byte) in self.breakpoints.clone() {
                            // 如果已经插入breakpoints，直接跳过
                            if prev_byte.is_some() {
                                continue;
                            }
                            let prev_byte = inferior
                                .write_byte(addr, 0xcc)
                                .expect("Errors: When setting breakpoint at {breakpoint}");
                            self.breakpoints.insert(addr, Some(prev_byte));
                        }
                        let status = inferior
                            .continue_exec(&self.breakpoints, &self.debug_data)
                            .expect("nix::error");

                        match status {
                            inferior::Status::Stopped(signal, rip) => {
                                let message = format!("Child stopped (signal {})", signal);
                                Debugger::report_message(&message);

                                let line = self.debug_data.get_line_from_addr(rip);
                                if let Some(line) = line {
                                    let message = format!("Stopped at {}", line);
                                    Debugger::report_message(&message);
                                }
                            }
                            inferior::Status::Exited(code) => {
                                let message = format!("Child exited (status {})", code);
                                Debugger::report_message(&message);
                            }
                            inferior::Status::Signaled(signal) => {
                                let message = format!("signaled by {}", signal);
                                Debugger::report_message(&message);
                            }
                        }
                    } else {
                        Debugger::report_message(&"Error starting subprocess".to_string());
                    }
                }
                DebuggerCommand::Quit => {
                    if self.inferior.is_some() {
                        let prev_proc = self.inferior.as_mut().unwrap();
                        prev_proc.try_kill();
                        self.inferior = None;
                    }
                    return;
                }
                DebuggerCommand::Continue => match &self.inferior {
                    Some(_) => {
                        let inferior = self.inferior.as_mut().unwrap();
                        let status = inferior
                            .continue_exec(&self.breakpoints, &self.debug_data)
                            .expect("nix::error");
                        match status {
                            inferior::Status::Stopped(signal, rip) => {
                                let message = format!("Child stopped (signal {})", signal);
                                Debugger::report_message(&message);
                                let line = self.debug_data.get_line_from_addr(rip);
                                if let Some(line) = line {
                                    let message = format!("Stopped at {}", line);
                                    Debugger::report_message(&message);
                                }
                            }
                            inferior::Status::Exited(code) => {
                                let message = format!("Child exited (status {})", code);
                                Debugger::report_message(&message);
                            }
                            inferior::Status::Signaled(signal) => {
                                let message = format!("signaled by {}", signal);
                                Debugger::report_message(&message);
                            }
                        }
                    }
                    None => println!("The program is not running currently!"),
                },
                DebuggerCommand::BackTrace => {
                    if self.inferior.is_some() {
                        self.inferior
                            .as_ref()
                            .unwrap()
                            .print_backtrace(&self.debug_data)
                            .unwrap();
                    }
                }
                DebuggerCommand::Break(mut addr) => {
                    if addr.starts_with("*0x") {
                        addr.remove(0);
                        let addr = Debugger::parse_address(&addr).unwrap();
                        Debugger::record_breakpoint(addr, &mut self.inferior, &mut self.breakpoints);
                    }else {
                        match Debugger::parse_address(&addr) {
                            Some(addr) => {
                                // get a line 
                                if let Some(addr) = self.debug_data.get_addr_for_line(None, addr) {
                                    Debugger::record_breakpoint(addr, &mut self.inferior, &mut self.breakpoints);
                                }else{
                                    let message = format!("No such line {}" , addr);
                                    Debugger::report_message(&message);
                                }
                            }
                            None => {
                                if let Some(addr) = self.debug_data.get_addr_for_function(None, &addr){
                                    // if self.breakpoints.contains_key(&addr) {
                                    //     // 如果已经插入了这个breakPoints，直接跳过
                                    //     let message = format!("BreakPoint {:#x} has been added ", addr);
                                    //     Debugger::report_message(&message);
                                    // } else {
                                    //     let message =
                                    //         format!("Set breakpoint {} at {:#x}", self.breakpoints.len(), addr);
                                    //     Debugger::report_message(&message);
                                    //     if self.inferior.is_some() {
                                    //         let inferior = self.inferior.as_mut().unwrap();
                                    //         let prev_byte = inferior
                                    //             .write_byte(addr, 0xcc)
                                    //             .expect("Errors: When setting breakpoint at {breakpoint}");
                                    //         self.breakpoints.insert(addr, Some(prev_byte));
                                    //     } else {
                                    //         self.breakpoints.insert(addr, None);
                                    //     }
                                    // }
                                    Debugger::record_breakpoint(addr, &mut self.inferior, &mut self.breakpoints);
                                }else{
                                    let message = format!("No such function {}" , addr);
                                    Debugger::report_message(&message);
                                }
                            }
                        }
                    }
                    
                }
            }
        }
    }
    
    fn record_breakpoint(addr :usize , inferior : &mut Option<Inferior> , breakpoints : &mut HashMap<usize, Option<u8>> ){
        if breakpoints.contains_key(&addr) {
            // 如果已经插入了这个breakPoints，直接跳过
            let message = format!("BreakPoint {:#x} has been added ", addr);
            Debugger::report_message(&message);
        } else {
            let message =
                format!("Set breakpoint {} at {:#x}", breakpoints.len(), addr);
            Debugger::report_message(&message);
            if inferior.is_some() {
                let inferior = inferior.as_mut().unwrap();
                let prev_byte = inferior
                    .write_byte(addr, 0xcc)
                    .expect("Errors: When setting breakpoint at {breakpoint}");
                breakpoints.insert(addr, Some(prev_byte));
            } else {
                breakpoints.insert(addr, None);
            }
        }
    }

    /// This function prompts the user to enter a command, and continues re-prompting until the user
    /// enters a valid command. It uses DebuggerCommand::from_tokens to do the command parsing.
    ///
    /// You don't need to read, understand, or modify this function.
    fn get_next_command(&mut self) -> DebuggerCommand {
        loop {
            // Print prompt and get next line of user input
            match self.readline.readline("(deet) ") {
                Err(ReadlineError::Interrupted) => {
                    // User pressed ctrl+c. We're going to ignore it
                    println!("Type \"quit\" to exit");
                }
                Err(ReadlineError::Eof) => {
                    // User pressed ctrl+d, which is the equivalent of "quit" for our purposes
                    return DebuggerCommand::Quit;
                }
                Err(err) => {
                    panic!("Unexpected I/O error: {:?}", err);
                }
                Ok(line) => {
                    if line.trim().len() == 0 {
                        continue;
                    }
                    self.readline.add_history_entry(line.as_str());
                    if let Err(err) = self.readline.save_history(&self.history_path) {
                        println!(
                            "Warning: failed to save history file at {}: {}",
                            self.history_path, err
                        );
                    }
                    let tokens: Vec<&str> = line.split_whitespace().collect();
                    if let Some(cmd) = DebuggerCommand::from_tokens(&tokens) {
                        return cmd;
                    } else {
                        println!("Unrecognized command.");
                    }
                }
            }
        }
    }

    fn parse_address(addr: &str) -> Option<usize> {
        let addr_without_0x = if addr.to_lowercase().starts_with("0x") {
            &addr[2..]
        } else {
            &addr
        };
        usize::from_str_radix(addr_without_0x, 16).ok()
    }
    fn report_message(message: &String) {
        println!();
        println!("==================Begin======================");
        println!("{message}");
        println!("===================End=======================");
        println!();
    }
}
