use actix::prelude::*;
use crate::remote::RemoteAddr;
use std::net::SocketAddr;

/// Message sent to ClusterListeners if members join or leave the cluster
#[derive(Message)]
#[rtype(result = "()")]
pub enum ClusterLog {
    NewMember(SocketAddr, RemoteAddr),
    MemberLeft(SocketAddr)
}

impl Clone for ClusterLog {
    fn clone(&self) -> Self {
        match self {
            ClusterLog::NewMember(addr, remote_addr) => ClusterLog::NewMember(addr.clone(), (*remote_addr).clone()),
            ClusterLog::MemberLeft(addr) => ClusterLog::MemberLeft(addr.clone())
        }
    }
}

/// Trait for actors to receive ClusterLog messages
pub trait ClusterListener: Actor + Handler<ClusterLog> {}
