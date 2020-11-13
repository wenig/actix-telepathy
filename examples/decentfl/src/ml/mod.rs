mod dataset;
mod model;
mod training;
mod score_storage;
mod protocols;

pub use training::{Training, Addresses, Epoch};
pub use model::Net;
pub use dataset::load_mnist;
