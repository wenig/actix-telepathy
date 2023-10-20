//! Actix-Telepathy is an extension to [Actix](https://docs.rs/actix) that enables remote messaging and clustering support.
//!
//! Telepathy does not change Actix' messaging system but _extends_ the
//!
//! - [actix::Actor](https://docs.rs/actix/latest/actix/trait.Actor.html) with the [RemoteActor](./trait.RemoteActor.html) trait and the
//! - [actix::Message](https://docs.rs/actix/latest/actix/trait.Message.html) with the [RemoteMessage](./trait.RemoteMessage.html) trait.
//!
//! Hence, an example actor receiving a remote message is defined as follows.
//! To connect multiple computers in a cluster, a [Cluster](./struct.Cluster.html) must be generated.
//!
//! ```rust
//! use actix::prelude::*;
//! use actix_broker::BrokerSubscribe;
//! use actix_telepathy::prelude::*;  // <-- Telepathy extension
//! use serde::{Serialize, Deserialize};
//! use std::net::SocketAddr;
//!
//! #[derive(RemoteMessage, Serialize, Deserialize)]  // <-- Telepathy extension
//! struct MyMessage {}
//!
//! #[derive(RemoteActor)]  // <-- Telepathy extension
//! #[remote_messages(MyMessage)]  // <-- Telepathy extension
//! struct MyActor {
//!     state: usize
//! }
//!
//! impl Actor for MyActor {
//!     type Context = Context<Self>;
//!
//!     fn started(&mut self, ctx: &mut Self::Context) {
//!         self.register(ctx.address().recipient());  // <-- Telepathy extension
//!     }
//! }
//!
//! impl Handler<MyMessage> for MyActor {
//!     type Result = ();
//!
//!     fn handle(&mut self, msg: MyMessage, ctx: &mut Self::Context) -> Self::Result {
//!         todo!()
//!     }
//! }
//!
//! #[actix_rt::main]
//! pub async fn start_cluster(own_addr: SocketAddr, seed_nodes: Vec<SocketAddr>) {
//!     let _addr = MyActor { state: 0 }.start();
//!     let _cluster = Cluster::new(own_addr, seed_nodes);
//!     tokio::time::sleep(std::time::Duration::from_secs(5)).await;
//! }
//!
//! ```
//!
//! The previous example will not do anything. However, the cluster will try to connect to the given addresses in `seed_nodes`.
//! To react to new joining members, a [ClusterListener](./trait.ClusterListener.html) actor should be used:
//!
//! ```rust
//! use actix::prelude::*;
//! use actix_broker::BrokerSubscribe;
//! use actix_telepathy::prelude::*;  // <-- Telepathy extension
//! use serde::{Serialize, Deserialize};
//! use std::net::SocketAddr;
//!
//! #[derive(RemoteMessage, Serialize, Deserialize)]  // <-- Telepathy extension
//! struct MyMessage {}
//!
//! #[derive(RemoteActor)]  // <-- Telepathy extension
//! #[remote_messages(MyMessage)]  // <-- Telepathy extension
//! struct MyActor {
//!     state: usize
//! }
//!
//! impl Actor for MyActor {
//!     type Context = Context<Self>;
//!
//!     fn started(&mut self, ctx: &mut Self::Context) {
//!         self.register(ctx.address().recipient());  // <-- Telepathy extension
//!         self.subscribe_system_async::<ClusterLog>(ctx);  // <-- Telepathy extension
//!     }
//! }
//!
//! impl Handler<MyMessage> for MyActor {
//!     type Result = ();
//!
//!     fn handle(&mut self, msg: MyMessage, ctx: &mut Self::Context) -> Self::Result {
//!         todo!()
//!     }
//! }
//!
//! impl Handler<ClusterLog> for MyActor {  // <-- Telepathy extension
//!     type Result = ();
//!
//!     fn handle(&mut self, msg: ClusterLog, ctx: &mut Self::Context) -> Self::Result {
//!         match msg {
//!             ClusterLog::NewMember(_node) => {
//!                 println!("New member joined the cluster.")
//!             },
//!             ClusterLog::MemberLeft(_ip_addr) => {
//!                 println!("Member left the cluster.")
//!             }
//!         }
//!     }
//! }
//! impl ClusterListener for MyActor {}  // <-- Telepathy extension
//!
//! #[actix_rt::main]
//! pub async fn start_cluster(own_addr: SocketAddr, seed_nodes: Vec<SocketAddr>) {
//!     let _addr = MyActor { state: 0 }.start();
//!     let _cluster = Cluster::new(own_addr, seed_nodes);  // <-- Telepathy extension
//!     tokio::time::sleep(std::time::Duration::from_secs(5)).await;
//! }
//!
//! ```
//!
//! Now, we receive a printed message whenever a new member joins the cluster or when a member leaves.
//! To send messages between remote actors to other members in the cluster, we have to utilize the
//! [RemoteAddr](./struct.RemoteAddr.html) that the [ClusterListener](./trait.ClusterListener.html) receives.
//!
//! ```rust
//! use actix::prelude::*;
//! use actix_broker::BrokerSubscribe;
//! use actix_telepathy::prelude::*;  // <-- Telepathy extension
//! use serde::{Serialize, Deserialize};
//! use std::net::SocketAddr;
//!
//! #[derive(RemoteMessage, Serialize, Deserialize)]  // <-- Telepathy extension
//! struct MyMessage {}
//!
//! #[derive(RemoteActor)]  // <-- Telepathy extension
//! #[remote_messages(MyMessage)]  // <-- Telepathy extension
//! struct MyActor {
//!     state: usize
//! }
//!
//! impl Actor for MyActor {
//!     type Context = Context<Self>;
//!
//!     fn started(&mut self, ctx: &mut Self::Context) {
//!         self.register(ctx.address().recipient());  // <-- Telepathy extension
//!         self.subscribe_system_async::<ClusterLog>(ctx);  // <-- Telepathy extension
//!     }
//! }
//!
//! impl Handler<MyMessage> for MyActor {
//!     type Result = ();
//!
//!     fn handle(&mut self, msg: MyMessage, ctx: &mut Self::Context) -> Self::Result {
//!         println!("RemoteMessage received!")
//!     }
//! }
//!
//! impl Handler<ClusterLog> for MyActor {  // <-- Telepathy extension
//!     type Result = ();
//!
//!     fn handle(&mut self, msg: ClusterLog, ctx: &mut Self::Context) -> Self::Result {
//!         match msg {
//!             ClusterLog::NewMember(node) => {
//!                 println!("New member joined the cluster.");
//!                 let remote_addr = node.get_remote_addr(Self::ACTOR_ID.to_string());
//!                 remote_addr.do_send(MyMessage {})
//!             },
//!             ClusterLog::MemberLeft(_ip_addr) => {
//!                 println!("Member left the cluster.")
//!             }
//!         }
//!     }
//! }
//! impl ClusterListener for MyActor {}  // <-- Telepathy extension
//!
//! #[actix_rt::main]
//! pub async fn start_cluster(own_addr: SocketAddr, seed_nodes: Vec<SocketAddr>) {
//!     let _addr = MyActor { state: 0 }.start();
//!     let _cluster = Cluster::new(own_addr, seed_nodes);  // <-- Telepathy extension
//!     tokio::time::sleep(std::time::Duration::from_secs(5)).await;
//! }
//!
//! ```
//!
//! Now, every new member receives a `MyMessage` from every [ClusterListener](./trait.ClusterListener.html) in the cluster.
//!
//! Before we could use the [RemoteAddr](./struct.RemoteAddr.html), we had to make sure, that it is pointing to the correct [RemoteActor](./trait.RemoteActor.html), which is `MyActor` in that case.
//! Therefore, we had to call `change_id` on the [RemoteAddr](./struct.RemoteAddr.html). A [RemoteAddr](./struct.RemoteAddr.html) points to a specific actor on a remote machine.
//! Per default, this is the [NetworkInterface](./struct.NetworkInterface.html) actor.

#[cfg(feature = "derive")]
pub use actix_telepathy_derive::*;
#[cfg(feature = "protocols")]
pub mod protocols;

mod cluster;
mod codec;
mod network;
mod remote;
mod serialization;
#[cfg(test)]
pub(crate) mod test_utils;
mod utils;

pub use crate::cluster::*;
pub use crate::codec::ClusterMessage;
pub use crate::network::*;
pub use crate::remote::*;
pub use crate::serialization::*;
pub use crate::utils::*;

pub mod prelude {
    #[cfg(feature = "derive")]
    pub use actix_telepathy_derive::*;

    pub use crate::cluster::{Cluster, ClusterListener, ClusterLog, NodeResolving};
    pub use crate::network::NetworkInterface;
    pub use crate::remote::{AnyAddr, RemoteActor, RemoteAddr, RemoteMessage, RemoteWrapper};
    pub use crate::serialization::{
        CustomSerialization, CustomSerializationError, DefaultSerialization,
    };
}
