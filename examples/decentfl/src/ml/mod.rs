mod dataset;
mod model;
mod training;
mod score_storage;
mod protocols;

use anyhow::Result;
use tch::{Device};
use tch::nn::{VarStore, ModuleT, Sgd, OptimizerConfig};
use tch::vision::dataset::Dataset;
use dataset::load_mnist;
use model::Net;
