//! E2E test for the iii quickstart template.
//!
//! Scaffolds the quickstart, starts the engine and all 4 workers, then
//! hits the `/orchestrate` endpoint and verifies the aggregated response.
//!
//! Run with:
//!   cargo test --test e2e_quickstart -- --ignored --nocapture
//!
//! Requires:
//!   - `iii` binary on PATH (or III_BIN env var)
//!   - Node.js + npm (for client and payment-worker)
//!   - Python 3 (for data-worker)
//!   - Rust/Cargo (for compute-worker)

mod e2e_harness;

use serde_json::json;
use std::time::Duration;

#[tokio::test]
#[ignore]
async fn quickstart_orchestrate_returns_all_workers() {
    let mut scenario = e2e_harness::Scenario::builder("quickstart", "iii")
        .build()
        .await;

    scenario.read_http_port();
    scenario.start_engine().await;
    scenario.start_workers().await;
    scenario.wait_for_http(Duration::from_secs(120)).await;

    let resp = scenario
        .http_post(
            "/orchestrate",
            json!({"data": {"message": "hello from e2e"}, "n": 42}),
        )
        .await;

    assert_eq!(resp.status(), 200, "orchestrate should return 200");

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["client"], "ok");
    assert!(
        body["errors"].as_array().unwrap().is_empty(),
        "expected no errors, got: {}",
        body["errors"]
    );
    assert_eq!(body["computeWorker"]["result"], 84);
    assert_eq!(body["computeWorker"]["input"], 42);
    assert_eq!(body["dataWorker"]["source"], "data-worker");
    assert_eq!(
        body["externalWorker"]["body"]["message"],
        "Payment recorded"
    );

    scenario.shutdown().await;
}

#[tokio::test]
#[ignore]
async fn quickstart_health_endpoint_returns_ok() {
    let mut scenario = e2e_harness::Scenario::builder("quickstart", "iii")
        .build()
        .await;

    scenario.read_http_port();
    scenario.start_engine().await;

    scenario.start_worker("workers/client").await;
    scenario.wait_for_http(Duration::from_secs(60)).await;

    let resp = scenario.http_get("/health").await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["healthy"], true);

    scenario.shutdown().await;
}
