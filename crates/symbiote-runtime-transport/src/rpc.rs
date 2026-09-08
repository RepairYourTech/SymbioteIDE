//! Explicit JSON-RPC 2.0 correlation. This is not a harness compatibility claim.
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RpcError {
    InvalidLimits,
    InvalidEnvelope,
    InvalidMethod,
    InvalidParams,
    PendingLimit,
    IdExhausted,
    UnknownResponse,
    UnsupportedPeerRequest,
}

/// Payloads can contain private source, prompts, and upstream error data.
/// Deliberately no Debug implementation: never use them as default diagnostics.
pub enum Incoming {
    Notification {
        method: String,
        params: Option<Value>,
    },
    Response {
        id: String,
        outcome: Result<Value, Value>,
    },
}

/// One connection generation. IDs never repeat within this object. A prepared
/// request remains pending even if a write is interrupted: delivery is unknown.
/// Do not automatically replay effectful requests after reconnect.
pub struct RpcSession {
    next_id: u64,
    pending: BTreeSet<String>,
    max_pending: usize,
}

impl RpcSession {
    pub fn new(max_pending: usize) -> Result<Self, RpcError> {
        if !(1..=4096).contains(&max_pending) {
            return Err(RpcError::InvalidLimits);
        }
        Ok(Self {
            next_id: 1,
            pending: BTreeSet::new(),
            max_pending,
        })
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn request(&mut self, method: &str, params: Option<Value>) -> Result<Value, RpcError> {
        validate_call(method, params.as_ref())?;
        if self.pending.len() >= self.max_pending {
            return Err(RpcError::PendingLimit);
        }
        let next = self.next_id.checked_add(1).ok_or(RpcError::IdExhausted)?;
        let id = self.next_id.to_string();
        let mut frame = Self::notification(method, params)?;
        frame
            .as_object_mut()
            .expect("constructed object")
            .insert("id".into(), json!(id));
        self.pending.insert(id);
        self.next_id = next;
        Ok(frame)
    }

    pub fn notification(method: &str, params: Option<Value>) -> Result<Value, RpcError> {
        validate_call(method, params.as_ref())?;
        let mut frame = json!({"jsonrpc":"2.0", "method":method});
        if let Some(params) = params {
            frame
                .as_object_mut()
                .expect("constructed object")
                .insert("params".into(), params);
        }
        Ok(frame)
    }

    /// A profile must select this exact dialect. Versionless app servers need
    /// their own explicitly tested decoder; there is no permissive auto-detect.
    /// Invalid responses leave pending requests intact. Duplicates are rejected
    /// as unknown once the first valid response has consumed the ID.
    pub fn receive(&mut self, frame: Value) -> Result<Incoming, RpcError> {
        let object = frame.as_object().ok_or(RpcError::InvalidEnvelope)?;
        if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return Err(RpcError::InvalidEnvelope);
        }
        if let Some(method) = object.get("method") {
            if object
                .keys()
                .any(|key| !["jsonrpc", "method", "params", "id"].contains(&key.as_str()))
            {
                return Err(RpcError::InvalidEnvelope);
            }
            let method = method.as_str().ok_or(RpcError::InvalidMethod)?;
            validate_call(method, object.get("params"))?;
            if object.contains_key("id") {
                return Err(RpcError::UnsupportedPeerRequest);
            }
            return Ok(Incoming::Notification {
                method: method.to_owned(),
                params: object.get("params").cloned(),
            });
        }
        if object
            .keys()
            .any(|key| !["jsonrpc", "id", "result", "error"].contains(&key.as_str()))
            || object.contains_key("result") == object.contains_key("error")
        {
            return Err(RpcError::InvalidEnvelope);
        }
        let id = object
            .get("id")
            .and_then(Value::as_str)
            .ok_or(RpcError::InvalidEnvelope)?;
        let outcome = if let Some(error) = object.get("error") {
            let fields = error.as_object().ok_or(RpcError::InvalidEnvelope)?;
            if fields.get("code").and_then(Value::as_i64).is_none()
                || fields.get("message").and_then(Value::as_str).is_none()
                || fields
                    .keys()
                    .any(|key| !["code", "message", "data"].contains(&key.as_str()))
            {
                return Err(RpcError::InvalidEnvelope);
            }
            Err(error.clone())
        } else {
            Ok(object["result"].clone())
        };
        if !self.pending.remove(id) {
            return Err(RpcError::UnknownResponse);
        }
        Ok(Incoming::Response {
            id: id.to_owned(),
            outcome,
        })
    }

