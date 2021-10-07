use std::collections::HashMap;
use actix::prelude::*;
use crate::prelude::*;
use crate::protocols::cluster_nodes::ClusterNodes;
use reduce::ReduceMessageF32;

pub mod reduce;
pub mod cluster_nodes;
mod helpers;

/// # ProtocolsReceiver<V>
///
/// A registry actor that is resposible for receiving and collecting messages. Node organistion is
/// handled by the `ClusterNodes` struct.
///
/// ## Template for new protocols
///
/// ```rust
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
#[remote_messages(ReduceMessageF32)]
pub struct ProtocolsReceiver<V> {
    message_buffer: HashMap<String, ProtocolBuffer<V>>,
    cluster_nodes: ClusterNodes,
    recipient: Option<Recipient<ProtocolFinished<V>>>
}

impl<V> ProtocolsReceiver<V> {
    pub fn add_to_buffer(&mut self, protocol_id: String, value: V, protocol_operation: ProtocolOperation) {
        match self.message_buffer.get_mut(&protocol_id) {
            None => {
                let mut protocol_buffer = ProtocolBuffer::new(protocol_operation);
                protocol_buffer.push(value);
                self.message_buffer.insert(protocol_id, protocol_buffer);
            },
            Some(buffer) => buffer.push(value)
        }
    }

    pub fn try_finish_protocol(&mut self, protocol_id: String) -> Option<Vec<V>> {
        let protocol_buffer = self.message_buffer.get(&protocol_id)
            .expect(&format!("Unknown protocol_id '{}' received", protocol_id));

        if protocol_buffer.all_received(self.cluster_nodes.len_incl_own()) {
            Some(self.message_buffer.remove(&protocol_id).unwrap().value_buffer)
        } else {
            None
        }
    }
}

impl<V> Actor for ProtocolsReceiver<V> {
    type Context = Context<Self>;
}

impl<V> Supervised for ProtocolsReceiver<V> {}

impl<V> Default for ProtocolsReceiver<V> {
    fn default() -> Self {
        Self {
            message_buffer: Default::default(),
            cluster_nodes: Default::default(),
            recipient: None
        }
    }
}

impl<V> SystemService for ProtocolsReceiver<V> {}


enum ProtocolOperation {
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
