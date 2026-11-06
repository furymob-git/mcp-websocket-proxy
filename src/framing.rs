use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
    Batch(Vec<Value>),
    Other(Value),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcMessage {
    pub fn parse(text: &str) -> Result<Self, serde_json::Error> {
        let val: Value = serde_json::from_str(text)?;
        if val.is_array() {
            return Ok(JsonRpcMessage::Batch(
                val.as_array().cloned().unwrap_or_default(),
            ));
        }

        if let Some(obj) = val.as_object() {
            let _has_jsonrpc = obj.contains_key("jsonrpc");
            let has_id = obj.contains_key("id");
            let has_method = obj.contains_key("method");
            let has_result = obj.contains_key("result");
            let has_error = obj.contains_key("error");

            if has_method && has_id {
                if let Ok(req) = serde_json::from_value::<JsonRpcRequest>(val.clone()) {
                    return Ok(JsonRpcMessage::Request(req));
                }
            } else if has_method && !has_id {
                if let Ok(notif) = serde_json::from_value::<JsonRpcNotification>(val.clone()) {
                    return Ok(JsonRpcMessage::Notification(notif));
                }
            } else if (has_result || has_error) && has_id {
                if let Ok(res) = serde_json::from_value::<JsonRpcResponse>(val.clone()) {
                    return Ok(JsonRpcMessage::Response(res));
                }
            }
        }

        Ok(JsonRpcMessage::Other(val))
    }
}

/// Utility to build a composite internal ID for zero-collision request multiplexing
pub fn make_internal_id(client_id: &str, raw_id: &Value) -> String {
    format!("_proxy:{}:{}", client_id, raw_id)
}

/// Utility to decode a composite internal ID if formatted as a string
pub fn parse_internal_id(val: &Value) -> Option<(String, Value)> {
    if let Some(s) = val.as_str() {
        if let Some(stripped) = s.strip_prefix("_proxy:") {
            if let Some((client_id, original_id_str)) = stripped.split_once(':') {
                let original_id = serde_json::from_str::<Value>(original_id_str)
                    .unwrap_or_else(|_| Value::String(original_id_str.to_string()));
                return Some((client_id.to_string(), original_id));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_request() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}"#;
        match JsonRpcMessage::parse(raw).unwrap() {
            JsonRpcMessage::Request(req) => {
                assert_eq!(req.method, "tools/list");
                assert_eq!(req.id, json!(1));
            }
            _ => panic!("Expected request"),
        }
    }

    #[test]
    fn test_parse_notification() {
        let raw = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        match JsonRpcMessage::parse(raw).unwrap() {
            JsonRpcMessage::Notification(notif) => {
                assert_eq!(notif.method, "notifications/initialized");
            }
            _ => panic!("Expected notification"),
        }
    }

    #[test]
    fn test_parse_response() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#;
        match JsonRpcMessage::parse(raw).unwrap() {
            JsonRpcMessage::Response(res) => {
                assert_eq!(res.id, json!(1));
                assert!(res.result.is_some());
            }
            _ => panic!("Expected response"),
        }
    }

    #[test]
    fn test_internal_id_roundtrip() {
        let client_id = "client-uuid-123";
        let original_id = json!(42);
        let internal_str = make_internal_id(client_id, &original_id);
        let internal_val = Value::String(internal_str);

        let (recovered_client, recovered_id) = parse_internal_id(&internal_val).unwrap();
        assert_eq!(recovered_client, client_id);
        assert_eq!(recovered_id, original_id);
    }
}
