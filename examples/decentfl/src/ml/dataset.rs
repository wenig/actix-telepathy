use tch::vision::dataset::Dataset;
use tch::vision;


pub fn load_mnist() -> Dataset {
    let dataset_dir = format!("{}/MNIST/raw", env!("TORCH_DATASETS"));
    let m = vision::mnist::load_dir(dataset_dir).unwrap();
    m
}