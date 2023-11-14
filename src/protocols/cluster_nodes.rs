use std::collections::hash_map::Iter;
use std::collections::HashMap;

use crate::{AnyAddr, RemoteAddr};
use actix::{Actor, Addr};

#[derive(Default, Clone, Debug)]
pub struct ClusterNodes {
    nodes: HashMap<usize, RemoteAddr>,
}

impl ClusterNodes {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn change_ids(&mut self, id: &str) {
        for (_, node) in self.nodes.iter_mut() {
            node.change_id(id.to_string());
        }
    }

    pub fn get_main_node(&self) -> Option<&RemoteAddr> {
        self.nodes.get(&0)
    }

    pub fn get(&self, key: &usize) -> Option<&RemoteAddr> {
        self.nodes.get(key)
    }

    pub fn get_own_idx(&self) -> usize {
        let n_nodes: usize = (0..self.len_incl_own()).sum();
        let keys_sum: usize = self.nodes.keys().sum();
        n_nodes - keys_sum
    }

    pub fn get_next_idx(&self) -> Option<usize> {
        let own_idx = self.get_own_idx();
        let possible_next_idx = own_idx + 1;
        if self.nodes.contains_key(&possible_next_idx) {
            Some(possible_next_idx)
        } else if self.nodes.contains_key(&0) & (own_idx != 0) {
            Some(0)
        } else {
            None
        }
    }

    pub fn to_any<T: Actor>(&self, addr: Addr<T>) -> AnyClusterNodes<T> {
        AnyClusterNodes::new(self.clone(), addr, None)
    }

    pub fn to_any_as<T: Actor>(&self, addr: Addr<T>, remote_actor_id: &str) -> AnyClusterNodes<T> {
        AnyClusterNodes::new(self.clone(), addr, Some(remote_actor_id.to_string()))
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn len_incl_own(&self) -> usize {
        self.len() + 1
    }

    pub fn get_as(&self, key: &usize, remote_actor_id: &str) -> Option<RemoteAddr> {
        match self.get(key) {
            Some(remote_addr) => {
                let mut remote_addr = remote_addr.clone();
                remote_addr.change_id(remote_actor_id.to_string());
                Some(remote_addr)
            }
            None => None,
        }
    }

    pub fn iter(&self) -> Iter<'_, usize, RemoteAddr> {
        self.nodes.iter()
    }
}

impl From<HashMap<usize, RemoteAddr>> for ClusterNodes {
    fn from(nodes: HashMap<usize, RemoteAddr>) -> Self {
        Self {
            nodes,
            ..Default::default()
        }
    }
}

// AnyClusterNodes

pub struct AnyClusterNodes<T: Actor> {
    pub nodes: ClusterNodes,
    pub local_addr: Addr<T>,
    pub remote_actor_id: Option<String>,
}

impl<T: Actor> AnyClusterNodes<T> {
    pub fn new(nodes: ClusterNodes, addr: Addr<T>, remote_actor_id: Option<String>) -> Self {
        Self {
            nodes,
            local_addr: addr,
            remote_actor_id,
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.nodes.len() + 1
    }
}

impl<T: Actor> IntoIterator for AnyClusterNodes<T> {
    type Item = AnyAddr<T>;
    type IntoIter = AnyClusterNodesIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        AnyClusterNodesIterator::from(self)
    }
}

impl<T: Actor> Clone for AnyClusterNodes<T> {
    fn clone(&self) -> Self {
        Self::new(
            self.nodes.clone(),
            self.local_addr.clone(),
            self.remote_actor_id.clone(),
        )
    }
}

// Iterator

pub struct AnyClusterNodesIterator<T: Actor> {
    any_cluster_nodes: AnyClusterNodes<T>,
    position: usize,
}

impl<T: Actor> AnyClusterNodesIterator<T> {
    pub fn last_position(&mut self) -> bool {
        self.position >= self.any_cluster_nodes.len()
    }

    pub fn get_position(&mut self) -> usize {
        self.position
    }
}

impl<T: Actor> Iterator for AnyClusterNodesIterator<T> {
    type Item = AnyAddr<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.any_cluster_nodes.len() {
            None
        } else {
            let addr = match self.any_cluster_nodes.nodes.get(&self.position) {
                Some(remote_addr) => {
                    let mut remote_addr = remote_addr.clone();
                    match &self.any_cluster_nodes.remote_actor_id {
                        Some(id) => remote_addr.change_id(id.clone()),
                        None => (),
                    }
                    AnyAddr::Remote(remote_addr)
                }
                None => AnyAddr::Local(self.any_cluster_nodes.local_addr.clone()),
            };
            self.position += 1;
            return Some(addr);
        }
    }
}

impl<T: Actor> From<AnyClusterNodes<T>> for AnyClusterNodesIterator<T> {
    fn from(any_cluster_nodes: AnyClusterNodes<T>) -> Self {
        Self {
            any_cluster_nodes,
            position: 0,
        }
    }
}
