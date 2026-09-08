use serde_json::json;
use std::{collections::BTreeMap, time::Duration};
use symbiote_runtime_transport::{
    JsonlTransport, SpawnSpec, TransportLimits,
    rpc::{Incoming, RpcError, RpcSession},
};

#[test]
fn actual_process_reorders_replies_without_repeating_completion() {
    let code = r#"
import json, sys
first = json.loads(sys.stdin.readline())
second = json.loads(sys.stdin.readline())
print(json.dumps({'jsonrpc':'2.0','method':'progress','params':{'step':1}}), flush=True)
for request in [second, first, second]:
    print(json.dumps({'jsonrpc':'2.0','id':request['id'],'result':request['method']}), flush=True)
"#;
    let mut transport = JsonlTransport::spawn(
        SpawnSpec {
            executable: "/usr/bin/python3".into(),
            args: vec!["-I".into(), "-u".into(), "-c".into(), code.into()],
            cwd: std::env::current_dir().unwrap(),
            env: BTreeMap::new(),
        },
        TransportLimits::default(),
    )
    .unwrap();
    let mut rpc = RpcSession::new(2).unwrap();
    let mut ids = Vec::new();
    for method in ["first", "second"] {
        let request = rpc.request(method, None).unwrap();
        ids.push(request["id"].as_str().unwrap().to_owned());
        transport.send(&request, Duration::from_secs(2)).unwrap();
    }
    let event = transport.recv(Duration::from_secs(2)).unwrap();
    assert!(matches!(
        rpc.receive(event),
        Ok(Incoming::Notification { .. })
    ));
    for (expected, expected_id) in ["second", "first"].into_iter().zip(ids.into_iter().rev()) {
        let response = transport.recv(Duration::from_secs(2)).unwrap();
        match rpc.receive(response).unwrap() {
            Incoming::Response {
                id,
                outcome: Ok(result),
            } => {
                assert_eq!(id, expected_id);
                assert_eq!(result, json!(expected));
            }
            _ => panic!("expected correlated response"),
        }
    }
    let duplicate = transport.recv(Duration::from_secs(2)).unwrap();
    assert!(matches!(
        rpc.receive(duplicate),
        Err(RpcError::UnknownResponse)
    ));
    assert!(rpc.into_unresolved().is_empty());
}
