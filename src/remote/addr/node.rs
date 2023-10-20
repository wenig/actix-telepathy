use std::hash::{Hash, Hasher};
use std::net::SocketAddr;

use actix::Addr;
use serde::{Deserialize, Serialize};

use crate::{AddrRepresentation, NetworkInterface, RemoteAddr};

#[derive(Deserialize, Serialize, Debug)]
pub struct Node {
    pub socket_addr: SocketAddr,
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub network_interface: Option<Addr<NetworkInterface>>,
}

impl Node {
    pub fn new(socket_addr: SocketAddr, network_interface: Option<Addr<NetworkInterface>>) -> Self {
        Self {
            socket_addr,
            network_interface,
        }
    }

    pub fn get_remote_addr(&self, id: String) -> RemoteAddr {
        RemoteAddr {
            node: self.clone(),
            id: AddrRepresentation::Key(id),
        }
    }
}

impl Clone for Node {
    fn clone(&self) -> Self {
        Node::new(self.socket_addr, self.network_interface.clone())
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.socket_addr.eq(&other.socket_addr)
    }
}

impl Eq for Node {}

impl Hash for Node {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.socket_addr.hash(state);
    }
}
