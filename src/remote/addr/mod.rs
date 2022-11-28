use std::any::{Any, TypeId};
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;
use std::str::FromStr;

use actix::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::codec::ClusterMessage;
use crate::remote::{AddrRepresentation, RemoteMessage, RemoteWrapper};
use crate::{NetworkInterface, WrappedClusterMessage, CustomSerialization};
use actix::dev::ToEnvelope;

pub use self::node::Node;
pub use self::response_dispatcher::{ResponseDispatcher, ResponseSubscribe};

pub mod node;
use tokio::sync::oneshot::{self, Receiver};
pub mod resolver;
pub mod response_dispatcher;
#[cfg(test)]
mod tests;

/// Similar to actix::prelude::Addr but supports communication to remote actors on other nodes.
#[derive(Deserialize, Serialize, Debug)]
pub struct RemoteAddr {
    pub node: Node,
    pub(crate) id: AddrRepresentation,
}

impl RemoteAddr {
    pub fn new(node: Node, id: AddrRepresentation) -> Self {
        RemoteAddr { node, id }
    }

    pub fn new_from_id(socket_addr: SocketAddr, id: &str) -> Self {
        RemoteAddr {
            node: Node::new(socket_addr, None),
            id: AddrRepresentation::from_str(id).unwrap(),
        }
    }

    pub fn new_from_key(
        socket_addr: SocketAddr,
        network_interface: Addr<NetworkInterface>,
        id: &str,
    ) -> Self {
        RemoteAddr {
            node: Node::new(socket_addr, Some(network_interface)),
            id: AddrRepresentation::from_str(id).unwrap(),
        }
    }

    pub fn new_gossip(
        socket_addr: SocketAddr,
        network_interface: Option<Addr<NetworkInterface>>,
    ) -> Self {
        RemoteAddr::new(
            Node::new(socket_addr, network_interface),
            AddrRepresentation::Gossip,
        )
    }

    pub fn set_network_interface(&mut self, network_interface: Addr<NetworkInterface>) {
        self.node.network_interface = Some(network_interface);
    }

    pub fn change_id(&mut self, id: String) {
        self.id = AddrRepresentation::Key(id);
    }

    pub fn do_send<T: RemoteMessage + Serialize>(&self, msg: T) {
        self.do_send_with_id(msg, None);
    }

    pub fn try_send<T: RemoteMessage + Serialize>(
        &self,
        _msg: T,
    ) -> RecipientRequest<ClusterMessage> {
        unimplemented!("So far, it is not possible to use this method!")
    }

    pub fn send<T, R>(&self, msg: T) -> ResponseFuture<R>
    where
        T: RemoteMessage + Serialize,
        R: RemoteMessage + Send + Deserialize<'static> + 'static
    {
        let conversation_id = Uuid::new_v4();
        let (tx, rx) = oneshot::channel();
        ResponseDispatcher::from_registry().do_send(ResponseSubscribe(conversation_id, tx));
        self.do_send_with_id(msg, Some(conversation_id));
        Box::pin(async move {
            let message_buffer = rx.blocking_recv().expect("Could not receive response.");
            R::generate_serializer().deserialize(&(message_buffer)[..]).expect("Cannot deserialized #name message")
        })
    }

    pub fn wait_send<T: RemoteMessage + Serialize>(
        &self,
        msg: T,
    ) -> Request<NetworkInterface, WrappedClusterMessage> {
        self.node
            .network_interface
            .as_ref()
            .expect("Network interface must be set!")
            .send(WrappedClusterMessage(ClusterMessage::Message(
                RemoteWrapper::new(self.clone(), msg, None),
            )))
    }

    fn do_send_with_id<T: RemoteMessage + Serialize>(&self, msg: T, conversation_id: Option<Uuid>) {
        let _r = self
            .node
            .network_interface
            .as_ref()
            .expect("Network interface must be set!")
            .do_send(ClusterMessage::Message(RemoteWrapper::new(
                self.clone(),
                msg,
                conversation_id,
            )));
    }
}

impl Clone for RemoteAddr {
    fn clone(&self) -> Self {
        RemoteAddr::new(self.node.clone(), self.id.clone())
    }
}

impl PartialEq for RemoteAddr {
    fn eq(&self, other: &Self) -> bool {
        self.node.eq(&other.node) && self.id.eq(&other.id)
    }
}

impl Eq for RemoteAddr {}

impl Hash for RemoteAddr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.node.hash(state);
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
        if let AnyAddr::Remote(addr) = self {
            addr.change_id(id.to_string());
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
