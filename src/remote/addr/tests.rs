use crate::{prelude::*, Node};
use crate::{AddrRepresentation, AddrRequest, AddrResolver, AddrResponse};
use actix::prelude::*;
use actix_broker::BrokerSubscribe;
use actix_telepathy_derive::{RemoteActor, RemoteMessage};
use core::panic;
use port_scanner::request_open_port;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

#[derive(RemoteMessage, Serialize, Deserialize)]
#[with_source(source)]
struct TestMessage {
    source: RemoteAddr,
}

#[derive(RemoteActor)]
#[remote_messages(TestMessage)]
enum TestActor {
    Test(Arc<Mutex<Vec<String>>>),
}

impl Actor for TestActor {
    type Context = Context<Self>;
}

impl Handler<TestMessage> for TestActor {
    type Result = ();

    fn handle(&mut self, _msg: TestMessage, ctx: &mut Context<Self>) -> Self::Result {
        AddrResolver::from_registry()
            .send(AddrRequest::ResolveRec(ctx.address().recipient()))
            .into_actor(self)
            .map(|res, act, _ctx| match res {
                Ok(res) => match res {
                    Ok(addr_res) => match addr_res {
                        AddrResponse::ResolveRec(identifer) => match act {
                            TestActor::Test(identifiers) => {
                                identifiers.lock().unwrap().push(identifer)
                            }
                        },
                        _ => panic!("Wrong Response returned!"),
                    },
                    Err(_) => panic!("Couldn't resolve Addr!"),
                },
                Err(_) => panic!("Couldn't resolve Addr!"),
            })
            .wait(ctx)
    }
}

#[actix_rt::test]
async fn addr_resolver_registers_and_resolves_addr() {
    let identifier = "testActor".to_string();
    let identifiers = Arc::new(Mutex::new(vec![]));
    let ta = TestActor::Test(identifiers.clone()).start();
    AddrResolver::from_registry().do_send(AddrRequest::Register(
        ta.clone().recipient(),
        identifier.clone(),
    ));
    ta.do_send(TestMessage {
        source: RemoteAddr::default(),
    });
    sleep(Duration::from_secs(1)).await;
    assert_eq!(
        (*(identifiers.lock().unwrap())).get(0).unwrap(),
        &identifier
    );
}

#[test]
fn addr_representation_eq_not_key() {
    let own = AddrRepresentation::NetworkInterface;
    let other1 = AddrRepresentation::NetworkInterface;
    let other2 = AddrRepresentation::Connector;
    let other3 = AddrRepresentation::Key("test".to_string());

    assert!(own.eq(&other1));
    assert!(own.ne(&other2));
    assert!(own.ne(&other3));
}

#[test]
fn addr_representation_eq_key() {
    let own = AddrRepresentation::Key("own".to_string());
    let other1 = AddrRepresentation::Key("own".to_string());
    let other2 = AddrRepresentation::Key("other2".to_string());

    assert!(own.eq(&other1));
    assert!(own.ne(&other2));
}

struct TestParams {
    ip: SocketAddr,
    seeds: Vec<SocketAddr>,
    last: bool,
}

#[test]
#[ignore] //github workflows don't get the timing right
fn remote_addr_ignores_hash() {
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
            last: false,
        },
        TestParams {
            ip: ip2.clone(),
            seeds: vec![ip1.clone()],
            last: false,
        },
        TestParams {
            ip: ip3.clone(),
            seeds: vec![ip1.clone()],
            last: true,
        },
    ];
    arr.par_iter()
        .for_each(|p| build_cluster(p.ip, p.seeds.clone(), p.last));
}

#[actix_rt::main]
async fn build_cluster(own_ip: SocketAddr, other_ip: Vec<SocketAddr>, last: bool) {
    let _cluster = Cluster::new(own_ip, other_ip);
    if last {
        let returned: Arc<Mutex<Option<usize>>> = Arc::new(Mutex::new(None));
        let _listener = OwnListenerGossipIntroduction {
            addrs: vec![],
            returned: returned.clone(),
        }
        .start();
        sleep(Duration::from_millis(200)).await;
        returned
            .lock()
            .unwrap()
            .expect("Something should be returned");
    } else {
        sleep(Duration::from_millis(200)).await;
    }
}

struct OwnListenerGossipIntroduction {
    pub addrs: Vec<Node>,
    pub returned: Arc<Mutex<Option<usize>>>,
}
impl ClusterListener for OwnListenerGossipIntroduction {}
impl Supervised for OwnListenerGossipIntroduction {}

impl Actor for OwnListenerGossipIntroduction {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        self.subscribe_system_async::<ClusterLog>(ctx);
    }
}

impl Handler<ClusterLog> for OwnListenerGossipIntroduction {
    type Result = ();

