use std::net::SocketAddr;
use std::thread::sleep;
use actix::prelude::*;
use ndarray::arr1;
use port_scanner::request_open_port;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use tokio::time::Duration;
use crate::{Cluster, CustomSystemService};
use crate::protocols::cluster_nodes::ClusterNodes;
use crate::protocols::{ProtocolDataType, ProtocolFinished, ProtocolsReceiver};
use crate::protocols::reduce::listener::TestClusterMemberListener;


impl Handler<ProtocolFinished<ProtocolDataType>> for TestClusterMemberListener {
    type Result = ();

    fn handle(&mut self, msg: ProtocolFinished<ProtocolDataType>, ctx: &mut Self::Context) -> Self::Result {
        println!("received {}", msg.result.unwrap());
        System::current().stop();
    }
}


#[test]
#[ignore]
fn reduce_correct_value() {
    env_logger::init();
    let main_addr = format!("127.0.0.1:{}", request_open_port().unwrap_or(1992)).parse().unwrap();
    let sub_addr = format!("127.0.0.1:{}", request_open_port().unwrap_or(1993)).parse().unwrap();
    let params = vec![
        (arr1(&[1_f32, 2_f32]), main_addr, vec![], main_addr),
        (arr1(&[3_f32, 4_f32]), sub_addr, vec![main_addr], main_addr)];
    params.into_par_iter().for_each(|(v, h, s, m)| run_single_reduction_worker(v, h, s, m));
}

#[actix_rt::main]
async fn run_single_reduction_worker(value: ProtocolDataType, host: SocketAddr, seed_nodes: Vec<SocketAddr>, main: SocketAddr) {
    let cluster_listener = TestClusterMemberListener::new(host.eq(&main), main, 2, host, value).start();
    let _protocol_receiver = ProtocolsReceiver::start_service_with(move || ProtocolsReceiver::new(cluster_listener.clone().recipient()));
    let _cluster = Cluster::new(host, seed_nodes);
    println!("testtest");

    sleep(Duration::from_secs(2));
}
