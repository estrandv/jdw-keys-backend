use std::net::{SocketAddrV4, UdpSocket};
use rosc::OscPacket;
use rosc::encoder;

pub struct OscClient {
    socket: UdpSocket,
    target_addr: SocketAddrV4
}

impl OscClient {

    pub fn new(socket: UdpSocket, target_addr: SocketAddrV4) -> OscClient {
        OscClient {
            socket,
            target_addr
        }
    }

    pub fn send(&mut self, packet: OscPacket) {
        let msg_buf = encoder::encode(&packet).unwrap();
        self.socket.send_to(&msg_buf, self.target_addr).unwrap();
    }
}