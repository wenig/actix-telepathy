use actix::prelude::*;
use actix_telepathy::prelude::*;
use tch::{Tensor, Kind, Device, IndexOp};
use crate::security::sub_cluster::{SubCluster, CollectiveApi};
use std::ops::{Add, Sub, Mul, Div};


pub struct Secret {
    name: String,
    share: Tensor,
    ctx: Addr<SubCluster>,
    kind: Kind
}

#[allow(dead_code)]
impl Secret {
    pub fn new(name: String, share: Tensor, ctx: Addr<SubCluster>) -> Self {
        Self {name, share, ctx, kind: Kind::Float}
    }

    pub fn empty_like(like: &Self) -> Self {
        Self::new(like.name.clone(), like.share.zeros_like(), like.ctx.clone())
    }

    pub fn share_with(value: &Tensor, ctx: Addr<SubCluster>, name: &str, size: usize) -> Self {
        let shares = random_additive(value, size as i64);
        let own_share = value.zeros_like();
        ctx.scatter(&shares, &own_share);
        Self::new(name.to_string(), own_share, ctx)
    }

    pub fn share_from(buffer: Tensor, src: RemoteAddr, ctx: Addr<SubCluster>, name: &str) -> Self {
        ctx.receive(&buffer, src);
        Self::new(name.to_string(), buffer, ctx)
    }

    pub fn all_share(value: &Tensor, ctx: Addr<SubCluster>, name: &str, size: usize) -> Vec<Self> {
        let shares = random_additive(value, size as i64);
        let secrets = shares.zeros_like();
        ctx.allscatter(&shares, &secrets);
        let mut results: Vec<Self> = vec![];
        for i in 0..size {
            results.push(Secret::new(name.to_string(), secrets.i(i as i64), ctx.clone()));
        }
        results
    }

    pub fn random(ctx: Addr<SubCluster>) -> Self {
        let share = Tensor::randint1(-100_000, 100_000, &[1], (Kind::Int64, Device::Cpu));
        Self::new(String::from("random"), share, ctx)
    }

    pub fn positive_random(ctx: Addr<SubCluster>) -> Self {
        let share = Tensor::randint1(1, 100_000, &[1], (Kind::Int64, Device::Cpu));
        Self::new(String::from("random"), share, ctx)
    }

    pub fn _max(secrets: &Vec<Self>) -> (&Self, usize) {
        let mut max = 0;
        for i in 1..secrets.len() {
            if secrets[i].gt(&secrets[max]) {
                max = i;
            }
        }
        (&secrets[max], max)
    }

    pub fn max(secrets: &Vec<Self>) -> &Self {
        Self::_max(secrets).0
    }

    pub fn argmax(secrets: &Vec<Self>) -> usize {
        Self::_max(secrets).1
    }

    pub fn sum_reveal(secrets: &Vec<Self>) -> Tensor {
        let mut acc_secret = Self::empty_like(&secrets[0]);
        for x in secrets.iter() {
            acc_secret.add(x);
        }
        acc_secret.reveal()
    }

    pub fn mean_reveal(secrets: &Vec<Self>) -> Tensor {
        Self::sum_reveal(secrets) / secrets.len() as i64
    }

    pub fn mean_reveal_diff(secrets: &Vec<Self>, diff: Tensor) -> Tensor {
        (Self::sum_reveal(secrets) - (secrets.len() as i64 * diff)) / secrets.len() as i64
    }

    pub fn reveal(&self) -> Tensor {
        let recvbuf = self.share.zeros_like();
        self.ctx.allreduce(self.share.as_ref(), &recvbuf);
        recvbuf
    }

    pub fn sum(&self) -> Self {
        let sum = self.share.sum(self.kind);
        Self::new(format!("{}.sum()", self.name), sum, self.ctx.clone())
    }

    // Operators

    pub fn add(&mut self, other: &Self) {
        self.share = (&self.share).add(&other.share);
        self.name = format!("({} + {})", self.name, other.name);
    }

    pub fn add_t(&mut self, other: &Tensor, size: usize) {
        self.share = (&self.share).add(other / size as i64);
        self.name = format!("({} + T)", self.name);
    }

    pub fn sub(&mut self, other: &Self) {
        self.share = (&self.share).sub(&other.share);
        self.name = format!("({} - {})", self.name, other.name);
    }

    pub fn sub_t(&mut self, other: &Tensor, size: usize) {
        self.share = (&self.share).sub(other / size as i64);
        self.name = format!("({} - T)", self.name);
    }

    pub fn mul(&mut self, _other: &Self) {
        // todo Beaver Store
    }

    pub fn mul_t(&mut self, other: &Tensor) {
        self.share = (&self.share).mul(other);
        self.name = format!("({} * T)", self.name);
    }

    pub fn div(&mut self, other: &Self) {
        let mut random = Self::random(self.ctx.clone());
        let mut random_copy = Self::new("copy".to_string(), random.share.copy(), self.ctx.clone());
        random.mul(other);
        let pseudo_other = random.reveal();
        self.div_t(&pseudo_other);
        random_copy.mul(self);
        self.name = format!("({} / {})", self.name, other.name);
        self.share = random_copy.share;
    }

    pub fn div_t(&mut self, other: &Tensor) {
        self.share = (&self.share).div(other);
        self.name = format!("({} / T)", self.name);
    }

    pub fn gt(&self, other: &Self) -> bool {
        assert_eq!(self.share.size()[0], 1);
        let random = Self::positive_random(self.ctx.clone());
        let mut copy = self.clone();
        copy.sub(other);
        copy.mul(&random);
        copy.reveal().int64_value(&[0]) > 0
    }

    pub fn lt(&self, other: &Self) -> bool {
        assert_eq!(self.share.size()[0], 1);
        let random = Self::positive_random(self.ctx.clone());
        let mut copy = self.clone();
        copy.sub(other);
        copy.mul(&random);
        copy.reveal().int64_value(&[0]) < 0
    }
}

impl Clone for Secret {
    fn clone(&self) -> Self {
        Self::new(self.name.clone(), self.share.copy(), self.ctx.clone())
    }
}


fn _random_tensor(value: &Tensor, splits: i64) -> Tensor {
    let mut random_shape = vec![splits];
    random_shape.extend(value.size());
    Tensor::randint1(
        -100000,
        100000,
        random_shape.as_slice(),
        (Kind::Int64, Device::Cpu))
}

fn _zero_index(value: &Tensor) -> Tensor {
    let summed_cols = value.sum1(&[0], false, Kind::Int64).eq(0);
    summed_cols.nonzero()
}

pub fn random_additive(value: &Tensor, splits: i64) -> Tensor {
    let mut x = _random_tensor(value, splits);
    let mut zero_index = _zero_index(&x);
    while zero_index.size()[0] > 0 {
        x = _random_tensor(value, splits);
        zero_index = _zero_index(&x);
    }
    let mut shares = x.copy() / x.sum1(&[0], false, Kind::Int64);
    shares = value.copy().mul(shares);
    shares
}

#[test]
fn correct_mpc_addition() {
    let a = Tensor::of_slice(&[1,2,3]);
    let b = Tensor::of_slice(&[3,2,1]);
    let a_encrypted = random_additive(&a, 2);
    let b_encrypted = random_additive(&b, 2);
    let sum_encrypted = (a_encrypted.copy() + b_encrypted)
        .sum1(&[0], false, a_encrypted.kind());
    assert_eq!(sum_encrypted.round(), (a+b))
}