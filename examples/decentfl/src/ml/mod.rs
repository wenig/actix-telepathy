mod dataset;
mod model;

use anyhow::Result;
use tch::{Device};
use tch::nn::{VarStore, ModuleT, Sgd, OptimizerConfig};
use tch::vision::dataset::Dataset;
use dataset::load_mnist;
use model::Net;


pub fn train(model: Net, dataset: Dataset, vs: VarStore) -> Result<()> {
    let mut optim = Sgd::default().build(&vs, 0.01)?;
    for epoch in 1..10 {
        for (bimages, blabels) in dataset.train_iter(64).shuffle().to_device(vs.device()) {
            let loss = model.forward_t(&bimages, true).cross_entropy_for_logits(&blabels);
            optim.backward_step(&loss);
        }
        let test_accuracy = model.batch_accuracy_for_logits(
            &dataset.test_images,
            &dataset.test_labels,
            vs.device(),
            1024
        );
        println!("epoch: {:4} test acc {:5.2}%", epoch, 100. * test_accuracy);
    }
    Ok(())
}