use log::*;
use actix::prelude::*;
use std::collections::{HashMap, HashSet};
use crate::network::NetworkInterface;
use crate::{RemoteAddr, Cluster, CustomSystemService, ConnectToNode, NodeEvents, Node};
use std::net::SocketAddr;
use std::iter::FromIterator;
use std::str::FromStr;
use rand::prelude::{IteratorRandom, ThreadRng};
use crate::cluster::connector::{Connector, ConnectorVariant};
use crate::cluster::connector::messages::{GossipEvent, GossipJoining, GossipMessage, NodeResolving};


const CONNECTOR: &str = "Connector";

enum GossipState {
    Lonely,
    Joining,
    Joined
}

pub struct Gossip {
    own_addr: SocketAddr,
    members: HashMap<SocketAddr, Addr<NetworkInterface>>,
    waiting_to_add: HashSet<SocketAddr>,
    state: GossipState,
    about_to_join: usize,
    gossip_msgs: Vec<GossipMessage>
}

impl Default for Gossip {
    fn default() -> Self {
        Self {
            own_addr: SocketAddr::from_str("127.0.0.1:8000").unwrap(),
            members: HashMap::new(),
            waiting_to_add: HashSet::new(),
            state: GossipState::Lonely,
            about_to_join: 0,
            gossip_msgs: vec![]
        }
    }
}

impl Gossip {
    pub fn new(own_addr: SocketAddr) -> Self {
        Self {own_addr, ..Default::default()}
    }

    fn add_member(&mut self, node: Node) {
        self.members.insert(node.socket_addr.clone(), node.network_interface.expect("Empty network interface"));
        debug!("Member {} added!", node.socket_addr.to_string());
    }

    fn remove_member(&mut self, addr: SocketAddr) {
        self.members.remove(&addr);
        debug!("Member {} removed", addr.to_string());
    }

    fn ignite_member_up(&self, new_addr: SocketAddr) {
        self.gossip_member_event(new_addr, GossipEvent::Join, HashSet::from_iter([self.own_addr.clone()]));
    }

    fn ignite_member_down(&self, leaving_addr: SocketAddr) {
        self.gossip_member_event(leaving_addr, GossipEvent::Leave, HashSet::from_iter([self.own_addr.clone()]));
    }

    fn gossip_member_event(&self, addr: SocketAddr, event: GossipEvent, seen: HashSet<SocketAddr>) {
        let random_members = self.choose_random_members(3);

        let gossip_message = GossipMessage {
            event,
            addr,
            seen
        };

        for member in random_members {
            member.do_send(gossip_message.clone())
        }
    }

    fn choose_random_members(&self, amount: usize) -> Vec<RemoteAddr> {
        let mut rng = ThreadRng::default();
        self.members.iter()
            .choose_multiple(&mut rng, amount).into_iter()
            .map(|(socket_addr, network_interface)| RemoteAddr::new_connector(socket_addr.clone(), Some(network_interface.clone())))
            .collect()
    }

    fn connect_to_node(&mut self, addr: &SocketAddr) {
        self.waiting_to_add.insert(addr.clone());
        Cluster::from_custom_registry().do_send(ConnectToNode(addr.clone()))
    }

    fn all_seen(&self, seen: &HashSet<SocketAddr>) -> bool {
        let members: HashSet<SocketAddr> = self.members.keys().cloned().collect();
        members.difference(seen).into_iter().collect::<HashSet<&SocketAddr>>().is_empty()
    }

    pub(crate) fn handle_gossip_message(&mut self, msg: GossipMessage) {
        let all_seen = self.all_seen(&msg.seen);
        let mut seen = msg.seen;
        let member_contains = self.members.contains_key(&msg.addr);

        match &msg.event {
            GossipEvent::Join => {
                if member_contains & all_seen {
                    return
                }

                if !member_contains {
                    seen.insert(self.own_addr);
                    self.connect_to_node(&msg.addr);
                }
            },
            GossipEvent::Leave => {
                if !member_contains & all_seen {
                    return
                }

                if member_contains {
                    seen.insert(self.own_addr);
                    self.members.remove(&msg.addr);
                }
            }
        }

        self.gossip_member_event(msg.addr, msg.event, seen);
    }

    pub(crate) fn handle_gossip_joining(&mut self, msg: GossipJoining) {
        self.about_to_join = msg.about_to_join;
        if self.about_to_join == self.members.len() {
            self.state = GossipState::Joined;
        }
    }
}


impl ConnectorVariant for Gossip {
    fn handle_node_event(&mut self, msg: NodeEvents, ctx: &mut Context<Connector>) {
        match msg {
            NodeEvents::MemberUp(node, seed) => {
                self.add_member(node.clone());
                if !self.waiting_to_add.remove(&node.socket_addr) {
                    match &self.state {
                        GossipState::Lonely => {
                            if seed { // if connecting node is seed, we are joining
                                self.state = GossipState::Joining;
                            } else { // else we are the seed and therefore are already joined
                                self.state = GossipState::Joined;
                            }
                        },
                        GossipState::Joining => {
                            if self.members.len() == self.about_to_join {
                                self.state = GossipState::Joined;
                                for _ in 0..self.gossip_msgs.len() {
                                    if let Some(gossip_msg) = self.gossip_msgs.pop() {
                                        ctx.address().do_send(gossip_msg);
                                    }
                                }
                            }
                        },
                        GossipState::Joined => {
                            let remote_addr = node.get_remote_addr(CONNECTOR.to_string());
                            remote_addr.do_send(GossipJoining { about_to_join: self.members.len() });
                            self.ignite_member_up(node.socket_addr);
                        }
                    }
                }
            }
            NodeEvents::MemberDown(host) => {
                self.remove_member(host.clone());
                self.ignite_member_down(host);
            }
        }
    }

    fn handle_node_resolving(&mut self, msg: NodeResolving, _ctx: &mut Context<Connector>) -> Result<Vec<Addr<NetworkInterface>>, ()> {
        Ok(msg.addrs.into_iter().filter_map(|x| {
            if x.clone() == self.own_addr {
                None
            } else {
                Some(self.members.get(&x).expect(&format!("Socket {} should be known!", &x)).clone())
            }
        }).collect())
    }
}
