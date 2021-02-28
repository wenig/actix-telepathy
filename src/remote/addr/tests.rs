use actix::prelude::*;
use serde::{Serialize, Deserialize};
use crate::prelude::*;
use actix_telepathy_derive::{RemoteActor, RemoteMessage};
use crate::{AddrResolver, AddrRequest, AddrResponse};
use tokio::time::delay_for;
use std::time::Duration;
use std::sync::{Arc, Mutex};


#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype(result = "()")]
struct TestMessage {}


#[derive(RemoteActor)]
#[remote_messages(TestMessage)]
struct TestActor {
    identifiers: Arc<Mutex<Vec<String>>>
}


impl Actor for TestActor {
    type Context = Context<Self>;
}

impl Handler<TestMessage> for TestActor {
    type Result = ();

    fn handle(&mut self, _msg: TestMessage, ctx: &mut Context<Self>) -> Self::Result {
        AddrResolver::from_registry().send(AddrRequest::ResolveRec(ctx.address().recipient()))
            .into_actor(self)
            .map(|res, act, _ctx| match res {
                Ok(res) => match res {
                    Ok(addr_res) => {
                        match addr_res {
                            AddrResponse::ResolveRec(identifer) => act.identifiers.lock().unwrap().push(identifer),
                            _ => panic!("Wrong Response returned!")
                        }
                    },
                    Err(_) => panic!("Couldn't resolve Addr!")
                },
                Err(_) => panic!("Couldn't resolve Addr!")
            }).wait(ctx)
    }
}


#[actix_rt::test]
async fn addr_resolver_registers_and_resolves_addr() {
    let identifier = "testActor".to_string();
    let identifiers = Arc::new(Mutex::new(vec![]));
    let ta = TestActor {identifiers: identifiers.clone()}.start();
    AddrResolver::from_registry().do_send(AddrRequest::Register(ta.clone().recipient(), identifier.clone()));
    ta.do_send(TestMessage {});
    delay_for(Duration::from_secs(1)).await;
    assert_eq!((*(identifiers.lock().unwrap())).get(0).unwrap(), &identifier);
}
