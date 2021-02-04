use actix::prelude::*;
use std::net;
use std::io::{Result as IoResult};
use tokio::net::{TcpListener, TcpStream};
use std::collections::{HashMap};
use log::*;
use crate::network::NetworkInterface;
use std::str::FromStr;
use futures::StreamExt;
use futures::executor::block_on;
use std::net::{SocketAddr};
use crate::cluster::gossip::{Gossip, GossipIgniting, MemberMgmt};
use crate::remote::{RemoteAddr, AddressResolver, AddressRequest, AddressResponse, RemoteWrapper};
use crate::{ClusterLog, CustomSystemService};


#[derive(MessageResponse)]
pub enum ConnectionApprovalResponse {
    Approved,
    Declined
}


#[derive(Message)]
#[rtype(result = "()")]
pub struct Test {
    pub msg: String
}


#[derive(Message)]
#[rtype(result = "ConnectionApprovalResponse")]
pub struct ConnectionApproval {
    pub addr: SocketAddr,
    pub send_addr: SocketAddr
}


#[derive(Message)]
#[rtype(result = "()")]
pub struct TcpConnect(pub TcpStream, pub SocketAddr);

#[derive(Message)]
#[rtype(result = "()")]
pub enum NodeEvents{
    MemberUp(SocketAddr, Addr<NetworkInterface>, RemoteAddr, bool),
    MemberDown(SocketAddr)
}

#[derive(Message)]
#[rtype(result = "()")]
pub enum NodeResolving{
    Request(SocketAddr, Recipient<NodeResolving>),
    VecRequest(Vec<SocketAddr>, Recipient<NodeResolving>),
    Response(Addr<NetworkInterface>),
    VecResponse(Vec<Option<Addr<NetworkInterface>>>)
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct GossipResponse(pub(crate) SocketAddr);

/// Central Actor for cluster handling
pub struct Cluster {
    ip_address: SocketAddr,
    addrs: Vec<SocketAddr>,
    listeners: Vec<Recipient<ClusterLog>>,
    gossip: Option<Addr<Gossip>>,
    address_resolver: Option<Addr<AddressResolver>>,
    own_addr: Option<Addr<Cluster>>,
    nodes: HashMap<SocketAddr, Addr<NetworkInterface>>,

    rec_to_be_registered: Vec<(Recipient<RemoteWrapper>, String)>
}


impl Actor for Cluster {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let listener = Cluster::bind(self.ip_address.to_string()).unwrap();

        ctx.add_message_stream(Box::leak(listener).incoming().map(|st| {
            let st = st.unwrap();
            let addr = st.peer_addr().unwrap();
            TcpConnect(st, addr)
        }));

        self.address_resolver = Some(AddressResolver::from_registry());
        for (rec, identifier) in self.rec_to_be_registered.iter() {
            self.address_resolver.as_ref().unwrap().do_send(AddressRequest::Register((*rec).clone(), identifier.to_string()));
        }

        self.own_addr = Some(ctx.address());
        let ip_addr4gossip = self.ip_address.clone();
        self.gossip = Some(Gossip::start_service_with(move || { Gossip::new(ip_addr4gossip) }));

        let addrs_len = self.addrs.len();
        for node_addr in 0..addrs_len {
            self.add_node(self.addrs.get(node_addr).unwrap().clone(), true);
        }
        debug!("Cluster started {}", self.ip_address.clone().to_string());
    }
}


impl Cluster {
    pub fn new(ip_address: SocketAddr, seed_nodes: Vec<SocketAddr>, cluster_listeners: Vec<Recipient<ClusterLog>>, rec_to_be_registered: Vec<(Recipient<RemoteWrapper>, String)>) -> Addr<Cluster> {
        debug!("Cluster created");

        Cluster::start_service_with(move ||
            Cluster {
                ip_address,
                addrs: seed_nodes.clone(),
                listeners: cluster_listeners.clone(),
                gossip: None,
                address_resolver: None,
                own_addr: None,
                nodes: Default::default(),
                rec_to_be_registered: rec_to_be_registered.clone()
            }
        )
    }

