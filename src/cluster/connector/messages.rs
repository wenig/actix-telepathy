use crate::NetworkInterface;
use crate::{DefaultSerialization, RemoteMessage};
use actix::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::SocketAddr;

#[derive(Message)]
#[rtype(result = "Result<Vec<Addr<NetworkInterface>>, ()>")]
pub struct NodeResolving {
    pub addrs: Vec<SocketAddr>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GossipEvent {
    Join,
    Leave,
}

#[derive(RemoteMessage, Serialize, Deserialize, Debug, Clone)]
pub struct GossipMessage {
    pub event: GossipEvent,
    pub addr: SocketAddr,
    pub seen: HashSet<SocketAddr>,
}

#[derive(RemoteMessage, Serialize, Deserialize, Debug, Clone)]
pub struct GossipJoining {
    pub about_to_join: usize,
}

#[derive(RemoteMessage, Serialize, Deserialize, Debug, Clone)]
pub struct SingleSeedMembers(pub Vec<SocketAddr>);
