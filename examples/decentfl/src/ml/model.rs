use tch::{nn, Tensor};
use tch::nn::{ConvConfig, ModuleT};
use std::iter::FromIterator;
use std::borrow::BorrowMut;


#[derive(Debug)]
pub struct Net {
    pub conv1: nn::Conv2D,
    pub conv2: nn::Conv2D,
    pub conv3: nn::Conv2D,
    pub conv4: nn::Conv2D,
    pub conv5: nn::Conv2D,
    pub fc1: nn::Linear,
    pub fc2: nn::Linear,
}


impl Net {
    pub fn new(vs: &nn::Path, out_channels: i64) -> Self {
        let mut conv_config = ConvConfig::default();
        conv_config.padding = 1;
        Net {
            conv1: nn::conv2d(vs, 1, 10, 5, Default::default()),
            conv2: nn::conv2d(vs, 10, 20, 5, Default::default()),
            conv3: nn::conv2d(vs, 20, 30, 3, conv_config.clone()),
            conv4: nn::conv2d(vs, 30, 40, 3, conv_config.clone()),
            conv5: nn::conv2d(vs, 40, 40, 3, conv_config),
            fc1: nn::linear(vs, 160, 50, Default::default()),
            fc2: nn::linear(vs, 50, out_channels, Default::default())
        }
    }

    pub fn new_with_seed(vs: &nn::Path, out_channels: i64, seed: i64) -> Self {
        tch::manual_seed(seed);
        Self::new(vs, out_channels)
    }

    pub fn get_parameters(&self) -> Vec<Tensor> {
        let parameters = self.get_parameters_ref();
        Vec::from_iter(parameters.iter().map(|&x| x.copy()))
    }

    pub fn get_parameters_ref(&self) -> Vec<&Tensor> {
        let mut parameters: Vec<&Tensor> = Vec::new();
        parameters.push(&self.conv1.ws);
        parameters.push(self.conv1.bs.as_ref().unwrap());
        parameters.push(&self.conv2.ws);
        parameters.push(self.conv2.bs.as_ref().unwrap());
        parameters.push(&self.conv3.ws);
        parameters.push(self.conv3.bs.as_ref().unwrap());
        parameters.push(&self.conv4.ws);
        parameters.push(self.conv4.bs.as_ref().unwrap());
        parameters.push(&self.conv5.ws);
        parameters.push(self.conv5.bs.as_ref().unwrap());
        parameters.push(&self.fc1.ws);
        parameters.push(&self.fc1.bs);
        parameters.push(&self.fc2.ws);
        parameters.push(&self.fc2.bs);
        parameters
    }

    pub fn get_parameters_mut(&mut self) -> Vec<&mut Tensor> {
        let mut parameters: Vec<&mut Tensor> = Vec::new();
        parameters.push(self.conv1.ws.borrow_mut());
        parameters.push(self.conv1.bs.as_mut().unwrap().borrow_mut());
        parameters.push(self.conv2.ws.borrow_mut());
        parameters.push(self.conv2.bs.as_mut().unwrap().borrow_mut());
        parameters.push(self.conv3.ws.borrow_mut());
        parameters.push(self.conv3.bs.as_mut().unwrap().borrow_mut());
        parameters.push(self.conv4.ws.borrow_mut());
        parameters.push(self.conv4.bs.as_mut().unwrap().borrow_mut());
        parameters.push(self.conv5.ws.borrow_mut());
        parameters.push(self.conv5.bs.as_mut().unwrap().borrow_mut());
        parameters.push(self.fc1.ws.borrow_mut());
        parameters.push(self.fc1.bs.borrow_mut());
        parameters.push(self.fc2.ws.borrow_mut());
        parameters.push(self.fc2.bs.borrow_mut());
        parameters
    }
}


