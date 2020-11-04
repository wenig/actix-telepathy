use actix_telepathy::CustomSerialization;
use serde::{Serialize, Deserialize};
use serde_json;


pub struct MySerializer {}

impl CustomSerialization for MySerializer {
    fn serialize<T>(&self, value: &T) -> Result<Vec<u8>, ()> where T: ?Sized + Serialize {
        debug!("using json serialization");
        match serde_json::to_vec(value) {
            Ok(vec) => Ok(vec),
            Err(_) => Err(())
        }
    }

    fn deserialize<'a, T>(&self, s: &'a [u8]) -> Result<T, ()> where T: ?Sized + Deserialize<'a> {
        match serde_json::from_slice(s) {
            Ok(v) => Ok(v),
            Err(_) => Err(())
        }
    }
}