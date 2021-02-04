mod serializer;

#[macro_use] extern crate log;

use actix_rt;
use actix_telepathy::prelude::*;
use actix::prelude::*;
use structopt::StructOpt;
use serde::{Serialize, Deserialize};
#[allow(unused_imports)]
use serializer::MySerializer;
use tokio;
use std::net::{ToSocketAddrs, SocketAddr};
use std::fs;


#[derive(Message, Serialize, Deserialize, RemoteMessage, MessageResponse)]
#[rtype(result = "Welcome")]
struct Welcome {}

fn from_addr(s: &str) -> SocketAddr {
    s.to_socket_addrs().expect("Could not parse seed node").next().unwrap()
}

#[derive(StructOpt, Debug)]
struct Parameters {
    local_ip: SocketAddr,
    #[structopt(parse(from_str = from_addr))]
    seed_nodes: Vec<SocketAddr>,
}

#[derive(RemoteActor)]
#[remote_messages(Welcome)]
struct OwnListener {
    count: usize,
    filename: String
}

impl OwnListener {
    const IDENTIFIER: &'static str = "own_listener";

    pub fn new(filename: String) -> Self {
        OwnListener {count: 0, filename}
    }
}

impl ClusterListener for OwnListener {}
impl Supervised for OwnListener {}

impl Actor for OwnListener {
    type Context = Context<Self>;
}

impl Handler<ClusterLog> for OwnListener {
    type Result = ();

    fn handle(&mut self, msg: ClusterLog, ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            ClusterLog::NewMember(addr, remote_addr) => {
                RemoteAddr::new_from_key(
                    addr,
                    remote_addr.network_interface.unwrap(),
                    OwnListener::IDENTIFIER
                ).send(Box::new(Welcome {}))
                    .into_actor(self)
                    .map(|res, act, ctx| {
                        match res {
                            Ok(v) => debug!("Future, {:?}", v),
                            Err(e) => error!("{}", e.to_string())
                        }
                    }).wait(ctx);
            },
            ClusterLog::MemberLeft(_addr) => debug!("ClusterLog: MemberLeft")
        }
    }
}

impl Handler<Welcome> for OwnListener {
    type Result = Welcome;

    fn handle(&mut self, _msg: Welcome, _ctx: &mut Context<Self>) -> Self::Result {
        self.count = self.count + 1;
        debug!("Welcome said {}x", self.count);
        Welcome {}
    }
}


impl Handler<AskRemoteWrapper> for OwnListener {
    type Result = ResponseActFuture<Self, Result<RemoteWrapper, ()>>;

    fn handle(&mut self, mut msg: AskRemoteWrapper, ctx: &mut Context<Self>) -> Self::Result {
        let conversation_id = msg.remote_wrapper.conversation_id.clone();
        let destination = msg.remote_wrapper.source.as_ref().unwrap().clone();

        let mut deserialized_msg: Welcome = Welcome::generate_serializer().deserialize(&(msg.remote_wrapper.message_buffer)[..]).expect("Cannot deserialized Welcome message");
        let result = ctx.address().send(deserialized_msg);
        let result = actix::fut::wrap_future::<_, Self>(result);

        let update_self = result.map(move |res, _act, _ctx| {
            match res {
                Ok(v) => {
                    Ok(RemoteWrapper::new(destination, Box::new(v), conversation_id))
                },
                Err(_e) => Err(())
            }
        });

        Box::pin(update_self)
    }
}


#[actix_rt::main]
async fn main() {
    env_logger::init();
    let args = Parameters::from_args();

    let cluster_listener = OwnListener::new(args.local_ip.to_string()).start();
    let cluster = Cluster::new(
        args.local_ip.to_socket_addrs().unwrap().next().unwrap(),
        args.seed_nodes,
        vec![cluster_listener.clone().recipient()],
        vec![(cluster_listener.recipient(), OwnListener::IDENTIFIER.to_string())]);

    cluster.do_send(Test {msg: "test".to_string()});

    tokio::signal::ctrl_c().await.unwrap();
    println!("Ctrl-C received, shutting down");
    System::current().stop();
}
