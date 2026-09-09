// [WFGY] Zone: RISK | λ: 0.2 | Fallbacks: 0 | Action: Minimal JSON schema validation (INV-SEC-3) prior to tool execution
//! Lightweight JSON Schema validator supporting `type`, `required`,
//! recursive `properties`, and `items` for array elements.
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("expected type '{expected}' but received '{actual}' at path '{path}'")]
    TypeMismatch { path: String, expected: String, actual: String },
    #[error("missing required property '{0}'")]
    MissingRequired(String),
}

pub fn validate(schema: &Value, value: &Value) -> Result<(), SchemaError> {
    validate_at("$", schema, value)
}

fn validate_at(path: &str, schema: &Value, value: &Value) -> Result<(), SchemaError> {
    if let Some(expected_type) = schema.get("type").and_then(Value::as_str) {
        if !matches_type(expected_type, value) {
            return Err(SchemaError::TypeMismatch {
                path: path.to_string(),
                expected: expected_type.to_string(),
                actual: json_type_name(value).to_string(),
            });
        }
    }

    if expected_object(schema) {
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for key in required {
                if let Some(key) = key.as_str() {
                    if value.get(key).is_none() {
                        return Err(SchemaError::MissingRequired(key.to_string()));
                    }
                }
            }
        }
        if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
            for (key, sub_schema) in properties {
                if let Some(sub_value) = value.get(key) {
                    validate_at(&format!("{path}.{key}"), sub_schema, sub_value)?;
                }
            }
        }
    }

    if let Some(items_schema) = schema.get("items") {
        if let Some(items) = value.as_array() {
            for (i, item) in items.iter().enumerate() {
                validate_at(&format!("{path}[{i}]"), items_schema, item)?;
            }
        }
    }

    Ok(())
}

fn expected_object(schema: &Value) -> bool {
    match schema.get("type").and_then(Value::as_str) {
        Some("object") => true,
        None => schema.get("properties").is_some() || schema.get("required").is_some(),
        _ => false,
    }
}

fn matches_type(expected: &str, value: &Value) -> bool {
    match (expected, value) {
        ("string", Value::String(_)) => true,
        ("number", Value::Number(_)) => true,
        ("integer", Value::Number(n)) => n.is_i64() || n.is_u64(),
        ("boolean", Value::Bool(_)) => true,
        ("array", Value::Array(_)) => true,
        ("object", Value::Object(_)) => true,
        ("null", Value::Null) => true,
        _ => false,
    }
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(n) if n.is_i64() || n.is_u64() => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn accepts_value_matching_schema() {
        let schema = json!({
            "type": "object",
            "required": ["name", "age"],
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer" }
            }
        });
        let val = json!({ "name": "alice", "age": 42 });
        assert!(validate(&schema, &val).is_ok());
    }

    #[test]
    fn rejects_missing_required_field() {
        let schema = json!({
            "type": "object",
            "required": ["name"],
            "properties": { "name": { "type": "string" } }
        });
        let val = json!({ "age": 42 });
        assert!(matches!(validate(&schema, &val), Err(SchemaError::MissingRequired(k)) if k == "name"));
    }

    #[test]
    fn rejects_wrong_type() {
        let schema = json!({
            "type": "object",
            "properties": { "age": { "type": "integer" } }
        });
        let val = json!({ "age": "not-an-int" });
        assert!(matches!(validate(&schema, &val), Err(SchemaError::TypeMismatch { .. })));
    }
}
