use log::*;
use actix::prelude::*;
use tch::vision::dataset::Dataset;
use crate::ml::model::Net;
use crate::ml::score_storage::ScoreStorage;
use tch::nn::{Sgd, Optimizer, OptimizerConfig, VarStore, ModuleT};
use tch::Device;
use crate::ml::protocols::{ModelAggregation, ModelMessage};


#[derive(Message)]
#[rtype("Result = ()")]
pub struct Epoch;

#[derive(Message)]
#[rtype("Result = ()")]
pub struct Test;


pub struct Training {
    model: Net,
    dataset: Dataset,
    batch_size: usize,
    optimizer: Optimizer<Sgd>,
    score_storage: ScoreStorage,
    current_epoch: usize,
    max_epochs: Option<usize>,
    test_every: usize,
    update_every: usize,
    own_addr: Option<Addr<Training>>,
    device: Device,
    aggregation_protocol: Option<Addr<ModelAggregation>>
}

impl Training {
    pub fn new(model: Net, var_store: VarStore, dataset: Dataset, lr: f64, batch_size: usize, test_every: usize, update_every: usize) -> Self {
        let score_storage = ScoreStorage::new();
        let optimizer = Sgd::default().build(&var_store, lr)?;
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
            aggregation_protocol: None
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
            let addr = self.own_addr.clone().expect("Own address should be set! Be sure to start the actor");
            addr.do_send(Test {});
        }

        if (self.current_epoch % self.update_every) == 0 {
            self.aggregation_protocol.clone().unwrap().do_send(ModelMessage::Request(self.model.clone()))
        } else {
            let addr = self.own_addr.clone().expect("Own address should be set! Be sure to start the actor");
            addr.do_send(Epoch {})
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
        debug!("epoch: {:4} test acc {:5.2}%", self.current_epoch, 100. * test_accuracy);
    }
}

impl Actor for Training {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.own_addr = Some(ctx.address());
        self.aggregation_protocol = Some(ModelAggregation::new(ctx.address().recipient()).start());
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
        self.model = msg.0;
        self.own_addr.clone().unwrap().do_send(Epoch {});
    }
}