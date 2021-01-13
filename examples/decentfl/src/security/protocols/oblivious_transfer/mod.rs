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
    use actix::{Actor, Handler, Context, System};
    use actix_telepathy::{RemoteAddr, NetworkInterface, ClusterMessage, AnyAddr};
    use std::net::SocketAddr;
    use core::str::FromStr;
    use tch::Tensor;
    use actix::actors::mocker::Mocker;
    use crate::security::protocols::oblivious_transfer::messages::{OTMessage1Request, OTStart};
    use crate::security::protocols::mascot::Mascot;
    use glass_pumpkin::prime;
    use rand::rngs::OsRng;
    use rand::prelude::ThreadRng;
    use std::thread::sleep;
    use actix::clock::Duration;
    use log::*;
    use std::ops::Deref;
    use std::any::{Any, TypeId};
    use std::io::Error;

    type MockOTSender = Mocker<ObliviousTransferSender>;
    type MockMascot = Mocker<Mascot>;
    type MockNode = Mocker<RemoteAddr>;

    #[test]
    fn oblivious_transfer_sends_correct_result() {
        env_logger::init();

        _oblivious_transfer_sends_correct_result()
    }

    #[actix_rt::main]
    async fn _oblivious_transfer_sends_correct_result() {
        let mocker_parent = MockMascot::mock(Box::new(move |_msg, _ctx| {
            debug!("Mascot received something!");
            Box::new(Some(true))
        }));

        let localhost = mocker_node.start();

        let parent = mocker_parent.start();

        let rng = ThreadRng::default();

        let receiver = ObliviousTransferReceiver::new(
            parent.recipient(),
            1,
            1 as u8
        ).start();

        let sender = ObliviousTransferSender::new(
            parent.clone().recipient(),
            128 as u64,
            100000 as u64,
            3,
            AnyAddr::Local(receiver),
            Tensor::of_slice(&[1,2]).view_(&[1, 2])
        ).start();

        sender.do_send(OTStart {});

        tokio::signal::ctrl_c().await.unwrap();
        println!("Ctrl-C received, shutting down");
        System::current().stop();
        assert!(true);
    }
}
