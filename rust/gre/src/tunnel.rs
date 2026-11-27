use std::{
    io::{self, ErrorKind, IoSlice, Read, Write},
    net::{Ipv4Addr, SocketAddrV4},
    os::fd::{AsFd, AsRawFd, BorrowedFd},
};

use nix::{
    errno::Errno,
    libc::IPPROTO_GRE,
    sys::select::{FdSet, select},
};
use pnet::packet::{
    Packet,
    gre::{GrePacket, MutableGrePacket},
    ip::IpNextHeaderProtocols,
    ipv4::Ipv4Packet,
};
use socket2::{Domain, MsgHdr, Protocol, SockAddr, Socket, Type};

use crate::{
    device::Device,
    error::{self, Result},
};

const GRE_IPV4_PROTO: u16 = 0x0800;

pub struct Tunnel {
    dev: Device,
    tun_buffer: [u8; 0xFFFF], // mah jumbo frame will come around <9000bytes
    socket: Socket,
    socket_buffer: [u8; 0xFFFF],
}

impl Tunnel {
    pub fn new(dev: Device) -> Self {
        Self {
            dev,
            tun_buffer: [0u8; 0xFFFF],
            socket: Self::build_raw_gre_socket(),
            socket_buffer: [0u8; 0xFFFF],
        }
    }

