use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

use nix::{
    errno::Errno,
    libc::{EXIT_FAILURE, exit},
    sys::socket::{Backlog, SockFlag, SockProtocol, SockaddrIn, accept, bind, listen, socket},
    unistd::{self, ForkResult},
};

use crate::socket::create_socket;

pub use crate::write::make_write;

const PORT: u16 = 8080;


pub fn select_server() {
    let sockfd = create_server();
    


}





fn create_server() -> OwnedFd {
    // create tcp socket

    let sockfd = create_socket();

    // create the socket address
    let sock_addr = SockaddrIn::new(0, 0, 0, 0, PORT);

    bind(sockfd.as_raw_fd(), &sock_addr).expect("failed to bind to the address");

    listen(&sockfd, Backlog::new(5).unwrap()).unwrap();
    sockfd
}
