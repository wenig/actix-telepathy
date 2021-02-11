use log::*;
use actix_rt;
use crate::{ClusterListener, ClusterLog, NetworkInterface, Gossip, NodeResolving, AddrResolver, AddrRequest};
use port_scanner::{local_port_available, request_open_port};
use actix::prelude::*;
use actix_broker::BrokerSubscribe;
use tokio::time::{delay_for, Duration};
use std::sync::{Arc, Mutex};
use actix_telepathy_derive::{RemoteActor, RemoteMessage};
use log::*;
use actix::prelude::*;
use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};
use crate::remote::{RemoteWrapper, RemoteMessage, RemoteActor};
use crate::{RemoteAddr, Cluster, CustomSystemService, GossipResponse};
use crate::{DefaultSerialization, CustomSerialization};
use std::net::SocketAddr;
use std::fmt::Debug;
use std::str::FromStr;

#[derive(Message, Serialize, Deserialize, Debug, RemoteMessage)]
#[rtype(result = "()")]
struct TestMessage {}


#[derive(RemoteActor)]
#[remote_messages(TestMessage)]
struct TestActor {}

impl Actor for TestActor {
    type Context = Context<Self>;
}


#[actix_rt::test]
async fn addr_resolver_registers_and_resolves_addr() {
    let ta = TestActor {}.start();
    AddrResolver::from_registry().do_send(AddrRequest::Register(ta.recipient(), "testActor".to_string()));

}