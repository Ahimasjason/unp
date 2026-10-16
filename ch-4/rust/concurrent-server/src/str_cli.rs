use std::{
    io,
    os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd},
};

use crate::readline::read_line;
use nix::{
    errno::Errno,
    libc::{EXIT_FAILURE, STDIN_FILENO, STDOUT_FILENO, exit},
    sys::{
        select::{FdSet, select},
        socket::{Shutdown, shutdown},
    },
    unistd::{self, close},
};

use super::write::make_write;

// read the line from the `stdin` and write to the server
pub fn cli_client(conn_id: OwnedFd) {
    let mut send_buf = Vec::new();
    let mut recv_buf = Vec::new();

    let sdin = io::stdin();
    let stdout = io::stdout();
    // read from the client

    loop {
        match read_line(&sdin, &mut send_buf) {
            Ok(0) => {
                // close the connection
                return;
            }
            Ok(n) => {
                let hwm = read_write_from_nw(&conn_id, &send_buf[..n], &mut recv_buf);
                make_write(&stdout, &recv_buf[..hwm]).expect("failed to write to the terminal");
            }
            Err(err) => {
                println!("failed to read from the stdin, error {}", err.desc());
                unsafe { exit(EXIT_FAILURE) };
            }
        }
    }
}

fn read_write_from_nw<FD: AsFd>(conn_id: &FD, send_buf: &[u8], recv_buf: &mut Vec<u8>) -> usize {
    make_write(conn_id, send_buf).expect("failed to write to the client");
    // now read the line from the client
    let until = read_line(conn_id, recv_buf).expect("failed to read");
    if until == 0 {
        close(conn_id.as_fd().as_raw_fd()).expect("failed to close the connection");
        println!("EOF from the server");
        unsafe { exit(EXIT_FAILURE) };
    }
    return until;
}

pub fn cli_client_multplex_1(sockfd: OwnedFd) {
    // set up
    let sockfd_fd = sockfd.as_fd();
    let stdin_fd = unsafe { BorrowedFd::borrow_raw(STDIN_FILENO) };
    let stdout_fd = unsafe { BorrowedFd::borrow_raw(STDOUT_FILENO) };
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

                    let n = read_line(&stdin_fd, &mut send_buf).expect("failed to read");

                    // handle epipe
                    make_write(&sockfd_fd, &send_buf[..n]).expect("failed to send to n/w");
                }
                if fdset.contains(sockfd_fd) {
                    println!("read from sockfd");
                    // read from server an write to the client
                    let n = read_line(&sockfd_fd, &mut recv_buf).expect("failed to read");
                    if n == 0 {
                        println!("socket is closed");
                        return;
                    }
                    make_write(&stdout_fd, &recv_buf[..n]).expect("failed to send to n/w");
                }
            }
            Err(Errno::EINTR) => {
                continue;
            }
            Err(err) => {
                println!("select failed, error {}", err.desc());
                unsafe { exit(EXIT_FAILURE) };
            }
        }
    }
}


pub fn str_cli_multiplex2(sockfd: OwnedFd) {
    let sockfd_fd = sockfd.as_fd();
    let stdin_fd = unsafe { BorrowedFd::borrow_raw(STDIN_FILENO) };
    let stdout_fd = unsafe { BorrowedFd::borrow_raw(STDOUT_FILENO) };

    let mut send_buff = [0u8; 1024];
    let mut recv_buff = [0u8; 1024];
    let mut stdin_closed = false;
    loop {
        let mut readset = FdSet::new();

        readset.insert(sockfd_fd);
        if !stdin_closed{
            readset.insert(stdin_fd);
        }
        
        let sr = select(None, Some(&mut readset), None, None, None);

        match sr {
            Err(Errno::EINTR) => continue,
            Err(err) => {
                println!("select error, error {}", err.desc());
                unsafe { exit(err as i32) };
            }
            Ok(0) => continue,
            Ok(_) => {
                // check if there is something to read from the stdin
                if readset.contains(stdin_fd) {
                    // read from the stdin

                    match unistd::read(stdin_fd, &mut send_buff) {
                        Ok(0) => {
                            // read end is closed
                            println!("closing the connection");
                            stdin_closed = true;
                            shutdown(sockfd_fd.as_raw_fd(), Shutdown::Write)
                                .expect("failed to close the connection");
                            readset.remove(stdin_fd);
                        }
                        Ok(n) => {
                            let mut nw = 0;
                            while nw < n {
                                match unistd::write(sockfd_fd, &send_buff[nw..n]) {
                                    Ok(0) | Err(Errno::EINTR) => continue,
                                    Ok(n) => nw += n,
                                    Err(err) => {
                                        println!("server error {}", err.desc());
                                        unsafe { exit(err as _) }
                                    }
                                }
                            }
                        }
                        Err(Errno::EINTR) => continue,
                        Err(e) => {
                            println!("failed to read from std in {}", e.desc());
                            unsafe { exit(e as _ )}
                        }
                    }
                }

                if readset.contains(sockfd_fd) {
                    //  read from the socket

                    match unistd::read(sockfd_fd, &mut recv_buff) {
                        Ok(0) => {
                            if stdin_closed {
                                return;
                            }
                            println!("server closed the connection");
                            unsafe { exit(EXIT_FAILURE) };
                        }
                        Ok(n) => {
                            // now write
                            let mut nw = 0;
                            while nw < n {
                                match unistd::write(stdout_fd, &recv_buff[nw..n]) {
                                    Ok(0) => break, 
                                    Err(Errno::EINTR) => continue,
                                    Ok(n) => nw += n,
                                    Err(err) => {
                                        println!("failed to write to terminal {}", err.desc());
                                        unsafe { exit(err as _) }
                                    }
                                }
                            }
                        }
                        Err(Errno::EINTR) => continue,
                        Err(e) => {
                            println!("failed to read from socket {}", e.desc());
                            unsafe { exit(e as _ )}
                        }
                    }
                }
            }
        }
    }
}
