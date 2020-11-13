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
        Tensor::cat(Vec::from_iter(params.iter().map(|x| x.copy().view(-1))).as_slice(), 0)
    }

    fn apply_flat_tensor(&mut self, tensor: Tensor) {
        let mut offset = 0;

        for p in self.get_parameters_mut() {
            let shape = p.copy().size();
            let l: i64 = shape.clone().iter().product();
            let slice = tensor.slice(0, offset, offset+l, 1).view(shape.as_slice());
            *p = slice;
            offset = offset + l;
        }
    }
}