    /// Consume the generation on disconnect. These requests have unknown
    /// delivery/effect disposition, not permission to retry them.
    pub fn into_unresolved(self) -> Vec<String> {
        self.pending.into_iter().collect()
    }
}

fn validate_call(method: &str, params: Option<&Value>) -> Result<(), RpcError> {
    if method.is_empty()
        || method.len() > 256
        || method.starts_with("rpc.")
        || method.chars().any(char::is_control)
    {
        return Err(RpcError::InvalidMethod);
    }
    if params.is_some_and(|value| !value.is_object() && !value.is_array()) {
        return Err(RpcError::InvalidParams);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reordered_responses_correlate_once_and_null_is_a_result() {
        let mut rpc = RpcSession::new(2).unwrap();
        let first = rpc.request("first", None).unwrap();
        let second = rpc.request("second", Some(json!([]))).unwrap();
        let response = json!({"jsonrpc":"2.0","id":second["id"],"result":null});
        assert!(matches!(
            rpc.receive(response.clone()),
            Ok(Incoming::Response {
                outcome: Ok(Value::Null),
                ..
            })
        ));
        assert!(matches!(
            rpc.receive(response),
            Err(RpcError::UnknownResponse)
        ));
        assert_eq!(rpc.into_unresolved(), vec![first["id"].as_str().unwrap()]);
    }

    #[test]
    fn malformed_response_never_consumes_pending_id() {
        let mut rpc = RpcSession::new(1).unwrap();
        let request = rpc.request("work", None).unwrap();
        for response in [
            json!({"jsonrpc":"2.0","id":request["id"],"result":1,"error":null}),
            json!({"jsonrpc":"2.0","id":request["id"],"error":{"code":"bad","message":"bad"}}),
            json!({"id":request["id"],"result":1}),
            json!({"jsonrpc":"2.0","id":request["id"],"result":1,"extension":true}),
        ] {
            assert!(matches!(
                rpc.receive(response),
                Err(RpcError::InvalidEnvelope)
            ));
            assert_eq!(rpc.pending_count(), 1);
        }
        assert!(matches!(
            rpc.receive(
                json!({"jsonrpc":"2.0","id":request["id"],"error":{"code":-1,"message":"failure"}})
            ),
            Ok(Incoming::Response {
                outcome: Err(_),
                ..
            })
        ));
    }

    #[test]
    fn notifications_are_unconfirmed_and_peer_requests_are_explicitly_unsupported() {
        let mut rpc = RpcSession::new(1).unwrap();
        assert!(matches!(
            rpc.receive(json!({"jsonrpc":"2.0","method":"event","params":{}})),
            Ok(Incoming::Notification { .. })
        ));
        assert_eq!(rpc.pending_count(), 0);
        assert!(matches!(
            rpc.receive(json!({"jsonrpc":"2.0","method":"approve","id":1})),
            Err(RpcError::UnsupportedPeerRequest)
        ));
        assert!(matches!(
            rpc.receive(json!([])),
            Err(RpcError::InvalidEnvelope)
        ));
    }

    #[test]
    fn admission_limits_fail_without_advancing_identity() {
        assert!(matches!(RpcSession::new(0), Err(RpcError::InvalidLimits)));
        let mut rpc = RpcSession::new(1).unwrap();
        assert_eq!(
            rpc.request("rpc.reserved", None),
            Err(RpcError::InvalidMethod)
        );
        assert_eq!(
            rpc.request("work", Some(Value::Null)),
            Err(RpcError::InvalidParams)
        );
        assert_eq!(rpc.request("work", None).unwrap()["id"], "1");
        assert_eq!(rpc.request("work", None), Err(RpcError::PendingLimit));
        assert_eq!(rpc.pending_count(), 1);
    }
}
