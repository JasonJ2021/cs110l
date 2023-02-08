use crate::debugger_command::DebuggerCommand;
use crate::dwarf_data::{DwarfData, Error as DwarfError};
use crate::inferior::{self, Inferior};
use rustyline::error::ReadlineError;
use rustyline::Editor;

pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<()>,
    inferior: Option<Inferior>,
    debug_data: DwarfData,
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

        Debugger {
            target: target.to_string(),
            history_path,
            readline,
            inferior: None,
            debug_data,
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
                        let status = inferior.continue_exec().expect("nix::error");
                        match status {
                            inferior::Status::Stopped(signal, _) => {
                                println!("Child stopped (signal {})", signal);
                                let line = inferior.get_execline(&self.debug_data).ok().unwrap();
                                println!("Stopped at {}", line);
                            }
                            inferior::Status::Exited(code) => {
                                println!("Child exited (status {})", code)
                            }
                            inferior::Status::Signaled(signal) => {
                                println!("signaled by {}", signal)
                            }
                        }
                    } else {
                        println!("Error starting subprocess");
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
                        let status = inferior.continue_exec().expect("nix::error");
                        match status {
                            inferior::Status::Stopped(signal, _) => {
                                println!("Child stopped (signal {})", signal);
                                let line = inferior.get_execline(&self.debug_data).ok().unwrap();
                                println!("Stopped at {}", line);
                            }
                            inferior::Status::Exited(code) => {
                                println!("Child exited (status {})", code)
                            }
                            inferior::Status::Signaled(signal) => {
                                println!("signaled by {}", signal)
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
}
