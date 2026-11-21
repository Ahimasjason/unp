// use nix::libc::getaddrinfo;
use pnet::packet::{
    Packet, icmp::{
        self, IcmpPacket, IcmpType, echo_reply::{EchoReplyPacket}, echo_request::MutableEchoRequestPacket,
    }, ipv4::Ipv4Packet,
};
use socket2::{Domain, MaybeUninitSlice, MsgHdrMut, Protocol, Socket, Type};
use std::{
    env, io::ErrorKind, mem::MaybeUninit, net::{SocketAddr, ToSocketAddrs}, os::fd::{AsRawFd, FromRawFd, RawFd}, process, slice, sync::{Arc, Barrier, atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed}}, thread, time::{Duration, SystemTime, UNIX_EPOCH},
};

static TOTAL_SEND : AtomicUsize = AtomicUsize::new(0);
static TOTAL_RECVD : AtomicUsize = AtomicUsize::new(0);

static  SHUTDOWN : AtomicBool = AtomicBool::new(false);


fn main() {

    ctrlc::set_handler(|| {
        SHUTDOWN.store(true, Relaxed);
    })
    .expect("failed to set the signal handler");

    let host = get_host();
    // let host = "google.com".to_string();
    let host_addr = host_serv(&host);
    //
    
    let socket = create_raw_socket();
    socket.set_read_timeout(Some(Duration::from_millis(100))).unwrap();

    println!("PING {} ({})", host, host_addr);
    let barrier = Arc::new(Barrier::new(3));
    
    thread::spawn({
        let barrier = Arc::clone(&barrier);
        let sock_addr = host_addr.clone();
        let sock = socket.try_clone().unwrap();
        move || send_loop(sock, sock_addr, barrier)
    });
    thread::spawn({
        let barrier = Arc::clone(&barrier);
        let sock_addr = host_addr.clone();
        let sock = socket.try_clone().unwrap();
        move || recv_loop(sock, sock_addr, barrier)
    });
    barrier.wait();
    let total_send = TOTAL_SEND.load(Relaxed);
    let total_recv = TOTAL_RECVD.load(Relaxed);
    let lost = total_send.saturating_sub(total_recv);
    println!(
        "Total Send = {}\n\
        Total Rcvd = {}\n\
        Lost = {}", total_send, total_recv, lost
    );
    // socket goes out of the scope there it will be cleand up
}


fn send_loop(sock_fd: Socket, sock_addr: SocketAddr, barrier : Arc<Barrier>){
    let mut seq_num = 0u16;
    let  ident = process::id() as u16;
    let socket = sock_fd;
    loop {
        if SHUTDOWN.load(Relaxed){break;}
        let ts = unix_timestamp();
        let packet = build_icmp_packet(seq_num, ident, &ts);
        match socket.send_to(&packet, &sock_addr.into()){
            Ok(_) => {
                TOTAL_SEND.fetch_add(1, Relaxed);
                thread::sleep(Duration::from_secs(1));
                seq_num += 1;
            },
            Err(err) if matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted) => continue,
            Err(err) => {
                println!("failed to send the packet, error {}", err);
                SHUTDOWN.store(true, Relaxed);
                break
            }
         }
    }
    barrier.wait();
}

fn recv_loop(sock_fd: Socket, sock_addr: SocketAddr, barrier : Arc<Barrier>) {
    let socket = sock_fd;
    let mut recv_buffer = [MaybeUninit::<u8>::uninit(); 1500 as usize];
    // let buff = MsgHdrMut::
    let mut sock_add = sock_addr.into();
    loop {
        if SHUTDOWN.load(Relaxed){break;}
        let mut buffer = [MaybeUninitSlice::new(&mut recv_buffer)];
        let mut msg = MsgHdrMut::new()
            .with_addr(&mut sock_add)
            .with_buffers(&mut buffer);

        match socket.recvmsg(&mut msg, 0) {
            Ok(r) => {
                if msg.flags().is_truncated() {
                    println!("messgae truncated");
                } else {
                    let packet =
                        unsafe { slice::from_raw_parts(recv_buffer.as_ptr().cast::<u8>(), r) };
                    let ip = Ipv4Packet::new(packet).unwrap();
                    if is_valid(&ip) {
                        if process_packet(&ip){
                            TOTAL_RECVD.fetch_add(1, Relaxed);
                        }
                    }
                }
            }
            Err(err) if matches!(err.kind(), ErrorKind::Interrupted | ErrorKind::WouldBlock )=> {
                continue;
            },
            Err(err) => {
                println!("failed to send message {}", err);
                SHUTDOWN.store(true, Relaxed);
                break;
            }
        }
    }
    barrier.wait();
}

fn is_valid(packet: &Ipv4Packet<'_>) -> bool {
    let header_len = packet.get_header_length() as usize * 4;
    // This 16-bit field defines the entire packet size in bytes, including header and data
    let total_len = packet.get_total_length();
    packet.get_version() == 4
        && header_len >= 20
        && total_len  >= header_len as _
        // The header claims more bytes than your buffer contains
        && total_len <= packet.packet().len() as _
}
// PING www.google.com (216.239.57.99): 56 data bytes
// 64 bytes from 216.239.57.99: seq=0, ttl=53, rtt=5.611 ms

