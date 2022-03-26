use std::net::SocketAddr;
use actix::prelude::*;
use log::*;
use crate::{CustomSystemService, Gossip, NetworkInterface, NodeEvents};
pub use crate::cluster::connector::messages::NodeResolving;
use crate::cluster::connector::messages::{GossipMessage, GossipJoining};
use crate::{RemoteActor, RemoteWrapper, RemoteMessage, CustomSerialization};

pub mod gossip;
mod messages;

#[derive(Debug, Clone, Copy)]
pub enum ConnectionProtocol {
    SingleSeed,
    Gossip
}

impl Default for ConnectionProtocol {
    fn default() -> Self {
        Self::Gossip
    }
}


#[derive(RemoteActor)]
#[remote_messages(GossipMessage, GossipJoining)]
pub enum Connector {
    Gossip(Gossip)
}

impl Connector {
    pub fn from_connection_protocol(connection_protocol: ConnectionProtocol, own_address: SocketAddr) -> Self {
        match connection_protocol {
            ConnectionProtocol::Gossip => Self::Gossip(Gossip::new(own_address)),
            ConnectionProtocol::SingleSeed => todo!("This must still be implemented.")
        }
    }

    pub fn start_service_from(connection_protocol: ConnectionProtocol, own_address: SocketAddr) {
        Self::start_service_with(move || { Connector::from_connection_protocol(connection_protocol, own_address) });
    }
}


impl Actor for Connector {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.register(ctx.address().recipient());
        debug!("{} actor started", Self::ACTOR_ID);
    }
}

impl Default for Connector {
    fn default() -> Self {
        Self::Gossip(Gossip::default())
    }
}


impl Supervised for Connector {}
impl SystemService for Connector {}
impl CustomSystemService for Connector {
    fn custom_service_started(&mut self, _ctx: &mut Context<Self>) {
        debug!("Gossip Service started");
    }
}

pub trait ConnectorVariant {
    fn handle_node_event(&mut self, msg: NodeEvents, ctx: &mut Context<Connector>);
    fn handle_node_resolving(&mut self, msg: NodeResolving, ctx: &mut Context<Connector>) -> Result<Vec<Addr<NetworkInterface>>, ()>;
}


impl Handler<NodeEvents> for Connector {
    type Result = ();

    fn handle(&mut self, msg: NodeEvents, ctx: &mut Self::Context) -> Self::Result {
        match self {
            Connector::Gossip(gossip) => gossip.handle_node_event(msg, ctx)
        }
    }
}

impl Handler<NodeResolving> for Connector {
    type Result = Result<Vec<Addr<NetworkInterface>>, ()>;

    fn handle(&mut self, msg: NodeResolving, ctx: &mut Context<Self>) -> Self::Result {
        match self {
            Connector::Gossip(gossip) => gossip.handle_node_resolving(msg, ctx)
        }
    }
}
