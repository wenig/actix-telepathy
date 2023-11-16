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
    about_to_join: Option<usize>,
    gossip_msgs: Vec<GossipMessage>,
    info_msgs_to_send: Vec<Node>,
    seed_nodes: Vec<SocketAddr>,
}

impl Default for Gossip {
    fn default() -> Self {
        Self {
            own_addr: SocketAddr::from_str("127.0.0.1:8000").unwrap(),
            members: HashMap::new(),
            waiting_to_add: HashSet::new(),
            state: GossipState::Lonely,
            about_to_join: None,
            gossip_msgs: vec![],
            info_msgs_to_send: vec![],
            seed_nodes: vec![],
        }
    }
}

impl Gossip {
    pub fn new(own_addr: SocketAddr, seed_nodes: Vec<SocketAddr>) -> Self {
        Self {
            own_addr,
            seed_nodes,
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
        let random_members = self.choose_random_members(3, &seen);

        let gossip_message = GossipMessage { event, addr, seen };

        for member in random_members {
            member.do_send(gossip_message.clone())
        }
    }

    fn choose_random_members(
        &self,
        amount: usize,
        except: &HashSet<SocketAddr>,
    ) -> Vec<RemoteAddr> {
        let mut rng = ThreadRng::default();
        self.members
            .iter()
            .filter(|(addr, _)| !except.contains(addr))
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
        seen.insert(self.own_addr);

        match &msg.event {
            GossipEvent::Join => {
                if member_contains & all_seen {
                    return;
                }

                if !member_contains {
                    self.connect_to_node(&msg.addr);
                }
            }
            GossipEvent::Leave => {
                if !member_contains & all_seen {
                    return;
                }

                if member_contains {
                    self.members.remove(&msg.addr);
                }
            }
        }

        self.gossip_member_event(msg.addr, msg.event, seen);
    }

    pub(crate) fn handle_gossip_joining(&mut self, msg: GossipJoining) {
        match self.state {
            GossipState::Joining => {
                debug!(target: &self.own_addr.to_string(), "Received a GossipJoining message while in JOINING state! {} members", msg.about_to_join);
                self.about_to_join = Some(msg.about_to_join);
                if msg.about_to_join <= self.members.len() {
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
                self.share_info_with_joining_members();
            }
            GossipState::Lonely => (),
        }
    }

    fn share_info_with_joining_member(&self, node: Node) {
        debug!(target: &self.own_addr.to_string(), "Sharing info with joining member {}", node.socket_addr.to_string());
        node.get_remote_addr(CONNECTOR.to_string())
            .do_send(GossipJoining {
                about_to_join: self.members.len(),
            });
        self.ignite_member_up(node.socket_addr);
    }

    fn share_info_with_joining_members(&mut self) {
        while let Some(node) = self.info_msgs_to_send.pop() {
            self.share_info_with_joining_member(node)
        }
    }

    fn seed_nodes_already_members(&self) -> bool {
        self.seed_nodes
            .iter()
            .all(|seed_node| self.members.contains_key(seed_node))
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
                                if self.seed_nodes_already_members() {
                                    self.change_state(GossipState::Joined);
                                    self.share_info_with_joining_member(node);
                                } else {
                                    self.change_state(GossipState::Joining);
                                    self.info_msgs_to_send.push(node);
                                }
                            } else {
                                // else we are not the seed and therefore need to join
                                self.change_state(GossipState::Joining);
                            }
                        }
                        GossipState::Joining => {
                            if seed {
                                warn!(target: &self.own_addr.to_string(), "Received a NodeEvent::MemberUp while in JOINING state but seed is true!");
                                self.info_msgs_to_send.push(node);
                            }

                            if let Some(about_to_join) = self.about_to_join {
                                if about_to_join == self.members.len() {
                                    self.change_state(GossipState::Joined);
                                }
                            } else {
                                warn!(target: &self.own_addr.to_string(), "Received a NodeEvent::MemberUp while in JOINING state but about_to_join is None!")
                            }
                        }
                        GossipState::Joined => {
                            if seed {
                                self.share_info_with_joining_member(node);
                            }
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
