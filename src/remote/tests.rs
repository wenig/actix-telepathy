use crate::{prelude::*, AddrRepresentation, ClusterMessage, Node};
use actix::prelude::*;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

#[derive(Serialize, Deserialize, RemoteMessage)]
struct MyRemoteMessage<T: Serialize + Send> {
    value: T,
}

type MyFloatMessage = MyRemoteMessage<f32>;

#[derive(RemoteActor)]
#[remote_messages(MyFloatMessage)]
struct MyRemoteActor<T: Send + Unpin + 'static + Sized> {
    phantom: PhantomData<T>,
}

impl<T: Sized + Unpin + 'static + Send> Actor for MyRemoteActor<T> {
    type Context = Context<Self>;
}

impl<T: Send + Unpin + 'static> Handler<MyFloatMessage> for MyRemoteActor<T> {
    type Result = ();

    fn handle(&mut self, msg: MyFloatMessage, _ctx: &mut Self::Context) -> Self::Result {
        let _r = msg.value + 1.0;
    }
}

#[actix_rt::test]
async fn generic_remote_messages() {
    let addr: Addr<MyRemoteActor<f32>> = MyRemoteActor {
        phantom: PhantomData,
    }
    .start();
    addr.do_send(MyRemoteMessage { value: 4.2 });
}
