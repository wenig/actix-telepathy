use log::*;
use actix::prelude::*;
use actix_telepathy::*;
use crate::security::protocols::grouping::messages::GroupingMessage;
use std::collections::{VecDeque, HashSet};
use std::iter::FromIterator;
use std::borrow::Borrow;


#[derive(RemoteActor)]
#[remote_messages(GroupingMessage)]
pub struct GroupingServer {
    unresolved: VecDeque<RemoteAddr>,
    cluster: Addr<Cluster>,
    groups: Vec<Vec<RemoteAddr>>,
    group_size: usize,
    history_length: usize,
    full_group_idx: usize,
    own_addr: Option<Recipient<NodeResolving>>
}


impl GroupingServer {
    pub fn new(cluster: Addr<Cluster>, group_size: usize, history_length: usize) -> Self {
        Self {
            unresolved: VecDeque::new(),
            cluster,
            groups: vec![],
            group_size,
            history_length,
            full_group_idx: 0,
            own_addr: None
        }
    }

    fn get_history(&self, client: &RemoteAddr) -> HashSet<&RemoteAddr> {
        let mut history: HashSet<&RemoteAddr> = HashSet::new();
        let mut count_groups: usize = 0;
        let groups = &self.groups;

        for group_idx in self.full_group_idx..0 {
            let group = groups.get(group_idx).unwrap();
            if group.iter().position(|x| x.socket_addr == client.socket_addr).is_some() {
                history.extend(group);
                count_groups = count_groups + 1;
                if count_groups == self.history_length {
                    break;
                }
            }
        }
        history
    }

    fn find_slot(&mut self, client: RemoteAddr) {
        let mut assigned_group: Option<usize> = None;
        let start_idx = self.full_group_idx.clone();
        let end_idx = self.groups.len();
        let groups = &self.groups;
        let history = self.get_history(&client);

        for group_idx in start_idx..end_idx {
            let group = groups.get(group_idx).unwrap();

            if history.intersection(&HashSet::from_iter(group)).peekable().peek().is_none() {
                assigned_group = Some(group_idx);
                break;
            }
        }

        match assigned_group {
            Some(idx) => {
                let group = self.groups.get_mut(idx).unwrap();
                group.push(client);
                for addr in group.iter() {
                    self.full_group_idx = self.full_group_idx + 1;
                    addr.clone().do_send(Box::new(GroupingMessage::Response(group.clone())));
                }
            }
            None => {
                let new_group = vec![client];
                self.groups.push(new_group);
            }
        };
    }

    fn resolve_socket(&mut self, client: RemoteAddr) {
        self.unresolved.push_front(client.clone());
        self.cluster.request_node_addr(client.socket_addr, self.own_addr.clone().unwrap());
    }
}


impl Actor for GroupingServer {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        debug!("started");
        self.own_addr = Some(ctx.address().recipient());
    }
}


impl Handler<GroupingMessage> for GroupingServer {
    type Result = ();

    fn handle(&mut self, msg: GroupingMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            GroupingMessage::Request(remote_addr) => self.resolve_socket(remote_addr),
            _ => ()
        }
    }
}


impl Handler<NodeResolving> for GroupingServer {
    type Result = ();

    fn handle(&mut self, msg: NodeResolving, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            NodeResolving::Response(node) => {
                let mut remote_addr = self.unresolved.pop_back().unwrap();
                remote_addr.set_network_interface(node);
                self.find_slot(remote_addr);
            },
            _ => ()
        }
    }
}
