mod connector;
mod listener;
#[cfg(test)]
mod tests;

pub use self::listener::{ClusterListener, ClusterLog};
pub use connector::NodeResolving;
pub use connector::{gossip::Gossip, single_seed::SingleSeed};

pub use crate::cluster::connector::ConnectionProtocol;
pub use crate::cluster::connector::Connector;
use crate::network::NetworkInterface;
use crate::remote::Node;
use crate::CustomSystemService;
use actix::prelude::*;
use actix_broker::BrokerIssue;
use futures::executor::block_on;
use futures::StreamExt;
use log::*;
use std::collections::HashMap;
use std::fmt::Display;
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

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub enum NodeEvent {
    /// (Node, and whether it is a seed node)
    MemberUp(Node, bool),
    MemberDown(SocketAddr),
}

impl Display for NodeEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeEvent::MemberUp(node, seed) => {
                write!(f, "MemberUp: {} (seed: {})", node.socket_addr, seed)
            }
            NodeEvent::MemberDown(addr) => write!(f, "MemberDown: {}", addr),
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ConnectToNode(pub(crate) SocketAddr);

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
            self.add_node(*self.addrs.get(node_addr).unwrap(), true);
        }
        debug!("Cluster started {}", self.ip_address.clone().to_string());
    }
}

impl Cluster {
    pub fn new(ip_address: SocketAddr, seed_nodes: Vec<SocketAddr>) -> Addr<Cluster> {
        Self::new_with_connection_protocol(ip_address, seed_nodes, ConnectionProtocol::SingleSeed)
    }

    pub fn new_with_connection_protocol(
        ip_address: SocketAddr,
        seed_nodes: Vec<SocketAddr>,
        connection_protocol: ConnectionProtocol,
    ) -> Addr<Cluster> {
        debug!("Cluster created");
        Connector::start_service_from(connection_protocol, ip_address, seed_nodes.clone());

        Cluster::start_service_with(move || Cluster {
            ip_address,
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
        let own_ip = self.ip_address;
        let node = NetworkInterface::from_stream(own_ip, addr, stream).start();
        self.nodes.insert(addr, node);
    }

    fn add_node(&mut self, node_addr: SocketAddr, seed: bool) {
        let own_ip = self.ip_address;
        self.nodes
            .entry(node_addr)
            .or_insert_with(|| NetworkInterface::new(own_ip, node_addr, seed).start());
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

impl Handler<ConnectToNode> for Cluster {
    type Result = ();

    fn handle(&mut self, msg: ConnectToNode, _ctx: &mut Self::Context) -> Self::Result {
        self.add_node(msg.0, false);
    }
}

impl Handler<NodeEvent> for Cluster {
    type Result = ();

    fn handle(&mut self, msg: NodeEvent, _ctx: &mut Self::Context) -> Self::Result {
        match &msg {
            NodeEvent::MemberUp(node, _seed) => {
                self.issue_system_async(ClusterLog::NewMember(node.clone()));
            }
            NodeEvent::MemberDown(host) => {
                self.issue_system_async(ClusterLog::MemberLeft(*host));
                self.nodes.remove(host);
            }
        }

        Connector::from_custom_registry().do_send(msg)
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
