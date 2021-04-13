use actix::prelude::*;
use serde::{Serialize, Deserialize};
use crate::prelude::*;
use actix_telepathy_derive::{RemoteActor, RemoteMessage};
use crate::{AddrResolver, AddrRequest, AddrResponse, AddrRepresentation};
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

#[test]
fn addr_representation_eq_not_key() {
    let own = AddrRepresentation::NetworkInterface;
    let other1 = AddrRepresentation::NetworkInterface;
    let other2 = AddrRepresentation::Gossip;
    let other3 = AddrRepresentation::Key("test".to_string());

    assert!(own.eq(&other1));
    assert!(own.ne(&other2));
    assert!(own.ne(&other3));
}

#[test]
fn addr_representation_eq_key() {
    let own = AddrRepresentation::Key("own".to_string());
    let other1 = AddrRepresentation::Key("own".to_string());
    let other2 = AddrRepresentation::Key("other2".to_string());

    assert!(own.eq(&other1));
    assert!(own.ne(&other2));
}
