use std::net::SocketAddr;

use actix::{Actor, WrapFuture};
use futures::TryFutureExt;
use port_scanner::request_open_port;
use rayon::iter::{ParallelIterator, IntoParallelIterator};

use crate::{Cluster, Connector, CustomSystemService, NodeResolving};


struct TestActor {}

impl Actor for TestActor {
    type Context = actix::Context<Self>;
}


#[actix_rt::test]
#[ignore]
async fn test_gossip_connector() {
    let local_ip: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();
    let other_ip: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();

    let nodes = vec![(local_ip.clone(), other_ip.clone()), (other_ip.clone(), local_ip.clone())];
    nodes.into_par_iter().for_each(|(own_ip, other_ip)| build_gossip_cluster(own_ip, vec![other_ip]));
}

async fn build_gossip_cluster(local_ip: SocketAddr, seed_nodes: Vec<SocketAddr>) {
    let other_ip = seed_nodes.clone().into_iter().next().unwrap();
    let _cluster = Cluster::new_with_connection_protocol(local_ip, seed_nodes.clone(), crate::ConnectionProtocol::Gossip);
    let connector = Connector::from_custom_registry();
    let addrs = connector.send(NodeResolving { addrs: seed_nodes}).await.unwrap().unwrap();
    addrs.contains(&other_ip);
}