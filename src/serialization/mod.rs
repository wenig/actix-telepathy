#[cfg(test)]
mod tests;
use std::fmt;

use serde::{Deserialize, Serialize};

/// Provides template for creating custom serializer
///
/// # Example Used in DefaultSerialization
/// ```
/// use serde::{Deserialize, Serialize};
/// use flexbuffers;
/// use actix_telepathy::{ CustomSerialization, CustomSerializationError };
///
/// pub struct DefaultSerialization {}
///
/// impl CustomSerialization for DefaultSerialization {
///     fn serialize<T>(&self, value: &T) -> Result<Vec<u8>, CustomSerializationError>
///     where
///         T: ?Sized + Serialize,
///     {
///         match flexbuffers::to_vec(value) {
///             Ok(vec) => Ok(vec),
///             Err(_) => Err(CustomSerializationError)
///         }
///     }
///
///     fn deserialize<'a, T>(&self, s: &'a [u8]) -> Result<T, CustomSerializationError>
///     where
///         T: ?Sized + Deserialize<'a>,
///     {
///         match flexbuffers::from_slice(s) {
///             Ok(val) => Ok(val),
///             Err(_) => Err(CustomSerializationError)
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
    fn serialize<T>(&self, value: &T) -> Result<Vec<u8>, CustomSerializationError>
    where
        T: ?Sized + Serialize;
    fn deserialize<'a, T>(&self, s: &'a [u8]) -> Result<T, CustomSerializationError>
    where
        T: ?Sized + Deserialize<'a>;
}

/// Occurs if either the serialization or the deserialization fails for the `CustomSerialization`
/// trait.
pub struct CustomSerializationError;

impl fmt::Display for CustomSerializationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "An error occurred during the custom (de)serialization")
    }
}

impl fmt::Debug for CustomSerializationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{{ file: {}, line: {} }}", file!(), line!())
    }
}

/// The default serialization used for remote messages
///
/// The default de/serializer is the Rust version of Flatbuffers - Flexbuffers.
pub struct DefaultSerialization {}

impl CustomSerialization for DefaultSerialization {
    fn serialize<T>(&self, value: &T) -> Result<Vec<u8>, CustomSerializationError>
    where
        T: ?Sized + Serialize,
    {
        match flexbuffers::to_vec(value) {
            Ok(vec) => Ok(vec),
            Err(_) => Err(CustomSerializationError),
        }
    }

    fn deserialize<'a, T>(&self, s: &'a [u8]) -> Result<T, CustomSerializationError>
    where
        T: ?Sized + Deserialize<'a>,
    {
        match flexbuffers::from_slice(s) {
            Ok(val) => Ok(val),
            Err(_) => Err(CustomSerializationError),
        }
    }
}
