#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};
use flexbuffers;


/// Provides template for creating custom serializer
///
/// # Example Used in DefaultSerialization
/// ```
/// use serde::{Deserialize, Serialize};
/// use flexbuffers;
/// use actix_telepathy::CustomSerialization;
///
/// pub struct DefaultSerialization {}
///
/// impl CustomSerialization for DefaultSerialization {
///     fn serialize<T>(&self, value: &T) -> Result<Vec<u8>, ()>
///     where
///         T: ?Sized + Serialize,
///     {
///         match flexbuffers::to_vec(value) {
///             Ok(vec) => Ok(vec),
///             Err(_) => Err(())
///         }
///     }
///
///     fn deserialize<'a, T>(&self, s: &'a [u8]) -> Result<T, ()>
///     where
///         T: ?Sized + Deserialize<'a>,
///     {
///         match flexbuffers::from_slice(s) {
///             Ok(val) => Ok(val),
///             Err(_) => Err(())
///         }
///     }
/// }
/// ```
///
/// # Telling Actix-Telepathy to use Custom Serializer
/// - create a `telepathy.yaml` file in your crates root
/// - add your custom struct's name:
/// ```yaml
/// custom_serializer: "MySerializer"
/// ```
/// - import your custom struct whenever you are using the `RemoteMessage` derive macro.
pub trait CustomSerialization {
    fn serialize<T>(&self, value: &T) -> Result<Vec<u8>, ()> where T: ?Sized + Serialize;
    fn deserialize<'a, T>(&self, s: &'a [u8]) -> Result<T, ()> where T: ?Sized + Deserialize<'a>;
}

/// The default serialization used for remote messages
///
/// The default de/serializer is the Rust version of Flatbuffers - Flatbuffers.
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