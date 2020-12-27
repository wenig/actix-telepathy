use actix::prelude::*;
use tch::Tensor;

#[derive(Message)]
#[rtype(result = "()")]
pub struct BeaverTriple {
    pub a: Tensor,
    pub b: Tensor,
    pub c: Tensor
}
