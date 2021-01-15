use log::*;
use actix::prelude::*;
use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};
use crate::network::NetworkInterface;
use crate::remote::{RemoteWrapper, RemoteMessage};
use crate::{RemoteAddr, Cluster, NodeResolving};
use crate::{DefaultSerialization, CustomSerialization};
use actix_telepathy_derive::{RemoteActor, RemoteMessage};
use crate::cluster::cluster::GossipResponse;
use serde::export::fmt::Debug;
use std::net::SocketAddr;
use std::str::FromStr;

#[derive(Message, Serialize, Deserialize, Debug, RemoteMessage)]
#[rtype(result = "()")]
pub struct GossipEvent {
    members: Vec<SocketAddr>
}

#[derive(Message)]
#[rtype(result = "()")]
pub enum GossipIgniting {
    MemberUp(SocketAddr, Addr<NetworkInterface>),
    MemberDown(SocketAddr)
}

#[derive(Message)]
#[rtype(result = "()")]
pub enum MemberMgmt {
    MemberUp(SocketAddr, Addr<NetworkInterface>),
    MemberDown(SocketAddr)
}

#[derive(RemoteActor)]
#[remote_messages(GossipEvent)]
pub struct Gossip {
    own_addr: SocketAddr,
    requested_members: HashSet<SocketAddr>,
    members: HashMap<SocketAddr, Addr<NetworkInterface>>,
    cluster: Addr<Cluster>
}

impl Default for Gossip {
    fn default() -> Self {
        Self {
            own_addr: SocketAddr::from_str("localhost:8000").unwrap(),
            requested_members: HashSet::new(),
            members: HashMap::new(),
            cluster: Cluster::from_registry()
        }
    }
}

impl Gossip {
    pub fn new(own_addr: SocketAddr) -> Gossip {
        Gossip {own_addr, ..Default::default()}
    }

    fn add_member(&mut self, new_addr: SocketAddr, node: Addr<NetworkInterface>) {
        self.requested_members.remove(&new_addr);
        self.members.insert(new_addr.clone(), node);
        debug!("Member {} added!", new_addr.to_string());
    }

    fn remove_member(&mut self, addr: SocketAddr) {
        self.members.remove(&addr);
        debug!("Member {} removed", addr.to_string());
    }

    fn member_up(&mut self, new_addr: SocketAddr) {
        //self.gossip_forward(new_addr, seen_addrs, true);
        self.gossip_members(new_addr)
    }

    fn member_down(&mut self, _addr: SocketAddr, _seen_addrs: Vec<SocketAddr>) {
        //self.gossip_forward(addr, seen_addrs, false);
    }

    fn gossip_members(&mut self, member_addr: SocketAddr) {
        let members: Vec<SocketAddr> = self.members.keys().into_iter().filter_map(|x| {
            if x.eq(&member_addr) {
                None
            } else {
                Some(x.clone())
            }
        }).collect();

        if members.len() > 0 {
            match self.members.get(&member_addr) {
                Some(node) =>
                    RemoteAddr::new_gossip(member_addr, Some(node.clone().recipient()))
                        .do_send(Box::new(GossipEvent { members })),
                None => error!("Should be known by now")
            }
        }
    }
}

impl Actor for Gossip {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        debug!("Gossip actor started");
    }
}

impl Handler<GossipEvent> for Gossip {
    type Result = ();

    fn handle(&mut self, msg: GossipEvent, _ctx: &mut Context<Self>) -> Self::Result {
        for addr in msg.members {
            self.cluster.do_send(GossipResponse(addr))
        }
    }
}

impl Handler<GossipIgniting> for Gossip {
    type Result = ();

    fn handle(&mut self, msg: GossipIgniting, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            GossipIgniting::MemberUp(new_addr, node) => {
                self.add_member(new_addr.clone(), node);
                self.member_up(new_addr)
            },
            GossipIgniting::MemberDown(addr) => {
                self.remove_member(addr.clone());
                self.member_down(addr, vec![self.own_addr.clone()])
            },
        }
    }
}

impl Handler<MemberMgmt> for Gossip {
    type Result = ();

    fn handle(&mut self, msg: MemberMgmt, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            MemberMgmt::MemberUp(new_addr, node) => self.add_member(new_addr, node),
            MemberMgmt::MemberDown(addr) => self.remove_member(addr),
        }
    }
}

impl Handler<NodeResolving> for Gossip {
    type Result = ();

    fn handle(&mut self, msg: NodeResolving, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            NodeResolving::Request(socket_addr, sender) => {
                let node_addr = self.members.get(&socket_addr);
                match node_addr {
                    Some(addr) => {
                        let _r = sender.do_send(NodeResolving::Response(addr.clone()));
                    },
                    None => error!("No NetworkInterface with id {} exists", socket_addr.to_string().as_str())
                };
            },
            NodeResolving::VecRequest(socket_addrs, sender) => {
                let node_addrs: Vec<Option<Addr<NetworkInterface>>> = socket_addrs.into_iter().map(|x| {
                    if x.clone() == self.own_addr {
                        None
                    } else {
                        Some(self.members.get(&x).expect(&format!("Socket {} should be known!", &x)).clone())
                    }
                }).collect();
                let _r = sender.do_send(NodeResolving::VecResponse(node_addrs));
            }
            _ => ()
        }
    }
}

impl Supervised for Gossip {}
impl SystemService for Gossip {}
