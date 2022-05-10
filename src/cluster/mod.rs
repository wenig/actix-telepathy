mod gossip;
mod listener;
#[cfg(test)]
mod tests;

pub use self::gossip::{Gossip, NodeResolving};
pub use self::listener::{ClusterListener, ClusterLog};

use crate::cluster::gossip::{GossipIgniting, MemberMgmt};
use crate::network::NetworkInterface;
use crate::remote::RemoteAddr;
use crate::CustomSystemService;
use actix::prelude::*;
use actix_broker::BrokerIssue;
use futures::executor::block_on;
use futures::StreamExt;
use log::*;
use std::collections::HashMap;
use std::io::Result as IoResult;
use std::net;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::wrappers::TcpListenerStream;

#[derive(MessageResponse)]
pub enum ConnectionApprovalResponse {
    Approved,
    Declined,
}

#[derive(Message)]
#[rtype(result = "ConnectionApprovalResponse")]
pub struct ConnectionApproval {
    pub addr: SocketAddr,
    pub send_addr: SocketAddr,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct TcpConnect(pub TcpStream, pub SocketAddr);

#[derive(Message)]
#[rtype(result = "()")]
pub enum NodeEvents {
    MemberUp(SocketAddr, Addr<NetworkInterface>, RemoteAddr, bool),
    MemberDown(SocketAddr),
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct GossipResponse(pub(crate) SocketAddr);

/// Central Actor for cluster handling
pub struct Cluster {
    ip_address: SocketAddr,
    addrs: Vec<SocketAddr>,
    own_addr: Option<Addr<Cluster>>,
    nodes: HashMap<SocketAddr, Addr<NetworkInterface>>,
}

impl Actor for Cluster {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let listener = Cluster::bind(self.ip_address.to_string()).unwrap();

        let st = Box::leak(listener).map(|st| {
            let st = st.unwrap();
            let addr = st.peer_addr().unwrap();
            TcpConnect(st, addr)
        });

        ctx.add_message_stream(st);

        self.own_addr = Some(ctx.address());

        let addrs_len = self.addrs.len();
        for node_addr in 0..addrs_len {
            self.add_node(self.addrs.get(node_addr).unwrap().clone(), true);
        }
        debug!("Cluster started {}", self.ip_address.clone().to_string());
    }
}

impl Cluster {
    pub fn new(ip_address: SocketAddr, seed_nodes: Vec<SocketAddr>) -> Addr<Cluster> {
        debug!("Cluster created");
        Gossip::start_service_with(move || Gossip::new(ip_address.clone()));

        Cluster::start_service_with(move || Cluster {
            ip_address: ip_address.clone(),
            addrs: seed_nodes.clone(),
            own_addr: None,
            nodes: Default::default(),
        })
    }

    fn bind(addr: String) -> IoResult<Box<TcpListenerStream>> {
        let addr = net::SocketAddr::from_str(&addr).unwrap();
        let listener = Box::new(TcpListenerStream::new(
            block_on(TcpListener::bind(&addr)).unwrap(),
        ));
        debug!("Listening on {}", addr);
        Ok(listener)
    }

    fn add_node_from_stream(&mut self, addr: SocketAddr, stream: TcpStream) {
        let own_ip = self.ip_address.clone();
        let node = NetworkInterface::from_stream(own_ip, addr.clone(), stream).start();
        self.nodes.insert(addr, node);
    }

    fn add_node(&mut self, node_addr: SocketAddr, seed: bool) {
        let own_ip = self.ip_address.clone();
        if !self.nodes.contains_key(&node_addr) {
            let node = NetworkInterface::new(own_ip, node_addr.clone(), seed).start();
            self.nodes.insert(node_addr.clone(), node);
        }
    }
}

// Singleton

impl Default for Cluster {
    fn default() -> Self {
        let ip_addr = "127.0.0.1:8000";

        Self {
            ip_address: SocketAddr::from_str(ip_addr).unwrap(),
            addrs: vec![],
            own_addr: None,
            nodes: HashMap::new(),
        }
    }
}

impl Supervised for Cluster {}
impl SystemService for Cluster {}
impl CustomSystemService for Cluster {}

impl Handler<TcpConnect> for Cluster {
    type Result = ();

    fn handle(&mut self, msg: TcpConnect, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Incoming TcpConnect");
        let stream = msg.0;
        let addr = msg.1;
        self.add_node_from_stream(addr, stream);
    }
}

impl Handler<NodeEvents> for Cluster {
    type Result = ();

    fn handle(&mut self, msg: NodeEvents, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            NodeEvents::MemberUp(host, node, remote_addr, seed) => {
                if seed {
                    Gossip::from_custom_registry()
                        .do_send(GossipIgniting::MemberUp(host.clone(), node));
                } else {
                    Gossip::from_custom_registry()
                        .do_send(MemberMgmt::MemberUp(host.clone(), node));
                }
                self.issue_system_async(ClusterLog::NewMember(host.clone(), remote_addr.clone()));
            }
            NodeEvents::MemberDown(host) => {
                Gossip::from_custom_registry().do_send(GossipIgniting::MemberDown(host.clone()));
                self.issue_system_async(ClusterLog::MemberLeft(host.clone()));
                self.nodes.remove(&host);
            }
        }
    }
}

impl Handler<GossipResponse> for Cluster {
    type Result = ();

    fn handle(&mut self, msg: GossipResponse, _ctx: &mut Context<Self>) -> Self::Result {
        self.add_node(msg.0, false);
    }
}

impl Handler<ConnectionApproval> for Cluster {
    type Result = ConnectionApprovalResponse;

    fn handle(
        &mut self,
        msg: ConnectionApproval,
        _ctx: &mut Self::Context,
    ) -> ConnectionApprovalResponse {
        if self.nodes.contains_key(&msg.addr) {
            ConnectionApprovalResponse::Declined
        } else {
            let node = self
                .nodes
                .get(&msg.send_addr)
                .expect("Should be filled")
                .clone();
            self.nodes.remove(&msg.send_addr);
            self.nodes.insert(msg.addr, node);
            ConnectionApprovalResponse::Approved
        }
    }
}
