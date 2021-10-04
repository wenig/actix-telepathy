use actix::prelude::*;
use actix_rt::time::sleep;
use serde::{Serialize, Deserialize};
use tokio::time::Duration;
use crate::prelude::*;


#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype(Result = "()")]
struct MyRemoteMessage<T: Serialize + Send> {
    value: T
}


type MyFloatMessage = MyRemoteMessage<f32>;


#[derive(RemoteActor)]
#[remote_messages(MyFloatMessage)]
struct MyRemoteActor {}


impl Actor for MyRemoteActor {
    type Context = Context<Self>;
}


impl Handler<MyFloatMessage> for MyRemoteActor {
    type Result = ();

    fn handle(&mut self, msg: MyFloatMessage, _ctx: &mut Self::Context) -> Self::Result {
        let _r = msg.value + 1.0;
    }
}


#[actix_rt::test]
async fn generic_remote_messages() {
    let addr = MyRemoteActor {}.start();
    addr.do_send(MyRemoteMessage { value: 4.2 });
}
