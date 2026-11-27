// create the /dev/net/tun
// make it as a virtual n/w dev using ioctl

use std::{
    fs::{File, OpenOptions}, io, net::Ipv4Addr, os::fd::{AsFd, AsRawFd},
};

use nix::libc;

use crate::config::Config;

nix::ioctl_write_ptr_bad!(tun_set_iff, libc::TUNSETIFF, libc::ifreq);

pub struct Device {
    file: File,
    conf: Config,
    is_up: bool,
}

impl Device {
    pub fn new(conf: Config) -> Device {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/net/tun")
            .expect("failed to open /dev/net/tun");

        Device {
            file,
            conf,
            is_up: false,
        }
    }

    fn create_iface(&mut self) -> nix::Result<()> {
        // create the zero size struct
        let mut iface: libc::ifreq = unsafe { std::mem::zeroed() };
        iface.ifr_ifru.ifru_flags = (libc::IFF_TUN | libc::IFF_NO_PI) as _;

        if self.conf.ifname.len() > libc::IFNAMSIZ as _ {
            panic!("interface name is gt than max size")
        }
        for (i, &c) in self.conf.ifname.as_bytes().iter().enumerate() {
            iface.ifr_name[i] = c as _;
        }

        unsafe { tun_set_iff(self.file.as_raw_fd(), &iface)? };

        Ok(())
    }

    fn set_ip_address(&mut self) -> io::Result<()> {
        let prefix = mask_prefix(&self.conf.mask)?;
        let addr = self.conf.address;
        let address = format!("{addr}/{prefix}");
        crate::command::run_command("ip", &["addr", "add", &address, "dev", &self.conf.ifname])
    }

    fn up_link(&mut self) -> io::Result<()> {
        let dev_name = format!("{}", self.conf.ifname);
        crate::command::run_command("ip", &["link", "set", "dev", &dev_name, "up"])
    }

    pub fn set_up(&mut self) {
        if !self.is_up {
            self.create_iface().expect("failed to create the interface");
            self.set_ip_address().expect("failed to set the address");
            self.up_link().expect("failed to up the link");
            self.is_up = true;
        }
    }

    pub fn get_incoming_address(&self) -> Ipv4Addr {
        self.conf.address
    }

    pub fn get_remote_address(&self) -> Ipv4Addr {
        self.conf.remote_address
    }
}

impl io::Read for Device {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.file.read(buf)
    }
}


impl io::Write for Device {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

fn mask_prefix(mask: &Ipv4Addr) -> io::Result<u32> {
    let bits = u32::from(*mask);
    let prefix = bits.leading_ones();
    if bits != u32::MAX.checked_shl(32 - prefix).unwrap_or(0) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid mask"));
    }
    Ok(prefix)
}


impl AsFd for Device {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        self.file.as_fd()
    }
}