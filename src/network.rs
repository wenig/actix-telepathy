use actix::prelude::*;
use log::*;
use futures::executor::block_on;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::str::FromStr;
use actix::prelude::fut::IntoActorFuture;
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead, LinesCodec, LinesCodecError, FramedWrite};
use std::io::{Error};
use futures::TryFutureExt;

use crate::cluster::{Cluster, ClusterListener, ClusterLog, NodeEvents};
use crate::codec::{JoinCluster, ConnectCodec};
use crate::remote_addr::RemoteAddr;
use futures::TryStreamExt;
use tokio::prelude::io::AsyncBufReadExt;
use actix::io::{WriteHandler};
use tokio::io::WriteHalf;
use std::iter::Copied;
use tokio::net::tcp::OwnedWriteHalf;
use actix::fut::err;
use futures::future::Remote;
use std::ops::Add;


pub struct NetworkInterface {
    own_ip: String,
    pub addr: SocketAddr,
    stream: Vec<TcpStream>,
    framed: Vec<actix::io::FramedWrite<JoinCluster, OwnedWriteHalf, ConnectCodec>>,
    listener: Option<Addr<ClusterListener>>,
    connected: bool,
    own_addr: Option<Addr<NetworkInterface>>,
    parent: Addr<Cluster>
}


impl Actor for NetworkInterface {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        debug!("NetworkInterface started!");
        self.own_addr = Some(ctx.address());
        if self.stream.is_empty() {
            self.connect_to_stream(ctx);
        } else {
            self.frame_stream(ctx);
        }
    }
}


impl NetworkInterface {
    pub fn new(own_ip: String, addr: SocketAddr, listener: Option<Addr<ClusterListener>>, parent: Addr<Cluster>) -> NetworkInterface {
        NetworkInterface {own_ip, addr, stream: vec![], framed: vec![], listener, connected: false, own_addr: None, parent }
    }

    pub fn from_stream(own_ip: String, addr: SocketAddr, stream: TcpStream, listener: Option<Addr<ClusterListener>>, parent: Addr<Cluster>) -> NetworkInterface {
        let mut ni = Self::new(own_ip, addr, listener, parent);
        ni.stream.push(stream);
        ni
    }

    fn frame_stream(&mut self, ctx: &mut Context<Self>){
        let mut stream = self.stream.pop().unwrap();
        let (mut r, w) = stream.into_split();

        let mut framed = actix::io::FramedWrite::new(w, ConnectCodec::new(), ctx);
        framed.write(JoinCluster::Response);
        self.framed.push(framed);

        ctx.add_stream(FramedRead::new(r, ConnectCodec::new()));
    }

    fn connect_to_stream(&mut self, ctx: &mut Context<Self>){
        let addr = self.addr.clone().to_string();
        actix::actors::resolver::Resolver::from_registry()
            .send(actix::actors::resolver::Connect::host(addr))
            .into_actor(self)
            .map(|res, act, ctx| match res {
                Ok(stream) => {
                    debug!("Connected to network node: {}", act.addr.clone().to_string());

                    let mut stream = stream.expect("error");
                    let (mut r, w) = stream.into_split();

                    // configure write side of the connection
                    let mut framed =
                        actix::io::FramedWrite::new(w, ConnectCodec::new(), ctx);
                    framed.write(JoinCluster::Request(act.own_ip.clone()));
                    act.framed.push(framed);

                    // read side of the connection
                    ctx.add_stream(FramedRead::new(r, ConnectCodec::new()));
                },
                Err(err) => error!("{}", err),
            })
            .wait(ctx);
    }

    fn finish_connecting(&mut self, peer_addr: Option<String>) {
        self.connected = true;
        match self.listener.clone() {
            Some(listener) => {
                match self.own_addr.clone() {
                    Some(addr) => {
                        let remote_address = RemoteAddr::new(self.addr, addr);
                        listener.do_send(ClusterLog::NewMember(remote_address));
                        let peer_addr = match peer_addr {
                            Some(p_a) => p_a,
                            None => self.addr.to_string()
                        };
                        match &self.own_addr {
                            Some(oa) => self.parent.do_send(NodeEvents::MemberUp(peer_addr, oa.clone())),
                            None => ()
                        }
                    },
                    None => error!("NetworkInterface might not have been started already!")
                }
            },
            None => debug!("No ClusterListener available!"),
        };
    }

    fn requested(&mut self, addr: String) {
        debug!("Request from {}", addr);
        self.finish_connecting(Some(addr));
    }

    fn responsed(&mut self) {
        debug!("Response");
        self.finish_connecting(None);
    }

    fn transmit_message(&mut self, msg: JoinCluster){
        &self.framed[0].write(msg);
    }
}

impl StreamHandler<Result<JoinCluster, Error>> for NetworkInterface {
    fn handle(&mut self, item: Result<JoinCluster, Error>, _ctx: &mut Context<Self>) {
        match item {
            Ok(msg) => match msg {
                JoinCluster::Request(addr) => self.requested(addr),
                JoinCluster::Response => self.responsed(),
                JoinCluster::Message(str_msg) => debug!("received '{}'", str_msg),
                _ => {}
            },
            Err(err) => error!("{}", err)
        }
    }
}

impl Handler<JoinCluster> for NetworkInterface {
    type Result = ();

    fn handle(&mut self, msg: JoinCluster, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Received local message");
        self.transmit_message(msg)
    }
}

impl WriteHandler<Error> for NetworkInterface {}
impl Supervised for NetworkInterface {}
