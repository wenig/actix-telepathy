use tch::{nn, Tensor};
use tch::nn::{ConvConfig, ModuleT};

#[derive(Debug)]
pub struct Net {
    conv1: nn::Conv2D,
    conv2: nn::Conv2D,
    conv3: nn::Conv2D,
    conv4: nn::Conv2D,
    conv5: nn::Conv2D,
    fc1: nn::Linear,
    fc2: nn::Linear,
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