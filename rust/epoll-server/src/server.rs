use std::os::fd::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd};

use libc::{
    EPOLL_CLOEXEC, EPOLL_CTL_ADD, EPOLL_CTL_DEL, EXIT_FAILURE, epoll_ctl, epoll_event, epoll_wait, exit,
};
use nix::{
    errno::Errno, fcntl::{
        self,
        FcntlArg::{self, F_GETFL},
        OFlag,
    }, sys::socket::{Backlog, SockProtocol, SockaddrIn, accept, bind, getpeername, getsockname, listen, socket}, unistd::{self, close, read},
};

struct SocketFd(OwnedFd);

impl SocketFd {
    fn bind_local_8080(&self) {
        let address = SockaddrIn::new(127, 0, 0, 1, 8080);
        bind(self.0.as_raw_fd(), &address).expect("failed to bind to the local addres");
    }
}

pub struct EpollServer {
    epoll_fd: OwnedFd,
    sock_fd: SocketFd,
}

impl EpollServer {
    pub fn new() -> Self {
        let epoll_fd = Self::epoll_create_server();

        EpollServer {
            epoll_fd,
            sock_fd: Self::create_socket(),
        }
    }

    fn epoll_create_server() -> OwnedFd {
        let fd = unsafe { libc::epoll_create1(EPOLL_CLOEXEC) };

        if -1 == fd {
            let err = std::io::Error::last_os_error();
            eprintln!("failed to create the epoll instance: {}", err);
            unsafe { exit(EXIT_FAILURE) };
        }

        unsafe { OwnedFd::from_raw_fd(fd) }
    }

    fn create_socket() -> SocketFd {
        use nix::sys::socket::{AddressFamily, SockFlag, SockType};

        let sfd = socket(
            AddressFamily::Inet,
            SockType::Stream,
            SockFlag::empty(),
            SockProtocol::Tcp,
        )
        .expect("failed to create hte socket");
        SocketFd(sfd)
    }

    fn add_to_epoll(&self, fd: i32) -> i32 {
        let mut event = epoll_event {
            events: (libc::EPOLLIN | libc::EPOLLERR | libc::EPOLLOUT) as u32,
            u64: fd as u64,
        };
        unsafe { epoll_ctl(self.epoll_fd.as_raw_fd(), EPOLL_CTL_ADD, fd, &raw mut event) }
    }

    fn remove_from_epoll(&self, fd: i32) {
        if -1
            == unsafe {
                epoll_ctl(
                    self.epoll_fd.as_raw_fd(),
                    EPOLL_CTL_DEL,
                    fd,
                    std::ptr::null_mut(),
                )
            }
        {
            eprintln!("failed to remove the fd {}, error {}", fd, Errno::last());
            unsafe { exit(EXIT_FAILURE) }
        }
    }

    fn make_non_blocking(&self, fd: i32) {
        let flags = fcntl::fcntl(unsafe { BorrowedFd::borrow_raw(fd) }, F_GETFL)
            .expect("failed to execte the fnctl GETFL");

        let flags = OFlag::from_bits(flags).unwrap() | OFlag::O_NONBLOCK;

        fcntl::fcntl(
            unsafe { BorrowedFd::borrow_raw(fd) },
            FcntlArg::F_SETFL(flags),
        )
        .expect("failed to set non blocking");
    }

    fn accept_loop(&self) {
        loop {
            match accept(self.sock_fd.0.as_raw_fd()) {
                Ok(conn_id) => {
                    // add conn to epoll
                    self.make_non_blocking(conn_id);
                    if self.add_to_epoll(conn_id) < 0 {
                        let socket =
                            getpeername::<SockaddrIn>(conn_id).expect("failed to get socket addr");
                            // getpeername(fd)
                        eprintln!(
                            "failed to add connection to epoll: {}:{}",
                            socket.ip(),
                            socket.port()
                        );
                    }
                    let socket =
                            getpeername::<SockaddrIn>(conn_id).expect("failed to get socket addr");
                    println!("connection established {}", socket);
                }
                Err(Errno::EAGAIN) => {
                    // there io nothing to accept, break the loop and wait for epoll events
                    break;
                }
                Err(Errno::EINTR) => {
                    continue;
                }
                Err(err) => {
                    eprintln!("failed to accept connection: {}", err.desc());
                    unsafe { exit(EXIT_FAILURE) };
                }
            }
        }
    }

    pub fn run_server(self) {
        let epfd = self.epoll_fd.as_raw_fd();
        let mut events = vec![epoll_event { events: 0, u64: 0 }; 1024];
        let sockfd = self.sock_fd.0.as_raw_fd();
        let mut buffer = [0u8; 4 * 1024];
        self.sock_fd.bind_local_8080();
        listen(&self.sock_fd.0, Backlog::MAXCONN).expect("failed to listen");
        self.make_non_blocking(sockfd);
        self.add_to_epoll(sockfd);
        loop {
            let n = unsafe { epoll_wait(epfd, events.as_mut_ptr(), events.len() as _, -1) };

            if n < 0 {
                let err = std::io::Error::last_os_error();
                eprintln!("epoll_wait failed: {}", err);
                unsafe { exit(EXIT_FAILURE) };
            }

            for event in &events[0..n as usize] {
                if event.u64 == sockfd as u64 {
                    self.accept_loop();
                    continue;
                }

                // handle other events for existing connections
                match event.events as _ {
                    libc::EPOLLIN => {
                        // handle read event

                        self.handle(event.u64 as i32, &mut buffer);
                    }
                    _ => {}
                }
            }
        }
    }

    fn handle(&self, fd: i32, buffer: &mut [u8]) {
        let bfd = unsafe { BorrowedFd::borrow_raw(fd) };

        loop {
            match read(&bfd, buffer) {
                Ok(0) => {
                    self.close_conn(fd);
                                    let sock = getsockname::<SockaddrIn>(fd)
                                        .expect("failed to get the socket");
                                    eprintln!("EOF received {}", sock);
                                    return;
                }
                Ok(n) => {
                    let mut nw = 0;
                    while nw < n {
                        match unistd::write(&bfd, &buffer[nw..n]) {
                            Ok(n) => {
                                if n == 0 {
                                    break;
                                }
                                nw += n;
                            }
                            Err(Errno::EAGAIN) => continue, // write buffer is full. we will come to that later . EPOLLOUT
                            Err(Errno::EINTR) => continue,
                            Err(err) => {
                                // remove the item from the ctl
                                self.close_conn(fd);
                                let sock = getsockname::<SockaddrIn>(fd)
                                    .expect("failed to get the socket");
                                eprintln!("write failed {}\n, error {}", sock, err.desc());
                                return;
                            }
                        }
                    }
                }
                Err(Errno::EAGAIN) => {
                    break;
                }
                Err(Errno::EINTR) => {
                    continue;
                }
                Err(err) => {
                    eprintln!("failed to read from fd {}: {}", fd, err.desc());
                    unsafe { exit(EXIT_FAILURE) };
                }
            }
        }
    }

    fn close_conn(&self, fd: i32) {
        let _ = close(fd);
        self.remove_from_epoll(fd);
    }
}
