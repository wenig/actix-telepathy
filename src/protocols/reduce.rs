use std::ops::{Add, Mul, Div};
use actix::prelude::*;
use crate::prelude::*;
use serde::{Serialize, Deserialize};

use crate::protocols::cluster_nodes::ClusterNodes;
use crate::protocols::helpers::ProtocolsHelper;
use crate::protocols::{ProtocolFinished, ProtocolOperation, ProtocolsReceiver};

pub trait Reduce {
    fn reduce<V: Add + Mul + Div>(&mut self, value: V, reduce_operation: ReduceOperation, receiver_node_id: usize, protocol_id: &str);
    fn reduce_to_main<V: Add + Mul + Div>(&mut self, value: V, reduce_operation: ReduceOperation, protocol_id: &str);
}

impl Reduce for ClusterNodes {
    fn reduce<V: Add + Mul + Div>(&self, value: V, reduce_operation: ReduceOperation, receiver_node_id: usize, protocol_id: &str) {
        let receiver = self.get_protocols_receiver_addr(receiver_node_id);
        receiver.do_send(ReduceMessage {
            reduce_operation,
            value,
            protocol_id: protocol_id.to_string()
        })
    }

    fn reduce_to_main<V: Add + Mul + Div>(&self, value: V, reduce_operation: ReduceOperation, protocol_id: &str){
        self.reduce(value, reduce_operation, 0, protocol_id)
    }
}

pub enum ReduceOperation {
    Sum,
    Product,
    Mean
}

impl ReduceOperation {
    pub fn apply<V: Add + Div + Mul>(&self, values: Vec<V>) -> V {
        match self {
            ReduceOperation::Sum => {
                values.into_iter().sum()
            },
            ReduceOperation::Product => {
                values.into_iter().product()
            },
            ReduceOperation::Mean => {
                let n = values.len() as V;
                values.into_iter().sum() / n
            }
        }
        .expect("Could not apply the reduce method.")
    }
}


#[derive(RemoteMessage, Serialize, Deserialize)]
pub struct ReduceMessage<V: Add + Mul + Div> {
    pub reduce_operation: ReduceOperation,
    pub value: V,
    pub protocol_id: String
}


impl<V: Add + Mul + Div> Handler<ReduceMessage<V>> for ProtocolsReceiver<V> {
    type Result = ();

    fn handle(&mut self, msg: ReduceMessage<V>, _ctx: &mut Self::Context) -> Self::Result {
        self.add_to_buffer(msg.protocol_id.clone(), msg.value, ProtocolOperation::Reduce);
        match self.try_finish_protocol(msg.protocol_id.clone()) {
            None => (),
            Some(values) => match &self.recipient {
                Some(recipient) => {
                    recipient.do_send(ProtocolFinished {
                        result: Some(msg.reduce_operation.apply(values)),
                        results: vec![],
                        protocol_id: msg.protocol_id
                    })
                },
                None => ()
            }
        }
    }
}

pub type ReduceMessageF32 = ReduceMessage<f32>;

#[cfg(test)]
mod tests {
    use std::ops::{Add, Div, Mul};
    use rayon::prelude::{IntoParallelIterator, ParallelIterator};

    #[test]
    #[ignore]
    fn reduce_correct_value() {
        let values = vec![3., 5.];
        values.into_par_iter().for_each(|v| run_single_reduction_worker(v));
    }

    fn run_single_reduction_worker<V: Add + Div + Mul>(value: V) {
        assert!(false);
    }
}
