use std::{collections::HashMap, any::Any};

use actix::{Supervised, SystemService, Actor, Context, Message, Handler};
use log::debug;
use tokio::sync::oneshot::Sender;
use uuid::Uuid;

use crate::RemoteWrapper;

#[derive(Message)]
#[rtype("()")]
pub struct ResponseSubscribe(pub Uuid, pub Sender<Box<dyn Any + Send>>);

#[derive(Default)]
pub struct ResponseDispatcher {
    subscribed_responses: HashMap<Uuid, Sender<Box<dyn Any + Send + 'static>>>
}

impl Supervised for ResponseDispatcher {}
impl SystemService for ResponseDispatcher {}

impl Actor for ResponseDispatcher {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        debug!("AddressResolver actor started");
    }
}

impl Handler<ResponseSubscribe> for ResponseDispatcher {
    type Result = ();

    fn handle(&mut self, msg: ResponseSubscribe, _ctx: &mut Self::Context) -> Self::Result {
        self.subscribed_responses.insert(msg.0, msg.1);
    }
}

impl Handler<RemoteWrapper> for ResponseDispatcher {
    type Result = ();

    fn handle(&mut self, msg: RemoteWrapper, ctx: &mut Self::Context) -> Self::Result {
        let sender = self.subscribed_responses.remove(
            &msg.conversation_id.as_ref().clone().expect("Conversation ID must be set by now.")).unwrap();
        sender.send(Box::new(msg.message_buffer)).unwrap();
        //let mut deserialized_msg = ::generate_serializer().deserialize(&(msg.message_buffer)[..]).expect("Cannot deserialized #name message");
    }
}

