use log::*;
use actix_rt;
use crate::Cluster;
use port_scanner::{local_port_available, request_open_port};


#[actix_rt::test]
async fn cluster_binds_port() {
    env_logger::init();

    let port = request_open_port().unwrap_or(8000);
    let _listener = Cluster::bind(format!("127.0.0.1:{}", port));

    assert_eq!(local_port_available(port), false);
}

#[actix_rt::test]
async fn cluster_adds_node() {

}

#[actix_rt::test]
async fn cluster_adds_node_from_stream() {

}

#[actix_rt::test]
async fn cluster_registers_actor() {

}
