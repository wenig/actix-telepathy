#[cfg(feature = "derive")]
pub use actix_telepathy_derive::*;

mod network;
mod codec;
mod cluster;
mod remote;
mod serialization;

pub use crate::cluster::*;
pub use crate::remote::*;
pub use crate::serialization::*;
pub use crate::network::*;


pub mod prelude {
    #[cfg(feature = "derive")]
    pub use actix_telepathy_derive::*;

    pub use crate::cluster::{Cluster, ClusterListener, ClusterLog, AddrApi, NodeResolving};
    pub use crate::remote::{RemoteMessage, RemoteAddr, RemoteWrapper};
    pub use crate::serialization::{DefaultSerialization, CustomSerialization};
    pub use crate::network::NetworkInterface;
}