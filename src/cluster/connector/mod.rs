pub use crate::cluster::connector::messages::NodeResolving;
use crate::cluster::connector::messages::{GossipJoining, GossipMessage};
use crate::{CustomSerialization, RemoteActor, RemoteMessage, RemoteWrapper};
use crate::{CustomSystemService, Gossip, SingleSeed, NetworkInterface, NodeEvent};
use actix::prelude::*;
use log::*;
use std::net::SocketAddr;

pub mod gossip;
pub mod single_seed;
mod messages;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy)]
pub enum ConnectionProtocol {
    SingleSeed,
    Gossip,
}

impl Default for ConnectionProtocol {
    fn default() -> Self {
        Self::Gossip
    }
}

#[derive(RemoteActor)]
#[remote_messages(GossipMessage, GossipJoining)]
pub enum Connector {
    Gossip(Gossip),
    SingleSeed(SingleSeed)
}

impl Connector {
    pub fn from_connection_protocol(
        connection_protocol: ConnectionProtocol,
        own_address: SocketAddr,
        seed_nodes: Vec<SocketAddr>,
    ) -> Self {
        match connection_protocol {
            ConnectionProtocol::Gossip => Self::Gossip(Gossip::new(own_address, seed_nodes)),
            ConnectionProtocol::SingleSeed => todo!("This must still be implemented."),
        }
    }

    pub fn start_service_from(connection_protocol: ConnectionProtocol, own_address: SocketAddr, seed_nodes: Vec<SocketAddr>) {
        Self::start_service_with(move || {
            Connector::from_connection_protocol(connection_protocol, own_address, seed_nodes.clone())
        });
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
        debug!("Connector Service started");
    }
}

pub trait ConnectorVariant {
    fn handle_node_event(&mut self, msg: NodeEvent, ctx: &mut Context<Connector>);
    fn handle_node_resolving(
        &mut self,
        msg: NodeResolving,
        ctx: &mut Context<Connector>,
    ) -> Result<Vec<Addr<NetworkInterface>>, ()>;
}

impl Handler<NodeEvent> for Connector {
    type Result = ();

    fn handle(&mut self, msg: NodeEvent, ctx: &mut Self::Context) -> Self::Result {
        match self {
            Connector::Gossip(gossip) => gossip.handle_node_event(msg, ctx),
            Connector::SingleSeed(single_seed) => single_seed.handle_node_event(msg, ctx),
        }
    }
}

impl Handler<NodeResolving> for Connector {
    type Result = Result<Vec<Addr<NetworkInterface>>, ()>;

    fn handle(&mut self, msg: NodeResolving, ctx: &mut Context<Self>) -> Self::Result {
        match self {
            Connector::Gossip(gossip) => gossip.handle_node_resolving(msg, ctx),
            Connector::SingleSeed(single_seed) => single_seed.handle_node_resolving(msg, ctx),
        }
    }
}

// --- Gossip impl ---

impl Handler<GossipMessage> for Connector {
    type Result = ();

    fn handle(&mut self, msg: GossipMessage, _ctx: &mut Self::Context) -> Self::Result {
        match self {
            Connector::Gossip(gossip) => gossip.handle_gossip_message(msg),
            _ => warn!("Connector can only handle GossipMessage if it is Connector::Gossip"),
        }
    }
}

impl Handler<GossipJoining> for Connector {
    type Result = ();

    fn handle(&mut self, msg: GossipJoining, _ctx: &mut Self::Context) -> Self::Result {
        match self {
            Connector::Gossip(gossip) => gossip.handle_gossip_joining(msg),
            _ => warn!("Connector can only handle GossipJoining if it is Connector::Gossip"),
        }
    }
}
