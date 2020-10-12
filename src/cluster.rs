use actix::prelude::*;
use std::net;
use std::io::{Error, ErrorKind, Result};
use tokio::net::TcpListener;
use std::collections::{HashMap, VecDeque};
use log::*;

use crate::utils;
use crate::network::NetworkInterface;
use std::str::FromStr;
use futures::StreamExt;
use crate::messages::TcpConnect;
use futures::executor::block_on;


pub struct Cluster {
    ip_address: String,
    pub addrs: Vec<String>,
    nodes: Vec<Addr<NetworkInterface>>,
}

impl Actor for Cluster {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("Connecting with seed nodes");
        for node_addr in self.addrs.iter() {
            let cloned_node_addr = node_addr.clone();
            let node = Supervisor::start(move |_| NetworkInterface::new(cloned_node_addr));
            self.nodes.push(node);
        }
    }
}

impl Handler<TcpConnect> for Cluster {
    type Result = ();

    fn handle(&mut self, _msg: TcpConnect, _ctx: &mut Self::Context) -> Self::Result {
        println!("Incoming TcpConnect");
    }
}

impl Cluster {
    pub fn new<S: Into<String>>(ip_address: String, seed_nodes: Option<S>) -> Addr<Cluster> {
        let addr = net::SocketAddr::from_str(&ip_address).unwrap();
        let listener = Box::new(block_on(TcpListener::bind(&addr)).unwrap());

        println!("Listening on {}", ip_address);
        Cluster::create(move |ctx| {
            ctx.add_message_stream(Box::leak(listener).incoming().map(|st| {
                let st = st.unwrap();
                let addr = st.peer_addr().unwrap();
                TcpConnect(st, addr)
            }));

            let mut addrs: Vec<String> = Vec::new();
            seed_nodes.map(|node_addr|{
                let node_addr = node_addr.into();
                addrs.push(node_addr.clone());
            });

            Cluster {ip_address, addrs, nodes: vec![]}
        })
    }
}


