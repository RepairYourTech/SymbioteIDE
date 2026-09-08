//! Local deterministic plumbing demonstration. No agent, network or credentials.
use serde_json::json;
use std::{collections::BTreeMap, time::Duration};
use symbiote_runtime_transport::{JsonlTransport, SpawnSpec, TransportLimits};

fn main() {
    let spec = SpawnSpec {
        executable: "/bin/cat".into(),
        args: vec![],
        cwd: std::env::current_dir().expect("working directory"),
        env: BTreeMap::new(),
    };
    let mut transport =
        JsonlTransport::spawn(spec, TransportLimits::default()).expect("start local echo fixture");
    let payload = json!({"fixture":"symbiote-jsonl", "sequence":1});
    transport
        .send(&payload, Duration::from_secs(2))
        .expect("send frame");
    let received = transport
        .recv(Duration::from_secs(2))
        .expect("receive frame");
    assert_eq!(received, payload, "echoed frame differs");
    transport
        .cancel(Duration::from_secs(2))
        .expect("reap echo fixture");
    println!(
        "Verified one structured round trip and fixture cancellation. No agent task was executed."
    );
}
