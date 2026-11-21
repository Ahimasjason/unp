use std::os::fd::{AsRawFd, OwnedFd};

use nix::{libc::{EXIT_FAILURE, exit}, sys::socket::{MsgFlags, SockFlag, SockaddrIn, bind, recvfrom, sendto, socket}};





pub fn create_udp_server() -> OwnedFd{


    // create the socket 

    let sockfd = socket(
            nix::sys::socket::AddressFamily::Inet ,
            nix::sys::socket::SockType::Datagram,
            SockFlag::empty(),
             Some(nix::sys::socket::SockProtocol::Udp)
    )
    .expect("Failed to create the socket for the udp");

    // issue will happen when we have multihomed interface
    if let Err(err) = bind(sockfd.as_raw_fd(),&SockaddrIn::new(127,0,0,1, 8080) ) {
        eprintln!("failed to biund the socket. error {}", err.desc());
        unsafe { exit(EXIT_FAILURE) }
    }
    sockfd

}


pub fn run_server(

){

    let sockfd = create_udp_server();

    let mut buffer = [0u8; 8 * 1024];
    
    loop {

        
        match recvfrom::<SockaddrIn>(sockfd.as_raw_fd(), &mut buffer){
            Ok((n, sock_addr)) => {

                if let Some(sockaddr) = sock_addr {
                    if let Err(err) = sendto(sockfd.as_raw_fd(), &buffer[..n], &sockaddr, MsgFlags::empty()) {
                        eprintln!("err  sendto {}", err.desc());
                    }
                }
            },
            Err(err) => {
                eprintln!("err recvfrom {}", err.desc());
            }
        }
    }
}


/// UDP server with msg_hdr
/// 
fn udp_server_msg_hdr(){

}