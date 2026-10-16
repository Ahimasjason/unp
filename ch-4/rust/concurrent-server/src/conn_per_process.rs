use std::{os::fd::{AsRawFd, FromRawFd, OwnedFd}};

use nix::{errno::Errno, libc::{EXIT_FAILURE, exit}, sys::socket::{Backlog, SockFlag, SockProtocol, SockaddrIn, accept, bind, listen, socket}, unistd::{self, ForkResult}};

use crate::signal::install_signal;

pub use crate::write::make_write;


const PORT : u16 = 8080;



pub fn conn_per_process(){

    install_signal();
    let sockfd = create_server();

    accept_connection(sockfd);

    
}




fn accept_connection( sockfd : OwnedFd) {

    loop {
        match  accept(sockfd.as_raw_fd()){
            Ok(connid) => {
                // handle the connection
                match unsafe { unistd::fork() } {
                    Ok(ForkResult::Child) => {
                        let ownedfd_conn = unsafe { OwnedFd::from_raw_fd(connid) };
                        drop(sockfd);
                        handle_connection(ownedfd_conn);
                        return;
                    },
                    Ok(ForkResult::Parent { child: _c }) => {
                        unistd::close(connid).expect("failed to close the connection ID");
                    },
                    Err(e) => {
                        println!("failed to fork, error {:?}", e.desc());
                        unsafe { exit(EXIT_FAILURE) };
                    }
                }
                
            },
            Err(nix::errno::Errno::EINTR) => {
                continue;
            },
            Err(err) => {
                println!("accept return error {:?}", err.desc());
                unsafe { exit(EXIT_FAILURE) };
            }
        }
    }

    // sigfillset(set);
}


fn handle_connection(connid : OwnedFd){


    let mut buffer = [0u8;1024];

    
    loop {
        
        // let mut remaining = 5;
        // while remaining > 0 {
        //     remaining = unistd::sleep(remaining);

        // }
        
        match unistd::read(&connid, &mut buffer){
            Ok(0) => { 
                unistd::close(connid).expect("failed to close conn");
                return;
            }
            Ok(n) => {
                // while written < n {

                match make_write(&connid, &buffer[..n]){
                        Ok(n) => {
                            println!("wrote {} bytes", n); 
                        },

                        Err(e) => {
                            unistd::close(connid).expect("failed to close the connection");
                            println!("error {}", e.desc());
                            unsafe { exit(EXIT_FAILURE) }
                        }
                    // }
                }
            },
            Err(Errno::EINTR) => continue,
            Err(err) =>{
                unistd::close(connid).expect("failed to close the connection");
                eprintln!("{}", err.desc());
                unsafe { exit(EXIT_FAILURE) };
            },
        }
    }
}


fn create_server()  -> OwnedFd{
    // create tcp socket 

    let sockfd = socket(
        nix::sys::socket::AddressFamily::Inet,
        nix::sys::socket::SockType::Stream,
        SockFlag::empty(),
        SockProtocol::Tcp
    ).expect("failed to create the socket");

    // create the socket address
    let sock_addr = SockaddrIn::new(0,0,0,0, PORT);

    bind(sockfd.as_raw_fd(),&sock_addr).expect("failed to bind to the address");

    listen(&sockfd, Backlog::new(5).unwrap()).unwrap();
    sockfd
}