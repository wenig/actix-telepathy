#[cfg(feature = "derive")]
pub use actix_telepathy_derive::*;
#[cfg(feature = "protocols")]
pub mod protocols;

mod cluster;
mod codec;
mod network;
mod remote;
mod serialization;
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
    pub use crate::serialization::{CustomSerialization, DefaultSerialization};
}
