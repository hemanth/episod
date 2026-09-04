use std::sync::Arc;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

use episod::{
    create_router, AppState, BackendReplica, ConsistentHashRouter, Episode, InMemoryStore,
    Message, MockAdapter, StateStore, ToolRegistry,
};
use episod::adapter::mock::MockBehavior;

fn setup_test_app(adapter: MockAdapter) -> (axum::Router, Arc<InMemoryStore>, ToolRegistry) {
    let store = Arc::new(InMemoryStore::new());
    let router = ConsistentHashRouter::new(50);
    router.add_replica(BackendReplica::new(
        "test-node-1",
        "http://127.0.0.1:8000",
        1,
    ));

    let tool_registry = ToolRegistry::new();
    let state = AppState::new(
        store.clone(),
        router,
        Arc::new(adapter),
        tool_registry.clone(),
    );

    (create_router(state), store, tool_registry)
}

#[tokio::test]
async fn test_dag_branching_and_linearization() {
    let mut ep = Episode::new("meta-llama/Llama-3.1-8B", Some("You are an AI.".into()));

    // Turn 1
    let t1 = ep.append_turn(
        Some(Message::user("Hello")),
        Some(Message::assistant("Hi there!")),
        vec![],
        vec![],
    );

    // Turn 2
    let t2 = ep.append_turn(
        Some(Message::user("Tell me a joke")),
        Some(Message::assistant("Why did the chicken cross the road?")),
        vec![],
        vec![],
    );

    assert_eq!(ep.active_leaf_id, Some(t2.clone()));

    // Fork back to Turn 1
    ep.fork(&t1).unwrap();
    assert_eq!(ep.active_leaf_id, Some(t1.clone()));

    // Turn 3 on the branched fork
    let t3 = ep.append_turn(
        Some(Message::user("Write a poem")),
        Some(Message::assistant("Roses are red...")),
        vec![],
        vec![],
    );

    // Turn 1 must have 2 children: t2 and t3
    let node1 = ep.nodes.get(&t1).unwrap();
    assert_eq!(node1.children_ids.len(), 2);
    assert!(node1.children_ids.contains(&t2));
    assert!(node1.children_ids.contains(&t3));

    // Linearize from branch t3: should only contain system + t1 + t3 (t2 must NOT be present!)
    let history = ep.linearize_history(Some(&t3));
    assert_eq!(history.len(), 5); // system, user(Hello), assistant(Hi), user(Write poem), assistant(Roses)
    assert_eq!(history[0].content, Some("You are an AI.".into()));
    assert_eq!(history[1].content, Some("Hello".into()));
    assert_eq!(history[2].content, Some("Hi there!".into()));
    assert_eq!(history[3].content, Some("Write a poem".into()));
    assert_eq!(history[4].content, Some("Roses are red...".into()));
}

