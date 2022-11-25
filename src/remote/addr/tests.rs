use crate::prelude::*;
use crate::{AddrRepresentation, AddrRequest, AddrResolver, AddrResponse, ResponseSubscribe};
use actix::prelude::*;
use actix_broker::BrokerSubscribe;
use actix_telepathy_derive::{RemoteActor, RemoteMessage};
use port_scanner::request_open_port;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use rayon::iter::IndexedParallelIterator;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use std::any::{Any, TypeId};
use tokio::time::sleep;
use tokio::sync::oneshot::Sender;

#[derive(RemoteMessage, Serialize, Deserialize)]
struct TestMessage {}

#[derive(RemoteActor)]
#[remote_messages(TestMessage)]
struct TestActor {
    identifiers: Arc<Mutex<Vec<String>>>,
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
                        AddrResponse::ResolveRec(identifer) => {
                            act.identifiers.lock().unwrap().push(identifer)
                        }
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
    let ta = TestActor {
        identifiers: identifiers.clone(),
    }
    .start();
    AddrResolver::from_registry().do_send(AddrRequest::Register(
        ta.clone().recipient(),
        identifier.clone(),
    ));
    ta.do_send(TestMessage {});
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
    pub addrs: Vec<RemoteAddr>,
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
            }
            _ => (),
        }
    }
}

#[derive(RemoteMessage, Serialize, Deserialize)]
struct ResponseTestMessage {}

#[derive(RemoteMessage, Serialize, Deserialize)]
struct ResponseTest(pub String);

#[derive(Debug)]
enum InternalError {
    Timeout,
}

#[derive(Message)]
#[rtype(result = "Result<(), InternalError>")]
struct InternalTrigger();

#[derive(RemoteActor)]
#[remote_messages(ResponseTestMessage, ResponseTest)]
struct SendTestActor {
    members: Arc<Mutex<Vec<RemoteAddr>>>,
    subscription_senders: HashMap<TypeId, Sender<Box<dyn Any + Send + 'static>>>,
}

impl SendTestActor {
    pub fn new() -> Self {
        Self {
            members: Arc::new(Mutex::new(vec![])),
            subscription_senders: HashMap::new(),
        }
    }
}

impl Actor for SendTestActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        self.register(ctx.address().recipient());
        self.subscribe_system_async::<ClusterLog>(ctx);
    }
}

impl Handler<InternalTrigger> for SendTestActor {
    type Result = ResponseFuture<Result<(), InternalError>>;
    fn handle(&mut self, _: InternalTrigger, ctx: &mut Context<Self>) -> Self::Result {
        let self_addr = ctx.address();
        let members = self.members.clone();
        Box::pin(async move {
            for remote_addr in members.lock().unwrap().iter() {
                let future = remote_addr.send::<_, ResponseTest, _>(ResponseTestMessage{}, &self_addr.clone());
                let res = tokio::time::timeout(Duration::from_secs(2), future).await.unwrap();
                res.map_err(|_| InternalError::Timeout)?;
            }
            Ok(())
        })
    }
}

impl Handler<ResponseTestMessage> for SendTestActor {
    type Result = ResponseFuture<()>;
    fn handle(&mut self, _: ResponseTestMessage, _: &mut Context<Self>) -> Self::Result {
        let members = self.members.clone();
        Box::pin(async move {
            let members = members.lock().unwrap();
            for remote_addr in members.iter() {
                remote_addr.do_send(ResponseTest("Hello from the other side".to_string()));
            }
        })
    }
}

impl Handler<ResponseSubscribe> for SendTestActor {
    type Result = ();
    fn handle(&mut self, ResponseSubscribe(id, tx): ResponseSubscribe, _: &mut Context<Self>) {
        self.subscription_senders.insert(id, tx);
    }
}

impl Handler<ResponseTest> for SendTestActor {
    type Result = ();
    fn handle(&mut self, msg: ResponseTest, _: &mut Context<Self>) {
        let tx = self.subscription_senders.remove(&TypeId::of::<ResponseTest>());
        if let Some(tx) = tx {
            tx.send(Box::new(msg)).unwrap();
        }
    }
}

impl ClusterListener for SendTestActor {}

impl Handler<ClusterLog> for SendTestActor {
    type Result = ResponseFuture<()>;
    fn handle(&mut self, msg: ClusterLog, _: &mut Context<Self>) -> Self::Result {
        let members = self.members.clone();
        Box::pin(async move {
            match msg {
                ClusterLog::NewMember(_addr, mut remote_addr) => {
                    remote_addr.change_id(Self::ACTOR_ID.to_string());
                    members.lock().unwrap().push(remote_addr);
                },
                _ => (),
            }
        })
    }
}

#[test]
fn test_send_with_response() {
    let _ = env_logger::builder().is_test(true).try_init();
    let ip1: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();
    let ip2: SocketAddr = format!("127.0.0.1:{}", request_open_port().unwrap_or(8000))
        .parse()
        .unwrap();
    let arr = [
        (ip1, vec![ip2]),
        (ip2, vec![]),
    ];
    let (tx, rx) = mpsc::channel();
    let tx = Arc::new(Mutex::new(tx));
    arr.par_iter()
        .enumerate()
        .for_each(|(i, (ip, vec))| match i {
            0 => test_start_actor(*ip, vec.clone()),
            1 => test_send_trigger(*ip, vec.clone(), tx.clone()),
            _ => (),
        });
    if let Err(err) = rx.try_recv().unwrap() {
        panic!("Error occured: {:?}", err);
    }
}

#[actix_rt::main]
async fn test_send_trigger(addr: SocketAddr, seeds: Vec<SocketAddr>, r: Arc<Mutex<mpsc::Sender<Result<(), InternalError>>>>) {
    let _cluster = Cluster::new(addr, seeds);
    let actor = SendTestActor::new().start();
    sleep(Duration::from_millis(300)).await;
    r.lock().unwrap().send(actor.send(InternalTrigger()).await.unwrap()).unwrap();
}

#[actix_rt::main]
async fn test_start_actor(addr: SocketAddr, seeds: Vec<SocketAddr>) {
    let _cluster = Cluster::new(addr, seeds);
    let _actor = SendTestActor::new().start();
    sleep(Duration::from_millis(500)).await;
}
