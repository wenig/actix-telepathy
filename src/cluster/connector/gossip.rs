use crate::cluster::connector::messages::{
    GossipEvent, GossipJoining, GossipMessage, NodeResolving,
};
use crate::cluster::connector::{Connector, ConnectorVariant};
use crate::network::NetworkInterface;
use crate::{Cluster, ConnectToNode, CustomSystemService, Node, NodeEvent, RemoteAddr};
use actix::prelude::*;
use log::*;
use rand::prelude::{IteratorRandom, ThreadRng};
use std::collections::{HashMap, HashSet};
use std::iter::FromIterator;
use std::net::SocketAddr;
use std::str::FromStr;

const CONNECTOR: &str = "Connector";

#[derive(Debug, Clone)]
enum GossipState {
    Lonely,
    Joining,
    Joined,
}

pub struct Gossip {
    own_addr: SocketAddr,
    members: HashMap<SocketAddr, Addr<NetworkInterface>>,
    waiting_to_add: HashSet<SocketAddr>,
    state: GossipState,
    about_to_join: usize,
    gossip_msgs: Vec<GossipMessage>,
}

impl Default for Gossip {
    fn default() -> Self {
        Self {
            own_addr: SocketAddr::from_str("127.0.0.1:8000").unwrap(),
            members: HashMap::new(),
            waiting_to_add: HashSet::new(),
            state: GossipState::Lonely,
            about_to_join: 0,
            gossip_msgs: vec![],
        }
    }
}

impl Gossip {
    pub fn new(own_addr: SocketAddr) -> Self {
        Self {
            own_addr,
            ..Default::default()
        }
    }

    fn add_member(&mut self, node: Node) {
        self.members.insert(
            node.socket_addr.clone(),
            node.network_interface.expect("Empty network interface"),
        );
        debug!(target: &self.own_addr.to_string(), "Member {} added!", node.socket_addr.to_string());
    }

    fn remove_member(&mut self, addr: SocketAddr) {
        self.members.remove(&addr);
        debug!(target: &self.own_addr.to_string(), "Member {} removed", addr.to_string());
    }

    fn ignite_member_up(&self, new_addr: SocketAddr) {
        debug!(target: &self.own_addr.to_string(), "Igniting member up {}", new_addr.to_string());
        self.gossip_member_event(
            new_addr,
            GossipEvent::Join,
            HashSet::from_iter([self.own_addr.clone(), new_addr.clone()]),
        );
    }

    fn ignite_member_down(&self, leaving_addr: SocketAddr) {
        debug!(target: &self.own_addr.to_string(), "Igniting member down {}", leaving_addr.to_string());
        self.gossip_member_event(
            leaving_addr,
            GossipEvent::Leave,
            HashSet::from_iter([self.own_addr.clone()]),
        );
    }

    fn gossip_member_event(&self, addr: SocketAddr, event: GossipEvent, seen: HashSet<SocketAddr>) {
        debug!(target: &self.own_addr.to_string(), "Gossiping member event {} {:?} {:?}", addr.to_string(), event, seen);
        let random_members = self.choose_random_members(3, addr.clone());

        let gossip_message = GossipMessage { event, addr, seen };

        for member in random_members {
            member.do_send(gossip_message.clone())
        }
    }

    fn choose_random_members(&self, amount: usize, except: SocketAddr) -> Vec<RemoteAddr> {
        let mut rng = ThreadRng::default();
        self.members
            .iter()
            .filter(|(addr, _)| !(*addr).eq(&except))
            .choose_multiple(&mut rng, amount)
            .into_iter()
            .map(|(socket_addr, network_interface)| {
                RemoteAddr::new_connector(socket_addr.clone(), Some(network_interface.clone()))
            })
            .collect()
    }

    fn connect_to_node(&mut self, addr: &SocketAddr) {
        self.waiting_to_add.insert(addr.clone());
        Cluster::from_custom_registry().do_send(ConnectToNode(addr.clone()))
    }

