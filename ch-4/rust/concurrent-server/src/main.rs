use std::{os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd}, println};

use nix::{errno::Errno::{self, EINTR}, libc::{EXIT_FAILURE, STDOUT_FILENO, exit, fprintf}, sys::socket::{Backlog, SockaddrIn, accept, bind, listen, socket}, unistd::{ForkResult, close, fork, read, write}};




const PORT : u16 = 8080;




fn main() {

    let address = SockaddrIn::new(0,0,0,0, PORT);
    let socket_fd = create_socket();
    bind(socket_fd.as_raw_fd(), &address).expect("failed to bind to localhost:8080");
    listen(&socket_fd, Backlog::new(5).unwrap()).expect("failed to listen");

    loop {
        let connfd = accept(socket_fd.as_raw_fd()).unwrap();


        
        match unsafe {fork()} {
            Ok(ForkResult::Child) => {
                // Only the parent accepts new connections.
                drop(socket_fd);
                echo_stream(connfd);
                // Exit main instead of entering the accept loop again.
                unsafe { exit(0) };
            },
            Ok(ForkResult::Parent { child }) => {

                close(connfd).expect("failed to close the connection");                    
            },
            Err(_) => {
                println!("failed to fork");
                unsafe { exit(EXIT_FAILURE) };
            }
            // == 0
        }

        
    }


}


fn echo_stream(fd: RawFd){


    let mut buffer = vec![0u8;1024];
    let ofd = unsafe { OwnedFd::from_raw_fd(fd) };


    loop {
        match read(&ofd, &mut buffer){
            Ok(n) => {

                // close the connection
                if n == 0 { return;}

                let mut written = 0 ;

                'write_loop : while written < n {

                    match write(&ofd, &buffer[written..n]){
                        Ok(n) => {
                            if n == 0 {
                               
                                // connection is closed
                                return
                            }
                            written += n;
                        },
                        Err(e) => {
                            
                             if e == EINTR {
                                    continue 'write_loop;
                                } else {
                                    println!("failed to write to the stream");
                                    unsafe { exit(EXIT_FAILURE) };

                                }

                        }
                    }
                }
            },
            Err(err_no) => {
                println!("failed to read from the connection ");
                unsafe { exit(err_no as _) };
            }
        }
    }

}

fn create_socket() -> OwnedFd {
    use nix::sys::socket::{
        SockType::Stream,
        AddressFamily::Inet,
        SockProtocol::Tcp,
        SockFlag
    };

    // crete the socket which will be used for binding the connectijon
    let socket_fd = socket(Inet, Stream, SockFlag::empty(), Tcp).unwrap();

    return socket_fd
}


