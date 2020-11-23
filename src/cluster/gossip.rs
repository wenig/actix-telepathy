use log::*;
use actix::prelude::*;
use std::collections::{HashMap};
use serde::{Serialize, Deserialize};
use crate::network::NetworkInterface;
use crate::codec::ClusterMessage;
use crate::remote::{RemoteWrapper, Remotable};
use crate::{RemoteAddr, Cluster, NodeResolving};
use crate::{DefaultSerialization, CustomSerialization};
use actix_telepathy_derive::{RemoteActor};
use rand::thread_rng;
use rand::prelude::{IteratorRandom};
use crate::cluster::cluster::GossipResponse;

#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct GossipEvent {
    addr: String,
    seen_addrs: Vec<String>,
    add: bool
}

impl Remotable for GossipEvent {
    type Serializer = DefaultSerialization;
    const IDENTIFIER: &'static str = "";

    fn get_serializer(&self) -> Box<Self::Serializer> {
        Box::new(DefaultSerialization {})
    }

    fn generate_serializer() -> Box<Self::Serializer> {
        Box::new(DefaultSerialization {})
    }

    fn set_source(&mut self, _addr: Addr<NetworkInterface>) {}
}

impl GossipEvent {
    pub fn member_up(addr: String, seen_addrs: Vec<String>) -> GossipEvent {
        GossipEvent {addr, seen_addrs, add: true}
    }

    pub fn member_down(addr: String, seen_addrs: Vec<String>) -> GossipEvent {
        GossipEvent {addr, seen_addrs, add: false}
    }
}

impl Clone for GossipEvent {
    fn clone(&self) -> Self {
        GossipEvent {addr: self.addr.clone(), seen_addrs: self.seen_addrs.clone(), add: self.add}
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub enum GossipIgniting {
    MemberUp(String, Addr<NetworkInterface>),
    MemberDown(String)
}

#[derive(RemoteActor)]
#[remote_messages(GossipEvent)]
pub struct Gossip {
    own_addr: String,
    requested_members: Vec<String>,
    members: HashMap<String, Addr<NetworkInterface>>,
    cluster: Addr<Cluster>
}

impl Gossip {
    pub fn new(own_addr: String, cluster: Addr<Cluster>) -> Gossip {
        Gossip {own_addr, requested_members: vec![], members: HashMap::new(), cluster}
    }

    fn add_member(&mut self, new_addr: String, node: Addr<NetworkInterface>) {
        debug!("Member {} added!", new_addr.clone());
        match self.requested_members.iter().position(|x| x.clone() == new_addr.clone()) {
            Some(pos) => { self.requested_members.remove(pos); },
            _ => {}
        }
        self.members.insert(new_addr.clone(), node);
        self.member_up(new_addr, vec![self.own_addr.clone()]);
    }

    fn remove_member(&mut self, addr: String) {
        debug!("Member {} removed", addr.clone());
        self.members.remove(&addr);
        self.member_down(addr, vec![self.own_addr.clone()]);
    }

    fn member_up(&mut self, new_addr: String, seen_addrs: Vec<String>) {
        match self.members.get(new_addr.as_str()) {
            Some(_) => {},
            None => {
                if new_addr.clone() != self.own_addr {
                    self.requested_members.push(new_addr.clone());
                    self.cluster.do_send(GossipResponse { 0: new_addr.clone() })
                }
            }
        }
        self.gossip_forward(new_addr, seen_addrs, true);
    }

    fn member_down(&mut self, addr: String, seen_addrs: Vec<String>) {
        self.gossip_forward(addr, seen_addrs, false);
    }

    fn gossip_forward(&mut self, member_addr: String, seen_addrs: Vec<String>, up: bool) {
        if seen_addrs.len() >= self.members.len() {
            return;
        }

        let rng = &mut thread_rng();
        let mut chosen_addrs: Vec<String> = self.members.keys().choose_multiple(rng, 3).iter().map(|&x| x.clone()).collect();
        chosen_addrs.extend(seen_addrs);
        chosen_addrs.sort();
        chosen_addrs.dedup();

        let gossip_event = if up {
            GossipEvent::member_up(member_addr.clone(), chosen_addrs.clone())
        } else {
            GossipEvent::member_down(member_addr.clone(), chosen_addrs.clone())
        };

        for addr in chosen_addrs.iter() {
            match self.members.get(addr) {
                Some(node) => node.do_send(ClusterMessage::Message(
                    RemoteWrapper::new(
                        RemoteAddr::new_gossip(addr.clone(), None),
                        Box::new(gossip_event.clone()),
                    )
                )),
                None => {}//debug!("doesn't know {}", addr)
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
        if msg.add {
            self.member_up(msg.addr, msg.seen_addrs)
        } else {
            self.member_down(msg.addr, msg.seen_addrs)
        }
    }
}

impl Handler<GossipIgniting> for Gossip {
    type Result = ();

    fn handle(&mut self, msg: GossipIgniting, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            GossipIgniting::MemberUp(new_addr, node) => self.add_member(new_addr, node),
            GossipIgniting::MemberDown(addr) => self.remove_member(addr),
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