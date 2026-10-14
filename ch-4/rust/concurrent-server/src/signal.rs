use std::ffi::{c_int, c_void};

use nix::{
    errno,
    libc::{STDOUT_FILENO, siginfo_t, write},
    sys::{
        signal::{SaFlags, SigAction, SigHandler, SigSet, Signal, sigaction},
        wait::{WaitPidFlag, WaitStatus, waitpid},
    },
    unistd::Pid,
};

// sigaction handler for SIGCHLD
pub extern "C" fn reap_child_handler(_num: c_int, _siginfo: *mut siginfo_t, _ctx: *mut c_void) {
    let err = errno::Errno::last();
    let flags = WaitPidFlag::WNOHANG;

    loop {
        match waitpid(Pid::from_raw(-1), Some(flags)) {
            Ok(WaitStatus::StillAlive) => break,
            Err(nix::errno::Errno::EINTR) => continue,
            Err(_) => break,
            _ => continue,
        }
    }
    
    
    unsafe  {
        let message = "reaped the childrens";
        write(STDOUT_FILENO, message.as_ptr().cast(), message.len());
    }
    // let _ = write(STDOUT_FILENO, "childrens reaped".as_bytes());
    err.set();
}

pub fn install_signal() {
    let handler = SigHandler::SigAction(reap_child_handler);
    // restart the sys call
    let flags = SaFlags::SA_RESTART;
    // block no other signal
    let set = SigSet::empty();
    let action = SigAction::new(handler, flags, set);
    unsafe { sigaction(Signal::SIGCHLD, &action) }.expect("failed to se tthe signal");
}
