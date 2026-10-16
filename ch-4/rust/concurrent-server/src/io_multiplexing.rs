use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd};

use nix::{
    errno::Errno, libc::{EXIT_FAILURE, STDIN_FILENO, exit}, sys::{select::{FdSet, select}, socket::{Backlog, SockFlag, SockProtocol, SockaddrIn, accept, bind, listen, socket}}, unistd::{self, ForkResult},
};

use crate::{readline::read_line, socket::create_socket};

pub use crate::write::make_write;

const PORT: u16 = 8080;


pub fn select_server() {
    let sockfd = create_server();
    


}



pub fn cli_client_multplex_1(sockfd: OwnedFd) {
    // set up
    let sockfd_fd = sockfd.as_fd();
    let stdin_fd = unsafe { BorrowedFd::borrow_raw(STDIN_FILENO) };
    let stdout_fd   = unsafe {BorrowedFd::borrow_raw(STDIN_FILENO)};
    let mut send_buf = Vec::new();
    let mut recv_buf = Vec::new();
    
    loop {
        let mut fdset = FdSet::new();
        fdset.insert(sockfd_fd);
        fdset.insert(stdin_fd);
        let r = select(None, Some(&mut fdset), None, None, None);
        match r {
            Ok(n) => {
                println!("number of read {}", n);
                if fdset.contains(stdin_fd) {
                    println!("read from stdin");
                    // read from the std in and write to the server
                   
                    let n= read_line(&stdin_fd, &mut send_buf).expect("failed to read");

                    // handle epipe
                    make_write(&sockfd_fd, &send_buf[..n]).expect("failed to send to n/w");

                
                }
                if fdset.contains(sockfd_fd) {
                    println!("read from sockfd");
                    // read from server an write to the client
                    let n= read_line(&sockfd_fd, &mut recv_buf).expect("failed to read");
                    if n == 0 {
                        println!("socket is closed");
                        return;
                    }
                    make_write(&stdout_fd, &recv_buf[..n]).expect("failed to send to n/w");
                    
                }
            }
            Err(Errno::EINTR) => {
                continue;
            },
            Err(err) => {

                println!("select failed, error {}", err.desc());
                unsafe { exit(EXIT_FAILURE) };
            }
        }
    }   
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
