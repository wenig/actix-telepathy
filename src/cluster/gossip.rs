use log::*;
use actix::prelude::*;
use crate::network::NetworkInterface;
use std::collections::HashMap;
use crate::cluster::cluster::NodeEvents;

#[derive(Message)]
#[rtype(result = "()")]
pub enum GossipEvent {
    MemberUp(String, Vec<String>),
    MemberDown(String, Vec<String>),
}

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
        self.gossip_event(GossipEvent::MemberUp(new_addr.clone(), vec![self.own_addr.clone(), new_addr]))
    }

    fn member_up(&mut self, new_addr: String, seen_addrs: Vec<String>) {
        // todo: tell cluster to add NetworkInterface
    }

    fn member_down(&mut self, new_addr: String, seen_addrs: Vec<String>) {
        // todo: tell cluster to remove NetworkInterface
    }

    fn gossip_event(&mut self, event: GossipEvent) {
        // todo: Randomly choose members to gossip to
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
        match msg {
            GossipEvent::MemberUp(new_addr, seen_addrs) => debug!("New member joined cluster"),
            GossipEvent::MemberDown(addr, seen_addrs) => debug!("Member left cluster"),
        }
    }
}

impl Handler<NodeEvents> for Gossip {
    type Result = ();

    fn handle(&mut self, msg: NodeEvents, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            NodeEvents::MemberUp(new_addr, node) => self.add_member(new_addr, node),
            NodeEvents::MemberDown(addr) => debug!("Member left cluster"),
        }
    }
}

impl Supervised for Gossip {}