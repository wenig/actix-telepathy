mod model_aggregation;
mod parameter_server;
mod parameter_client;

pub use model_aggregation::{ModelMessage, ModelAggregation};
pub use parameter_server::{ParameterServer, CentralModelAggregation};
pub use parameter_client::ParameterClient;