    fn bind(addr: String) -> IoResult<Box<TcpListener>> {
        let addr = net::SocketAddr::from_str(&addr).unwrap();
        let listener = Box::new(block_on(TcpListener::bind(&addr)).unwrap());
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

    #[allow(dead_code)]
    fn register_actor(&self, rec: Recipient<RemoteWrapper>, actor_identifier: &str) {
        self.own_addr.as_ref().unwrap().register_actor(rec, actor_identifier)
    }
}

// Singleton

impl Default for Cluster {
    fn default() -> Self {
        let ip_addr = "127.0.0.1:8000";

        Self {
            ip_address: SocketAddr::from_str(ip_addr).unwrap(),
            addrs: vec![],
            listeners: vec![],
            gossip: None,
            address_resolver: None,
            own_addr: None,
            nodes: HashMap::new(),

            rec_to_be_registered: vec![]
        }
    }
}

impl Supervised for Cluster {}
impl SystemService for Cluster {}
impl CustomSystemService for Cluster {}


impl Handler<Test> for Cluster {
    type Result = ();

    fn handle(&mut self, msg: Test, _ctx: &mut Self::Context) -> Self:: Result {
        debug!("{}", msg.msg)
    }
}


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
                    self.gossip.clone().unwrap().do_send(GossipIgniting::MemberUp(host.clone(), node));
                } else {
                    self.gossip.clone().unwrap().do_send(MemberMgmt::MemberUp(host.clone(), node));
                }
                for listener in self.listeners.iter() {
                    let _r = listener.do_send(ClusterLog::NewMember(host.clone(), remote_addr.clone()));
                }
            },
            NodeEvents::MemberDown(host) => {
                self.gossip.clone().unwrap().do_send(GossipIgniting::MemberDown(host.clone()));
                for listener in self.listeners.iter() {
                    let _r = listener.do_send(ClusterLog::MemberLeft(host.clone()));
                }
            }
        }
    }
}

impl Handler<AddressRequest> for Cluster {
    type Result = Result<AddressResponse, ()>;

    fn handle(&mut self, msg: AddressRequest, _ctx: &mut Context<Self>) -> Self::Result {
        self.address_resolver.as_ref().unwrap().do_send(msg);
        Ok(AddressResponse::Register)
    }
}

impl Handler<NodeResolving> for Cluster {
    type Result = ();

    fn handle(&mut self, msg: NodeResolving, _ctx: &mut Context<Self>) -> Self::Result {
        self.gossip.clone().unwrap().do_send(msg);
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

    fn handle(&mut self, msg: ConnectionApproval, _ctx: &mut Self::Context) -> ConnectionApprovalResponse {
        if self.nodes.contains_key(&msg.addr) {
            ConnectionApprovalResponse::Declined
        } else {
            let node = self.nodes.get(&msg.send_addr).expect("Should be filled").clone();
            self.nodes.remove(&msg.send_addr);
            self.nodes.insert(msg.addr, node);
            ConnectionApprovalResponse::Approved
        }
    }
}


/// Helper for registering actors to the cluster
pub trait AddrApi {
    fn register_actor(&self, rec: Recipient<RemoteWrapper>, actor_identifier: &str);
    fn request_node_addr(&self, socket_addr: SocketAddr, rec: Recipient<NodeResolving>);
    fn request_node_addrs(&self, socket_addrs: Vec<SocketAddr>, rec: Recipient<NodeResolving>);
}


impl AddrApi for Addr<Cluster> {
    fn register_actor(&self, addr: Recipient<RemoteWrapper>, actor_identifier: &str) -> () {
        let _r = self.do_send(AddressRequest::Register(addr, actor_identifier.to_string()));
    }

    fn request_node_addr(&self, socket_addr: SocketAddr, rec: Recipient<NodeResolving>) -> () {
        self.do_send(NodeResolving::Request(socket_addr, rec))
    }

    fn request_node_addrs(&self, socket_addrs: Vec<SocketAddr>, rec: Recipient<NodeResolving>) -> () {
        self.do_send(NodeResolving::VecRequest(socket_addrs, rec))
    }
}
