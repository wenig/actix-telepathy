use std::net::SocketAddr;

use actix::Actor;
use port_scanner::request_open_port;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{Cluster, Connector, CustomSystemService, NodeResolving};

const FAILED_TO_RESOLVE_NODES: &str = "Failed to resolve nodes";

struct TestActor {}

impl Actor for TestActor {
    type Context = actix::Context<Self>;
}

#[test]
#[ignore]
fn test_gossip_connector_size_2() {
    let local_ip: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();
    let other_ip: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();

    let variables = vec![
        (
            local_ip.clone(),
            vec![other_ip.clone()],
            vec![other_ip.clone()],
        ),
        (other_ip.clone(), vec![], vec![local_ip.clone()]),
    ];

    let results: Vec<Result<usize, ()>> = variables
        .into_par_iter()
        .map(|(own_ip, seed_nodes, other_ips)| build_gossip_cluster(own_ip, seed_nodes, other_ips))
        .collect();

    for result in results {
        assert_eq!(result.unwrap(), 1);
    }
}

#[test]
#[ignore]
fn test_gossip_connector_size_3_one_seed() {
    let ip_0: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();
    let ip_1: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();
    let ip_2: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();

    let variables = vec![
        (
            ip_0.clone(),
            vec![ip_1.clone()],
            vec![ip_1.clone(), ip_2.clone()],
        ),
        (ip_1.clone(), vec![], vec![ip_0.clone(), ip_2.clone()]),
        (
            ip_2.clone(),
            vec![ip_1.clone()],
            vec![ip_0.clone(), ip_1.clone()],
        ),
    ];

    let results: Vec<Result<usize, ()>> = variables
        .into_par_iter()
        .map(|(own_ip, seed_nodes, other_ips)| build_gossip_cluster(own_ip, seed_nodes, other_ips))
        .collect();

    for result in results {
        assert_eq!(result.unwrap(), 2);
    }
}

#[test]
#[ignore]
fn test_gossip_connector_size_3_two_seeds() {
    let ip_0: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();
    let ip_1: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();
    let ip_2: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();

    let variables = vec![
        (
            ip_0.clone(),
            vec![ip_1.clone()],
            vec![ip_1.clone(), ip_2.clone()],
        ),
        (ip_1.clone(), vec![], vec![ip_0.clone(), ip_2.clone()]),
        (
            ip_2.clone(),
            vec![ip_0.clone()],
            vec![ip_0.clone(), ip_1.clone()],
        ),
    ];

    let results: Vec<Result<usize, ()>> = variables
        .into_par_iter()
        .map(|(own_ip, seed_nodes, other_ips)| build_gossip_cluster(own_ip, seed_nodes, other_ips))
        .collect();

    for result in results {
        assert_eq!(result.unwrap(), 2);
    }
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
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    let connector = Connector::from_custom_registry();
    let addrs = connector
        .send(NodeResolving { addrs: other_ips })
        .await
        .expect(FAILED_TO_RESOLVE_NODES)
        .expect(FAILED_TO_RESOLVE_NODES);
    Ok(addrs.len())
}
