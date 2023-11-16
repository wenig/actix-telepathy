use std::{env, net::SocketAddr};

use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{
    test_utils::get_n_local_socket_addrs, Cluster, Connector, CustomSystemService, NodeResolving,
};

const FAILED_TO_RESOLVE_NODES: &str = "Failed to resolve nodes";

/// all nodes have the same seed node
fn test_gossip_connector_one_seed(n: usize) {
    let ips = get_n_local_socket_addrs(n);

    let seed_nodes = ips
        .iter()
        .enumerate()
        .map(|(i, _)| if i == 0 { vec![] } else { vec![ips[0].clone()] })
        .collect::<Vec<Vec<SocketAddr>>>();

    let other_nodes = ips
        .iter()
        .enumerate()
        .map(|(i, _)| {
            ips.iter()
                .enumerate()
                .filter_map(|(j, _)| if i == j { None } else { Some(ips[j].clone()) })
                .collect::<Vec<SocketAddr>>()
        })
        .collect::<Vec<Vec<SocketAddr>>>();

    let variables = ips
        .iter()
        .zip(seed_nodes.iter())
        .zip(other_nodes.iter())
        .map(|((own_ip, seed_nodes), other_ips)| {
            (own_ip.clone(), seed_nodes.clone(), other_ips.clone())
        })
        .collect::<Vec<(SocketAddr, Vec<SocketAddr>, Vec<SocketAddr>)>>();

    let results: Vec<Result<usize, ()>> = variables
        .into_par_iter()
        .map(|(own_ip, seed_nodes, other_ips)| build_gossip_cluster(own_ip, seed_nodes, other_ips))
        .collect();

    for result in results {
        assert_eq!(result.unwrap(), n - 1);
    }
}

#[test]
#[ignore]
fn test_gossip_connector_one_seed_2() {
    test_gossip_connector_one_seed(2);
}

#[test]
#[ignore]
fn test_gossip_connector_one_seed_3() {
    test_gossip_connector_one_seed(3);
}

#[test]
#[ignore]
fn test_gossip_connector_one_seed_8() {
    test_gossip_connector_one_seed(8);
}

/// Node i has seed nodes i-1
fn test_gossip_connector_chain_seeds(n: usize) {
    let ips = get_n_local_socket_addrs(n);

    let seed_nodes = ips
        .iter()
        .enumerate()
        .map(|(i, _)| {
            if i == 0 {
                vec![]
            } else {
                vec![ips[i - 1].clone()]
            }
        })
        .collect::<Vec<Vec<SocketAddr>>>();

    let other_nodes = ips
        .iter()
        .enumerate()
        .map(|(i, _)| {
            ips.iter()
                .enumerate()
                .filter_map(|(j, _)| if i == j { None } else { Some(ips[j].clone()) })
                .collect::<Vec<SocketAddr>>()
        })
        .collect::<Vec<Vec<SocketAddr>>>();

    let variables = ips
        .iter()
        .zip(seed_nodes.iter())
        .zip(other_nodes.iter())
        .map(|((own_ip, seed_nodes), other_ips)| {
            (own_ip.clone(), seed_nodes.clone(), other_ips.clone())
        })
        .collect::<Vec<(SocketAddr, Vec<SocketAddr>, Vec<SocketAddr>)>>();

    let results: Vec<Result<usize, ()>> = variables
        .into_par_iter()
        .map(|(own_ip, seed_nodes, other_ips)| build_gossip_cluster(own_ip, seed_nodes, other_ips))
        .collect();

    for result in results {
        assert_eq!(result.unwrap(), n - 1);
    }
}

#[test]
#[ignore]
fn test_gossip_connector_chain_seeds_3() {
    test_gossip_connector_chain_seeds(3);
}

#[test]
#[ignore]
fn test_gossip_connector_chain_seeds_8() {
    test_gossip_connector_chain_seeds(8);
}

#[actix_rt::main]
async fn build_gossip_cluster(
    local_ip: SocketAddr,
    seed_nodes: Vec<SocketAddr>,
    other_ips: Vec<SocketAddr>,
) -> Result<usize, ()> {
    let _cluster = Cluster::new_with_connection_protocol(
        local_ip,
        seed_nodes.clone(),
        crate::ConnectionProtocol::Gossip,
    );
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    let connector = Connector::from_custom_registry();
    let addrs = connector
        .send(NodeResolving { addrs: other_ips })
        .await
        .expect(FAILED_TO_RESOLVE_NODES)
        .expect(FAILED_TO_RESOLVE_NODES);
    Ok(addrs.len())
}
