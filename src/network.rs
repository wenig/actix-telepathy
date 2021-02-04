use actix::prelude::*;
use log::*;
use std::net::{SocketAddr};
use tokio::net::TcpStream;
use tokio_util::codec::{FramedRead};
use std::io::{Error};

use crate::cluster::{Cluster, NodeEvents, Gossip};
use crate::codec::{ClusterMessage, ConnectCodec, AskClusterMessage};
use crate::remote::{RemoteAddr, RemoteWrapper, AddrRepresentation, AddressResolver};
use actix::io::{WriteHandler};
use tokio::net::tcp::OwnedWriteHalf;
use std::thread::sleep;
use actix::clock::Duration;
use std::fmt;
use crate::{ConnectionApproval, ConnectionApprovalResponse, CustomSystemService};
use crate::response_resolver::{ResponseResolver, RegisterResponse};


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
    counter: i8,
    seed: bool
}


impl Actor for NetworkInterface {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        debug!("NetworkInterface started! {}", self.addr);
        self.own_addr = Some(ctx.address());
        self.counter = 0;
        if self.stream.is_empty() {
            self.connect_to_stream(ctx);
        } else {
            self.frame_stream(ctx);
        }
    }

    fn stopping(&mut self, ctx: &mut Context<Self>) -> Running {
        if self.counter < 2 {
            self.stream = vec![];
            self.connect_to_stream(ctx);
            return Running::Continue
        }
        Running::Stop
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("NetworkInterface stopped! {}", self.addr);
        //self.parent.do_send(NodeEvents::MemberDown(self.addr.clone().to_string()));
    }
}


impl NetworkInterface {
    pub fn new(own_ip: SocketAddr, addr: SocketAddr, seed: bool) -> NetworkInterface {
        let parent = Cluster::from_custom_registry();
        let gossip = Gossip::from_custom_registry();
        let address_resolver = AddressResolver::from_registry();
        NetworkInterface {own_ip, addr, stream: vec![], framed: vec![], connected: false, own_addr: None, parent, gossip, address_resolver, counter: 0, seed }
    }

    pub fn from_stream(own_ip: SocketAddr, addr: SocketAddr, stream: TcpStream) -> NetworkInterface {
        let mut ni = Self::new(own_ip, addr, false);
        ni.stream.push(stream);
        ni
    }

