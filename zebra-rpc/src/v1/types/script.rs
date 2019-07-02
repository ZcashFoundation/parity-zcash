use global_script::ScriptType as GlobalScriptType;
use serde::de::Unexpected;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum ScriptType {
    NonStandard,
    PubKey,
    PubKeyHash,
    ScriptHash,
    Multisig,
    NullData,
}

impl From<GlobalScriptType> for ScriptType {
    fn from(script_type: GlobalScriptType) -> Self {
        match script_type {
            GlobalScriptType::NonStandard => ScriptType::NonStandard,
            GlobalScriptType::PubKey => ScriptType::PubKey,
            GlobalScriptType::PubKeyHash => ScriptType::PubKeyHash,
            GlobalScriptType::ScriptHash => ScriptType::ScriptHash,
            GlobalScriptType::Multisig => ScriptType::Multisig,
            GlobalScriptType::NullData => ScriptType::NullData,
        }
    }
}

impl Serialize for ScriptType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            ScriptType::NonStandard => "nonstandard".serialize(serializer),
            ScriptType::PubKey => "pubkey".serialize(serializer),
            ScriptType::PubKeyHash => "pubkeyhash".serialize(serializer),
            ScriptType::ScriptHash => "scripthash".serialize(serializer),
            ScriptType::Multisig => "multisig".serialize(serializer),
            ScriptType::NullData => "nulldata".serialize(serializer),
        }
    }
}

impl<'a> Deserialize<'a> for ScriptType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        use serde::de::Visitor;

        struct ScriptTypeVisitor;

        impl<'b> Visitor<'b> for ScriptTypeVisitor {
            type Value = ScriptType;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("script type")
            }

            fn visit_str<E>(self, value: &str) -> Result<ScriptType, E>
            where
                E: ::serde::de::Error,
            {
                match value {
                    "nonstandard" => Ok(ScriptType::NonStandard),
                    "pubkey" => Ok(ScriptType::PubKey),
                    "pubkeyhash" => Ok(ScriptType::PubKeyHash),
                    "scripthash" => Ok(ScriptType::ScriptHash),
                    "multisig" => Ok(ScriptType::Multisig),
                    "nulldata" => Ok(ScriptType::NullData),
                    _ => Err(E::invalid_value(Unexpected::Str(value), &self)),
                }
            }
        }

        deserializer.deserialize_identifier(ScriptTypeVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::ScriptType;
    use serde_json;

    #[test]
    fn script_type_serialize() {
        assert_eq!(
            serde_json::to_string(&ScriptType::NonStandard).unwrap(),
            r#""nonstandard""#
        );
        assert_eq!(
            serde_json::to_string(&ScriptType::PubKey).unwrap(),
            r#""pubkey""#
        );
        assert_eq!(
            serde_json::to_string(&ScriptType::PubKeyHash).unwrap(),
            r#""pubkeyhash""#
        );
        assert_eq!(
            serde_json::to_string(&ScriptType::ScriptHash).unwrap(),
            r#""scripthash""#
        );
        assert_eq!(
            serde_json::to_string(&ScriptType::Multisig).unwrap(),
            r#""multisig""#
        );
        assert_eq!(
            serde_json::to_string(&ScriptType::NullData).unwrap(),
            r#""nulldata""#
        );
    }

    #[test]
    fn script_type_deserialize() {
        assert_eq!(
            serde_json::from_str::<ScriptType>(r#""nonstandard""#).unwrap(),
            ScriptType::NonStandard
        );
        assert_eq!(
            serde_json::from_str::<ScriptType>(r#""pubkey""#).unwrap(),
            ScriptType::PubKey
        );
        assert_eq!(
            serde_json::from_str::<ScriptType>(r#""pubkeyhash""#).unwrap(),
            ScriptType::PubKeyHash
        );
        assert_eq!(
            serde_json::from_str::<ScriptType>(r#""scripthash""#).unwrap(),
            ScriptType::ScriptHash
        );
        assert_eq!(
            serde_json::from_str::<ScriptType>(r#""multisig""#).unwrap(),
            ScriptType::Multisig
        );
        assert_eq!(
            serde_json::from_str::<ScriptType>(r#""nulldata""#).unwrap(),
            ScriptType::NullData
        );
    }
}
