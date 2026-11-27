use std::env;

use crate::{config::Config, device::Device, tunnel::Tunnel};

pub mod device;
pub mod command;
pub mod config;
pub mod tunnel;
pub mod error;

fn main() {
    //  to run the app
    // cargo run  -- tun0 10.0.

    let args = env::args().collect::<Vec<String>>();

    let tun_name = args.get(1).expect("tun name not provided");
    let tun_ip = args.get(2).expect("tun ip not provided");
    let tun_mask = args.get(3).expect("tun mask not provided");
    let remote_ip = args.get(4).expect("remote ip not provided");
    let cnf = Config::new(
        tun_name.to_string(),
        tun_ip.trim(),
         tun_mask.trim(),
         remote_ip.trim()
        );
    let  dev = Device::new(cnf);
    let mut tun= Tunnel::new(dev);
    tun.process().expect("failed to run the tun process");
}