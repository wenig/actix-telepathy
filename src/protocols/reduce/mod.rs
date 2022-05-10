mod listener;
#[cfg(test)]
mod tests;

use crate::prelude::*;
use actix::prelude::*;
use ndarray::Array1;
use serde::{Deserialize, Serialize};

use crate::protocols::cluster_nodes::ClusterNodes;
use crate::protocols::helpers::ProtocolsHelper;
use crate::protocols::{ProtocolDataType, ProtocolFinished, ProtocolOperation, ProtocolsReceiver};

pub trait Reduce {
    fn reduce(
        &self,
        value: ProtocolDataType,
        reduce_operation: ReduceOperation,
        receiver_node_id: usize,
        protocol_id: &str,
    );
    fn reduce_to_main(
        &self,
        value: ProtocolDataType,
        reduce_operation: ReduceOperation,
        protocol_id: &str,
    );
}

impl Reduce for ClusterNodes {
    fn reduce(
        &self,
        value: ProtocolDataType,
        reduce_operation: ReduceOperation,
        receiver_node_id: usize,
        protocol_id: &str,
    ) {
        let receiver = self.get_protocols_receiver_addr(receiver_node_id);
        receiver.do_send(ReduceMessage {
            reduce_operation,
            value,
            protocol_id: protocol_id.to_string(),
        })
    }

    fn reduce_to_main(
        &self,
        value: ProtocolDataType,
        reduce_operation: ReduceOperation,
        protocol_id: &str,
    ) {
        self.reduce(value, reduce_operation, 0, protocol_id)
    }
}

#[derive(Serialize, Deserialize)]
pub enum ReduceOperation {
    Sum,
    Product,
}

impl ReduceOperation {
    pub fn apply(&self, values: Vec<ProtocolDataType>) -> ProtocolDataType {
        let shape = values
            .first()
            .expect("Should at least have one element")
            .shape();
        match self {
            ReduceOperation::Sum => {
                let zero = Array1::zeros(shape[0]);
                values.into_iter().fold(zero, |a, b| a + b)
            }
            ReduceOperation::Product => {
                let one = Array1::ones(shape[0]);
                values.into_iter().fold(one, |a, b| a * b)
            }
        }
    }
}

#[derive(RemoteMessage, Serialize, Deserialize)]
pub struct ReduceMessage {
    pub reduce_operation: ReduceOperation,
    pub value: ProtocolDataType,
    pub protocol_id: String,
}

impl Handler<ReduceMessage> for ProtocolsReceiver {
    type Result = ();

    fn handle(&mut self, msg: ReduceMessage, _ctx: &mut Self::Context) -> Self::Result {
        self.add_to_buffer(
            msg.protocol_id.clone(),
            msg.value,
            ProtocolOperation::Reduce,
        );
        match self.try_finish_protocol(msg.protocol_id.clone()) {
            None => (),
            Some(values) => match &self.recipient {
                Some(recipient) => {
                    recipient
                        .do_send(ProtocolFinished {
                            result: Some(msg.reduce_operation.apply(values)),
                            results: vec![],
                            protocol_id: msg.protocol_id,
                        })
                        .unwrap();
                }
                None => (),
            },
        }
    }
}
