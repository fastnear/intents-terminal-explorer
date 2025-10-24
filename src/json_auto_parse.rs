/// Auto-parse JSON-serialized strings in nested transaction data
///
/// NEAR transactions often contain JSON-serialized strings as values, e.g.:
///   "msg": "{\"foo\":\"bar\"}"
///
/// This utility recursively detects and parses these strings for better readability.
/// Ported from csli-dashboard's json-auto-parse.ts
use serde_json::Value;

/// Recursively walks a JSON value and parses JSON-serialized strings
///
/// # Arguments
/// * `value` - The JSON value to process
/// * `max_depth` - Maximum recursion depth (prevents infinite loops)
/// * `current_depth` - Current recursion depth (internal)
///
/// # Returns
/// Processed JSON value with parsed nested JSON strings
///
/// # Examples
/// ```
/// use serde_json::json;
/// let input = json!({"msg": "{\"action\":\"swap\"}"});
/// let output = auto_parse_nested_json(input, 5, 0);
/// // output: {"msg": {"action": "swap"}}
/// ```
pub fn auto_parse_nested_json(value: Value, max_depth: usize, current_depth: usize) -> Value {
    // Safety guard: prevent infinite recursion
    if current_depth >= max_depth {
        return value;
    }

    match value {
        // Handle arrays: recursively process each element
        Value::Array(arr) => {
            Value::Array(
                arr.into_iter()
                    .map(|v| auto_parse_nested_json(v, max_depth, current_depth + 1))
                    .collect()
            )
        }

        // Handle objects: recursively process each value
        Value::Object(obj) => {
            Value::Object(
                obj.into_iter()
                    .map(|(k, v)| (k, auto_parse_nested_json(v, max_depth, current_depth + 1)))
                    .collect()
            )
        }

        // Handle strings: detect and parse JSON
        Value::String(s) => {
            // Quick check: does it look like JSON?
            let trimmed = s.trim();
            // Catch all JSON structures: objects {}, arrays [], including [{...}], [123], etc.
            if (trimmed.starts_with('{') || trimmed.starts_with('['))
                && (trimmed.ends_with('}') || trimmed.ends_with(']'))
            {
                // Attempt to parse as JSON
                if let Ok(parsed) = serde_json::from_str::<Value>(trimmed) {
                    // Recursively process the result in case it contains nested JSON strings
                    return auto_parse_nested_json(parsed, max_depth, current_depth + 1);
                }
            }
            // Not valid JSON or doesn't look like JSON, return original string
            Value::String(s)
        }

        // Primitive values (numbers, booleans, null) - return as-is
        _ => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_simple_json_string() {
        let input = json!({"msg": "{\"action\":\"swap\"}"});
        let output = auto_parse_nested_json(input, 5, 0);
        assert_eq!(output, json!({"msg": {"action": "swap"}}));
    }

    #[test]
    fn test_nested_json_strings() {
        let input = json!({"msg": "{\"inner\":\"{\\\"value\\\":42}\"}"});
        let output = auto_parse_nested_json(input, 5, 0);
        assert_eq!(output, json!({"msg": {"inner": {"value": 42}}}));
    }

    #[test]
    fn test_array_json_string() {
        let input = json!({"data": "[\"a\",\"b\",\"c\"]"});
        let output = auto_parse_nested_json(input, 5, 0);
        assert_eq!(output, json!({"data": ["a", "b", "c"]}));
    }

    #[test]
    fn test_non_json_string() {
        let input = json!({"msg": "hello world"});
        let output = auto_parse_nested_json(input, 5, 0);
        assert_eq!(output, json!({"msg": "hello world"}));
    }

    #[test]
    fn test_max_depth() {
        let input = json!({"a": "{\"b\":\"{\\\"c\\\":\\\"{\\\\\\\"d\\\\\\\":1}\\\"}\"}"});
        let output = auto_parse_nested_json(input, 2, 0);
        // Should stop parsing after 2 levels
        assert!(output.is_object());
    }

    #[test]
    fn test_invalid_json_string() {
        let input = json!({"msg": "{invalid json}"});
        let output = auto_parse_nested_json(input, 5, 0);
        // Should return original string on parse error
        assert_eq!(output, json!({"msg": "{invalid json}"}));
    }

    #[test]
    fn test_array_of_objects_json_string() {
        // Real-world case from screenshot: msg field with array of objects
        let input = json!({"msg": "[{\"pool_id\":6518,\"token_in\":\"jambo\"}]"});
        let output = auto_parse_nested_json(input, 5, 0);
        assert_eq!(output, json!({"msg": [{"pool_id": 6518, "token_in": "jambo"}]}));
    }

    #[test]
    fn test_array_of_numbers_json_string() {
        let input = json!({"data": "[1,2,3,4,5]"});
        let output = auto_parse_nested_json(input, 5, 0);
        assert_eq!(output, json!({"data": [1, 2, 3, 4, 5]}));
    }
}