    fn handle(&mut self, msg: ClusterLog, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            ClusterLog::NewMember(node) => {
                self.addrs.push(node);
                if self.addrs.len() >= 2 {
                    let node = self.addrs.get(0).unwrap().clone();
                    let mut node2 = node.clone();
                    node2.network_interface = None;
                    //let remote_addr3 = self.addrs.get(1).unwrap().clone();

                    let mut map = HashMap::<Node, usize>::new();
                    map.insert(node, 0);
                    (*(self.returned.lock().unwrap())) = map.remove(&node2);
                }
            }
            _ => (),
        }
    }
}

// ----------------------------------------------

struct OwnListenerGossipIntroduction2 {
    pub returned: Arc<Mutex<Option<RemoteAddr>>>,
}
impl ClusterListener for OwnListenerGossipIntroduction2 {}
impl Supervised for OwnListenerGossipIntroduction2 {}

impl Actor for OwnListenerGossipIntroduction2 {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        self.subscribe_system_async::<ClusterLog>(ctx);
    }
}

impl Handler<ClusterLog> for OwnListenerGossipIntroduction2 {
    type Result = ();

    fn handle(&mut self, msg: ClusterLog, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            ClusterLog::NewMember(node) => {
                let remote_addr = node.get_remote_addr(TestActor2::ACTOR_ID.to_string());
                (*(self.returned.lock().unwrap())) = Some(remote_addr);
            }
            _ => (),
        }
    }
}

#[derive(RemoteActor)]
#[remote_messages(TestMessage)]
struct TestActor2 {
    pub returned: Arc<Mutex<Option<RemoteAddr>>>,
}

impl Actor for TestActor2 {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        self.register(ctx.address().recipient());
    }
}

impl Handler<TestMessage> for TestActor2 {
    type Result = ();

    fn handle(&mut self, msg: TestMessage, _ctx: &mut Context<Self>) -> Self::Result {
        self.returned.lock().unwrap().replace(msg.source);
    }
}

struct TestParams2 {
    ip: SocketAddr,
    seeds: Vec<SocketAddr>,
}

#[test]
#[ignore] //github workflows don't get the timing right
fn remote_addr_filled_in_with_source() {
    let ip1: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();
    let ip2: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();

    let arr = [
        TestParams2 {
            ip: ip1.clone(),
            seeds: vec![],
        },
        TestParams2 {
            ip: ip2.clone(),
            seeds: vec![ip1.clone()],
        },
    ];
    arr.par_iter()
        .for_each(|p| build_cluster_2(p.ip, p.seeds.clone()));
}

#[actix_rt::main]
async fn build_cluster_2(own_ip: SocketAddr, other_ip: Vec<SocketAddr>) {
    let _cluster = Cluster::new(own_ip, other_ip);
    let returned_received: Arc<Mutex<Option<RemoteAddr>>> = Arc::new(Mutex::new(None));
    let _test_actor_addr = TestActor2 {
        returned: returned_received.clone(),
    }
    .start();
    let returned: Arc<Mutex<Option<RemoteAddr>>> = Arc::new(Mutex::new(None));
    let _listener = OwnListenerGossipIntroduction2 {
        returned: returned.clone(),
    }
    .start();
    sleep(Duration::from_millis(200)).await;
    let guard = returned.lock().unwrap();
    let remote_addr = guard.as_ref().expect("Something should be returned");
    let mut own_remote_addr = remote_addr.clone();
    own_remote_addr.node.socket_addr = own_ip;
    remote_addr.do_send(TestMessage {
        source: own_remote_addr,
    });
    sleep(Duration::from_millis(200)).await;
    assert_eq!(
        (returned_received.lock().unwrap())
            .as_ref()
            .unwrap()
            .node
            .socket_addr,
        remote_addr.node.socket_addr
    );
}

#[derive(RemoteMessage, Serialize, Deserialize)]
struct FakeMessage {}

#[actix_rt::test]
async fn addr_resolver_does_not_panic_wrong_id() {
    testing_logger::setup();
    let listening_ip: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();
    let _cluster = Cluster::new(listening_ip.clone(), vec![]);
    let fake_addr = RemoteAddr::new(
        Node::new(listening_ip, None),
        AddrRepresentation::Key("test".to_string()),
    );
    let fake_wrapper = RemoteWrapper::new(fake_addr, FakeMessage {}, None);
    AddrResolver::from_registry()
        .send(fake_wrapper)
        .await
        .unwrap();
    testing_logger::validate(|captured_logs| {
        let warnings_count = captured_logs
            .iter()
            .filter(|l| l.level == log::Level::Warn)
            .filter(|l| l.body.contains("Message is abandoned."))
            .count();
        assert_eq!(warnings_count, 1);
    });
}
