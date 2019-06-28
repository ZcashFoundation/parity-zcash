use keys::Address;
use serde::de::{Unexpected, Visitor};
use serde::{Deserializer, Serialize, Serializer};
use std::fmt;

pub fn serialize<S>(address: &Address, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    address.to_string().serialize(serializer)
}

pub fn deserialize<'a, D>(deserializer: D) -> Result<Address, D::Error>
where
    D: Deserializer<'a>,
{
    deserializer.deserialize_any(AddressVisitor)
}

#[derive(Default)]
pub struct AddressVisitor;

impl<'b> Visitor<'b> for AddressVisitor {
    type Value = Address;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an address")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: ::serde::de::Error,
    {
        value
            .parse()
            .map_err(|_| E::invalid_value(Unexpected::Str(value), &self))
    }
}

pub mod vec {
    use super::AddressVisitor;
    use keys::Address;
    use serde::de::Visitor;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(addresses: &Vec<Address>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        addresses
            .iter()
            .map(|address| address.to_string())
            .collect::<Vec<_>>()
            .serialize(serializer)
    }

    pub fn deserialize<'a, D>(deserializer: D) -> Result<Vec<Address>, D::Error>
    where
        D: Deserializer<'a>,
    {
        <Vec<&'a str> as Deserialize>::deserialize(deserializer)?
            .into_iter()
            .map(|value| AddressVisitor::default().visit_str(value))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use keys::Address;
    use serde_json;
    use v1::types;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestStruct {
        #[serde(with = "types::address")]
        address: Address,
    }

    impl TestStruct {
        fn new(address: Address) -> Self {
            TestStruct { address: address }
        }
    }

    #[test]
    fn address_serialize() {
        let test = TestStruct::new("t2UNzUUx8mWBCRYPRezvA363EYXyEpHokyi".into());
        assert_eq!(
            serde_json::to_string(&test).unwrap(),
            r#"{"address":"t2UNzUUx8mWBCRYPRezvA363EYXyEpHokyi"}"#
        );
    }

    #[test]
    fn address_deserialize() {
        let test = TestStruct::new("t2UNzUUx8mWBCRYPRezvA363EYXyEpHokyi".into());
        assert_eq!(
            serde_json::from_str::<TestStruct>(
                r#"{"address":"t2UNzUUx8mWBCRYPRezvA363EYXyEpHokyi"}"#
            )
            .unwrap(),
            test
        );
    }
}
