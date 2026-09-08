//! Bounded configuration decoding without duplicate-field normalization or raw errors.
use serde::de::{Deserialize, DeserializeOwned, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
use std::fmt;

pub(crate) fn parse<T: DeserializeOwned>(input: &str, max_bytes: usize) -> Result<T, ()> {
    if input.len() > max_bytes {
        return Err(());
    }
    // from_str retains serde_json's recursion limit and rejects trailing data.
    let value = serde_json::from_str::<UniqueValue>(input).map_err(|_| ())?;
    serde_json::from_value(value.0).map_err(|_| ())
}

struct UniqueValue(Value);

impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct UniqueVisitor;

        impl<'de> Visitor<'de> for UniqueVisitor {
            type Value = UniqueValue;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("JSON with unique object keys")
            }

            fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Bool(value)))
            }

            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Number(value.into())))
            }

            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Number(value.into())))
            }

            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(value)
                    .map(|number| UniqueValue(Value::Number(number)))
                    .ok_or_else(|| E::custom("invalid JSON number"))
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::String(value.into())))
            }

            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::String(value)))
            }

            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Ok(UniqueValue(Value::Null))
            }

            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = sequence.next_element::<UniqueValue>()? {
                    values.push(value.0);
                }
                Ok(UniqueValue(Value::Array(values)))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(serde::de::Error::custom("duplicate JSON key"));
                    }
                    values.insert(key, map.next_value::<UniqueValue>()?.0);
                }
                Ok(UniqueValue(Value::Object(values)))
            }
        }

        deserializer.deserialize_any(UniqueVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[derive(Debug, PartialEq, serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Document {
        version: u32,
        extensions: BTreeMap<String, Value>,
    }

    #[test]
    fn typed_valid_document_preserves_values_and_whitespace() {
        let input = " {\"version\":1,\"extensions\":{\"vendor\":[true,null,1,1.5,\"text\"]}} \n";
        let document = parse::<Document>(input, input.len()).unwrap();
        assert_eq!(document.version, 1);
        assert_eq!(
            document.extensions["vendor"],
            serde_json::json!([true, null, 1, 1.5, "text"])
        );
        assert_eq!(parse::<u64>("18446744073709551615", 32).unwrap(), u64::MAX);
    }

    #[test]
    fn duplicate_keys_are_rejected_at_every_level_including_extensions() {
        for input in [
            r#"{"version":1,"version":2,"extensions":{}}"#,
            r#"{"version":1,"extensions":{"vendor":{"key":1,"key":2}}}"#,
            r#"{"version":1,"extensions":{"vendor":{},"vendor":{}}}"#,
            r#"{"version":1,"extensions":{"vendor":[{"nested":1,"nested":2}]}}"#,
            r#"{"version":1,"ver\u0073ion":2,"extensions":{}}"#,
        ] {
            assert!(parse::<Document>(input, 4096).is_err());
        }
    }

    #[test]
    fn byte_limit_includes_utf8_and_is_checked_before_parsing() {
        let input = "\"🦀\"";
        assert!(parse::<String>(input, input.len() - 1).is_err());
        assert_eq!(parse::<String>(input, input.len()).unwrap(), "🦀");
        assert!(parse::<Value>(&"x".repeat(1025), 1024).is_err());
    }

    #[test]
    fn recursion_limit_and_trailing_data_remain_strict() {
        let deep = format!("{}null{}", "[".repeat(256), "]".repeat(256));
        assert!(parse::<Value>(&deep, 4096).is_err());
        for input in ["{} {}", "{} trailing", "null true", "[1,]"] {
            assert!(parse::<Value>(input, 4096).is_err());
        }
    }

    #[test]
    fn typed_schema_rejects_unknown_fields_and_wrong_types() {
        assert!(
            parse::<Document>(r#"{"version":1,"extensions":{},"unknown":true}"#, 4096).is_err()
        );
        assert!(parse::<Document>(r#"{"version":"1","extensions":{}}"#, 4096).is_err());
    }
}
