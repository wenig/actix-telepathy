use actix::prelude::*;
use std::net::SocketAddr;
use crate::network::NetworkInterface;
use crate::codec::JoinCluster;

pub struct RemoteAddr {
    socket: SocketAddr,
    network_interface: Addr<NetworkInterface>
}

impl RemoteAddr {
    pub fn new(socket: SocketAddr, network_interface: Addr<NetworkInterface>) -> RemoteAddr {
        RemoteAddr{socket, network_interface}
    }

    pub fn send(&mut self, msg: String) {
        let msg = JoinCluster::Message(msg);
        self.network_interface.send(msg);
    }
}

impl ToString for RemoteAddr {
    fn to_string(&self) -> String {
        self.socket.to_string()
    }
}