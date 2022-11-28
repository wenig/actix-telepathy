use std::net::SocketAddr;
use std::hash::{Hash, Hasher};

use actix::Addr;
use serde::{Deserialize, Serialize};

use crate::{NetworkInterface, RemoteAddr, AddrRepresentation};

#[derive(Deserialize, Serialize, Debug)]
pub struct Node {  // or ClusterNode?
    pub socket_addr: SocketAddr,
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub network_interface: Option<Addr<NetworkInterface>>,
}

impl Node {
    pub fn new(
        socket_addr: SocketAddr,
        network_interface: Option<Addr<NetworkInterface>>
    ) -> Self {
        Self {
            socket_addr,
            network_interface,
        }
    }

    pub fn into_remote_addr(self) -> RemoteAddr {  // Maybe then we don't need this method
        RemoteAddr {
            node: self,
            id: AddrRepresentation::NetworkInterface,
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
        Node::new(
            self.socket_addr.clone(),
            self.network_interface.clone(),
        )
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