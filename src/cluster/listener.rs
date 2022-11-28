use crate::Node;
use actix::prelude::*;
use std::net::SocketAddr;

/// Message sent to ClusterListeners if members join or leave the cluster
#[derive(Message)]
#[rtype(result = "()")]
pub enum ClusterLog {
    NewMember(Node),
    MemberLeft(SocketAddr),
}

impl Clone for ClusterLog {
    fn clone(&self) -> Self {
        match self {
            ClusterLog::NewMember(node) => ClusterLog::NewMember(node.clone()),
            ClusterLog::MemberLeft(addr) => ClusterLog::MemberLeft(*addr),
        }
    }
}

/// Trait for actors to receive ClusterLog messages
pub trait ClusterListener: Actor + Handler<ClusterLog> {}
