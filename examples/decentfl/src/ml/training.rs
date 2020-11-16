use log::*;
use actix::prelude::*;
use tch::vision::dataset::Dataset;
use crate::ml::model::{Net, FlattenModel};
use crate::ml::score_storage::ScoreStorage;
use tch::nn::{Sgd, Optimizer, OptimizerConfig, VarStore, ModuleT};
use tch::Device;
use crate::ml::protocols::{ModelAggregation, ModelMessage};
use actix_telepathy::{RemoteAddr, Cluster};


#[derive(Message)]
#[rtype("Result = ()")]
pub struct Epoch;

#[derive(Message)]
#[rtype("Result = ()")]
pub struct Test;

#[derive(Message)]
#[rtype("Result = ()")]
pub struct Addresses {
    socket_addr: String,
    server_addr: RemoteAddr,
    cluster: Addr<Cluster>
}

impl Addresses {
    pub fn new(socket_addr: String, server_addr: RemoteAddr, cluster: Addr<Cluster>) -> Self {
        Self {
            socket_addr,
            server_addr,
            cluster
        }
    }
}


pub struct Training {
    model: Net,
    dataset: Dataset,
    batch_size: usize,
    optimizer: Optimizer<Sgd>,
    #[allow(dead_code)]
    score_storage: ScoreStorage,
    current_epoch: usize,
    max_epochs: Option<usize>,
    test_every: usize,
    update_every: usize,
    own_addr: Option<Addr<Training>>,
    device: Device,
    aggregation_protocol: Option<Addr<ModelAggregation>>,
}

impl Training {
    pub fn new(model: Net, var_store: VarStore, dataset: Dataset, lr: f64, batch_size: usize, test_every: usize, update_every: usize) -> Self {
        let score_storage = ScoreStorage::new();
        let optimizer = Sgd::default().build(&var_store, lr).unwrap();
        Self {
            model,
            dataset,
            batch_size,
            optimizer,
            score_storage,
            current_epoch: 0,
            max_epochs: None,
            test_every,
            update_every,
            own_addr: None,
            device: var_store.device(),
            aggregation_protocol: None,
        }
    }

    fn epoch(&mut self) {
        match self.max_epochs {
            Some(m) => {
                if self.current_epoch >= (m - 1) {
                    debug!("Training finished!");
                    return;
                }
            }
            None => ()
        }

        debug!("Start Epoch");

        for (images, labels) in self.dataset.train_iter(self.batch_size as i64).shuffle().to_device(self.device) {
            let loss = self.model.forward_t(&images, true).cross_entropy_for_logits(&labels);
            self.optimizer.backward_step(&loss);
        }

        debug!("Finish Epoch");

        if (self.current_epoch % self.test_every) == 0 {
            self.own_addr.test();
        }

        if (self.current_epoch % self.update_every) == 0 {
            self.aggregation_protocol.clone().unwrap().do_send(ModelMessage::Request(self.model.to_flat_tensor().copy()))
        } else {
            self.own_addr.next_epoch();
        }
        self.current_epoch = self.current_epoch + 1;
    }

    fn test(&self) {
        debug!("Start Test");

        let test_accuracy = self.model.batch_accuracy_for_logits(
            &self.dataset.test_images,
            &self.dataset.test_labels,
            self.device,
            1024
        );
        info!("epoch: {:4} test acc {:5.2}%", self.current_epoch, 100. * test_accuracy);
    }
}

impl Actor for Training {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.own_addr = Some(ctx.address());
    }
}

impl Handler<Addresses> for Training {
    type Result = ();

    fn handle(&mut self, msg: Addresses, _ctx: &mut Self::Context) -> Self::Result {
        self.aggregation_protocol = Some(ModelAggregation::new(
            self.own_addr.clone().unwrap().recipient(),
            msg.cluster,
            msg.socket_addr,
            msg.server_addr
        ).start());
    }
}

impl Handler<Epoch> for Training {
    type Result = ();

    fn handle(&mut self, _msg: Epoch, _ctx: &mut Self::Context) -> Self::Result {
        self.epoch()
    }
}

impl Handler<Test> for Training {
    type Result = ();

    fn handle(&mut self, _msg: Test, _ctx: &mut Self::Context) -> Self::Result {
        self.test()
    }
}

impl Handler<ModelMessage> for Training {
    type Result = ();

    fn handle(&mut self, msg: ModelMessage, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            ModelMessage::Response(t) => {
                self.model.apply_flat_tensor(t);
                self.own_addr.next_epoch();
            },
            _ => ()
        }

    }
}

trait ApiHelper {
    fn next_epoch(&self);
    fn test(&self);
}

impl ApiHelper for Option<Addr<Training>> {
    fn next_epoch(&self) {
        let addr = self.clone().expect("Own address should be set! Be sure to start the actor");
        addr.do_send(Epoch {});
    }

    fn test(&self) {
        let addr = self.clone().expect("Own address should be set! Be sure to start the actor");
        addr.do_send(Test {});
    }
}