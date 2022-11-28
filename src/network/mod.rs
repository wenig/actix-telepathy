mod resolver;
mod writer;

use actix::prelude::*;
use log::*;
use std::io::Error;
use std::net::SocketAddr;
use tokio::net::TcpStream;

use crate::cluster::{Cluster, Gossip, NodeEvents};
use crate::codec::{ClusterMessage, ConnectCodec};
use crate::network::resolver::{Connect, Resolver};
use crate::network::writer::Writer;
use crate::remote::{AddrRepresentation, AddrResolver, RemoteWrapper};
use crate::{ConnectionApproval, ConnectionApprovalResponse, CustomSystemService, Node};
use actix::io::WriteHandler;
use std::fmt;
use std::thread::sleep;
use tokio::time::Duration;
use tokio_util::codec::FramedRead;

pub struct NetworkInterface {
    own_ip: SocketAddr,
    pub addr: SocketAddr,
    stream: Vec<TcpStream>,
    connected: bool,
    own_addr: Option<Addr<NetworkInterface>>,
    counter: i8,
    seed: bool,
    writer: Option<Addr<Writer>>,
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
            return Running::Continue;
        }
        Cluster::from_custom_registry().do_send(NodeEvents::MemberDown(self.addr));
        Running::Stop
    }

    fn stopped(&mut self, _ctx: &mut Context<Self>) {
        debug!("NetworkInterface stopped! {}", self.addr);
    }
}

impl NetworkInterface {
    pub fn new(own_ip: SocketAddr, addr: SocketAddr, seed: bool) -> NetworkInterface {
        NetworkInterface {
            own_ip,
            addr,
            stream: vec![],
            connected: false,
            own_addr: None,
            counter: 0,
            seed,
            writer: None,
        }
    }

    pub fn from_stream(
        own_ip: SocketAddr,
        addr: SocketAddr,
        stream: TcpStream,
    ) -> NetworkInterface {
        let mut ni = Self::new(own_ip, addr, false);
        ni.stream.push(stream);
        ni
    }

    fn frame_stream(&mut self, ctx: &mut Context<Self>) {
        let stream = self.stream.pop().unwrap();
        let (r, w) = stream.into_split();

        let framed = actix::io::FramedWrite::new(w, ConnectCodec::new(), ctx);
        self.writer = Some(Writer::new(framed).start());
        ctx.add_stream(FramedRead::new(r, ConnectCodec::new()));
    }

    fn connect_to_stream(&mut self, ctx: &mut Context<Self>) {
        let addr = self.addr.clone().to_string();

        Resolver::from_registry()
            .send(Connect::host(addr))
            .into_actor(self)
            .map(|res, act, ctx| match res {
                Ok(stream) => {
                    if let Ok(stream) = stream {
                        debug!(
                            "Connected to network node: {}",
                            act.addr.clone().to_string()
                        );

                        let (r, w) = stream.into_split();

                        // configure write side of the connection
                        let mut framed = actix::io::FramedWrite::new(w, ConnectCodec::new(), ctx);
                        let reply_port = act.own_ip.port();
                        framed.write(ClusterMessage::Request(reply_port, act.seed));
                        act.writer = Some(Writer::new(framed).start());

                        ctx.add_stream(FramedRead::new(r, ConnectCodec::new()));
                    } else {
                        debug!("Connection refused! Trying to reconnect!");
                        act.counter += 1;
                        sleep(Duration::from_secs(1));
                        ctx.stop();
                    }
                }
                Err(err) => error!("{}", err.to_string()),
            })
            .wait(ctx);
    }

    fn finish_connecting(&mut self) {
        self.connected = true;

        match self.own_addr.clone() {
            Some(addr) => {
                debug!("finish connecting to {}", self.addr.to_string());
                let node = Node::new(self.addr, Some(addr));
                Cluster::from_custom_registry().do_send(NodeEvents::MemberUp(node, self.seed));
            }
            None => error!("NetworkInterface might not have been started already!"),
        };
    }

    fn transmit_message(&mut self, msg: ClusterMessage) {
        self.writer.as_ref().unwrap().do_send(msg);
    }

    fn received_message(&mut self, mut msg: RemoteWrapper) {
        msg.source = self.own_addr.clone();
        match msg.destination.id {
            AddrRepresentation::NetworkInterface => {
                panic!("NetworkInterface does not interact as RemoteActor")
            }
            AddrRepresentation::Gossip => Gossip::from_custom_registry().do_send(msg),
            AddrRepresentation::Key(_) => AddrResolver::from_registry().do_send(msg),
        }
    }

    fn set_reply_port(&mut self, port: u16, ctx: &mut Context<Self>, seed: bool) {
        let send_addr = self.addr;
        self.addr.set_port(port);
        let addr = self.addr;
        self.seed = seed;

        Cluster::from_custom_registry()
            .send(ConnectionApproval { addr, send_addr })
            .into_actor(self)
            .map(|res, act, ctx| {
                if let Ok(message_response) = res {
                    match message_response {
                        ConnectionApprovalResponse::Approved => {
                            act.transmit_message(ClusterMessage::Response);
                            act.finish_connecting()
                        }
                        ConnectionApprovalResponse::Declined => {
                            act.transmit_message(ClusterMessage::Decline);
                            ctx.stop()
                        }
                    }
                }
            })
            .wait(ctx);
    }
}

impl StreamHandler<Result<ClusterMessage, Error>> for NetworkInterface {
    fn handle(&mut self, item: Result<ClusterMessage, Error>, ctx: &mut Context<Self>) {
        match item {
            Ok(msg) => match msg {
                ClusterMessage::Request(reply_port, seed) => {
                    self.set_reply_port(reply_port, ctx, seed)
                }
                ClusterMessage::Response => self.finish_connecting(),
                ClusterMessage::Message(remote_message) => self.received_message(remote_message),
                ClusterMessage::Decline => ctx.stop(),
            },
            Err(err) => warn!("{}", err),
        }
    }
}

impl Handler<ClusterMessage> for NetworkInterface {
    type Result = ();

    fn handle(&mut self, msg: ClusterMessage, _ctx: &mut Context<Self>) -> Self::Result {
        self.transmit_message(msg);
    }
}

#[derive(Message)]
#[rtype(result = "Result<(), MailboxError>")]
pub struct WrappedClusterMessage(pub(crate) ClusterMessage);

impl Handler<WrappedClusterMessage> for NetworkInterface {
    type Result = ResponseFuture<Result<(), MailboxError>>;

    fn handle(&mut self, msg: WrappedClusterMessage, _ctx: &mut Self::Context) -> Self::Result {
        Box::pin(self.writer.as_ref().unwrap().send(msg.0))
    }
}

impl WriteHandler<Error> for NetworkInterface {}
impl Supervised for NetworkInterface {}

impl fmt::Debug for NetworkInterface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NetworkInterface({})", self.addr)
    }
}
