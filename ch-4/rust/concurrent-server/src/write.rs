use std::os::fd::AsFd;

use nix::{errno::Errno, unistd};






pub fn make_write<FD: AsFd>(fd : &FD, buf  : &[u8]) -> nix::Result<usize> {
    let mut written = 0;
    let len = buf.len();
    while written < len {
        match unistd::write(fd, &buf[written..]){
            // if we continue it possible to go on infinite loop .
            // now the caller can re call incase if something is pending to write
            Ok(0) => break,
            Err(Errno::EINTR) => continue,
            Err(Errno::EAGAIN) => return  Ok(written),
            Ok(n) => written += n,
            Err(e) => return Err(e),
        }
    }
    Ok(written)
}