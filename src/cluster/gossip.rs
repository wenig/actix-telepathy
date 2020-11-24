use log::*;
use actix::prelude::*;
use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};
use crate::network::NetworkInterface;
use crate::codec::ClusterMessage;
use crate::remote::{RemoteWrapper, Remotable};
use crate::{RemoteAddr, Cluster, NodeResolving};
use crate::{DefaultSerialization, CustomSerialization};
use actix_telepathy_derive::{RemoteActor, RemoteMessage};
use rand::thread_rng;
use rand::prelude::{IteratorRandom};
use crate::cluster::cluster::GossipResponse;
use std::iter::FromIterator;
use serde::export::fmt::Debug;
use serde::export::Formatter;
use std::fmt;

#[derive(Message, Serialize, Deserialize, Debug, RemoteMessage)]
#[rtype(result = "()")]
pub struct GossipEvent {
    members: Vec<String>
}

#[derive(Message)]
#[rtype(result = "()")]
pub enum GossipIgniting {
    MemberUp(String, Addr<NetworkInterface>),
    MemberDown(String)
}

#[derive(Message)]
#[rtype(result = "()")]
pub enum MemberMgmt {
    MemberUp(String, Addr<NetworkInterface>),
    MemberDown(String)
}

#[derive(RemoteActor)]
#[remote_messages(GossipEvent)]
pub struct Gossip {
    own_addr: String,
    requested_members: HashSet<String>,
    members: HashMap<String, Addr<NetworkInterface>>,
    cluster: Addr<Cluster>
}

impl Gossip {
    pub fn new(own_addr: String, cluster: Addr<Cluster>) -> Gossip {
        Gossip {own_addr, requested_members: HashSet::new(), members: HashMap::new(), cluster}
    }

    fn add_member(&mut self, new_addr: String, node: Addr<NetworkInterface>) {
        self.requested_members.remove(&new_addr);
        self.members.insert(new_addr.clone(), node);
        debug!("Member {} added! {:?}", new_addr.clone(), self.members.keys());
    }

    fn remove_member(&mut self, addr: String) {
        self.members.remove(&addr);
        debug!("Member {} removed", addr.clone());
    }

    fn member_up(&mut self, new_addr: String) {
        //self.gossip_forward(new_addr, seen_addrs, true);
        self.gossip_members(new_addr)
    }

    fn member_down(&mut self, addr: String, seen_addrs: Vec<String>) {
        //self.gossip_forward(addr, seen_addrs, false);
    }

    fn gossip_members(&mut self, member_addr: String) {
        let members: Vec<String> = self.members.keys().into_iter().filter_map(|x| {
            if x.as_str() == member_addr.as_str() {
                None
            } else {
                Some(x.clone())
            }
        }).collect();

        if members.len() > 0 {
            match self.members.get(member_addr.as_str()) {
                Some(node) =>
                    RemoteAddr::new_gossip(member_addr, Some(node.clone()))
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
        debug!("gossip received: {:?}", msg);
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
                let node_addr = self.members.get(socket_addr.as_str());
                match node_addr {
                    Some(addr) => {
                        let _r = sender.do_send(NodeResolving::Response(addr.clone()));
                    },
                    None => error!("No NetworkInterface with id {} exists", socket_addr.as_str())
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
