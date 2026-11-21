use std::{io::Write, os::fd::{AsRawFd, BorrowedFd, RawFd}};

use concurrent_server::{readline::read_line, write::make_write};
use nix::{errno::Errno, libc::{EXIT_FAILURE, STDIN_FILENO, exit}, sys::socket::{AddressFamily::Inet, MsgFlags, SockFlag, SockProtocol, SockaddrIn, SockaddrLike, recvfrom, sendto, socket}, unistd::write};

fn main() {
    let sockfd = {
        socket(
            Inet,
            nix::sys::socket::SockType::Datagram,
            SockFlag::empty(),
            SockProtocol::Udp,
        ).expect("failed to create the socket")
        
    };

    // 10.61.112.40  
    // let server_address = SockaddrIn::new(10, 61, 112, 42, 8080);
    
    let server_address = SockaddrIn::new(127, 0, 0, 1, 8080);

    let mut sendline = Vec::new();
    let mut recvline = [0u8; 1024 * 4];
    let stdinfd = unsafe {
        BorrowedFd::borrow_raw(STDIN_FILENO)
    };

    let stdout = unsafe {
        BorrowedFd::borrow_raw(nix::libc::STDOUT_FILENO)
    };
    
    loop {

        match read_line(&stdinfd, &mut sendline){
            Ok(0) => break,
            Ok(n) => {
                
                Sendto(sockfd.as_raw_fd(), &sendline[0..n], &server_address, MsgFlags::empty());                   
                match recvfrom::<SockaddrIn>(sockfd.as_raw_fd(), &mut recvline[..] ){
                    Ok((n, sockadd)) => {
                        if let Some(sockaddr) = sockadd {
                            print!("{}>> ", sockaddr);
                            std::io::stdout().flush().expect("failed to flush");
                        }
                        make_write(&stdout, &recvline[..n]).expect("failed to write to std out");
                    },
                    Err(e) => {
                        // incase of EINTER continue it or 
                        eprintln!("failed to recv {}", e.desc());
                    }
                }
                println!("done recvfrom");

            },
            Err(err) => {
                eprintln!("failed to read from the stdin, error {}", err.desc());
                unsafe { exit(EXIT_FAILURE) }
            }
        }
    }
}



fn Sendto( 
        fd : RawFd,
        buf : &[u8],
        addr : &dyn SockaddrLike,
        flags : MsgFlags
){

    let mut nw = 0;
    let len = buf.len();
    while nw < buf.len() {

        match sendto(fd, &buf[nw..len], addr, flags){
            Ok(n) => {
                nw += n;
            },
            Err(Errno::EINTR) => continue,
            Err(err) => {
                eprintln!("failed to send to the udp , {}", err.desc());
                unsafe { exit(EXIT_FAILURE) }
            }
        }
    }
 
}

