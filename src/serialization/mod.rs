use serde::{Deserialize, Serialize};
use flexbuffers;
use log::*;

pub trait CustomSerialization {
    fn serialize<T>(&self, value: &T) -> Result<Vec<u8>, ()> where T: ?Sized + Serialize;
    fn deserialize<'a, T>(&self, s: &'a [u8]) -> Result<T, ()> where T: ?Sized + Deserialize<'a>;
}

pub struct DefaultSerialization {}

impl CustomSerialization for DefaultSerialization {
    fn serialize<T>(&self, value: &T) -> Result<Vec<u8>, ()>
    where
        T: ?Sized + Serialize,
    {
        match flexbuffers::to_vec(value) {
            Ok(vec) => Ok(vec),
            Err(_) => Err(())
        }
    }

    fn deserialize<'a, T>(&self, s: &'a [u8]) -> Result<T, ()>
    where
        T: ?Sized + Deserialize<'a>,
    {
        match flexbuffers::from_slice(s) {
            Ok(val) => Ok(val),
            Err(_) => Err(())
        }
    }
}