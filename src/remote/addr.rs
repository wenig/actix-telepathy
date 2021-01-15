use actix::prelude::*;
use crate::network::NetworkInterface;
use crate::codec::ClusterMessage;
use crate::remote::{RemoteWrapper, RemoteMessage, AddrRepresentation};
use std::str::FromStr;
use serde::{Serialize, Deserialize};
use std::hash::{Hash};
use std::net::SocketAddr;
use std::ops::Deref;


/// Similar to actix::prelude::Addr but supports communication to remote actors on other nodes.
#[derive(Deserialize, Serialize, Hash, Debug)]
pub struct RemoteAddr {
    pub socket_addr: SocketAddr,
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub network_interface: Option<Recipient<ClusterMessage>>,
    pub(crate) id: AddrRepresentation
}

impl RemoteAddr {
    pub fn new(socket_addr: SocketAddr, network_interface: Option<Recipient<ClusterMessage>>, id: AddrRepresentation) -> Self {
        RemoteAddr{socket_addr, network_interface, id}
    }

    pub fn new_from_id(socket_addr: SocketAddr, id: &str) -> Self {
        RemoteAddr{socket_addr, network_interface: None, id: AddrRepresentation::from_str(id).unwrap()}
    }

    pub fn new_from_key(socket_addr: SocketAddr, network_interface: Recipient<ClusterMessage>, id: &str) -> Self {
        RemoteAddr{socket_addr, network_interface: Some(network_interface), id: AddrRepresentation::from_str(id).unwrap()}
    }

    pub fn new_gossip(socket_addr: SocketAddr, network_interface: Option<Recipient<ClusterMessage>>) -> Self {
        RemoteAddr::new(socket_addr, network_interface, AddrRepresentation::Gossip)
    }

    pub fn set_network_interface(&mut self, network_interface: Recipient<ClusterMessage>) {
        self.network_interface = Some(network_interface);
    }

    pub fn change_id(&mut self, id: String) {
        self.id = AddrRepresentation::Key(id);
    }

    pub fn do_send<T: RemoteMessage + Serialize>(&mut self, msg: Box<T>) -> () {
        self.network_interface.as_ref().expect("Network interface must be set!").do_send(ClusterMessage::Message(
            RemoteWrapper::new(self.clone(), msg)
        ));
    }

    pub fn send<T: RemoteMessage + Serialize>(&mut self, msg: Box<T>) -> RecipientRequest<ClusterMessage> {
        self.network_interface.as_ref().expect("Network interface must be set!").send(ClusterMessage::Message(
            RemoteWrapper::new(self.clone(), msg)
        ))
    }
}

impl Clone for RemoteAddr {
    fn clone(&self) -> Self {
        RemoteAddr::new( self.socket_addr.clone(), self.network_interface.clone(), self.id.clone())
    }
}

impl PartialEq for RemoteAddr {
    fn eq(&self, other: &Self) -> bool {
        self.socket_addr.eq(&other.socket_addr)
    }
}

impl Eq for RemoteAddr {}


#[derive(Deserialize, Serialize)]
pub enum AnyAddr<T: Actor> {
    #[serde(skip_serializing, skip_deserializing)]
    Local(Addr<T>),
    Remote(RemoteAddr)
}

impl<T: Actor> Clone for AnyAddr<T> {
    fn clone(&self) -> Self {
        match self {
            AnyAddr::Local(addr) => {AnyAddr::Local(addr.clone())},
            AnyAddr::Remote(addr) => {AnyAddr::Remote(addr.clone())}
        }
    }
}
