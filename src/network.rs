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

use crate::messages::{RemoteMessage, LocalMessage, TcpConnect};
use crate::codec::{JoinCluster, ConnectCodec};
use futures::TryStreamExt;
use tokio::prelude::io::AsyncBufReadExt;
use actix::io::{WriteHandler};
use tokio::io::WriteHalf;
use std::iter::Copied;
use tokio::net::tcp::OwnedWriteHalf;


pub struct NetworkInterface {
    pub addr: SocketAddr,
    stream: Vec<TcpStream>,
    framed: Option<actix::io::FramedWrite<JoinCluster, OwnedWriteHalf, ConnectCodec>>
}


impl Actor for NetworkInterface {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        debug!("NetworkInterface started!");
        if self.stream.is_empty() {
            self.connect_to_stream(ctx);
        } else {
            self.frame_stream(ctx);
        }
    }
}


impl NetworkInterface {
    pub fn new(addr: SocketAddr) -> NetworkInterface {
        NetworkInterface {addr, stream: vec![], framed: None }
    }

    pub fn from_stream(addr: SocketAddr, stream: TcpStream) -> NetworkInterface {
        let mut ni = Self::new(addr);
        ni.stream.push(stream);
        ni
    }

    fn frame_stream(&mut self, ctx: &mut Context<Self>){
        let mut stream = self.stream.pop().unwrap();
        let (mut r, w) = stream.into_split();
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
                    framed.write(JoinCluster::Request(act.addr.clone().to_string()));
                    act.framed = Some(framed);

                    // read side of the connection
                    ctx.add_stream(FramedRead::new(r, ConnectCodec::new()));

                },
                Err(err) => error!("{}", err),
            })
            .wait(ctx);
    }
}

impl StreamHandler<Result<JoinCluster, Error>> for NetworkInterface {
    fn handle(&mut self, item: Result<JoinCluster, Error>, _ctx: &mut Context<Self>) {
        match item {
            Ok(msg) => match msg {
                JoinCluster::Request(addr) => debug!("Request from {}", addr),
                JoinCluster::Response(addr) => debug!("Response from {}", addr),
                _ => debug!("Other stuff"),
            },
            Err(err) => error!("{}", err)
        }
    }
}

impl Handler<LocalMessage> for NetworkInterface {
    type Result = ();

    fn handle(&mut self, _msg: LocalMessage, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Received LocalMessage");
    }
}

impl WriteHandler<Error> for NetworkInterface {}
impl Supervised for NetworkInterface {}
