use actix::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct PoisonPill {}
