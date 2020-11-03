use serde::{Serialize, Deserialize};
use actix_telepathy::{CustomSerialization, DefaultSerialization};


struct CustomSerializer {}

impl CustomSerialization for CustomSerializer {
    fn serialize<T>(value: &T) -> Result<String> where T: ?Sized + Serialize {
        DefaultSerialization::serialize(value)
    }

    fn deserialize<'a, T>(s: &'a str) -> Result<T> where T: ?Sized + Deserialize<'a> {
        DefaultSerialization::deserialize(s)
    }
}