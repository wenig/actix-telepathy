use actix::prelude::*;
use log::*;
use futures::executor::block_on;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::str::FromStr;
use actix::prelude::fut::IntoActorFuture;
use tokio::net::TcpStream;
use std::io::Error;

use crate::messages::{RemoteMessage, LocalMessage, TcpConnect};


pub struct NetworkInterface {
    pub addr: SocketAddr,
}


impl Actor for NetworkInterface {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        println!("NetworkInterface started!");
        //let stream = block_on(TcpStream::connect(self.addr.clone())).unwrap();
        //Self::add_stream(stream, _ctx);
    }
}


impl NetworkInterface {
    pub fn new(addr: String) -> NetworkInterface {
        let sock_addr = SocketAddr::from_str(&addr).unwrap();
        NetworkInterface {addr: sock_addr}
    }
}

impl StreamHandler<RemoteMessage> for NetworkInterface {
    fn handle(&mut self, _item: RemoteMessage, _ctx: &mut Context<Self>) {
        println!("Received RemoteMessage");
    }
}

impl Handler<LocalMessage> for NetworkInterface {
    type Result = ();

    fn handle(&mut self, _msg: LocalMessage, _ctx: &mut Context<Self>) -> Self::Result {
        println!("Received LocalMessage");
    }
}

impl Supervised for NetworkInterface {}
