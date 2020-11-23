use actix::prelude::*;
use log::*;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead};
use std::io::{Error};

use crate::cluster::{Cluster, NodeEvents, Gossip, ConnectionApproval, ConnectionApprovalResponse};
use crate::codec::{ClusterMessage, ConnectCodec};
use crate::remote::{RemoteAddr, RemoteWrapper, AddrRepresentation, AddressResolver};
use actix::io::{WriteHandler};
use tokio::net::tcp::OwnedWriteHalf;
use std::thread::sleep;
use actix::clock::Duration;
use std::fmt;


pub struct NetworkInterface {
    own_ip: SocketAddr,
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
        /*if self.counter < 5 {
            self.stream = vec![];
            self.connect_to_stream(ctx);
            return Running::Continue
        }*/
        Running::Stop
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("NetworkInterface stopped! {}", self.addr);
        //self.parent.do_send(NodeEvents::MemberDown(self.addr.clone().to_string()));
    }
}


impl NetworkInterface {
    pub fn new(own_ip: SocketAddr, addr: SocketAddr, parent: Addr<Cluster>, gossip: Addr<Gossip>, address_resolver: Addr<AddressResolver>) -> NetworkInterface {
        NetworkInterface {own_ip, addr, stream: vec![], framed: vec![], connected: false, own_addr: None, parent, gossip, address_resolver, counter: 0 }
    }

    pub fn from_stream(own_ip: SocketAddr, addr: SocketAddr, stream: TcpStream, parent: Addr<Cluster>, gossip: Addr<Gossip>, address_resolver: Addr<AddressResolver>) -> NetworkInterface {
        let mut ni = Self::new(own_ip, addr, parent, gossip, address_resolver);
        ni.stream.push(stream);
        ni
    }

    fn frame_stream(&mut self, ctx: &mut Context<Self>){
        let stream = self.stream.pop().unwrap();
        let (r, w) = stream.into_split();

        let mut framed = actix::io::FramedWrite::new(w, ConnectCodec::new(), ctx);
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
                        /*debug!("Connection refused! Trying to reconnect!");
                        act.counter += 1;
                        sleep(Duration::from_secs(1));*/
                        ctx.stop();
                    } else {
                        debug!("Connected to network node: {}", act.addr.clone().to_string());

                        let stream = stream.unwrap();

                        let (r, w) = stream.into_split();

                        // configure write side of the connection
                        let mut framed =
                            actix::io::FramedWrite::new(w, ConnectCodec::new(), ctx);
                        let reply_port = act.own_ip.port();
                        framed.write(ClusterMessage::Request(reply_port));
                        act.framed.push(framed);

                        // read side of the connection
                        ctx.add_stream(FramedRead::new(r, ConnectCodec::new()));
                    }
                },
                Err(err) => error!("{}", err),
            })
            .wait(ctx);
    }

    fn finish_connecting(&mut self) {
        self.connected = true;

        match self.own_addr.clone() {
            Some(addr) => {
                let remote_address = RemoteAddr::new(self.addr.to_string(), Some(addr.clone()), AddrRepresentation::NetworkInterface);
                let peer_addr =  self.addr.to_string();
                self.parent.do_send(NodeEvents::MemberUp(peer_addr, addr, remote_address));
            },
            None => error!("NetworkInterface might not have been started already!")
        };
    }

    fn connection_approved(&mut self) {
        &self.framed[0].write(ClusterMessage::Response);
    }

    fn requested(&mut self, port: u16, ctx: &mut Context<Self>) {
        debug!("Request from {}:{}", self.addr.ip().to_string(), port);

        let send_addr = self.addr.to_string();
        self.addr.set_port(port);
        let addr = self.addr.to_string();

        self.parent.send(ConnectionApproval { addr, send_addr })
            .into_actor(self)
            .map(|res, act, ctx| match res {
                Ok(message_response) => match message_response {
                    ConnectionApprovalResponse::Approved => {
                        &act.framed[0].write(ClusterMessage::Response);
                        act.finish_connecting();
                    },
                    ConnectionApprovalResponse::Declined => {
                        &act.framed[0].write(ClusterMessage::Decline);
                        ctx.stop();
                    }
                },
                Err(_) => {}
            }).wait(ctx);
    }

    fn responsed(&mut self) {
        self.finish_connecting();
    }

    fn transmit_message(&mut self, msg: ClusterMessage) {
        &self.framed[0].write(msg);
    }

    fn received_message(&mut self, mut msg: RemoteWrapper) {
        msg.source = Some(self.own_addr.clone().unwrap());
        match msg.destination.id {
            AddrRepresentation::NetworkInterface => debug!("NetworkInterface does not interact as RemoteActor"),
            AddrRepresentation::Gossip => self.gossip.do_send(msg),
            AddrRepresentation::Key(_) => { self.address_resolver.do_send(msg); }
        };
    }
}

impl StreamHandler<Result<ClusterMessage, Error>> for NetworkInterface {
    fn handle(&mut self, item: Result<ClusterMessage, Error>, ctx: &mut Context<Self>) {
        match item {
            Ok(msg) => match msg {
                ClusterMessage::Request(port) => self.requested(port, ctx),
                ClusterMessage::Response => self.responsed(),
                ClusterMessage::Message(remote_message) => self.received_message(remote_message),
                ClusterMessage::Decline => ctx.stop()
            },
            Err(err) => error!("{}", err)
        }
    }
}

impl Handler<ClusterMessage> for NetworkInterface {
    // todo: give back future in case of send instead of do_send
    type Result = ();

    fn handle(&mut self, msg: ClusterMessage, _ctx: &mut Context<Self>) -> Self::Result {
        self.transmit_message(msg);
    }
}

impl WriteHandler<Error> for NetworkInterface {}
impl Supervised for NetworkInterface {}

impl fmt::Debug for NetworkInterface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NetworkInterface({})", self.addr.clone().to_string())
    }
}
