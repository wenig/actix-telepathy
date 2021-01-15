mod receiver;
mod sender;
mod messages;
mod bigint_extensions;

pub use sender::ObliviousTransferSender;
pub use receiver::ObliviousTransferReceiver;
pub use messages::OTDone;


// ----------------------------
//            TESTS
// ----------------------------

#[cfg(test)]
mod tests {
    use crate::security::protocols::oblivious_transfer::{ObliviousTransferSender, OTDone, ObliviousTransferReceiver};
    use actix::{Actor, Handler, Context, System, Addr, SystemRunner};
    use actix_telepathy::{RemoteAddr, NetworkInterface, ClusterMessage, AnyAddr};
    use std::net::SocketAddr;
    use core::str::FromStr;
    use tch::{Tensor, IndexOp};
    use actix::actors::mocker::Mocker;
    use crate::security::protocols::oblivious_transfer::messages::{OTMessage1Request, OTStart};
    use crate::security::protocols::mascot::Mascot;
    use glass_pumpkin::prime;
    use rand::rngs::OsRng;
    use rand::prelude::ThreadRng;
    use log::*;
    use std::ops::Deref;
    use std::any::{Any, TypeId};
    use std::io::Error;
    use std::alloc::dealloc;
    use actix::prelude::dev::ToEnvelope;
    use std::panic;
    use std::time::Duration;
    use actix_rt::Runtime;
    use tokio::sync::oneshot::Receiver;
    use async_std::task::sleep;
    use std::sync::{Arc, Mutex};

    type MockOTSender = Mocker<ObliviousTransferSender>;
    type MockMascot = Mocker<Mascot>;
    type MockNode = Mocker<RemoteAddr>;

    pub struct SystemTester {
        exit_code: i32
    }

    impl SystemTester {
        pub fn exit(&mut self, exit_code: i32) {
            self.exit_code = exit_code;
        }
    }

    #[test]
    fn oblivious_transfer_sends_correct_result() {

        let system_tester = Arc::new(Mutex::new(SystemTester {exit_code: -1}));
        let cloned = Arc::clone(&system_tester);

        let mut system = actix_rt::System::new("test");
        system.block_on(_oblivious_transfer_sends_correct_result(cloned));
        let exit_code = (*system_tester.lock().unwrap()).exit_code;
        assert_eq!(exit_code, 0);
    }

    async fn _oblivious_transfer_sends_correct_result(cloned_system_tester: Arc<Mutex<SystemTester>>) {
        //env_logger::init();

        let mocker_sender_parent = MockMascot::mock(Box::new(move |msg, _ctx| {
            Box::new(())
        })).start();

        let mocker_receiver_parent = MockMascot::mock(Box::new(move |msg, _ctx| {
            let message: &OTDone = msg.downcast_ref().unwrap();

            let result = match message {
                OTDone::Sender => -1,
                OTDone::Receiver(tensor, _sender) => tensor.int64_value(&[0])
            };

            let mut system_tester = cloned_system_tester.lock().unwrap();

            if result == 2 {
                System::current().stop();
                (*system_tester).exit(0);
                debug!("System stopped");
            } else {
                System::current().stop();
                (*system_tester).exit(1);
                debug!("wrong result");
            }

            Box::new(())
        })).start();

        let rng = ThreadRng::default();

        let receiver = ObliviousTransferReceiver::new(
            mocker_receiver_parent.clone().recipient(),
            1,
            1 as u8
        ).start();

        let sender = ObliviousTransferSender::new(
            mocker_sender_parent.recipient(),
            128 as u64,
            100000 as u64,
            1,
            AnyAddr::Local(receiver),
            Tensor::of_slice(&[1,2]).view_(&[1, 2])
        ).start();

        sender.do_send(OTStart {});

        sleep(Duration::from_secs(1)).await;
    }
}
