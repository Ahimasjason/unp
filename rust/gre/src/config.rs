use std::net::Ipv4Addr;



pub struct Config {

    pub ifname : String,
    pub address : Ipv4Addr,
    pub mask : Ipv4Addr,
    pub remote_address : Ipv4Addr,
}

impl Config {

    pub fn new(ifname :String, ifaddress : &str, mask : &str, remote_address: &str) -> Self {


        let ifaddr : Ipv4Addr = ifaddress.parse().expect("invalid ip address");
        let mask : Ipv4Addr = mask.parse().expect("invalid ip address"); 
        let remote_address : Ipv4Addr = remote_address.parse().expect("invalid ip address");
        Self { ifname: ifname.to_string(), address: ifaddr, mask, remote_address }
    }
}


