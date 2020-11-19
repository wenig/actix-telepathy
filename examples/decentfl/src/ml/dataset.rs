use tch::vision::dataset::Dataset;
use tch::{vision, Tensor, Kind, Device};
use rand::{seq::SliceRandom, SeedableRng};
use rand::prelude::StdRng;
use std::borrow::BorrowMut;


pub fn load_mnist() -> Dataset {
    let dataset_dir = format!("{}/MNIST/raw", env!("TORCH_DATASETS"));
    let m = vision::mnist::load_dir(dataset_dir).unwrap();
    m
}

pub trait Subset {
    fn partition(&mut self, split: usize, size: i64, seed: u64);
}

impl Subset for Dataset {
    fn partition(&mut self, split: usize, size: i64, seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        let total_size = self.train_images.size().get(0).expect("Tensor is probably empty").clone();
        let mut indices: Vec<i64> = Vec::from(Tensor::arange(total_size, (Kind::Int64, Device::Cpu)));
        indices.shuffle(rng.borrow_mut());
        let indices = Tensor::of_slice(indices.as_slice());
        let partition_indices = indices.split(total_size / size, 0);
        let own_split = partition_indices.get(split).expect("Split is too large for number of partitions");
        self.train_images = self.train_images.index_select(0, own_split);
        self.train_labels = self.train_labels.index_select(0, own_split);
    }
}


#[test]
fn subset_is_correctly_split() {
    let control = load_mnist();
    let mut a = load_mnist();
    a.partition(0, 2, 1002);
    let mut b = load_mnist();
    b.partition(1, 2, 1002);
    assert_eq!(a.train_images.size().get(0).unwrap(),
               &(control.train_images.size().get(0).unwrap() / 2));
    assert_eq!(b.train_images.size().get(0).unwrap(),
                &(control.train_images.size().get(0).unwrap() / 2));
}