    fn all_seen(&self, seen: &HashSet<SocketAddr>) -> bool {
        let members: HashSet<SocketAddr> = self.members.keys().cloned().collect();
        members
            .difference(seen)
            .into_iter()
            .collect::<HashSet<&SocketAddr>>()
            .is_empty()
    }

    fn handle_gossip_queue(&mut self) {
        for _ in 0..self.gossip_msgs.len() {
            if let Some(gossip_msg) = self.gossip_msgs.pop() {
                self.handle_gossip_message(gossip_msg);
            }
        }
    }

    pub(crate) fn handle_gossip_message(&mut self, msg: GossipMessage) {
        match &self.state {
            GossipState::Lonely => {
                error!(target: &self.own_addr.to_string(), "Received a GossipMessage while in LONELY state!")
            }
            GossipState::Joining => {
                self.gossip_msgs.push(msg);
                return;
            }
            GossipState::Joined => (),
        }

        let all_seen = self.all_seen(&msg.seen);
        let mut seen = msg.seen;
        let member_contains = self.members.contains_key(&msg.addr);

        match &msg.event {
            GossipEvent::Join => {
                if member_contains & all_seen {
                    return;
                }

                if !member_contains {
                    seen.insert(self.own_addr);
                    self.connect_to_node(&msg.addr);
                }
            }
            GossipEvent::Leave => {
                if !member_contains & all_seen {
                    return;
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
        match self.state {
            GossipState::Joining => {
                self.about_to_join = msg.about_to_join;
                if self.about_to_join == self.members.len() {
                    self.change_state(GossipState::Joined);
                }
            }
            _ => {
                warn!(target: &self.own_addr.to_string(), "Received a GossipJoining message while not in JOINING state!")
            }
        }
    }

    fn change_state(&mut self, state: GossipState) {
        debug!(target: &self.own_addr.to_string(), "changing state to {:?}", state);
        self.state = state.clone();
        match state {
            GossipState::Joining => (),
            GossipState::Joined => {
                self.handle_gossip_queue();
            }
            GossipState::Lonely => (),
        }
    }
}

impl ConnectorVariant for Gossip {
    fn handle_node_event(&mut self, msg: NodeEvent, _ctx: &mut Context<Connector>) {
        match msg {
            NodeEvent::MemberUp(node, seed) => {
                self.add_member(node.clone());
                if !self.waiting_to_add.remove(&node.socket_addr) {
                    match &self.state {
                        GossipState::Lonely => {
                            if seed {
                                // if current node is considered seed, we are the seed and therefore are already joined
                                self.change_state(GossipState::Joined);

                                let remote_addr = node.get_remote_addr(CONNECTOR.to_string());
                                remote_addr.do_send(GossipJoining {
                                    about_to_join: self.members.len(),
                                });
                                self.ignite_member_up(node.socket_addr);
                            } else {
                                // else we are not the seed and therefore need to join
                                self.change_state(GossipState::Joining);
                            }
                        }
                        GossipState::Joining => {
                            if self.members.len() == self.about_to_join {
                                self.change_state(GossipState::Joined);
                            }
                        }
                        GossipState::Joined => {
                            let remote_addr = node.get_remote_addr(CONNECTOR.to_string());
                            remote_addr.do_send(GossipJoining {
                                about_to_join: self.members.len(),
                            });
                            self.ignite_member_up(node.socket_addr);
                        }
                    }
                }
            }
            NodeEvent::MemberDown(host) => {
                self.remove_member(host.clone());
                self.ignite_member_down(host);
            }
        }
    }

    fn handle_node_resolving(
        &mut self,
        msg: NodeResolving,
        _ctx: &mut Context<Connector>,
    ) -> Result<Vec<Addr<NetworkInterface>>, ()> {
        Ok(msg
            .addrs
            .into_iter()
            .filter_map(|x| {
                if x.clone() == self.own_addr {
                    None
                } else {
                    Some(
                        self.members
                            .get(&x)
                            .expect(&format!("Socket {} should be known!", &x))
                            .clone(),
                    )
                }
            })
            .collect())
    }
}
