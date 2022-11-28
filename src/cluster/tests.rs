use crate::test_utils::cluster_listener::TestClusterListener;
use crate::{
    Cluster, ClusterListener, ClusterLog, CustomSystemService, Gossip, NetworkInterface,
    NodeResolving,
};
use actix::prelude::*;
use actix_broker::BrokerSubscribe;
use actix_rt;
use log::*;
use port_scanner::{local_port_available, request_open_port};
use rayon::prelude::*;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};

type SocketTestClusterListener = TestClusterListener<Arc<Mutex<Vec<SocketAddr>>>>;

impl Actor for SocketTestClusterListener {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        self.subscribe_system_async::<ClusterLog>(ctx);
    }
}

impl Handler<ClusterLog> for SocketTestClusterListener {
    type Result = ();

    fn handle(&mut self, msg: ClusterLog, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            ClusterLog::NewMember(node) => {
                debug!("new member {}", node.socket_addr.to_string());
                (*(self.content.as_ref().unwrap().lock().unwrap())).push(node.socket_addr);
            }
            ClusterLog::MemberLeft(_addr) => {}
        }
    }
}

struct OwnListenerAskingGossip {
    pub asking: SocketAddr,
    pub addrs: Arc<Mutex<Vec<Addr<NetworkInterface>>>>,
}
impl ClusterListener for OwnListenerAskingGossip {}
impl Supervised for OwnListenerAskingGossip {}

impl Actor for OwnListenerAskingGossip {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        self.subscribe_system_async::<ClusterLog>(ctx);
    }
}

impl Handler<ClusterLog> for OwnListenerAskingGossip {
    type Result = ();

    fn handle(&mut self, msg: ClusterLog, ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            ClusterLog::NewMember(_node) => {
                Gossip::from_custom_registry()
                    .send(NodeResolving {
                        addrs: vec![self.asking.clone()],
                    })
                    .into_actor(self)
                    .map(
                        |res: Result<Result<Vec<Addr<NetworkInterface>>, ()>, MailboxError>,
                         act,
                         _ctx| match res.unwrap() {
                            Ok(addrs) => {
                                (*(act.addrs.lock().unwrap())).push(addrs.get(0).unwrap().clone())
                            }
                            Err(_) => (),
                        },
                    )
                    .wait(ctx);
            }
            ClusterLog::MemberLeft(addr) => {
                Gossip::from_custom_registry()
                    .send(NodeResolving { addrs: vec![addr] })
                    .into_actor(self)
                    .map(
                        |res: Result<Result<Vec<Addr<NetworkInterface>>, ()>, MailboxError>,
                         _act,
                         _ctx| match res.unwrap() {
                            Ok(addrs) => assert_eq!(addrs.len(), 0),
                            Err(_) => (),
                        },
                    )
                    .wait(ctx);
            }
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
    let own_listener = SocketTestClusterListener::new_with_content(Arc::clone(&connections));
    let _addr = own_listener.start();
    let _network_interface = NetworkInterface::new(
        "127.0.0.1:1993".parse().unwrap(),
        "127.0.0.1:1992".parse().unwrap(),
        true,
    )
    .start();
    sleep(Duration::from_secs(1)).await;
    assert!(connections
        .lock()
        .unwrap()
        .contains(&("127.0.0.1:1993".parse().unwrap())));
    assert!(connections
        .lock()
        .unwrap()
        .contains(&("127.0.0.1:1992".parse().unwrap())));
}

// Gossip

#[actix_rt::test]
async fn gossip_adds_member_and_resolves_it() {
    let local_ip: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();
    let other_ip: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();
    let _cluster = Cluster::new(local_ip.clone(), vec![]);
    let addrs = Arc::new(Mutex::new(vec![]));
    let _own_listener = OwnListenerAskingGossip {
        asking: other_ip.clone(),
        addrs: Arc::clone(&addrs),
    }
    .start();
    let _network_interface = NetworkInterface::new(other_ip, local_ip, true).start();
    sleep(Duration::from_secs(1)).await;
    assert_eq!(addrs.lock().unwrap().len(), 2);
}

#[actix_rt::test]
async fn gossip_removes_member() {
    let local_ip: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();
    let other_ip: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();
    let _cluster = Cluster::new(local_ip.clone(), vec![]);
    let addrs = Arc::new(Mutex::new(vec![]));
    let _own_listener = OwnListenerAskingGossip {
        asking: other_ip.clone(),
        addrs: Arc::clone(&addrs),
    }
    .start();
    let _network_interface = NetworkInterface::new(other_ip, local_ip, true).start();
    sleep(Duration::from_secs(1)).await;
    assert_eq!(addrs.lock().unwrap().len(), 2);
}

struct TestParams {
    ip: SocketAddr,
    seeds: Vec<SocketAddr>,
    start: u64,
    delay: u64,
    end: u64,
    expect: usize,
}

#[test]
#[ignore] //github workflows don't get the timing right
fn gossip_adds_member_and_introduces_other_members() {
    let ip1: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();
    let ip2: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();
    let ip3: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();

    let arr = [
        TestParams {
            ip: ip1.clone(),
            seeds: vec![],
            start: 0,
            delay: 0,
            end: 2000,
            expect: 0,
        },
        TestParams {
            ip: ip2.clone(),
            seeds: vec![ip1.clone()],
            start: 1000,
            delay: 1000,
            end: 0,
            expect: 2,
        },
        TestParams {
            ip: ip3.clone(),
            seeds: vec![ip1.clone()],
            start: 1000,
            delay: 1000,
            end: 0,
            expect: 2,
        },
    ];
    arr.par_iter()
        .for_each(|p| build_cluster(p.ip, p.seeds.clone(), p.start, p.delay, p.end, p.expect));
}

#[actix_rt::main]
async fn build_cluster(
    own_ip: SocketAddr,
    other_ip: Vec<SocketAddr>,
    start: u64,
    delay: u64,
    end: u64,
    expect: usize,
) {
    sleep(Duration::from_millis(start)).await;
    let _cluster = Cluster::new(own_ip, other_ip);
    let addrs = Arc::new(Mutex::new(vec![]));
    let _cluster_listener = SocketTestClusterListener::new_with_content(Arc::clone(&addrs)).start();
    sleep(Duration::from_millis(delay)).await;
    assert_eq!((*(addrs.lock().unwrap())).len(), expect);
    sleep(Duration::from_millis(end)).await;
}
