use std::os::fd::{AsRawFd, OwnedFd};

use nix::sys::socket::{Backlog, SockaddrIn, bind, listen, socket};

pub fn create_socket() -> OwnedFd {
    use nix::sys::socket::{AddressFamily::Inet, SockFlag, SockProtocol::Tcp, SockType::Stream};

    // crete the socket which will be used for binding the connectijon
    let socket_fd = socket(Inet, Stream, SockFlag::empty(), Tcp).unwrap();

    return socket_fd;
}

const PORT : u16 = 8080;

pub fn run_local_server()  -> OwnedFd {

    let sockfd = create_socket();
    // create the socket address
    let sock_addr = SockaddrIn::new(0,0,0,0, PORT);

    bind(sockfd.as_raw_fd(),&sock_addr).expect("failed to bind to the address");

    listen(&sockfd, Backlog::new(5).unwrap()).unwrap();
    sockfd
}
