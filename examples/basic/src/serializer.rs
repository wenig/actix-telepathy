use actix_telepathy::CustomSerialization;
use serde::{Serialize, Deserialize};
use flexbuffers;


pub struct MySerializer {}

impl CustomSerialization for MySerializer {
    fn serialize<T>(&self, value: &T) -> Result<Vec<u8>, ()> where T: ?Sized + Serialize {
        debug!("using flexbuffers");
        match flexbuffers::to_vec(value) {
            Ok(vec) => Ok(vec),
            Err(_) => Err(())
        }
    }

    fn deserialize<'a, T>(&self, s: &'a [u8]) -> Result<T, ()> where T: ?Sized + Deserialize<'a> {
        match flexbuffers::from_slice(s) {
            Ok(v) => Ok(v),
            Err(_) => Err(())
        }
    }
}