impl ModuleT for Net {
    fn forward_t(&self, xs: &Tensor, train: bool) -> Tensor {
        let batch_size = xs.size()[0];
        xs.view([-1, 1, 28, 28])
            .apply(&self.conv1).max_pool2d_default(2).relu()
            .apply(&self.conv2).max_pool2d_default(2).relu()
            .apply(&self.conv3).relu()
            .apply(&self.conv4).relu()
            .apply(&self.conv5).dropout(0.5, train).max_pool2d_default(2).relu()
            .view([batch_size, -1])
            .apply(&self.fc1).relu().dropout(0.5, train)
            .apply(&self.fc2)
    }
}


pub trait FlattenModel {
    fn to_flat_tensor(&self) -> Tensor;
    fn apply_flat_tensor(&mut self, tensor: Tensor);
}

impl FlattenModel for Net {
    fn to_flat_tensor(&self) -> Tensor {
        let params = self.get_parameters();
        Tensor::cat(Vec::from_iter(params.iter().map(|x| x.copy().detach().view(-1))).as_slice(), 0)
    }

    fn apply_flat_tensor(&mut self, tensor: Tensor) {
        tch::no_grad(
            || {
                 let mut offset = 0;
                 for p in self.get_parameters_mut() {
                     let shape = p.copy().size();
                     let l: i64 = shape.clone().iter().product();
                     let slice = tensor.slice(0, offset, offset + l, 1).view(shape.as_slice());
                     let _ = p.set_1(&slice);
                     offset = offset + l;
                 }
            }
        )
    }
}


#[test]
fn flat_tensor_is_right() {
    use tch::{Device};
    use tch::nn::VarStore;
    use crate::ml::load_mnist;

    let dataset = load_mnist();
    let vs = VarStore::new(Device::Cpu);
    let model = Net::new(&vs.root(), 10);
    let mut model2 = Net::new(&vs.root(), 10);
    let flat = model.to_flat_tensor();
    model2.apply_flat_tensor(flat.copy());
    let flat2 = model2.to_flat_tensor();
    assert_eq!(flat, flat2);

    let out = model.forward_t(&dataset.train_images.get(0).view([1, -1]), false);
    let out2 = model2.forward_t(&dataset.train_images.get(0).view([1, -1]), false);

    assert_eq!(out, out2);
}


#[test]
fn model_trains() {
    use tch::{Device};
    use tch::nn::{VarStore, Sgd, OptimizerConfig};
    use crate::ml::load_mnist;
    use crate::ml::Subset;

    let mut dataset = load_mnist();
    dataset.partition(0, 10, 1111);
    let vs = VarStore::new(Device::Cpu);
    let model = Net::new(&vs.root(), 10);
    let mut optimizer = Sgd::default().build(&vs, 0.1).unwrap();

    let before_tensor = model.to_flat_tensor();

    optimizer.zero_grad();
    for (images, labels) in dataset.train_iter(8).shuffle().to_device(Device::Cpu) {
        let loss = model.forward_t(&images, true).cross_entropy_for_logits(&labels);
        optimizer.backward_step(&loss);
        break
    }

    let after_tensor = model.to_flat_tensor();

    assert_ne!(before_tensor, after_tensor);
}


#[test]
fn model_trains_with_new_params() {
    use tch::{Device};
    use tch::nn::{VarStore, Sgd, OptimizerConfig};
    use crate::ml::load_mnist;
    use crate::ml::Subset;

    let mut dataset = load_mnist();
    dataset.partition(0, 10, 1111);
    let vs = VarStore::new(Device::Cpu);
    let mut model = Net::new(&vs.root(), 10);
    let model2 = Net::new(&vs.root(), 10);
    let mut optimizer = Sgd::default().build(&vs, 0.1).unwrap();

    let before_tensor = model2.to_flat_tensor();
    model.apply_flat_tensor(before_tensor.copy());

    optimizer.zero_grad();
    for (images, labels) in dataset.train_iter(8).shuffle().to_device(Device::Cpu) {
        let loss = model.forward_t(&images, true).cross_entropy_for_logits(&labels);
        optimizer.backward_step(&loss);
        break
    }

    let after_tensor = model.to_flat_tensor();

    assert_ne!(before_tensor, after_tensor);
}
