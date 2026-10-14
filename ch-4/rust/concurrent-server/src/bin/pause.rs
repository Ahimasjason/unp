
use std::sync::atomic::AtomicIsize;

use nix::{libc, sys::signal::{SigSet, SigmaskHow, Signal, signal, sigprocmask}, unistd::{self, sleep}};

static COUNT : AtomicIsize = AtomicIsize::new(0);

extern "C" fn handler(num : libc::c_int ){
    COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    println!("received the signal");
}

fn main() {


    // work_and_mask();
    incr_signal();
    

}


fn incr_signal() {

    unsafe { signal(Signal::SIGINT, nix::sys::signal::SigHandler::Handler(handler)).expect("faield to set the signal") };    // sigprocmask(how, set, oldset)
    


    loop {
        if COUNT.load(std::sync::atomic::Ordering::Relaxed) >= 10 {
            break;
        }
        println!("entering into the loop");
        // sleep(5);
        unistd::pause();
        
    }
    println!("total received {} ", COUNT.load(std::sync::atomic::Ordering::Relaxed) );

}



fn work_and_mask() {


    unsafe { signal(Signal::SIGINT, nix::sys::signal::SigHandler::Handler(handler)).expect("faield to set the signal") };    // sigprocmask(how, set, oldset)

    let mut  sigset = SigSet::empty();
    sigset.add(Signal::SIGINT);
    let mut oldset = SigSet::empty();

    sigprocmask(SigmaskHow::SIG_BLOCK, Some(&sigset), Some(&mut oldset))
        .expect("failed to block the signal");

    println!("blocked the signal");
    sleep(5);

    sigprocmask(SigmaskHow::SIG_SETMASK, Some(&oldset), None).expect("failed to reset");



    println!("--unblocked the signal");
    sleep(5);

}