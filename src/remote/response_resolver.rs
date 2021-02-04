use actix::prelude::*;
use std::collections::HashMap;
use crate::RemoteWrapper;
use futures::executor::block_on;
use uuid::Uuid;


type RemoteWrapperQueue = deadqueue::limited::Queue<RemoteWrapper>;

#[derive(Message)]
#[rtype(result = "Result<RemoteWrapper, ()>")]
pub struct RegisterResponse {
    pub id: Uuid
}

pub struct ResponseResolver {
    responses: HashMap<Uuid, RemoteWrapperQueue>
}

impl Actor for ResponseResolver {
    type Context = Context<ResponseResolver>;
}


impl Handler<RegisterResponse> for ResponseResolver {
    type Result = Result<RemoteWrapper, ()>;

    fn handle(&mut self, msg: RegisterResponse, _ctx: &mut Self::Context) -> Self::Result {
        self.responses.insert(msg.id.clone(), RemoteWrapperQueue::new(1));
        let q = self.responses.get(&msg.id).unwrap();

        Ok(block_on(q.pop()))
    }
}

impl Default for ResponseResolver {
    fn default() -> Self {
        Self {
            responses: HashMap::new()
        }
    }
}


impl Supervised for ResponseResolver {}
impl SystemService for ResponseResolver {}


