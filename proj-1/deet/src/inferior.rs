use crate::dwarf_data::DwarfData;
use crate::dwarf_data::Line;
use crate::inferior;
use addr2line::gimli::DebugAddrBase;
use nix::sys::ptrace;
use nix::sys::signal;
use nix::sys::signal::Signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::convert::TryInto;
use std::mem::size_of;
use std::os::unix::process::CommandExt;
use std::process::Child;
use std::process::Command;
use std::collections::HashMap;

fn align_addr_to_word(addr: usize) -> usize {
    addr & (-(size_of::<usize>() as isize) as usize)
}
pub enum Status {
    /// Indicates inferior stopped. Contains the signal that stopped the process, as well as the
    /// current instruction pointer that it is stopped at.
    Stopped(signal::Signal, usize),

    /// Indicates inferior exited normally. Contains the exit status code.
    Exited(i32),

    /// Indicates the inferior exited due to a signal. Contains the signal that killed the
    /// process.
    Signaled(signal::Signal),
}

/// This function calls ptrace with PTRACE_TRACEME to enable debugging on a process. You should use
/// pre_exec with Command to call this in the child process.
fn child_traceme() -> Result<(), std::io::Error> {
    ptrace::traceme().or(Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "ptrace TRACEME failed",
    )))
}

pub struct Inferior {
    child: Child,
}

impl Inferior {
    /// Attempts to start a new inferior process. Returns Some(Inferior) if successful, or None if
    /// an error is encountered.
    pub fn new(target: &str, args: &Vec<String>) -> Option<Inferior> {
        // TODO: implement me!
        // 1. create a new Command
        let mut com = Command::new(target);
        com.args(args);
        // 2. pre_exec call child_traceme
        unsafe {
            com.pre_exec(child_traceme);
        }
        let _child = com.spawn().ok()?;
        let inferior = Inferior { child: _child };
        match inferior.wait(None).ok()? {
            Status::Stopped(signal, _) => match signal {
                Signal::SIGTRAP => Some(()),
                _ => None,
            },
            _ => None,
        }?;
        Some(inferior)
    }

    /// Returns the pid of this inferior.
    pub fn pid(&self) -> Pid {
        nix::unistd::Pid::from_raw(self.child.id() as i32)
    }

    /// Calls waitpid on this inferior and returns a Status to indicate the state of the process
    /// after the waitpid call.
    pub fn wait(&self, options: Option<WaitPidFlag>) -> Result<Status, nix::Error> {
        Ok(match waitpid(self.pid(), options)? {
            WaitStatus::Exited(_pid, exit_code) => Status::Exited(exit_code),
            WaitStatus::Signaled(_pid, signal, _core_dumped) => Status::Signaled(signal),
            WaitStatus::Stopped(_pid, signal) => {
                let regs = ptrace::getregs(self.pid())?;
                Status::Stopped(signal, regs.rip as usize)
            }
            other => panic!("waitpid returned unexpected status: {:?}", other),
        })
    }

    pub fn continue_exec(&mut self , breakpoints: &HashMap<usize, Option<u8>> , debug_data: &DwarfData) -> Result<Status, nix::Error> {
        // wake up the proc
        let mut regs = ptrace::getregs(self.pid())?;
        let rip: usize = regs.rip.try_into().expect("get rip failed");
        let addr = rip - 1;
        if breakpoints.contains_key(&addr) {
            // restore prev_byte
            self.write_byte(addr, breakpoints.get(&addr).unwrap().expect("breakpoint should been injected"))?;
            regs.rip = regs.rip - 1;
            let function_name = debug_data.get_function_from_addr(addr).unwrap();
            let line = debug_data.get_line_from_addr(addr).unwrap();
            println!("Breakpoint at {} , {}" , function_name , line);
            println!("============================================");
            ptrace::setregs(self.pid(), regs)?;
            ptrace::step(self.pid(), None)?;
            match self.wait(None)? {
                inferior::Status::Stopped(signal, _) => {
                    assert_eq!(signal , Signal::SIGTRAP);
                }
                inferior::Status::Exited(code) => {
                    println!("Child exited (status {})", code);
                    return Ok(inferior::Status::Exited(code));  
                }
                inferior::Status::Signaled(signal) => {
                    return Ok(inferior::Status::Signaled(signal));
                }
            };
            self.write_byte(addr, 0xcc)?;
        }    
        ptrace::cont(self.pid(), None)?;
        self.wait(None)
    }
    pub fn try_kill(&mut self) {
        if Child::kill(&mut self.child).is_ok() {
            println!("Killing running inferior (pid {})", self.pid());
            self.wait(None).unwrap();
        }
    }
    pub fn print_backtrace(&self, debug_data: &DwarfData) -> Result<(), nix::Error> {
        let mut instruction_ptr: usize = ptrace::getregs(self.pid())?.rip.try_into().unwrap();
        let mut base_ptr: usize = ptrace::getregs(self.pid())?.rbp.try_into().unwrap();
        loop {
            let function_name = debug_data.get_function_from_addr(instruction_ptr).unwrap();
            let line = debug_data.get_line_from_addr(instruction_ptr).unwrap();
            println!("{} ({})", function_name, line);
            if function_name == "main" {
                break;
            }
            instruction_ptr =
                ptrace::read(self.pid(), (base_ptr + 8) as ptrace::AddressType)? as usize;
            base_ptr = ptrace::read(self.pid(), base_ptr as ptrace::AddressType)? as usize;
        }
        Ok(())
    }
    pub fn get_execline(&self, debug_data: &DwarfData) -> Result<Line, nix::Error> {
        let instruction_ptr: usize = ptrace::getregs(self.pid())?.rip.try_into().unwrap();
        let line = debug_data.get_line_from_addr(instruction_ptr).unwrap();
        Ok(line)
    }
    pub fn write_byte(&mut self, addr: usize, val: u8) -> Result<u8, nix::Error> {
        let aligned_addr = align_addr_to_word(addr);
        let byte_offset = addr - aligned_addr;
        let word = ptrace::read(self.pid(), aligned_addr as ptrace::AddressType)? as u64;
        let orig_byte = (word >> 8 * byte_offset) & 0xff;
        let masked_word = word & !(0xff << 8 * byte_offset);
        let updated_word = masked_word | ((val as u64) << 8 * byte_offset);
        unsafe {
            ptrace::write(
                self.pid(),
                aligned_addr as ptrace::AddressType,
                updated_word as *mut std::ffi::c_void,
            )?;
        }

        Ok(orig_byte as u8)
    }
}
