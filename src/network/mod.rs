mod resolver;
mod writer;

use actix::prelude::*;
use futures::channel::oneshot::{Sender, channel};
use log::*;
use uuid::Uuid;
use std::collections::HashMap;
use std::io::Error;
use std::net::SocketAddr;
use tokio::net::TcpStream;

use crate::cluster::{Cluster, NodeEvent};
use crate::codec::{ClusterMessage, ConnectCodec};
use crate::network::resolver::{Connect, Resolver};
use crate::network::writer::Writer;
use crate::remote::{AddrRepresentation, AddrResolver, RemoteWrapper};
use crate::{ConnectionApproval, ConnectionApprovalResponse, Connector, CustomSystemService};
use crate::{Node, RemoteResponse};
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
    futures: HashMap<Uuid,Sender<Vec<u8>>>
}

impl Actor for NetworkInterface {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        debug!(target: &self.own_ip.to_string(), "NetworkInterface started! {}", self.addr);
        self.own_addr = Some(ctx.address());
        self.counter = 0;
        if self.stream.is_empty() {
            self.connect_to_stream(ctx);
        } else {
            self.frame_stream(ctx);
        }
    }

    fn stopping(&mut self, ctx: &mut Context<Self>) -> Running {
        warn!(target: &self.own_ip.to_string(), "NetworkInterface stopping! {}, counter: {}", self.addr, self.counter);
        if self.counter < 2 {
            self.stream = vec![];
            self.connect_to_stream(ctx);
            return Running::Continue;
        }

        Cluster::from_custom_registry().do_send(NodeEvent::MemberDown(self.addr));
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
            futures: HashMap::new()
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
                        debug!("Connected to network node: {}", act.addr);

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
                Err(err) => {
                    error!("{} | {}", err, act.addr);
                    act.counter += 1;
                    sleep(Duration::from_secs(1));
                    ctx.stop();
                }
            })
            .wait(ctx);
    }

    fn finish_connecting(&mut self, self_is_seed: bool) {
        self.connected = true;

        match self.own_addr.clone() {
            Some(addr) => {
                debug!(target: &self.own_ip.to_string(), "finish connecting to {}", self.addr);
                let node = Node::new(self.addr, Some(addr));
                Cluster::from_custom_registry().do_send(NodeEvent::MemberUp(node, self_is_seed));
            }
            None => error!("NetworkInterface might not have been started already!"),
        };
    }

    fn transmit_message(&mut self, msg: ClusterMessage) {
        self.writer.as_ref().unwrap().do_send(msg);
    }

    fn wait_transmit_message(&mut self, msg: ClusterMessage)-> ResponseFuture<Result<(), MailboxError>>  {
        Box::pin(self.writer.as_ref().unwrap().send(msg))
    }

    fn received_message(&mut self, mut msg: RemoteWrapper) {
        msg.source = self.own_addr.clone();
        match msg.destination.id {
            AddrRepresentation::NetworkInterface => {
                panic!("NetworkInterface does not interact as RemoteActor")
            }
            AddrRepresentation::Connector => Connector::from_custom_registry().do_send(msg),
            AddrRepresentation::Key(_) => AddrResolver::from_registry().do_send(msg),
        }
    }

    fn received_reply(&mut self, msg: RemoteResponse) {
        if let Some(fut) = self.futures.remove(&msg.0) {
            let _ = fut.send(msg.1);
        }
    }

    fn set_reply_port(&mut self, port: u16, ctx: &mut Context<Self>, seed: bool) {
        let send_addr = self.addr;
        self.addr.set_port(port);
        let addr = self.addr;

        Cluster::from_custom_registry()
            .send(ConnectionApproval { addr, send_addr })
            .into_actor(self)
            .map(move |res, act, ctx| {
                if let Ok(message_response) = res {
                    match message_response {
                        ConnectionApprovalResponse::Approved => {
                            act.transmit_message(ClusterMessage::Response);
                            act.finish_connecting(seed)
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
                ClusterMessage::Response => self.finish_connecting(false),
                ClusterMessage::Message(remote_message) => self.received_message(remote_message),
                ClusterMessage::MessageResponse(remote_response) => self.received_reply(remote_response),
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

impl Handler<RemoteResponse> for NetworkInterface {
    type Result = ();

    fn handle(&mut self, msg: RemoteResponse, _ctx: &mut Context<Self>) -> Self::Result {
        self.transmit_message(ClusterMessage::MessageResponse(msg));
    }
}

#[derive(Message)]
#[rtype(result = "Result<Vec<u8>, MailboxError>")]
pub struct WrappedClusterMessage(pub(crate) ClusterMessage);

impl Handler<WrappedClusterMessage> for NetworkInterface {
    type Result = ResponseActFuture<Self, Result<Vec<u8>, MailboxError>>;

    fn handle(&mut self, wmsg: WrappedClusterMessage, _ctx: &mut Self::Context) -> Self::Result {
        let msg = wmsg.0;
        let id = get_converstation_id(&msg);
        let send = self.wait_transmit_message(msg);

        let (sender, receiver) = channel::<Vec<u8>>();
        if let Some(id) = id {
            self.futures.insert(id, sender);
        }

        Box::pin(
            async move {
                let sent = send.await;
                if sent.is_err() {
                    Err(sent.unwrap_err())
                } else if id.is_some()  {
                    Ok(receiver.await.unwrap()) // todo: add timeout in config yaml return MailboxError::timeout and select! between the timeout and the receiver
                } else {
                    Ok(Vec::new()) // placeholder for wait_send(), not to be deserialized
                }
            }
            .into_actor(self)
        )
    }
}

impl WriteHandler<Error> for NetworkInterface {}
impl Supervised for NetworkInterface {}

impl fmt::Debug for NetworkInterface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NetworkInterface({})", self.addr)
    }
}

fn get_converstation_id(msg: &ClusterMessage) -> Option<Uuid> {
    if let ClusterMessage::Message(rem) = msg {
        rem.conversation_id
    } else {
        None
    }
}