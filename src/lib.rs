#[cfg(feature = "derive")]
pub use actix_telepathy_derive::*;

mod network;
mod codec;
mod cluster;
mod remote;
mod serialization;
mod utils;

pub use crate::cluster::*;
pub use crate::remote::*;
pub use crate::serialization::*;
pub use crate::network::*;
pub use crate::codec::ClusterMessage;
pub use crate::utils::*;


pub mod prelude {
    #[cfg(feature = "derive")]
    pub use actix_telepathy_derive::*;

    pub use crate::cluster::{Cluster, ClusterListener, ClusterLog, NodeResolving};
    pub use crate::remote::{RemoteMessage, RemoteAddr, RemoteWrapper, AnyAddr, RemoteActor};
    pub use crate::serialization::{DefaultSerialization, CustomSerialization};
    pub use crate::network::NetworkInterface;
}