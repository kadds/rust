pub use super::unsupported::{
    ChildPipe, Command, CommandArgs, EnvKey, ExitCode, ExitStatus, ExitStatusError, Process, Stdio,
    output, read_output,
};

pub fn getpid() -> u32 {
    crate::sys::pal::naos::getpid()
}
