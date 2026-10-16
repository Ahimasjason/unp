use std::os::fd::AsRawFd;

use concurrent_server::socket::create_socket;
use concurrent_server::str_cli::{cli_client, cli_client_multplex_1, str_cli_multiplex2};
use nix::{
    sys::socket::{SockaddrIn, connect}
};

const PORT: u16 = 8080;

fn main() -> nix::Result<()> {
    // create the tcp socket
    let socket = create_socket();
   connect(socket.as_raw_fd(), &SockaddrIn::new(127,0,0,1, PORT))?;
    // cli_client(socket);
    // cli_client_multplex_1(socket);
    str_cli_multiplex2(socket);
    Ok(())
}
