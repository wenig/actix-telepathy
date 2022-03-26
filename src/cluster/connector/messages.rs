use std::collections::HashSet;
use std::net::SocketAddr;
use actix::prelude::*;
use crate::NetworkInterface;
use serde::{Serialize, Deserialize};
use crate::{RemoteMessage, DefaultSerialization};


#[derive(Message)]
#[rtype(result = "Result<Vec<Addr<NetworkInterface>>, ()>")]
pub struct NodeResolving{
    pub addrs: Vec<SocketAddr>
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GossipEvent {
    Join,
    Leave
}

#[derive(RemoteMessage, Serialize, Deserialize, Debug, Clone)]
pub struct GossipMessage {
    pub event: GossipEvent,
    pub addr: SocketAddr,
    pub seen: HashSet<SocketAddr>
}

#[derive(RemoteMessage, Serialize, Deserialize, Debug, Clone)]
pub struct GossipJoining {
    pub about_to_join: usize
}
