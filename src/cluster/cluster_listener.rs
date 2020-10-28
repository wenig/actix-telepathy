use actix::prelude::*;
use log::*;
use crate::remote::RemoteAddr;
use actix::dev::channel::AddressReceiver;

#[derive(Message)]
#[rtype(result = "()")]
pub enum ClusterLog {
    NewMember(String, RemoteAddr),
    MemberLeft(String)
}

impl Clone for ClusterLog {
    fn clone(&self) -> Self {
        match self {
            ClusterLog::NewMember(str, remote_addr) => ClusterLog::NewMember(str.clone(), (*remote_addr).clone()),
            ClusterLog::MemberLeft(str) => ClusterLog::MemberLeft(str.clone())
        }
    }
}

pub trait ClusterListener: Actor + Handler<ClusterLog> {}
