use serde_json::{Map, Value};

pub(crate) fn canonical_json_bytes(value: &Value) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(&ordered(value))
}

fn ordered(value: &Value) -> Value {
    match value {
        Value::Object(object) => {
            let mut entries: Vec<_> = object.iter().collect();
            entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
            let mut sorted = Map::new();
            for (key, value) in entries {
                sorted.insert(key.clone(), ordered(value));
            }
            Value::Object(sorted)
        }
        Value::Array(array) => Value::Array(array.iter().map(ordered).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn all_control_characters_roundtrip_in_values_and_nested_keys() {
        let controls: String = (0_u8..32).map(char::from).collect();
        let mut nested = Map::new();
        nested.insert(controls.clone(), Value::String(controls.clone()));
        let value = json!({"nested": [Value::Object(nested)], "value": controls});
        let bytes = canonical_json_bytes(&value).unwrap();
        assert!(!bytes.iter().any(|byte| *byte < 0x20));
        assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), value);
        assert_eq!(canonical_json_bytes(&serde_json::from_slice(&bytes).unwrap()).unwrap(), bytes);
    }

    #[test]
    fn standard_escaping_covers_nul_backspace_formfeed_and_quotes() {
        let value = json!({"\0": "\u{8}\u{c}\u{1f}\"\\\n\r\t"});
        let bytes = canonical_json_bytes(&value).unwrap();
        assert_eq!(std::str::from_utf8(&bytes).unwrap(),
            r#"{"\u0000":"\b\f\u001f\"\\\n\r\t"}"#);
        assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), value);
    }

    #[test]
    fn recursive_key_order_preserves_array_order() {
        let value = json!({"z": [{"z": 1, "a": 2}, 0], "a": {"y": false, "b": null}});
        assert_eq!(canonical_json_bytes(&value).unwrap(),
            br#"{"a":{"b":null,"y":false},"z":[{"a":2,"z":1},0]}"#);
    }

    #[test]
    fn utf8_key_order_preserves_distinct_unicode_spellings() {
        let mut object = Map::new();
        for key in ["\u{10000}", "\u{e000}", "é", "e\u{301}"] {
            object.insert(key.into(), Value::String(key.into()));
        }
        let value = Value::Object(object);
        let bytes = canonical_json_bytes(&value).unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        let positions: Vec<_> = ["e\u{301}", "é", "\u{e000}", "\u{10000}"].iter()
            .map(|key| text.find(&serde_json::to_string(key).unwrap()).unwrap()).collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), value);
    }

    #[test]
    fn numbers_use_the_actual_serde_json_number_serializer() {
        let value: Value = serde_json::from_str(
            "[-0.0,0,1,1.0,-1,18446744073709551615,1.25,1e20,1e-7]",
        ).unwrap();
        let bytes = canonical_json_bytes(&value).unwrap();
        assert_eq!(bytes, serde_json::to_vec(&value).unwrap());
        let parsed: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(canonical_json_bytes(&parsed).unwrap(), bytes);
    }
}
