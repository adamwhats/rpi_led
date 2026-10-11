// Minimal E1.31 (sACN) receiver for one multicast universe. Used instead of the
// sacn crate because that identifies senders by CID, which OpenRGB leaves as
// all zeros, so two OpenRGB instances look like one sender; here every packet
// carries the address it came from.

use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::thread;
use std::time::Duration;

const PORT: u16 = 5568;
const ACN_PACKET_ID: &[u8] = b"ASC-E1.17\0\0\0";
const VECTOR_ROOT_E131_DATA: u32 = 0x4;
const VECTOR_E131_DATA_PACKET: u32 = 0x2;
const VECTOR_DMP_SET_PROPERTY: u8 = 0x2;
const OPTION_PREVIEW_DATA: u8 = 0x80;
const OPTION_STREAM_TERMINATED: u8 = 0x40;

pub struct Packet<'a> {
    pub sender: SocketAddr,
    // DMX channel values, without the start code
    pub dmx: &'a [u8],
    pub terminated: bool,
}

pub struct Receiver {
    socket: UdpSocket,
    universe: u16,
    buf: [u8; 1144],
}

impl Receiver {
    pub fn new(universe: u16) -> Self {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, PORT)).unwrap();
        let group = Ipv4Addr::new(239, 255, (universe >> 8) as u8, universe as u8);
        // At boot the network may not be up yet
        while let Err(e) = socket.join_multicast_v4(&group, &Ipv4Addr::UNSPECIFIED) {
            eprintln!("E1.31: joining {group} failed ({e}), retrying");
            thread::sleep(Duration::from_secs(2));
        }
        Receiver { socket, universe, buf: [0; 1144] }
    }

    // Waits up to `timeout` (which must be non-zero) for a datagram. Returns
    // None on timeout, and for anything that isn't live data for our universe.
    pub fn recv(&mut self, timeout: Duration) -> Option<Packet<'_>> {
        self.socket.set_read_timeout(Some(timeout)).ok()?;
        let (len, sender) = self.socket.recv_from(&mut self.buf).ok()?;
        let (dmx, terminated) = parse(&self.buf[..len], self.universe)?;
        Some(Packet { sender, dmx, terminated })
    }
}

fn parse(p: &[u8], universe: u16) -> Option<(&[u8], bool)> {
    if p.len() < 126 {
        return None;
    }
    let be16 = |i: usize| u16::from_be_bytes([p[i], p[i + 1]]);
    let be32 = |i: usize| u32::from_be_bytes([p[i], p[i + 1], p[i + 2], p[i + 3]]);
    let options = p[112];
    if &p[4..16] != ACN_PACKET_ID
        || be32(18) != VECTOR_ROOT_E131_DATA
        || be32(40) != VECTOR_E131_DATA_PACKET
        || be16(113) != universe
        || p[117] != VECTOR_DMP_SET_PROPERTY
        || p[125] != 0 // DMX start code; anything else isn't level data
        || options & OPTION_PREVIEW_DATA != 0
    {
        return None;
    }
    // The property count includes the start code
    let end = (125 + be16(123) as usize).min(p.len());
    Some((&p[126..end], options & OPTION_STREAM_TERMINATED != 0))
}
