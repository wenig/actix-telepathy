use actix::prelude::*;
use std::net;
use std::io::{Error, ErrorKind, Result};
use tokio::net::{TcpListener, TcpStream};
use std::collections::{HashMap, VecDeque};
use log::*;

use crate::{utils, ClusterLog, ClusterListener};
use crate::network::NetworkInterface;
use std::str::FromStr;
use futures::StreamExt;
use futures::executor::block_on;
use std::net::SocketAddr;
use std::any::Any;
use crate::cluster::gossip::{Gossip, GossipEvent, GossipIgniting};
use crate::remote_addr::RemoteAddr;


#[derive(Message)]
#[rtype(result = "()")]
pub struct TcpConnect(pub TcpStream, pub SocketAddr);

#[derive(Message)]
#[rtype(result = "()")]
pub enum NodeEvents{
    MemberUp(String, Addr<NetworkInterface>, RemoteAddr),
    MemberDown(String)
}

pub struct Cluster {
    ip_address: String,
    addrs: Vec<String>,
    listeners: Vec<Recipient<ClusterLog>>,
    gossip: Addr<Gossip>,
    own_addr: Option<Addr<Cluster>>,
    nodes: Vec<Addr<NetworkInterface>>
}

pub type CL = dyn ClusterListener<Context=dyn Any, Result=()>;

impl Actor for Cluster {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.own_addr = Some(ctx.address());
        for node_addr in self.addrs.iter() {
            let addr = SocketAddr::from_str(&node_addr).unwrap();
            let own_ip = self.ip_address.clone();
            let parent = self.own_addr.clone().unwrap();
            let node = Supervisor::start(move |_| NetworkInterface::new(own_ip, addr, parent));
            self.nodes.push(node);
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
        let node = Supervisor::start(move |_| NetworkInterface::from_stream(own_ip, addr, stream, parent));
        self.nodes.push(node);
    }
}

impl Handler<NodeEvents> for Cluster {
    type Result = ();

    fn handle(&mut self, msg: NodeEvents, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            NodeEvents::MemberUp(host, node, remote_addr) => {
                self.gossip.do_send(GossipIgniting::MemberUp(host.clone(), node));
                for listener in self.listeners.iter() {
                    listener.do_send(ClusterLog::NewMember(host.clone(), remote_addr.clone()));
                }
            },
            NodeEvents::MemberDown(host) => {
                self.gossip.do_send(GossipIgniting::MemberDown(host.clone()));
                for listener in self.listeners.iter() {
                    listener.do_send(ClusterLog::MemberLeft(host.clone()));
                }
            }
        }

    }
}

impl Cluster {
    pub fn new<S: Into<String>>(ip_address: String, seed_nodes: Option<S>, cluster_listeners: Vec<Recipient<ClusterLog>>) -> Addr<Cluster> {
        let listener = Cluster::bind(ip_address.clone()).unwrap();

        debug!("Listening on {}", ip_address);
        Cluster::create(move |ctx| {
            ctx.add_message_stream(Box::leak(listener).incoming().map(|st| {
                let st = st.unwrap();
                let addr = st.peer_addr().unwrap();
                TcpConnect(st, addr)
            }));

            let mut addrs: Vec<String> = Vec::new();
            seed_nodes.map(|node_addr|{
                let node_addr = node_addr.into();
                addrs.push(node_addr.clone());
            });

            let own_ip_addres = ip_address.clone();
            let gossip = Supervisor::start(move |_| Gossip::new(own_ip_addres));
            Cluster {ip_address, addrs, listeners: cluster_listeners, gossip, own_addr: None, nodes: vec![]}
        })
    }

    fn bind(addr: String) -> Result<Box<TcpListener>> {
        let addr = net::SocketAddr::from_str(&addr).unwrap();
        let listener = Box::new(block_on(TcpListener::bind(&addr)).unwrap());
        Ok(listener)
    }
}
