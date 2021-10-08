use actix::SystemService;
use crate::{AnyAddr, CustomSystemService};
use crate::protocols::cluster_nodes::ClusterNodes;
use crate::protocols::ProtocolsReceiver;
use crate::RemoteActor;
use log::*;

pub trait ProtocolsHelper {
    fn get_protocols_receiver_addr(&self, receiver_node_id: usize) -> AnyAddr<ProtocolsReceiver>;
}

impl ProtocolsHelper for ClusterNodes {
    fn get_protocols_receiver_addr(&self, receiver_node_id: usize) -> AnyAddr<ProtocolsReceiver> {
        let mut receiver = match self.get(&receiver_node_id) {
            None => {
                AnyAddr::Local(ProtocolsReceiver::from_custom_registry())
            },
            Some(remote_addr) => {
                AnyAddr::Remote(remote_addr.clone())
            }
        };
        receiver.change_id(ProtocolsReceiver::ACTOR_ID);
        receiver
    }
}
