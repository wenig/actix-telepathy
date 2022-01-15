use log::*;
use actix::prelude::*;
use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};
use crate::network::NetworkInterface;
use crate::remote::{RemoteWrapper, RemoteMessage, RemoteActor};
use crate::{RemoteAddr, Cluster, CustomSystemService, GossipResponse};
use crate::{DefaultSerialization, CustomSerialization};
use actix_telepathy_derive::{RemoteActor, RemoteMessage};
use std::net::SocketAddr;
use std::fmt::Debug;
use std::str::FromStr;
use rand::prelude::{IteratorRandom, ThreadRng};


#[derive(Message)]
#[rtype(result = "()")]
pub enum GossipIgniting {
    MemberUp(SocketAddr, Addr<NetworkInterface>),
    MemberDown(SocketAddr)
}

#[derive(Clone, Debug)]
pub enum GossipEvent {
    Join,
    Leave
}

#[derive(RemoteMessage, Serialize, Deserialize, Debug)]
pub struct GossipMessage {
    pub event: GossipEvent,
    addr: SocketAddr,
    counter: usize
}

#[derive(Message)]
#[rtype(result = "()")]
pub enum MemberMgmt {
    MemberUp(SocketAddr, Addr<NetworkInterface>),
    MemberDown(SocketAddr)
}

#[derive(Message)]
#[rtype(result = "Result<Vec<Addr<NetworkInterface>>, ()>")]
pub struct NodeResolving{
    pub addrs: Vec<SocketAddr>
}

#[derive(RemoteActor)]
#[remote_messages(GossipEvent)]
pub struct Gossip {
    own_addr: SocketAddr,
    cluster: Addr<Cluster>,
    members: HashMap<SocketAddr, Addr<NetworkInterface>>,
}

impl Default for Gossip {
    fn default() -> Self {
        Self {
            own_addr: SocketAddr::from_str("127.0.0.1:8000").unwrap(),
            cluster: Cluster::from_custom_registry(),
            members: HashMap::new(),
        }
    }
}

impl Gossip {
    pub fn new(own_addr: SocketAddr) -> Gossip {
        Gossip {own_addr, ..Default::default()}
    }

    fn add_member(&mut self, new_addr: SocketAddr, node: Addr<NetworkInterface>) {
        self.members.insert(new_addr.clone(), node);
        debug!("Member {} added!", new_addr.to_string());
    }

    fn remove_member(&mut self, addr: SocketAddr) {
        self.members.remove(&addr);
        debug!("Member {} removed", addr.to_string());
    }

    fn ignite_member_up(&self, new_addr: SocketAddr) {
        self.gossip_member_event(new_addr, GossipEvent::Join, 1);
    }

    fn ignite_member_down(&self, leaving_addr: SocketAddr) {
        self.gossip_member_event(leaving_addr, GossipEvent::Leave, 1);
    }

    fn gossip_member_event(&self, addr: SocketAddr, event: GossipEvent, counter: usize) {
        let random_members = self.choose_random_members(3);

        let gossip_message = GossipMessage {
            event,
            addr,
            counter
        };

        for member in random_members {
            member.do_send(gossip_message.clone())
        }
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
                    RemoteAddr::new_gossip(member_addr, Some(node.clone()))
                        .do_send(GossipEvent { members }),
                None => error!("Should be known by now")
            }
        }
    }

    fn choose_random_members(&self, amount: usize) -> Vec<RemoteAddr> {
        let mut rng = ThreadRng::default();
        self.members.iter()
            .choose_multiple(&mut rng, amount).into_iter()
            .map(|(socket_addr, network_interface)| RemoteAddr::new_gossip(socket_addr.clone(), Some(network_interface.clone())))
            .collect()
    }
}

impl Actor for Gossip {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.register(ctx.address().recipient());
        debug!("{} actor started", Self::ACTOR_ID);
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
                self.ignite_member_up(new_addr);
                self.add_member(new_addr.clone(), node);
            },
            GossipIgniting::MemberDown(addr) => {
                self.ignite_member_down(addr);
                self.remove_member(addr.clone());
            },
        }
    }
}

impl Handler<GossipMessage> for Gossip {
    type Result = ();

    fn handle(&mut self, msg: GossipMessage, _ctx: &mut Self::Context) -> Self::Result {
        match &msg.event {
            GossipEvent::Join => {
                if self.members.contains_key(&msg.addr) {
                    if msg.counter != self.members.len() { // todo: check if own addr is part of it
                        self.gossip_member_event(msg.addr, msg.event, msg.counter + 1);
                    }
                } else {
                    self.gossip_member_event(msg.addr.clone(), msg.event.clone(), msg.counter + 1);
                    // member is added after node connects
                }
            }
            GossipEvent::Leave => {
                if !self.members.contains_key(&msg.addr) {
                    if msg.counter != self.members.len() { // todo: check if own addr is part of it
                        self.gossip_member_event(msg.addr, msg.event, msg.counter + 1);
                    }
                } else {
                    self.gossip_member_event(msg.addr.clone(), msg.event, msg.counter + 1);
                    self.remove_member(msg.addr)
                }
            }
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
    type Result = Result<Vec<Addr<NetworkInterface>>, ()>;

    fn handle(&mut self, msg: NodeResolving, _ctx: &mut Context<Self>) -> Self::Result {
        Ok(msg.addrs.into_iter().filter_map(|x| {
            if x.clone() == self.own_addr {
                None
            } else {
                Some(self.members.get(&x).expect(&format!("Socket {} should be known!", &x)).clone())
            }
        }).collect())
    }
}

impl Supervised for Gossip {}
impl SystemService for Gossip {}
impl CustomSystemService for Gossip {
    fn custom_service_started(&mut self, _ctx: &mut Context<Self>) {
        debug!("Gossip Service started");
    }
}
