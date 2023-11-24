use super::{messages::SingleSeedMembers, ConnectorVariant};
use crate::{
    Cluster, ConnectToNode, CustomSystemService, NetworkInterface, Node, NodeEvent, RemoteAddr,
};
use actix::Addr;
use log::*;
use std::{collections::HashMap, net::SocketAddr, str::FromStr};

/// The SingleSeed connector variant expects all nodes to have the same seed node (except the seed node itself, it has no seed node).
/// If another node is added, it will be added to the cluster by the seed node.
/// If a node has a different seed node, errors can occur.
/// This variant is recommended for a fast connection setup, but it is not recommended if the seed node is not always available.
pub struct SingleSeed {
    own_addr: SocketAddr,
    members: HashMap<SocketAddr, Addr<NetworkInterface>>,
}

impl Default for SingleSeed {
    fn default() -> Self {
        Self {
            own_addr: SocketAddr::from_str("127.0.0.1:8000").unwrap(),
            members: HashMap::new(),
        }
    }
}

impl SingleSeed {
    pub fn new(own_addr: SocketAddr) -> Self {
        Self {
            own_addr,
            ..Default::default()
        }
    }

    fn add_member(&mut self, node: &Node) {
        self.members
            .insert(node.socket_addr, node.clone().network_interface.unwrap());
        debug!(target: &self.own_addr.to_string(), "Member {} added!", node.socket_addr.to_string());
    }

    fn remove_member(&mut self, addr: SocketAddr) {
        self.members.remove(&addr);
        debug!("Member {} removed", addr.to_string());
    }

    fn give_information(&mut self, member_addr: SocketAddr) {
        let members: Vec<SocketAddr> = self
            .members
            .keys()
            .filter_map(|x| if x.eq(&member_addr) { None } else { Some(*x) })
            .collect();

        if !members.is_empty() {
            match self.members.get(&member_addr) {
                Some(node) => RemoteAddr::new_connector(member_addr, Some(node.clone()))
                    .do_send(SingleSeedMembers(members)),
                None => error!("Should be known by now"),
            }
        }
    }

    pub(crate) fn handle_single_seed_members(&mut self, msg: SingleSeedMembers) {
        for addr in msg.0 {
            Cluster::from_custom_registry().do_send(ConnectToNode(addr))
        }
    }
}

impl ConnectorVariant for SingleSeed {
    fn handle_node_event(
        &mut self,
        msg: crate::NodeEvent,
        _ctx: &mut actix::prelude::Context<crate::Connector>,
    ) {
        match msg {
            NodeEvent::MemberUp(node, seed) => {
                self.add_member(&node);
                if seed {
                    self.give_information(node.socket_addr);
                }
            }
            NodeEvent::MemberDown(addr) => {
                self.remove_member(addr);
            }
        }
    }

    fn get_member_map(
        &mut self,
        _msg: crate::NodeResolving,
        _ctx: &mut actix::prelude::Context<crate::Connector>,
    ) -> &HashMap<SocketAddr, Addr<NetworkInterface>> {
        &self.members
    }

    fn get_own_addr(&self) -> SocketAddr {
        self.own_addr
    }
}
