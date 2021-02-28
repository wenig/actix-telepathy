use actix_rt;
use crate::CustomSystemService;
use actix::{Supervised, Actor, SystemService, Message, Handler, Context};
use futures::{FutureExt};


#[derive(Message)]
#[rtype(result = "Result<usize, ()>")]
struct TestMessage {}


struct TestService {
    value: usize
}

impl Actor for TestService {
    type Context = Context<Self>;
}

impl Default for TestService {
    fn default() -> Self {
        unimplemented!()
    }
}

impl Supervised for TestService {}
impl SystemService for TestService {}
impl CustomSystemService for TestService {}

impl Handler<TestMessage> for TestService {
    type Result = Result<usize, ()>;

    fn handle(&mut self, _msg: TestMessage, _ctx: &mut Context<Self>) -> Self::Result {
        Ok(self.value)
    }
}


#[actix_rt::test]
async fn system_service_created_and_retrieved() {
    let value: usize = 7;
    let _test_service = TestService::start_service_with(move || {
        TestService { value: value.clone() }
    });

    let test_service = TestService::from_custom_registry();
    test_service.send(TestMessage {})
        .map(|res| {
            match res {
                Ok(result) => match result {
                    Ok(v) => assert_eq!(v, value),
                    Err(_) => panic!("Received error")
                },
                Err(_) => panic!("Received MailboxError")
            }
        }).await;
}

#[actix_rt::test]
#[should_panic]
async fn panics_when_not_yet_started() {
    TestService::from_custom_registry();
}