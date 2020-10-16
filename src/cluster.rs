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
use std::net::SocketAddr;


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
            let cloned_node_addr = SocketAddr::from_str(&node_addr).unwrap();
            //let node = Supervisor::start(move |_| NetworkInterface::new(cloned_node_addr));
            let node = NetworkInterface::new(cloned_node_addr).start();
            self.nodes.push(node);
        }
    }
}

impl Handler<TcpConnect> for Cluster {
    type Result = ();

    fn handle(&mut self, msg: TcpConnect, _ctx: &mut Self::Context) -> Self::Result {
        debug!("Incoming TcpConnect");
        let stream = msg.0;
        let addr = msg.1;
        //let node = Supervisor::start(move |_| NetworkInterface::from_stream(addr, stream));
        let node = NetworkInterface::from_stream(addr, stream).start();
        self.nodes.push(node);
    }
}

impl Cluster {
    pub fn new<S: Into<String>>(ip_address: String, seed_nodes: Option<S>) -> Addr<Cluster> {
        let listener = Cluster::bind(ip_address.clone()).unwrap();

        debug!("Listening on {}", ip_address);
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

    fn bind(addr: String) -> Result<Box<TcpListener>> {
        let addr = net::SocketAddr::from_str(&addr).unwrap();
        let listener = Box::new(block_on(TcpListener::bind(&addr)).unwrap());
        Ok(listener)
    }
}


