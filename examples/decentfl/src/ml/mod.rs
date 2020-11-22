mod dataset;
mod model;
mod training;
mod score_storage;
mod protocols;

pub use training::{Training, Addresses, Epoch};
pub use model::{Net, FlattenModel};
pub use dataset::{load_mnist, Subset};
pub use score_storage::ScoreStorage;
pub use protocols::ModelAggregation;
