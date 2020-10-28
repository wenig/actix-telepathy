use log::*;
use actix::prelude::*;
use std::net::SocketAddr;
use crate::network::NetworkInterface;
use crate::codec::ClusterMessage;
use crate::remote::{RemoteMessage, AddrRepresentation};
use crate::Sendable;
use std::str::FromStr;
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use std::borrow::Borrow;


#[derive(Deserialize, Serialize)]
pub struct RemoteAddr {
    socket_addr: String,
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    network_interface: Option<Addr<NetworkInterface>>,
    pub(crate) id: AddrRepresentation
}

impl RemoteAddr {
    pub fn new(socket_addr: String, network_interface: Option<Addr<NetworkInterface>>, id: AddrRepresentation) -> Self {
        RemoteAddr{socket_addr, network_interface, id}
    }

    pub fn new_gossip(socket_addr: String, network_interface: Option<Addr<NetworkInterface>>) -> Self {
        RemoteAddr::new(socket_addr, network_interface, AddrRepresentation::Gossip)
    }

    pub fn set_network_interface(&mut self, network_interface: Addr<NetworkInterface>) {
        self.network_interface = Some(network_interface);
    }

    pub fn do_send<T: Sendable>(&mut self, msg: Box<T>) -> () {
        self.network_interface.as_ref().expect("Cannot send to None!").do_send(ClusterMessage::Message(
            RemoteMessage::new(self.clone(), msg)
        ));
    }

    fn to_string_remote_addr(&self) -> StringRemoteAddr {
        StringRemoteAddr {socket_addr: self.socket_addr.clone(), id: self.id.to_string()}
    }
}

impl ToString for RemoteAddr {
    fn to_string(&self) -> String {
        self.to_string_remote_addr().to_string()
    }
}

impl FromStr for RemoteAddr {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let string_remote_addr = StringRemoteAddr::from_str(s).unwrap();
        Ok(RemoteAddr::new(string_remote_addr.socket_addr,
                           None,
                           AddrRepresentation::from_str(string_remote_addr.id.as_str()).unwrap()))
    }
}

impl Clone for RemoteAddr {
    fn clone(&self) -> Self {
        RemoteAddr::new( self.socket_addr.clone(), self.network_interface.clone(), self.id.clone())
    }
}


#[derive(Serialize, Deserialize)]
struct StringRemoteAddr {
    socket_addr: String,
    id: String
}

impl ToString for StringRemoteAddr {
    fn to_string(&self) -> String {
        serde_json::to_string(self).expect("Could not serialize RemoteAddr")
    }
}

impl FromStr for StringRemoteAddr {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s).expect("Could not deserialize RemoteAddr")
    }
}