    fn build_raw_gre_socket() -> Socket {
        let sock = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::from(IPPROTO_GRE)))
            .expect("failed to create the socket");
        sock
    }

    fn read(&mut self) -> io::Result<usize> {
        self.dev.read(&mut self.tun_buffer)
    }

    fn read_from_socket(&mut self) -> io::Result<usize> {
        self.socket.read(&mut self.socket_buffer)
    }
    fn write_to_dev<W: Write>(buf: &[u8], dev: &mut W) -> io::Result<usize> {
        loop {
            match dev.write(buf) {
                Ok(n) => {
                    println!("wrote {} bytes to the dev", n);
                    return Ok(n);
                }
                //  i should handle the would block i have put this request in the queue and hand ver to event loop but for now i will just ignore it and continue
                Err(err)
                    if matches!(
                        err.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    continue;
                }
                Err(err) => {
                    return Err(err);
                }
            }
        }
    }

    pub fn process(&mut self) -> Result<()> {
        self.dev.set_up();
        let tunfd = self.dev.as_fd().as_raw_fd();
        let sockfd = self.socket.as_fd().as_raw_fd();
        loop {
            let mut read_set = FdSet::new();
            read_set.insert(unsafe { BorrowedFd::borrow_raw(tunfd) });
            read_set.insert(unsafe { BorrowedFd::borrow_raw(sockfd) });

            match select(None, Some(&mut read_set), None, None, None) {
                Ok(n) => {
                    if read_set.contains(self.socket.as_fd()) {
                        match self.read_from_socket() {
                            Ok(0) => {}
                            Ok(n) => {
                                // it usually read one packet
                                match Self::incoming_process(
                                    &self.socket_buffer[..n],
                                    &mut self.dev,
                                ) {
                                    Ok(_) => {}
                                    Err(err) => {
                                        println!("error processing incoming packet: {}", err);
                                    }
                                }
                            }
                            Err(err)
                                if matches!(
                                    err.kind(),
                                    ErrorKind::Interrupted | ErrorKind::WouldBlock
                                ) => {}
                            Err(err) => return Err(err.into()),
                        }
                    }

                    if read_set.contains(self.dev.as_fd()) {
                        match self.read() {
                            Ok(0) => {
                                break;
                            }
                            Ok(n) => match self.process_out_goining(&self.tun_buffer[..n]) {
                                Ok(_) => {}
                                Err(err) => {
                                    println!("error processing outgoing packet: {}", err);
                                }
                            },
                            Err(err)
                                if matches!(
                                    err.kind(),
                                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                                ) =>
                            {
                                continue;
                            }
                            Err(err) => {
                                panic!("{}", err);
                            }
                        }
                    }
                }
                Err(Errno::EINTR) => continue,
                Err(err) => {
                    return Err(crate::error::Error::from(io::Error::from_raw_os_error(
                        err as i32,
                    )));
                }
            }
        }
        println!("done reading the dev, signing off");
        Ok(())
    }

    // packets coming from the socket and send to the tun device
    //  check if the packet is gre encapsulated

    fn incoming_process<W: Write>(payload: &[u8], dev: &mut W) -> Result<()> {
        match Ipv4Packet::new(payload) {
            Some(packet) => {
                //  IP | GRE | IP
                let inner_protocol: pnet::packet::ip::IpNextHeaderProtocol =
                    packet.get_next_level_protocol();
                if inner_protocol != IpNextHeaderProtocols::Gre {
                    println!("not a gre packet, dropping");
                    return Err(crate::error::Error::IncorrectGREPacket);
                }

                match GrePacket::new(packet.payload()) {
                    Some(gre_packet) => {
                        // check the inner protocol
                        if !Self::validate_gre_packet(&gre_packet) {
                            return Err(crate::error::Error::IncorrectGREPacket);
                        }

                        let inner_payload = gre_packet.payload();
                        loop {
                            //  tun works on the principle of writing the whole packet at once, if the packet is not written completely it will be dropped, so i will keep trying to write until the whole packet is written
                            if inner_payload.len()
                                == Self::write_to_dev(inner_payload, dev)
                                    .map_err(|err| crate::error::Error::IoError(err))?
                            {
                                break;
                            }
                        }

                        Ok(())
                    }
                    None => {
                        println!("not a gre packet, dropping");
                        Err(crate::error::Error::NotAGrePacket)
                    }
                }
            }
            None => return Err(crate::error::Error::IncorrectIPV4Packet),
        }
    }

    fn parse_and_validate_ipv4(payload: &[u8]) -> Result<Ipv4Packet<'_>> {
        match Ipv4Packet::new(payload) {
            Some(packet) => {
                if packet.get_version() != 4 {
                    return Err(crate::error::Error::NotAIpv4Packet);
                }

                let header_len = packet.get_header_length() as usize * 4;
                let totoal_len = packet.get_total_length() as usize;
                if header_len < 20
                    || totoal_len < header_len
                    || totoal_len > payload.len()
                    || header_len > payload.len()
                {
                    return Err(crate::error::Error::IncorrectIPV4Packet);
                }
                Ok(packet)
            }
            None => Err(crate::error::Error::IncorrectIPV4Packet),
        }
    }

    fn process_out_goining(&self, payload: &[u8]) -> Result<()> {
        let _ = self.send_gre_4_packet(Self::parse_and_validate_ipv4(payload)?);
        Ok(())
    }
    fn validate_gre_packet(packet: &GrePacket<'_>) -> bool {
        packet.get_protocol_type() == GRE_IPV4_PROTO
            && packet.get_version() == 0
            && packet.get_checksum_present() == 0
            && packet.get_routing_present() == 0
            && packet.get_key_present() == 0
            && packet.get_sequence_present() == 0
            && packet.get_strict_source_route() == 0
            && packet.get_recursion_control() == 0
            && packet.get_zero_flags() == 0
    }
    /**
     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
     |C|R|K|S|s|Recur|  Flags  | Ver |         Protocol Type         |
     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
     |      Checksum (optional)      |       Offset (optional)       |
     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
     |                         Key (optional)                        |
     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
     |                    Sequence Number (optional)                 |
     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
     |                         Routing (optional)
     +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

    Bit position	Field	                               Size
            0	    Checksum Present (C)	                1 bit
            1	    Routing Present (R), legacy	            1 bit
            2	    Key Present (K)	                        1 bit
            3	    Sequence Present (S)	                1 bit
            4	    Strict Source Route, legacy	            1 bit
            5–7	    Recursion Control, legacy	            3 bits
            8–12	Reserved Flags	                        5 bits
            13–15	Version	                                3 bits
            16–31	Protocol Type	                        16 bits


    | Optional field | Size     | Condition |
    |---             |---:      |---        |
    | Checksum       | 2 bytes  | C = 1 |
    | Reserved1      | 2 bytes  | C = 1 |
    | Key            | 4 bytes  | K = 1 |
    | Sequence Number | 4 bytes | S = 1 |
    | Routing Information | Variable | R = 1 (legacy) |

    */

    fn send_gre_4_packet(&self, packet: Ipv4Packet) -> Result<usize> {
        let mut gre_packet = vec![0u8; MutableGrePacket::minimum_packet_size()];
        let mut gre = MutableGrePacket::new(&mut gre_packet).unwrap();
        gre.set_protocol_type(GRE_IPV4_PROTO);
        let dest_address: SockAddr = SocketAddrV4::new(self.dev.get_remote_address(), 0).into();
        let bufs = [IoSlice::new(gre.packet()), IoSlice::new(packet.packet())];
        let msg = MsgHdr::new().with_addr(&dest_address).with_buffers(&bufs);
        let send = self
            .socket
            .sendmsg(&msg, 0)
            .map_err(|e| crate::error::Error::from(e))?;
        Ok(send)
    }
}

#[cfg(test)]
mod tests {
    use pnet::packet::{Packet, gre::MutableGrePacket};

    use super::*;

    #[test]
    fn test_build_gre() {
        let mut pack = vec![0u8; MutableGrePacket::minimum_packet_size()];
        let mut gre = MutableGrePacket::new(&mut pack).unwrap();
        gre.set_protocol_type(GRE_IPV4_PROTO);
        let packet = gre.packet();
        assert_eq!([0, 0, 8, 0], packet);
    }
}
