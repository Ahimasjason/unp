use std::os::fd::OwnedFd;

use nix::sys::socket::socket;

pub fn create_socket() -> OwnedFd {
    use nix::sys::socket::{AddressFamily::Inet, SockFlag, SockProtocol::Tcp, SockType::Stream};

    // crete the socket which will be used for binding the connectijon
    let socket_fd = socket(Inet, Stream, SockFlag::empty(), Tcp).unwrap();

    return socket_fd;
}
