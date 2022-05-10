use std::hash::{Hash, Hasher};
use std::net::SocketAddr;
use std::str::FromStr;

use actix::prelude::*;
use serde::{Deserialize, Serialize};

use crate::codec::ClusterMessage;
use crate::remote::{AddrRepresentation, RemoteMessage, RemoteWrapper};
use crate::{NetworkInterface, WrappedClusterMessage};
use actix::dev::ToEnvelope;

pub mod resolver;
#[cfg(test)]
mod tests;

/// Similar to actix::prelude::Addr but supports communication to remote actors on other nodes.
#[derive(Deserialize, Serialize, Debug)]
pub struct RemoteAddr {
    pub socket_addr: SocketAddr,
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub network_interface: Option<Addr<NetworkInterface>>,
    pub(crate) id: AddrRepresentation,
}

impl RemoteAddr {
    pub fn new(
        socket_addr: SocketAddr,
        network_interface: Option<Addr<NetworkInterface>>,
        id: AddrRepresentation,
    ) -> Self {
        RemoteAddr {
            socket_addr,
            network_interface,
            id,
        }
    }

    pub fn new_from_id(socket_addr: SocketAddr, id: &str) -> Self {
        RemoteAddr {
            socket_addr,
            network_interface: None,
            id: AddrRepresentation::from_str(id).unwrap(),
        }
    }

    pub fn new_from_key(
        socket_addr: SocketAddr,
        network_interface: Addr<NetworkInterface>,
        id: &str,
    ) -> Self {
        RemoteAddr {
            socket_addr,
            network_interface: Some(network_interface),
            id: AddrRepresentation::from_str(id).unwrap(),
        }
    }

    pub fn new_gossip(
        socket_addr: SocketAddr,
        network_interface: Option<Addr<NetworkInterface>>,
    ) -> Self {
        RemoteAddr::new(socket_addr, network_interface, AddrRepresentation::Gossip)
    }

    pub fn set_network_interface(&mut self, network_interface: Addr<NetworkInterface>) {
        self.network_interface = Some(network_interface);
    }

    pub fn change_id(&mut self, id: String) {
        self.id = AddrRepresentation::Key(id);
    }

    pub fn do_send<T: RemoteMessage + Serialize>(&self, msg: T) -> () {
        let _r = self
            .network_interface
            .as_ref()
            .expect("Network interface must be set!")
            .do_send(ClusterMessage::Message(RemoteWrapper::new(
                self.clone(),
                msg,
                None,
            )));
    }

    pub fn try_send<T: RemoteMessage + Serialize>(
        &self,
        _msg: Box<T>,
    ) -> RecipientRequest<ClusterMessage> {
        unimplemented!("So far, it is not possible to use this method!")
    }

    pub fn send<T: RemoteMessage + Serialize>(&self, _msg: Box<T>) -> () {
        unimplemented!(
            "So far, it is not possible to receive responses from remote destinations as futures!"
        )
    }

    pub fn wait_send<T: RemoteMessage + Serialize>(
        &self,
        msg: T,
    ) -> Request<NetworkInterface, WrappedClusterMessage> {
        self.network_interface
            .as_ref()
            .expect("Network interface must be set!")
            .send(WrappedClusterMessage(ClusterMessage::Message(
                RemoteWrapper::new(self.clone(), msg, None),
            )))
    }
}

impl Clone for RemoteAddr {
    fn clone(&self) -> Self {
        RemoteAddr::new(
            self.socket_addr.clone(),
            self.network_interface.clone(),
            self.id.clone(),
        )
    }
}

impl PartialEq for RemoteAddr {
    fn eq(&self, other: &Self) -> bool {
        self.socket_addr.eq(&other.socket_addr) && self.id.eq(&other.id)
    }
}

impl Eq for RemoteAddr {}

impl Hash for RemoteAddr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.socket_addr.hash(state);
        self.id.hash(state);
    }
}

#[derive(Deserialize, Serialize)]
pub enum AnyAddr<A: Actor> {
    #[serde(skip_serializing, skip_deserializing)]
    Local(Addr<A>),
    Remote(RemoteAddr),
}

impl<A: Actor> AnyAddr<A> {
    pub fn do_send<M>(&self, msg: M)
    where
        M: RemoteMessage,
        M::Result: Send,
        A: Handler<M>,
        A::Context: ToEnvelope<A, M>,
    {
        match self {
            AnyAddr::Local(addr) => addr.do_send(msg),
            AnyAddr::Remote(addr) => addr.do_send(msg),
        }
    }

    pub fn change_id(&mut self, id: &str) {
        match self {
            AnyAddr::Remote(addr) => addr.change_id(id.to_string()),
            _ => (),
        }
    }
}

impl<T: Actor> Clone for AnyAddr<T> {
    fn clone(&self) -> Self {
        match self {
            AnyAddr::Local(addr) => AnyAddr::Local(addr.clone()),
            AnyAddr::Remote(addr) => AnyAddr::Remote(addr.clone()),
        }
    }
}

impl<T: Actor> PartialEq for AnyAddr<T> {
    fn eq(&self, other: &Self) -> bool {
        match self {
            AnyAddr::Local(addr) => match other {
                AnyAddr::Local(other_addr) => addr.eq(other_addr),
                AnyAddr::Remote(_) => false,
            },
            AnyAddr::Remote(addr) => match other {
                AnyAddr::Local(_) => false,
                AnyAddr::Remote(other_addr) => addr.eq(other_addr),
            },
        }
    }
}

impl<T: Actor> Eq for AnyAddr<T> {}

impl<T: Actor> Hash for AnyAddr<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            AnyAddr::Local(addr) => addr.hash(state),
            AnyAddr::Remote(addr) => addr.hash(state),
        }
    }
}
