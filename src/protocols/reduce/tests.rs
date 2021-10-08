use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use actix::prelude::*;
use ndarray::{arr1, Array1};
use port_scanner::request_open_port;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use tokio::time::{Duration, sleep};
use crate::{Cluster, CustomSystemService};
use crate::protocols::cluster_nodes::ClusterNodes;
use crate::protocols::{ProtocolDataType, ProtocolFinished, ProtocolsReceiver};
use crate::protocols::reduce::listener::TestClusterMemberListener;
use log::*;


impl Handler<ProtocolFinished<ProtocolDataType>> for TestClusterMemberListener {
    type Result = ();

    fn handle(&mut self, msg: ProtocolFinished<ProtocolDataType>, _ctx: &mut Self::Context) -> Self::Result {
        *(self.expected.as_ref().unwrap().lock().unwrap()) = Some(msg.result.unwrap());
    }
}


#[test]
#[ignore]
fn reduce_correct_value() {
    let main_addr = format!("127.0.0.1:{}", request_open_port().unwrap_or(1992)).parse().unwrap();
    let sub_addr = format!("127.0.0.1:{}", request_open_port().unwrap_or(1993)).parse().unwrap();
    let expected_value = arr1(&[4_f32, 6_f32]);
    let expected_value_arc: Arc<Mutex<Option<Array1<f32>>>> = Arc::new(Mutex::new(None));
    let params = vec![
        (arr1(&[1_f32, 2_f32]), main_addr, vec![], main_addr, Some(expected_value_arc.clone())),
        (arr1(&[3_f32, 4_f32]), sub_addr, vec![main_addr], main_addr, None)];
    params.into_par_iter().for_each(|(v, h, s, m, e)| run_single_reduction_worker(v, h, s, m, e));
    let received = (&*(expected_value_arc.lock().unwrap()));
    assert_eq!(expected_value, received.as_ref().unwrap());
}

#[actix_rt::main]
async fn run_single_reduction_worker(value: ProtocolDataType, host: SocketAddr, seed_nodes: Vec<SocketAddr>, main: SocketAddr, expected: Option<Arc<Mutex<Option<Array1<f32>>>>>) {
    let _cluster_listener = TestClusterMemberListener::new(host.eq(&main), main, 2, host, value, expected).start();
    let _cluster = Cluster::new(host, seed_nodes);

    sleep(Duration::from_millis(200)).await;
}
