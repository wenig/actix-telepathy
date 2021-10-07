use std::ops::{Add, Div, Mul};
use actix::SystemService;
use crate::AnyAddr;
use crate::protocols::cluster_nodes::ClusterNodes;
use crate::protocols::ProtocolsReceiver;

pub trait ProtocolsHelper {
    fn get_protocols_receiver_addr<V: Add + Mul + Div>(&self, receiver_node_id: usize) -> AnyAddr<ProtocolsReceiver<V>>;
}

impl ProtocolsHelper for ClusterNodes {
    fn get_protocols_receiver_addr<V: Add + Mul + Div>(&self, receiver_node_id: usize) -> AnyAddr<ProtocolsReceiver<V>> {
        let mut receiver = match self.get(&receiver_node_id) {
            None => AnyAddr::Local(ProtocolsReceiver::from_registry()),
            Some(remote_addr) => AnyAddr::Remote(remote_addr.clone())
        };
        receiver.change_id(ProtocolsReceiver::ACTOR_ID);
        receiver
    }
}
