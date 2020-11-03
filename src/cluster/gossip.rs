use log::*;
use actix::prelude::*;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::str::FromStr;
use rand::thread_rng;
use rand::seq::SliceRandom;
use crate::network::NetworkInterface;
use crate::cluster::cluster::NodeEvents;
use crate::codec::ClusterMessage;
use crate::remote::{RemoteWrapper, Remotable, AddrRepresentation};
use crate::RemoteAddr;
use crate::{DefaultSerialization, CustomSerialization};
use actix_telepathy_derive::{RemoteActor, RemoteMessage};

#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype(result = "()")]
pub struct GossipEvent {
    addr: String,
    seen_addrs: Vec<String>,
    add: bool
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
    members: HashMap<String, Addr<NetworkInterface>>
}

impl Gossip {
    pub fn new(own_addr: String) -> Gossip {
        Gossip {own_addr, members: HashMap::new()}
    }

    fn add_member(&mut self, new_addr: String, node: Addr<NetworkInterface>) {
        debug!("Member {} added!", new_addr.clone());
        self.members.insert(new_addr.clone(), node);
        self.gossip_event(GossipEvent::member_up(new_addr.clone(), vec![self.own_addr.clone(), new_addr]));
    }

    fn remove_member(&mut self, addr: String) {
        debug!("Member {} removed", addr.clone());
        self.members.remove(&addr);
        self.gossip_event(GossipEvent::member_down(addr, vec![self.own_addr.clone()]));
    }

    fn member_up(&mut self, new_addr: String, seen_addrs: Vec<String>) {
        // todo: tell cluster to add NetworkInterface (randomly choose nodes from seen_addrs)
        /*let mut rng = &mut thread_rng();
        let chosen_addrs: Vec<String> = seen_addrs.choose_multiple(rng, 3);*/
    }

    fn member_down(&mut self, new_addr: String, seen_addrs: Vec<String>) {
        // todo: tell cluster to remove NetworkInterface (randomly choose nodes from seen_addrs)
    }

    fn gossip_event(&mut self, event: GossipEvent) {
        for (addr, network_interface) in self.members.iter() {
            network_interface.do_send(ClusterMessage::Message(
                RemoteWrapper::new(
                    RemoteAddr::new_gossip(addr.clone(), None),
                    Box::new(event.clone()))
            ));
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
            debug!("Member joined cluster");
        } else {
            debug!("Member left cluster");
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

impl Supervised for Gossip {}