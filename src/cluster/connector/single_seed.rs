use std::{net::SocketAddr, collections::HashMap};
use actix::Addr;
use crate::{NetworkInterface, NodeEvent};
use super::ConnectorVariant;

pub struct SingleSeed {
    own_addr: SocketAddr,
    members: HashMap<SocketAddr, Addr<NetworkInterface>>,
}


impl ConnectorVariant for SingleSeed {
    fn handle_node_event(&mut self, msg: crate::NodeEvent, ctx: &mut actix::prelude::Context<crate::Connector>) {
        match msg {
            NodeEvent::MemberUp(node, seed) => {}
            NodeEvent::MemberDown(addr) => {}
        }
    }

    fn handle_node_resolving(
        &mut self,
        msg: crate::NodeResolving,
        _ctx: &mut actix::prelude::Context<crate::Connector>,
    ) -> Result<Vec<actix::prelude::Addr<crate::NetworkInterface>>, ()> {
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