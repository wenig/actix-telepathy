use log::*;
use actix_rt;
use crate::{Cluster, ClusterListener, ClusterLog, NetworkInterface, Gossip, CustomSystemService, NodeResolving};
use port_scanner::{local_port_available, request_open_port};
use std::net::SocketAddr;
use actix::prelude::*;
use actix_broker::BrokerSubscribe;
use tokio::time::{delay_for, Duration};
use std::sync::{Arc, Mutex};
use crate::cluster::gossip::MemberMgmt;


struct OwnListener {
    pub connections: Arc<Mutex<Vec<SocketAddr>>>
}
impl ClusterListener for OwnListener {}
impl Supervised for OwnListener {}

impl Actor for OwnListener {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        self.subscribe_system_async::<ClusterLog>(ctx);
    }
}

impl Handler<ClusterLog> for OwnListener {
    type Result = ();

    fn handle(&mut self, msg: ClusterLog, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            ClusterLog::NewMember(addr, _remote_addr) => {
                debug!("new member {}", addr.to_string());
                (*(self.connections.lock().unwrap())).push(addr);
            },
            ClusterLog::MemberLeft(_addr) => {}
        }
    }
}


// Cluster

#[actix_rt::test]
async fn cluster_binds_port() {
    let port = request_open_port().unwrap_or(8000);
    let _listener = Cluster::bind(format!("127.0.0.1:{}", port));

    assert_eq!(local_port_available(port), false);
}

#[actix_rt::test]
async fn cluster_adds_node_and_from_stream() {
    let _cluster = Cluster::new("127.0.0.1:1992".parse().unwrap(), vec![]);
    let connections = Arc::new(Mutex::new(vec![]));
    let own_listener = OwnListener {connections: Arc::clone(&connections)};
    let _addr = own_listener.start();
    let _network_interface = NetworkInterface::new("127.0.0.1:1993".parse().unwrap(),
                                                   "127.0.0.1:1992".parse().unwrap(), true).start();
    delay_for(Duration::from_secs(1)).await;
    assert!(connections.lock().unwrap().contains(&("127.0.0.1:1993".parse().unwrap())));
    assert!(connections.lock().unwrap().contains(&("127.0.0.1:1992".parse().unwrap())));
}


// Gossip

#[actix_rt::test]
async fn gossip_adds_member_and_resolves_it() {
    let local_ip: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000)).parse().unwrap();
    let other_ip: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000)).parse().unwrap();
    let cluster = Cluster::new(local_ip.clone(), vec![]);
    let _network_interface = NetworkInterface::new(other_ip, local_ip, true).start();
    delay_for(Duration::from_secs(1)).await;
    let r = Gossip::from_custom_registry().send(NodeResolving {addrs: vec![other_ip]});
    // todo: read result
}

#[actix_rt::test]
async fn gossip_adds_member_and_introduces_other_members() {
}

#[actix_rt::test]
async fn gossip_removes_member() {
}



// ClusterListener


