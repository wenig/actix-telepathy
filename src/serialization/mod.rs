use serde_json::Result;
use serde::{Deserialize, Serialize};
use log::*;

pub trait CustomSerialization {
    fn serialize<T>(value: &T) -> Result<String> where T: ?Sized + Serialize;
    fn deserialize<'a, T>(s: &'a str) -> Result<T> where T: ?Sized + Deserialize<'a>;
}

pub struct DefaultSerialization {}

impl CustomSerialization for DefaultSerialization {
    fn serialize<T>(value: &T) -> Result<String>
    where
        T: ?Sized + Serialize,
    {
        serde_json::to_string(value)
    }

    fn deserialize<'a, T>(s: &'a str) -> Result<T>
    where
        T: ?Sized + Deserialize<'a>,
    {
        serde_json::from_str(s)
    }
}