    fn frame_stream(&mut self, ctx: &mut Context<Self>){
        let stream = self.stream.pop().unwrap();
        let (r, w) = stream.into_split();

        let framed = actix::io::FramedWrite::new(w, ConnectCodec::new(), ctx);
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

                        let stream = stream.unwrap();

                        let (r, w) = stream.into_split();

                        // configure write side of the connection
                        let mut framed =
                            actix::io::FramedWrite::new(w, ConnectCodec::new(), ctx);
                        let reply_port = act.own_ip.port();
                        framed.write(ClusterMessage::Request(reply_port, act.seed));
                        act.framed.push(framed);

                        // read side of the connection
                        ctx.add_stream(FramedRead::new(r, ConnectCodec::new()));
                    }
                },
                Err(err) => error!("{}", err.to_string()),
            })
            .wait(ctx);
    }

    fn finish_connecting(&mut self) {
        self.connected = true;

        match self.own_addr.clone() {
            Some(addr) => {
                debug!("finish connecting to {}", self.addr.to_string());
                let remote_address = RemoteAddr::new(self.addr.clone(), Some(addr.clone().recipient()), AddrRepresentation::NetworkInterface);
                self.parent.do_send(NodeEvents::MemberUp(self.addr.clone(), addr, remote_address, self.seed));
            },
            None => error!("NetworkInterface might not have been started already!")
        };
    }

    fn transmit_message(&mut self, msg: ClusterMessage) {
        &self.framed[0].write(msg);
    }

    fn received_message(&mut self, mut msg: RemoteWrapper, ctx: &mut Context<Self>) {
        msg.source = Some(self.own_addr.clone().unwrap());
        match msg.destination.id {
            AddrRepresentation::NetworkInterface => panic!("NetworkInterface does not interact as RemoteActor"),
            AddrRepresentation::Gossip => self.forward_message(true, msg, ctx),
            AddrRepresentation::Key(_) => self.forward_message(false, msg, ctx)
        }
    }

    fn forward_message(&mut self, to_gossip: bool, msg: RemoteWrapper, ctx: &mut Context<Self>) {
        match msg.conversation_id {
            Some(_) => {
                assert!(!to_gossip, "Gossip cannot receive AskRemoteWrappers");
                self.address_resolver.send(msg.as_ask())
                    .into_actor(self)
                    .map(|res: Result<Result<RemoteWrapper, ()>, MailboxError>, act, _ctx| {
                        match res {
                            Ok(res) => match res {
                                Ok(remote_wrapper) => act.transmit_message(
                                    ClusterMessage::Message(remote_wrapper)
                                ),
                                Err(_) => error!("No RemoteWrapper was returned!")
                            },
                            Err(e) => error!("{}", e.to_string())
                        }
                    })
                    .wait(ctx);
            },
            None => {
                if to_gossip {
                    self.gossip.do_send(msg);
                } else {
                    self.address_resolver.do_send(msg);
                }
            }
        };
    }

    fn set_reply_port(&mut self, port: u16, ctx: &mut Context<Self>, seed: bool) {
        let send_addr = self.addr.clone();
        self.addr.set_port(port);
        let addr = self.addr.clone();
        self.seed = seed;

        self.parent.send(ConnectionApproval { addr, send_addr })
            .into_actor(self)
            .map(|res, act, ctx| match res {
                Ok(message_response) => match message_response {
                    ConnectionApprovalResponse::Approved => {
                        act.transmit_message(ClusterMessage::Response);
                        act.finish_connecting()
                    },
                    ConnectionApprovalResponse::Declined => {
                        act.transmit_message(ClusterMessage::Decline);
                        ctx.stop()
                    }
                },
                Err(_) => {}
            }).wait(ctx);
    }
}

impl StreamHandler<Result<ClusterMessage, Error>> for NetworkInterface {
    fn handle(&mut self, item: Result<ClusterMessage, Error>, ctx: &mut Context<Self>) {
        match item {
            Ok(msg) => match msg {
                ClusterMessage::Request(reply_port, seed) => self.set_reply_port(reply_port, ctx, seed),
                ClusterMessage::Response => self.finish_connecting(),
                ClusterMessage::Message(remote_message) => self.received_message(remote_message, ctx),
                ClusterMessage::Decline => ctx.stop()
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

impl Handler<AskClusterMessage> for NetworkInterface {
    type Result = ResponseActFuture<Self, Result<RemoteWrapper, ()>>;

    fn handle(&mut self, msg: AskClusterMessage, _ctx: &mut Context<Self>) -> Self::Result {
        let remote_wrapper = match &msg.msg {
            ClusterMessage::Message(remote_wrapper) => Some(remote_wrapper),
            _ => None
        }.expect("Only ClusterMessage::Message can be send in an asking way!");

        let id = remote_wrapper.conversation_id.unwrap().clone();
        let send_to_other = ResponseResolver::from_registry().send(RegisterResponse {id});
        let send_to_other = actix::fut::wrap_future::<_, Self>(send_to_other);

        let update_self = send_to_other.map(|res, _act, _ctx| {
            match res {
                Ok(v) => {
                    v
                },
                Err(_e) => Err(())
            }
        });

        self.transmit_message(msg.msg);

        Box::pin(update_self)
    }
}

impl WriteHandler<Error> for NetworkInterface {}
impl Supervised for NetworkInterface {}

impl fmt::Debug for NetworkInterface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NetworkInterface({})", self.addr.clone().to_string())
    }
}
