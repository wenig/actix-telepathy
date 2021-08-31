use actix::prelude::*;
use serde::{Serialize, Deserialize};
use crate::prelude::*;
use actix_telepathy_derive::{RemoteActor, RemoteMessage};
use crate::{AddrResolver, AddrRequest, AddrResponse, AddrRepresentation};
use tokio::time::delay_for;
use std::time::Duration;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::net::SocketAddr;
use port_scanner::request_open_port;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use actix_broker::BrokerSubscribe;


#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype(result = "()")]
struct TestMessage {}


#[derive(RemoteActor)]
#[remote_messages(TestMessage)]
struct TestActor {
    identifiers: Arc<Mutex<Vec<String>>>
}


impl Actor for TestActor {
    type Context = Context<Self>;
}

impl Handler<TestMessage> for TestActor {
    type Result = ();

    fn handle(&mut self, _msg: TestMessage, ctx: &mut Context<Self>) -> Self::Result {
        AddrResolver::from_registry().send(AddrRequest::ResolveRec(ctx.address().recipient()))
            .into_actor(self)
            .map(|res, act, _ctx| match res {
                Ok(res) => match res {
                    Ok(addr_res) => {
                        match addr_res {
                            AddrResponse::ResolveRec(identifer) => act.identifiers.lock().unwrap().push(identifer),
                            _ => panic!("Wrong Response returned!")
                        }
                    },
                    Err(_) => panic!("Couldn't resolve Addr!")
                },
                Err(_) => panic!("Couldn't resolve Addr!")
            }).wait(ctx)
    }
}


#[actix_rt::test]
async fn addr_resolver_registers_and_resolves_addr() {
    let identifier = "testActor".to_string();
    let identifiers = Arc::new(Mutex::new(vec![]));
    let ta = TestActor {identifiers: identifiers.clone()}.start();
    AddrResolver::from_registry().do_send(AddrRequest::Register(ta.clone().recipient(), identifier.clone()));
    ta.do_send(TestMessage {});
    delay_for(Duration::from_secs(1)).await;
    assert_eq!((*(identifiers.lock().unwrap())).get(0).unwrap(), &identifier);
}

#[test]
fn addr_representation_eq_not_key() {
    let own = AddrRepresentation::NetworkInterface;
    let other1 = AddrRepresentation::NetworkInterface;
    let other2 = AddrRepresentation::Gossip;
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
    last: bool
}

#[test]
#[ignore] //github workflows don't get the timing right
fn remote_addr_ignores_hash() {
    let ip1: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000)).parse().unwrap();
    let ip2: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000)).parse().unwrap();
    let ip3: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000)).parse().unwrap();

    let arr = [
        TestParams {ip: ip1.clone(), seeds: vec![], last: false},
        TestParams {ip: ip2.clone(), seeds: vec![ip1.clone()], last: false},
        TestParams {ip: ip3.clone(), seeds: vec![ip1.clone()], last: true},
    ];
    arr.par_iter().for_each(|p| build_cluster(p.ip, p.seeds.clone(), p.last));
}

#[actix_rt::main]
async fn build_cluster(own_ip: SocketAddr, other_ip: Vec<SocketAddr>, last: bool) {
    let _cluster = Cluster::new(own_ip, other_ip);
    if last {
        let returned: Arc<Mutex<Option<usize>>> = Arc::new(Mutex::new(None));
        let _listener = OwnListenerGossipIntroduction {addrs: vec![], returned: returned.clone()}.start();
        delay_for(Duration::from_millis(200)).await;
        returned.lock().unwrap().expect("Something should be returned");
    } else {
        delay_for(Duration::from_millis(200)).await;
    }
}

struct OwnListenerGossipIntroduction {
    pub addrs: Vec<RemoteAddr>,
    pub returned: Arc<Mutex<Option<usize>>>
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
            ClusterLog::NewMember(_addr, remote_addr) => {
                self.addrs.push(remote_addr);
                if self.addrs.len() >= 2 {
                    let remote_addr = self.addrs.get(0).unwrap().clone();
                    let mut remote_addr2 = remote_addr.clone();
                    remote_addr2.network_interface = None;
                    //let remote_addr3 = self.addrs.get(1).unwrap().clone();

                    let mut map = HashMap::<RemoteAddr, usize>::new();
                    map.insert(remote_addr, 0);
                    (*(self.returned.lock().unwrap())) = map.remove(&remote_addr2);
                }
            },
            _ => ()
        }
    }
}
