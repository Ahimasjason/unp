use std::os::fd::AsFd;

use nix::{errno::Errno, unistd};





pub fn read_line<FD: AsFd>(fd : &FD, buf : &mut Vec<u8>) -> nix::Result<usize>{
    let mut byte = [0u8;1];
    // making sure to bring the cursor to the begining 
    buf.clear();
    loop {
        match unistd::read(&fd, &mut byte){
            Ok(0) => {
                return Ok(buf.len())
                
            },
            Ok(_n) => {
                buf.push(byte[0]);
                if byte[0] == b'\n' {
                    return Ok(buf.len())
                }
            }
            Err(Errno::EINTR)  => continue,
            // can cause the cpu to use more by polling again and again   
            // | Err(Errno::EAGAIN) => continue,
            Err(err) => {
                // println!("failed to read, error: {}", err.desc());
                return Err(err)
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use std::{io::{Read, Write}, os::unix::net::UnixStream};

    use super::*;

    #[test]
    fn test_read_line() {
        
        let (mut sender, rx) = UnixStream::pair().unwrap();
        let message = b"hello World!\n";
        sender.write_all(message).unwrap();
        drop(sender);

        let mut st = vec![];
        read_line(&rx.as_fd(), &mut st).expect("failed to read the line");
        assert_eq!(&st, message);
    }
}