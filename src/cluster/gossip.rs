use crate::network::NetworkInterface;
use crate::remote::{RemoteActor, RemoteMessage, RemoteWrapper};
use crate::{Cluster, CustomSystemService, GossipResponse, RemoteAddr};
use crate::{CustomSerialization, DefaultSerialization};
use actix::prelude::*;
use actix_telepathy_derive::{RemoteActor, RemoteMessage};
use log::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::net::SocketAddr;
use std::str::FromStr;

#[derive(RemoteMessage, Serialize, Deserialize, Debug)]
pub struct GossipEvent {
    members: Vec<SocketAddr>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub enum GossipIgniting {
    MemberUp(SocketAddr, Addr<NetworkInterface>),
    MemberDown(SocketAddr),
}

#[derive(Message)]
#[rtype(result = "()")]
pub enum MemberMgmt {
    MemberUp(SocketAddr, Addr<NetworkInterface>),
    MemberDown(SocketAddr),
}

#[derive(Message)]
#[rtype(result = "Result<Vec<Addr<NetworkInterface>>, ()>")]
pub struct NodeResolving {
    pub addrs: Vec<SocketAddr>,
}

#[derive(RemoteActor)]
#[remote_messages(GossipEvent)]
pub struct Gossip {
    own_addr: SocketAddr,
    cluster: Addr<Cluster>,
    requested_members: HashSet<SocketAddr>,
    members: HashMap<SocketAddr, Addr<NetworkInterface>>,
}

impl Default for Gossip {
    fn default() -> Self {
        Self {
            own_addr: SocketAddr::from_str("127.0.0.1:8000").unwrap(),
            cluster: Cluster::from_custom_registry(),
            requested_members: HashSet::new(),
            members: HashMap::new(),
        }
    }
}

impl Gossip {
    pub fn new(own_addr: SocketAddr) -> Gossip {
        Gossip {
            own_addr,
            ..Default::default()
        }
    }

    fn add_member(&mut self, new_addr: SocketAddr, node: Addr<NetworkInterface>) {
        self.requested_members.remove(&new_addr);
        self.members.insert(new_addr, node);
        debug!("Member {} added!", new_addr.to_string());
    }

    fn remove_member(&mut self, addr: SocketAddr) {
        self.members.remove(&addr);
        debug!("Member {} removed", addr.to_string());
    }

    fn member_up(&mut self, new_addr: SocketAddr) {
        self.gossip_members(new_addr)
    }

    fn member_down(&mut self, _addr: SocketAddr, _seen_addrs: Vec<SocketAddr>) {}

    fn gossip_members(&mut self, member_addr: SocketAddr) {
        let members: Vec<SocketAddr> = self
            .members
            .keys()
            .filter_map(|x| if x.eq(&member_addr) { None } else { Some(*x) })
            .collect();

        if !members.is_empty() {
            match self.members.get(&member_addr) {
                Some(node) => RemoteAddr::new_gossip(member_addr, Some(node.clone()))
                    .do_send(GossipEvent { members }),
                None => error!("Should be known by now"),
            }
        }
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
                self.add_member(new_addr, node);
                self.member_up(new_addr)
            }
            GossipIgniting::MemberDown(addr) => {
                self.remove_member(addr);
                self.member_down(addr, vec![self.own_addr])
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
        Ok(msg
            .addrs
            .into_iter()
            .filter_map(|x| {
                if x == self.own_addr {
                    None
                } else {
                    Some(
                        self.members
                            .get(&x)
                            .unwrap_or_else(|| panic!("Socket {} should be known!", &x))
                            .clone(),
                    )
                }
            })
            .collect())
    }
}

impl Supervised for Gossip {}
impl SystemService for Gossip {}
impl CustomSystemService for Gossip {
    fn custom_service_started(&mut self, _ctx: &mut Context<Self>) {
        debug!("Gossip Service started");
    }
}