fn process_packet<'a>(ipv4: &Ipv4Packet<'_>) -> bool {
    if ipv4.get_next_level_protocol() != pnet::packet::ip::IpNextHeaderProtocols::Icmp {
        println!("not icmp packet");
        return false;
    }

    if let Some(ec_replay) = EchoReplyPacket::new(ipv4.payload()) {
        if ec_replay.get_icmp_type() != pnet::packet::icmp::IcmpTypes::EchoReply{
            println!("not a echo reply");
            return false
        }
        let seq = ec_replay.get_sequence_number();
        if process::id() as u16 != ec_replay.get_identifier(){
            println!("not our process id");
            return false;
        }

        let ttl = ipv4.get_ttl();
        let sa = ipv4.get_source();
        let mut ts_buffer = [0u8; 8];
        let icmp_payload = ec_replay.payload();
        if icmp_payload.len() != 8 {
            return false;
        }
        // first 8 byte will be ts;
        ts_buffer.copy_from_slice(icmp_payload);
        let ts = to_unix_ts(&ts_buffer);
        let Some(rtt) = unix_ts().checked_sub(ts) else {
            return false;
        };
        println!(
            "\
    {} bytes from {}: seq={}, ttl={}, rtt={} ms",
            ipv4.payload().len(),
            sa,
            seq,
            ttl,
            rtt.as_millis()
        );
        return true
    }
    false
}

fn unix_timestamp() -> [u8; 8] {
    let systime = unix_ts();
    let mico = (systime.as_micros() as u64).to_be_bytes();
    mico
}

fn unix_ts() -> Duration {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
}
fn to_unix_ts(by: &[u8; 8]) -> Duration {
    Duration::from_micros(u64::from_be_bytes(by.clone()))
}

fn host_serv(address: &str) -> std::net::SocketAddr {
    let address = (address, 0);
    let mut iter = address.to_socket_addrs().unwrap();
    let addr = iter.find(|ip| ip.is_ipv4()).unwrap();
    addr
}

// get the host
fn get_host() -> String {
    env::args().nth(1).expect("failed to get the url")
}

// create the socket to send the raw packets
fn create_raw_socket() -> Socket {
    let socket = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4)).unwrap();
    // socket.set_header_included_v4(true).expect("failed to set the IP_HDRINCL on");
    socket
}

// https://www.rfc-editor.org/info/rfc792/

// 0  Echo Reply
// 3  Destination Unreachable
// 4  Source Quench
// 5  Redirect
// 8  Echo
// 11 Time Exceeded
// 12 Parameter Problem
// 13 Timestamp
// 14 Timestamp Reply
// 15 Information Request
// 16 Information Reply

// type = 8 -> ECHO Request
// code = 0
fn build_icmp_packet(seq: u16, ident: u16, data: &[u8]) -> Vec<u8> {
    const TYPE: usize = 1; // 1 byte 
    const CODE: usize = 1;
    const CHECKSUM: usize = 2;
    const IDENT: usize = 2;
    const SEQ: usize = 2;
    let mut bytes = vec![0u8; TYPE + CODE + CHECKSUM + IDENT + SEQ + data.len()];
    let mut echo_req = MutableEchoRequestPacket::new(&mut bytes).unwrap();
    echo_req.set_icmp_type(IcmpType::new(8));
    echo_req.set_icmp_code(pnet::packet::icmp::IcmpCode(0));
    echo_req.set_identifier(ident);
    echo_req.set_sequence_number(seq);
    echo_req.set_payload(data);
    let icmp = IcmpPacket::new(echo_req.packet()).unwrap();
    let cs = icmp::checksum(&icmp);
    echo_req.set_checksum(cs);
    bytes
}

#[cfg(test)]
mod tests {
    use std::net::ToSocketAddrs;

    use super::*;

    #[test]
    fn test_google_addr() {
        let address = ("google.com", 0);
        let mut iter = address.to_socket_addrs().unwrap();
        let addr = iter.find(|ip| ip.is_ipv4()).unwrap();
        println!("addr {:?}", addr);
    }

    use pnet::packet::{
        Packet,
        icmp::{self, IcmpCode, IcmpPacket, IcmpTypes, echo_reply::MutableEchoReplyPacket},
        ip::IpNextHeaderProtocols,
        ipv4::{self, Ipv4Packet, MutableIpv4Packet},
    };
    use std::{net::Ipv4Addr, time::Duration};

    fn dummy_echo_reply(timestamp: [u8; 8], identifier: u16, sequence: u16) -> [u8; 36] {
        let mut bytes = [0u8; 36];

        // ICMP Echo Reply: 8-byte header + 8-byte timestamp.
        {
            let mut reply = MutableEchoReplyPacket::new(&mut bytes[20..]).unwrap();

            reply.set_icmp_type(IcmpTypes::EchoReply);
            reply.set_icmp_code(IcmpCode(0));
            reply.set_identifier(identifier);
            reply.set_sequence_number(sequence);
            reply.set_payload(&timestamp);

            let checksum = icmp::checksum(&IcmpPacket::new(reply.packet()).unwrap());
            reply.set_checksum(checksum);
        }

        // IPv4 header without options.
        {
            let mut ip = MutableIpv4Packet::new(&mut bytes).unwrap();

            ip.set_version(4);
            ip.set_header_length(5);
            ip.set_total_length(36);
            ip.set_identification(1);
            ip.set_ttl(64);
            ip.set_next_level_protocol(IpNextHeaderProtocols::Icmp);
            ip.set_source(Ipv4Addr::new(192, 0, 2, 1));
            ip.set_destination(Ipv4Addr::new(192, 0, 2, 2));

            let checksum = ipv4::checksum(&ip.to_immutable());
            ip.set_checksum(checksum);
        }

        bytes
    }


    #[test]
    fn test_icmp_process(){
        let d = dummy_echo_reply(unix_timestamp(),2,0);
        let ip = Ipv4Packet::new(&d).unwrap();
        process_packet(&ip);
    }
}
