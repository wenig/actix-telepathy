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

use crate::cluster::{Cluster, ClusterLog, NodeEvents, Gossip};
use crate::codec::{ClusterMessage, ConnectCodec};
use crate::remote::{RemoteAddr, RemoteMessage, AddrRepresentation, AddressResolver, AddressRequest};
use actix_telepathy_derive::RemoteActor;
use futures::TryStreamExt;
use tokio::prelude::io::AsyncBufReadExt;
use actix::io::{WriteHandler};
use tokio::io::WriteHalf;
use std::iter::Copied;
use tokio::net::tcp::OwnedWriteHalf;
use actix::fut::err;
use futures::future::Remote;
use std::ops::Add;
use std::thread::sleep;
use actix::clock::Duration;
use futures::task::Poll;


pub struct NetworkInterface {
    own_ip: String,
    pub addr: SocketAddr,
    stream: Vec<TcpStream>,
    framed: Vec<actix::io::FramedWrite<ClusterMessage, OwnedWriteHalf, ConnectCodec>>,
    connected: bool,
    own_addr: Option<Addr<NetworkInterface>>,
    parent: Addr<Cluster>,
    gossip: Addr<Gossip>,
    address_resolver: Addr<AddressResolver>,
    counter: i8
}


impl Actor for NetworkInterface {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        debug!("NetworkInterface started!");
        self.own_addr = Some(ctx.address());
        self.counter = 0;
        if self.stream.is_empty() {
            self.connect_to_stream(ctx);
        } else {
            self.frame_stream(ctx);
        }
    }

    fn stopping(&mut self, ctx: &mut Context<Self>) -> Running {
        if self.counter < 5 {
            self.stream = vec![];
            self.connect_to_stream(ctx);
            return Running::Continue
        }
        Running::Stop
    }

    fn stopped(&mut self, ctx: &mut Context<Self>) {
        debug!("NetworkInterface stopped! {}", self.addr);
        self.parent.do_send(NodeEvents::MemberDown(self.addr.clone().to_string()));
    }
}


impl NetworkInterface {
    pub fn new(own_ip: String, addr: SocketAddr, parent: Addr<Cluster>, gossip: Addr<Gossip>, address_resolver: Addr<AddressResolver>) -> NetworkInterface {
        NetworkInterface {own_ip, addr, stream: vec![], framed: vec![], connected: false, own_addr: None, parent, gossip, address_resolver, counter: 0 }
    }

    pub fn from_stream(own_ip: String, addr: SocketAddr, stream: TcpStream, parent: Addr<Cluster>, gossip: Addr<Gossip>, address_resolver: Addr<AddressResolver>) -> NetworkInterface {
        let mut ni = Self::new(own_ip, addr, parent, gossip, address_resolver);
        ni.stream.push(stream);
        ni
    }

    fn frame_stream(&mut self, ctx: &mut Context<Self>){
        let mut stream = self.stream.pop().unwrap();
        let (mut r, w) = stream.into_split();

        let mut framed = actix::io::FramedWrite::new(w, ConnectCodec::new(), ctx);
        framed.write(ClusterMessage::Response);
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
                    if stream.is_err() {
                        debug!("Connection refused! Trying to reconnect!");
                        act.counter += 1;
                        sleep(Duration::from_secs(1));
                        ctx.stop();
                    } else {
                        debug!("Connected to network node: {}", act.addr.clone().to_string());

                        let mut stream = stream.unwrap();

                        let (mut r, w) = stream.into_split();

                        // configure write side of the connection
                        let mut framed =
                            actix::io::FramedWrite::new(w, ConnectCodec::new(), ctx);
                        framed.write(ClusterMessage::Request(act.own_ip.clone()));
                        act.framed.push(framed);

                        // read side of the connection
                        ctx.add_stream(FramedRead::new(r, ConnectCodec::new()));
                    }
                },
                Err(err) => error!("{}", err),
            })
            .wait(ctx);
    }

    fn finish_connecting(&mut self, peer_addr: Option<String>) {
        self.connected = true;

        match self.own_addr.clone() {
            Some(addr) => {
                let remote_address = RemoteAddr::new(self.addr.to_string(), Some(addr), AddrRepresentation::NetworkInterface);
                let peer_addr = match peer_addr {
                    Some(p_a) => {
                        self.addr = SocketAddr::from_str(&p_a).unwrap();
                        self.addr.to_string()
                    },
                    None => self.addr.to_string()
                };
                match &self.own_addr {
                    Some(oa) => self.parent.do_send(NodeEvents::MemberUp(peer_addr, oa.clone(), remote_address)),
                    None => ()
                }
            },
            None => error!("NetworkInterface might not have been started already!")
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

    fn transmit_message(&mut self, msg: ClusterMessage) {
        &self.framed[0].write(msg);
    }

    fn received_message(&mut self, mut msg: RemoteMessage) {
        match msg.destination.id {
            AddrRepresentation::NetworkInterface => debug!("NetworkInterface does not interact as RemoteActor"),
            AddrRepresentation::Gossip => self.gossip.do_send(msg),
            AddrRepresentation::Key(_) => self.address_resolver.do_send(msg)
        };
    }
}

impl StreamHandler<Result<ClusterMessage, Error>> for NetworkInterface {
    fn handle(&mut self, item: Result<ClusterMessage, Error>, _ctx: &mut Context<Self>) {
        match item {
            Ok(msg) => match msg {
                ClusterMessage::Request(addr) => self.requested(addr),
                ClusterMessage::Response => self.responsed(),
                ClusterMessage::Message(remote_message) => self.received_message(remote_message),
            },
            Err(err) => error!("{}", err)
        }
    }
}

impl Handler<ClusterMessage> for NetworkInterface {
    type Result = ();

    fn handle(&mut self, msg: ClusterMessage, _ctx: &mut Context<Self>) -> Self::Result {
        self.transmit_message(msg);
    }
}

impl WriteHandler<Error> for NetworkInterface {}
impl Supervised for NetworkInterface {}
