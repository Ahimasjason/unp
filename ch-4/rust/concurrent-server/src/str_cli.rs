use std::{
    io,
    os::fd::{AsFd, AsRawFd, OwnedFd},
};

use crate::readline::read_line;
use nix::{
    libc::{EXIT_FAILURE, exit},
    unistd::close,
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
