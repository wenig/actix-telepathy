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
use std::net::SocketAddr;
use crate::cluster::gossip::{Gossip, GossipIgniting};
use crate::remote::{RemoteAddr, AddressResolver, AddressRequest, AddressResponse, RemoteWrapper};
use crate::ClusterLog;


#[derive(Message)]
#[rtype(result = "()")]
pub struct TcpConnect(pub TcpStream, pub SocketAddr);

#[derive(Message)]
#[rtype(result = "()")]
pub enum NodeEvents{
    MemberUp(String, Addr<NetworkInterface>, RemoteAddr),
    MemberDown(String)
}

#[derive(Message)]
#[rtype(result = "()")]
pub enum NodeResolving{
    Request(String, Recipient<NodeResolving>),
    VecRequest(Vec<String>, Recipient<NodeResolving>),
    Response(Addr<NetworkInterface>),
    VecResponse(Vec<Option<Addr<NetworkInterface>>>)
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct GossipResponse(pub(crate) String);

/// Central Actor for cluster handling
pub struct Cluster {
    ip_address: String,
    addrs: Vec<String>,
    listeners: Vec<Recipient<ClusterLog>>,
    gossip: Option<Addr<Gossip>>,
    address_resolver: Addr<AddressResolver>,
    own_addr: Option<Addr<Cluster>>,
    nodes: HashMap<String, Addr<NetworkInterface>>
}


impl Actor for Cluster {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.own_addr = Some(ctx.address());
        let ip_addr4gossip = self.ip_address.clone();
        let addr4gossip = ctx.address().clone();
        self.gossip = Some(Supervisor::start(move |_| Gossip::new(ip_addr4gossip, addr4gossip)));

        for node_addr in self.addrs.iter() {
            let addr = SocketAddr::from_str(&node_addr).unwrap();
            let own_ip = self.ip_address.clone();
            let parent = self.own_addr.clone().unwrap();
            let gossip = self.gossip.clone().unwrap();
            let address_resolver = self.address_resolver.clone();
            let node = NetworkInterface::new(own_ip, addr, parent, gossip, address_resolver).start();
            self.nodes.insert(node_addr.clone(), node);
        }
    }
}

impl Handler<TcpConnect> for Cluster {
    type Result = ();

    fn handle(&mut self, msg: TcpConnect, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Incoming TcpConnect");
        let stream = msg.0;
        let addr = msg.1;
        let own_ip = self.ip_address.clone();
        let parent = self.own_addr.clone().unwrap();
        let gossip = self.gossip.clone().unwrap();
        let address_resolver = self.address_resolver.clone();
        let node = NetworkInterface::from_stream(own_ip, addr, stream, parent, gossip, address_resolver).start();
        self.nodes.insert(addr.to_string(), node);
    }
}

impl Handler<NodeEvents> for Cluster {
    type Result = ();

    fn handle(&mut self, msg: NodeEvents, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            NodeEvents::MemberUp(host, node, remote_addr) => {
                self.gossip.clone().unwrap().do_send(GossipIgniting::MemberUp(host.clone(), node));
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
        self.address_resolver.do_send(msg);
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
        self.add_node(msg.0);
    }
}

impl Cluster {
    pub fn new(ip_address: String, seed_nodes: Vec<String>, cluster_listeners: Vec<Recipient<ClusterLog>>, rec_to_be_registered: Vec<(Recipient<RemoteWrapper>, &str)>) -> Addr<Cluster> {
        let listener = Cluster::bind(ip_address.clone()).unwrap();

        debug!("Listening on {}", ip_address);
        Cluster::create(move |ctx| {
            ctx.add_message_stream(Box::leak(listener).incoming().map(|st| {
                let st = st.unwrap();
                let addr = st.peer_addr().unwrap();
                TcpConnect(st, addr)
            }));

            let address_resolver = Supervisor::start(|_| AddressResolver::new());
            for (rec, identifier) in rec_to_be_registered.iter() {
                address_resolver.do_send(AddressRequest::Register(rec.clone(), identifier.to_string()));
            }
            Cluster {ip_address, addrs: seed_nodes, listeners: cluster_listeners, gossip: None, address_resolver, own_addr: None, nodes: HashMap::new()}
        })
    }

    fn bind(addr: String) -> IoResult<Box<TcpListener>> {
        let addr = net::SocketAddr::from_str(&addr).unwrap();
        let listener = Box::new(block_on(TcpListener::bind(&addr)).unwrap());
        Ok(listener)
    }

    fn add_node(&mut self, node_addr: String) {
        let addr = SocketAddr::from_str(&node_addr).unwrap();
        let own_ip = self.ip_address.clone();
        let parent = self.own_addr.clone().unwrap();
        let gossip = self.gossip.clone().unwrap();
        let address_resolver = self.address_resolver.clone();
        let node = NetworkInterface::new(own_ip, addr, parent, gossip, address_resolver).start();
        self.nodes.insert(node_addr.clone(), node);
    }

    #[allow(dead_code)]
    fn register_actor(&self, rec: Recipient<RemoteWrapper>, actor_identifier: &str) {
        self.own_addr.as_ref().unwrap().register_actor(rec, actor_identifier)
    }
}

/// Helper for registering actors to the cluster
pub trait AddrApi {
    fn register_actor(&self, rec: Recipient<RemoteWrapper>, actor_identifier: &str);
    fn request_node_addr(&self, socket_addr: String, rec: Recipient<NodeResolving>);
    fn request_node_addrs(&self, socket_addrs: Vec<String>, rec: Recipient<NodeResolving>);
}


impl AddrApi for Addr<Cluster> {
    fn register_actor(&self, addr: Recipient<RemoteWrapper>, actor_identifier: &str) -> () {
        let _r = self.do_send(AddressRequest::Register(addr, actor_identifier.to_string()));
    }

    fn request_node_addr(&self, socket_addr: String, rec: Recipient<NodeResolving>) -> () {
        self.do_send(NodeResolving::Request(socket_addr, rec))
    }

    fn request_node_addrs(&self, socket_addrs: Vec<String>, rec: Recipient<NodeResolving>) -> () {
        self.do_send(NodeResolving::VecRequest(socket_addrs, rec))
    }
}
