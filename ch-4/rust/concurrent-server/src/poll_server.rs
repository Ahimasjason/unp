use std::os::fd::{AsFd, AsRawFd, BorrowedFd};

use nix::{
    errno::Errno,
    libc::{POLLHUP, exit},
    poll::{PollFd, PollFlags, PollTimeout, poll},
    sys::socket::accept,
    unistd::{self, SysconfVar, close},
};

use crate::socket::run_local_server;

pub fn poll_server() {
    let socket_id = run_local_server();
    let mut pollfds = Vec::new();
    // o is server
    pollfds.push(PollFd::new(socket_id.as_fd(), PollFlags::POLLIN));

    let open_max = unistd::sysconf(SysconfVar::OPEN_MAX)
        .expect("failed to read open max")
        .unwrap_or(1024) as usize;
    let mut buffer = [0u8; 1024 * 4];
    let mut remove_idx = vec![];
    loop {
        match poll(&mut pollfds[..], PollTimeout::NONE) {
            Err(Errno::EINTR) => {}
            Err(e) => {
                println!("poll error, error {}", e.desc());
            }
            Ok(mut n) => {
                remove_idx.clear();
                // there is connection is on the server we have to handle it
                let server_event = &pollfds[0];
                if server_event.any().unwrap()
                    && server_event.revents().unwrap().contains(PollFlags::POLLIN)
                {
                    if pollfds.len() == open_max {
                        eprintln!("too many connections");
                    }

                    if pollfds.len() < open_max {
                        match accept_connection(socket_id.as_raw_fd()) {
                            Ok(Some(id)) => {
                                let raw_id = unsafe { BorrowedFd::borrow_raw(id) };
                                // Safety!
                                // The resource pointed to by fd must remain open for the duration of the returned BorrowedFd.
                                let pfd = PollFd::new(raw_id, PollFlags::POLLIN);
                                pollfds.push(pfd);
                            }
                            Ok(None) => {}
                            Err(e) => {
                                //  close all the connections
                                println!("failed to accept the connection");
                                unsafe { exit(e as _) }
                            }
                        }
                    }
                    n -= 1;
                    if n <= 0 {
                        continue;
                    }
                }

                //  now read from all other socket descriptor
                for idx in 1..pollfds.len() {
                    
                    if pollfds[idx].any().unwrap() {
                        let mut remove_this = false;
                        let client = &pollfds[idx];
                        if let Some(events) = client.revents() {
                            if events.intersects(
                                PollFlags::POLLERR | PollFlags::POLLIN | PollFlags::POLLHUP,
                            ) {
                                match unistd::read(client.as_fd(), &mut buffer) {
                                    Ok(0) | Err(Errno::ECONNRESET) => {
                                        //  Connection closed so remove from pollfds
                                        close(client.as_fd().as_raw_fd())
                                            .expect("failed to close the connection");
                                        // this is inexpensive, there are better sol than this for now
                                        // it is for simple and working purpose
                                        remove_this = true;
                                    }
                                    Ok(n) => {
                                        let mut nw = 0;
                                        while nw < n {
                                            match unistd::write(client.as_fd(), &buffer[nw..n]) {
                                                Ok(0) => break,
                                                Ok(n) => nw += n,
                                                Err(Errno::EINTR) => continue,
                                                Err(err) => {
                                                    println!(
                                                        "failed to write to the connection error {} ",
                                                        err.desc()
                                                    );
                                                    // pollfds.remove(idx);
                                                    remove_this = true;
                                                    //  send FIN also close this
                                                    close(client.as_fd().as_raw_fd())
                                                        .expect("failed to close the connection");
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    Err(Errno::EINTR) => {}
                                    Err(err) => {
                                        println!("failed to read from the client {}", err.desc());
                                        close(client.as_fd().as_raw_fd())
                                            .expect("failed to close the conn");
                                        remove_this = true
                                    }
                                }
                            }
                            n -= 1;
                            if remove_this {
                                remove_idx.push(idx);
                            }
                            if n <= 0 {
                                break;
                            }
                        }
                    }
                }
            }
        }

        for rm in remove_idx.drain(..).rev() {
            pollfds.remove(rm);
        }
    }
}

// if interrupt it will return None, so that next time we can sucessully
// accept the connection
fn accept_connection<FD: AsRawFd>(sockfd: FD) -> nix::Result<Option<i32>> {
    match accept(sockfd.as_raw_fd()) {
        Ok(connid) => return Ok(Some(connid)),
        Err(Errno::EINTR) => return Ok(None),
        Err(err) => return Err(err),
    }
}
