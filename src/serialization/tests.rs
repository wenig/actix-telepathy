use crate::{CustomSerialization, DefaultSerialization};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct TestValue {
    pub value: usize,
}

#[test]
fn default_serializer_correctly_ser_des() {
    let test_usize: usize = 8;
    let test_value = TestValue { value: test_usize };
    let default_serialization = DefaultSerialization {};
    let serialized = default_serialization
        .serialize(&test_value)
        .expect("Could not serialize!");
    let deserialized_value: TestValue = default_serialization
        .deserialize(&serialized)
        .expect("Could not deserialize");

    assert_eq!(deserialized_value.value, test_usize);
}
