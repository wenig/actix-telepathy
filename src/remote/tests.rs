use actix::prelude::*;
use actix_rt::time::sleep;
use serde::{Serialize, Deserialize};
use tokio::time::Duration;
use crate::prelude::*;


#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype(Result = "()")]
struct MyRemoteMessage<T> {
    value: T
}


#[derive(RemoteActor)]
#[remote_messages(MyRemoteMessage<f32>)]
struct MyRemoteActor {}


impl Actor for MyRemoteActor {
    type Context = Context<Self>;
}


impl Handler<MyRemoteMessage<f32>> for MyRemoteActor {
    type Result = ();

    fn handle(&mut self, msg: MyRemoteMessage<f32>, _ctx: &mut Self::Context) -> Self::Result {
        let _r = msg.value + 1.0;
    }
}


#[actix_rt::test]
fn generic_remote_messages() {
    let addr = MyRemoteActor {}.start();
    addr.do_send(MyRemoteMessage { value: 4.2 });
    sleep(Duration::from_millis(200));
}