#[tokio::test]
async fn test_api_health_and_episode_creation() {
    let adapter = MockAdapter::new();
    let (app, _store, _tools) = setup_test_app(adapter);

    // 1. Health check
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["status"], "ok");

    // 2. Create episode
    let create_payload = json!({
        "model": "gpt-4o",
        "system_prompt": "Always be polite."
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/episodes")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&create_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let ep_json: Value = serde_json::from_slice(&body).unwrap();
    let episode_id = ep_json["id"].as_str().unwrap();
    assert!(episode_id.starts_with("ep_"));

    // 3. Get episode by ID
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/v1/episodes/{}", episode_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_turn_streaming_and_hitl_approval() {
    let adapter = MockAdapter::new();
    // Simulate model calling the sensitive tool "system_command"
    adapter.add_behavior(MockBehavior::ToolCall {
        id: "call_sys_99".to_string(),
        name: "system_command".to_string(),
        arguments: r#"{"command":"rm -rf /tmp/cache"}"#.to_string(),
    });

    let (app, store, _tools) = setup_test_app(adapter);

    // Create episode
    let ep = Episode::new("test-model", None);
    let ep_id = ep.id.clone();
    store.create_episode(ep).await.unwrap();

    // Submit a turn that triggers the approval-required tool
    let turn_payload = json!({
        "message": "Clear the temporary cache please.",
        "execute_tools": true
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/episodes/{}/turns", ep_id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&turn_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Stream SSE output
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body);

    // The stream should emit turn_started, tool_call_delta, tool_call_completed, approval_required
    assert!(body_str.contains("event: turn_started"));
    assert!(body_str.contains("event: approval_required"));
    assert!(body_str.contains("system_command"));

    // Verify episode in store has pending approval
    let updated_ep = store.get_episode(&ep_id).await.unwrap().unwrap();
    assert!(updated_ep.pending_approvals.contains_key("call_sys_99"));

    // Approve the pending tool call via /v1/episodes/{id}/approve
    let approve_payload = json!({
        "tool_call_id": "call_sys_99",
        "approved": true
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/episodes/{}/approve", ep_id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&approve_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let approve_res: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(approve_res["status"], "approved");
    assert!(approve_res["result"]["output"].as_str().unwrap().contains("Executed command"));

    // Verify pending approval was cleared
    let finalized_ep = store.get_episode(&ep_id).await.unwrap().unwrap();
    assert!(!finalized_ep.pending_approvals.contains_key("call_sys_99"));
}

#[tokio::test]
async fn test_auto_approve_tool_execution_loop() {
    let adapter = MockAdapter::new();
    // 1st step: model calls calculator
    adapter.add_behavior(MockBehavior::ToolCall {
        id: "call_calc_1".to_string(),
        name: "calculator".to_string(),
        arguments: r#"{"expression":"100 + 45"}"#.to_string(),
    });
    // 2nd step: model responds with the final answer
    adapter.add_behavior(MockBehavior::TextReply("The sum is 145.".to_string()));

    let (app, store, _tools) = setup_test_app(adapter);

    let ep = Episode::new("test-model", None);
    let ep_id = ep.id.clone();
    store.create_episode(ep).await.unwrap();

    let turn_payload = json!({
        "message": "Calculate 100 + 45",
        "execute_tools": true
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/episodes/{}/turns", ep_id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&turn_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body);

    // Should contain tool execution started & completed events
    assert!(body_str.contains("event: tool_execution_started"));
    assert!(body_str.contains("event: tool_execution_completed"));
    assert!(body_str.contains("145"));
    assert!(body_str.contains("event: turn_completed"));

    // Verify episode in store has recorded the tool results
    let updated = store.get_episode(&ep_id).await.unwrap().unwrap();
    let leaf_node = updated.nodes.get(updated.active_leaf_id.as_ref().unwrap()).unwrap();
    assert_eq!(leaf_node.tool_results.len(), 1);
    assert_eq!(leaf_node.tool_results[0].output, "145");
}

#[tokio::test]
async fn test_fork_via_api() {
    let adapter = MockAdapter::new();
    let (app, store, _tools) = setup_test_app(adapter);

    let mut ep = Episode::new("test-model", None);
    let n1 = ep.append_turn(
        Some(Message::user("Turn 1")),
        Some(Message::assistant("Ans 1")),
        vec![],
        vec![],
    );
    let _n2 = ep.append_turn(
        Some(Message::user("Turn 2")),
        Some(Message::assistant("Ans 2")),
        vec![],
        vec![],
    );
    let ep_id = ep.id.clone();
    store.create_episode(ep).await.unwrap();

    // Fork back to n1 via POST /v1/episodes/{id}/fork
    let fork_payload = json!({ "from_node_id": n1 });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/episodes/{}/fork", ep_id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&fork_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let ep_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(ep_json["active_leaf_id"], n1);
}

#[tokio::test]
async fn test_input_guardrail_rejects_oversized_payload() {
    let adapter = MockAdapter::new();
    let (app, store, _tools) = setup_test_app(adapter);

    let ep = Episode::new("test-model", None);
    let ep_id = ep.id.clone();
    store.create_episode(ep).await.unwrap();

    // Payload exceeding max 32,000 chars
    let huge_text = "A".repeat(40_000);
    let turn_payload = json!({
        "message": huge_text,
        "execute_tools": false
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/episodes/{}/turns", ep_id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&turn_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_tool_guardrail_output_truncation_end_to_end() {
    let adapter = MockAdapter::new();
    adapter.add_behavior(MockBehavior::ToolCall {
        id: "call_verbose_1".to_string(),
        name: "verbose_tool".to_string(),
        arguments: "{}".to_string(),
    });
    adapter.add_behavior(MockBehavior::TextReply("Finished.".to_string()));

    let (app, store, tools) = setup_test_app(adapter);

    // Register a tool that returns 50,000 characters
    tools.register_tool(
        episod::models::ToolDefinition::new_function("verbose_tool", "Returns huge data", json!({})),
        episod::ToolPolicy::AutoApprove,
        |_| async { Ok("DATA_BLOCK_".repeat(5000)) }, // 55,000 chars
    );

    let ep = Episode::new("test-model", None);
    let ep_id = ep.id.clone();
    store.create_episode(ep).await.unwrap();

    let turn_payload = json!({
        "message": "Run verbose tool",
        "execute_tools": true
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/episodes/{}/turns", ep_id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&turn_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body);

    // Output must be truncated by guardrail
    assert!(body_str.contains("output truncated by guardrail"));

    let updated = store.get_episode(&ep_id).await.unwrap().unwrap();
    let leaf_node = updated.nodes.get(updated.active_leaf_id.as_ref().unwrap()).unwrap();
    assert!(leaf_node.tool_results[0].output.len() <= 8500);
}

#[tokio::test]
async fn test_parallel_tool_calling_execution() {
    let adapter = MockAdapter::new();
    // Step 1: Model emits TWO tool calls in parallel!
    adapter.add_behavior(MockBehavior::ParallelToolCalls(vec![
        episod::adapter::mock::MockToolCall {
            id: "call_par_1".to_string(),
            name: "calculator".to_string(),
            arguments: r#"{"expression":"10 + 20"}"#.to_string(),
        },
        episod::adapter::mock::MockToolCall {
            id: "call_par_2".to_string(),
            name: "calculator".to_string(),
            arguments: r#"{"expression":"30 * 4"}"#.to_string(),
        },
    ]));
    // Step 2: Model returns the final answer
    adapter.add_behavior(MockBehavior::TextReply("Calculated 30 and 120.".to_string()));

    let (app, store, _tools) = setup_test_app(adapter);

    let ep = Episode::new("test-model", None);
    let ep_id = ep.id.clone();
    store.create_episode(ep).await.unwrap();

    let turn_payload = json!({
        "message": "Calculate both expressions in parallel.",
        "execute_tools": true
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/episodes/{}/turns", ep_id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&turn_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body);

    // Stream must contain BOTH tool calls and results
    assert!(body_str.contains("call_par_1"));
    assert!(body_str.contains("call_par_2"));
    assert!(body_str.contains("30"));
    assert!(body_str.contains("120"));

    // DAG leaf node must have recorded BOTH tool calls and BOTH tool results
    let updated = store.get_episode(&ep_id).await.unwrap().unwrap();
    let leaf_node = updated.nodes.get(updated.active_leaf_id.as_ref().unwrap()).unwrap();
    assert_eq!(leaf_node.tool_calls.len(), 2);
    assert_eq!(leaf_node.tool_results.len(), 2);
    assert_eq!(leaf_node.tool_results[0].output, "30");
    assert_eq!(leaf_node.tool_results[1].output, "120");
}

#[tokio::test]
async fn test_client_disconnect_cancellation() {
    let adapter = MockAdapter::new();
    // Simulate model emitting tokens
    adapter.add_behavior(MockBehavior::TextReply("A long response that should be cancelled.".to_string()));

    let (app, store, _tools) = setup_test_app(adapter);

    let ep = Episode::new("test-model", None);
    let ep_id = ep.id.clone();
    store.create_episode(ep).await.unwrap();

    let turn_payload = json!({
        "message": "Start stream",
        "execute_tools": false
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/episodes/{}/turns", ep_id))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&turn_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Simulate client abruptly dropping the stream (disconnecting before reading)
    drop(response);

    // Wait a brief moment to ensure background cancellation runs cleanly without panicking
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
}
