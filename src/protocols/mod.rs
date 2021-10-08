use std::collections::HashMap;
use actix::prelude::*;
use ndarray::Array1;
use crate::prelude::*;
use crate::protocols::cluster_nodes::ClusterNodes;
use reduce::{ReduceMessage};
use crate::CustomSystemService;
use log::*;

pub mod reduce;
pub mod cluster_nodes;
mod helpers;


pub type ProtocolDataType = Array1<f32>;

/// # ProtocolsReceiver<V>
///
/// A registry actor that is responsible for receiving and collecting messages. Node organistion is
/// handled by the `ClusterNodes` struct.
///
/// ## Template for new protocols
///
/// ```rust
/// // V: Sum + Product + Send + Unpin + 'static + Sized + Serialize
/// pub trait ProtocolName {
///     fn protocol_name<V: Add + Mul + Div>(&mut self, value: V, receiver_node_id: usize, /* ... */);
///     /* ... */
/// }
///
/// impl ProtocolName for ClusterNodes {
///     fn protocol_name<V: Add + Mul + Div>(&mut self, value: V, receiver_node_id: usize) {
///         let receiver = self.get_protocols_receiver_addr(receiver_node_id);
///         receiver.do_send(ProtocolNameMessage {
///             value,
///             /* ... */
///         })
///     }
/// }
///
/// #[derive(RemoteMessage, Serialize, Deserialize)]
/// pub struct ProtocolNameMessage<V: Add + Mul + Div> {
///     pub value: V,
///     pub protocol_id: String,
///     /* ... */
/// }
///
/// ```
#[derive(RemoteActor)]
#[remote_messages(ReduceMessage)]
pub struct ProtocolsReceiver {
    message_buffer: HashMap<String, ProtocolBuffer<ProtocolDataType>>,
    cluster_nodes: ClusterNodes,
    recipient: Option<Recipient<ProtocolFinished<ProtocolDataType>>>
}

impl ProtocolsReceiver {
    pub fn new(recipient: Recipient<ProtocolFinished<ProtocolDataType>>, cluster_nodes: ClusterNodes) -> Self {
        Self {
            recipient: Some(recipient),
            cluster_nodes,
            ..Default::default()
        }
    }

    pub fn add_to_buffer(&mut self, protocol_id: String, value: ProtocolDataType, protocol_operation: ProtocolOperation) {
        match self.message_buffer.get_mut(&protocol_id) {
            None => {
                let mut protocol_buffer = ProtocolBuffer::new(protocol_operation);
                protocol_buffer.push(value);
                self.message_buffer.insert(protocol_id, protocol_buffer);
            },
            Some(buffer) => {
                buffer.push(value);
            }
        }
    }

    pub fn try_finish_protocol(&mut self, protocol_id: String) -> Option<Vec<ProtocolDataType>> {
        let protocol_buffer = self.message_buffer.get(&protocol_id)
            .expect(&format!("Unknown protocol_id '{}' received", protocol_id));

        if protocol_buffer.all_received(self.cluster_nodes.len_incl_own()) {
            Some(self.message_buffer.remove(&protocol_id).unwrap().value_buffer)
        } else {
            None
        }
    }
}

impl Actor for ProtocolsReceiver {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.register(ctx.address().recipient());
    }
}

impl Supervised for ProtocolsReceiver {}

impl Default for ProtocolsReceiver {
    fn default() -> Self {
        Self {
            message_buffer: Default::default(),
            cluster_nodes: Default::default(),
            recipient: None
        }
    }
}

impl SystemService for ProtocolsReceiver {}
impl CustomSystemService for ProtocolsReceiver {
    fn custom_service_started(&mut self, _ctx: &mut Context<Self>) {
        debug!("ProtocolReceiver started");
    }
}


pub enum ProtocolOperation {
    Reduce,
    AllReduce,
    Collect,
    AllCollect
}

struct ProtocolBuffer<V> {
    pub value_buffer: Vec<V>,
    protocol_operation: ProtocolOperation
}

impl<V> ProtocolBuffer<V> {
    pub fn new(protocol_operation: ProtocolOperation) -> Self {
        Self {
            value_buffer: vec![],
            protocol_operation
        }
    }

    pub fn push(&mut self, value: V) {
        self.value_buffer.push(value)
    }

    pub fn all_received(&self, n: usize) -> bool {
        self.value_buffer.len().eq(&n)
    }
}

#[derive(Message)]
#[rtype(Result = "()")]
pub struct ProtocolFinished<V> {
    pub result: Option<V>,
    pub results: Vec<V>,
    pub protocol_id: String
}
