pub use crate::cluster::connector::messages::NodeResolving;
use crate::cluster::connector::messages::{GossipJoining, GossipMessage};
use crate::{CustomSerialization, RemoteActor, RemoteMessage, RemoteWrapper};
use crate::{CustomSystemService, Gossip, NetworkInterface, NodeEvent, SingleSeed};
use actix::prelude::*;
use log::*;
use std::collections::HashMap;
use std::net::SocketAddr;

use self::messages::SingleSeedMembers;

pub mod gossip;
mod messages;
pub mod single_seed;

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
#[remote_messages(GossipMessage, GossipJoining, SingleSeedMembers)]
pub enum Connector {
    Gossip(Gossip),
    SingleSeed(SingleSeed),
}

impl Connector {
    pub fn from_connection_protocol(
        connection_protocol: ConnectionProtocol,
        own_address: SocketAddr,
        seed_nodes: Vec<SocketAddr>,
    ) -> Self {
        match connection_protocol {
            ConnectionProtocol::Gossip => Self::Gossip(Gossip::new(own_address, seed_nodes)),
            ConnectionProtocol::SingleSeed => Self::SingleSeed(SingleSeed::new(own_address)),
        }
    }

    pub fn start_service_from(
        connection_protocol: ConnectionProtocol,
        own_address: SocketAddr,
        seed_nodes: Vec<SocketAddr>,
    ) {
        Self::start_service_with(move || {
            Connector::from_connection_protocol(
                connection_protocol,
                own_address,
                seed_nodes.clone(),
            )
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
    fn get_member_map(
        &mut self,
        msg: NodeResolving,
        ctx: &mut Context<Connector>,
    ) -> &HashMap<SocketAddr, Addr<NetworkInterface>>;
    fn get_own_addr(&self) -> SocketAddr;
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
        let addrs = msg.addrs.clone();
        let own_addr = match self {
            Connector::Gossip(gossip) => gossip.get_own_addr(),
            Connector::SingleSeed(single_seed) => single_seed.get_own_addr(),
        };

        let member_map = match self {
            Connector::Gossip(gossip) => gossip.get_member_map(msg, ctx),
            Connector::SingleSeed(single_seed) => single_seed.get_member_map(msg, ctx),
        };

        Ok(addrs
            .into_iter()
            .filter_map(|x| {
                if x == own_addr {
                    None
                } else {
                    Some(
                        member_map
                            .get(&x)
                            .unwrap_or_else(|| panic!("Socket {} should be known!", &x))
                            .clone(),
                    )
                }
            })
            .collect())
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

// --- SingleSeed impl ---

impl Handler<SingleSeedMembers> for Connector {
    type Result = ();

    fn handle(&mut self, msg: SingleSeedMembers, _ctx: &mut Self::Context) -> Self::Result {
        match self {
            Connector::SingleSeed(single_seed) => single_seed.handle_single_seed_members(msg),
            _ => {
                warn!("Connector can only handle SingleSeedMembers if it is Connector::SingleSeed")
            }
        }
    }
}
