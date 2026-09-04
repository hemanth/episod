# Episod (`episod`)

> **The stateful agentic gateway for LLM inference.**
> Giving stateless inference engines (vLLM, SGLang, Ollama, LiteLLM) episodic conversational memory, KV-cache prefix affinity, and hybrid tool orchestration.

[![Build & Test](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Rust 2024](https://img.shields.io/badge/rust-2024-orange.svg)]()
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-blue.svg)]()

---

## 💡 Why Episod?

Stateless LLM engines (like vLLM, SGLang, and Ollama) are built for raw token throughput, not stateful agent loops. Today, building multi-turn agents requires client apps to constantly replay the entire prompt history, recompute KV-cache tokens across random GPU replicas, and manually orchestrate tools and human approvals.

**`episod`** sits as an ultra-low latency (<0.2ms) gateway in front of any generic inference endpoint, providing:

1. **Episodic Memory & DAG of Turns**: Conversations form a Directed Acyclic Graph (DAG) of nodes. Replay, fork, branch, and rollback without mutating history.
2. **KV-Cache / Prefix Affinity Routing**: Consistent hash ring pins session turns and shared prompt prefixes to the same GPU replica, hitting 100% prefix cache rates.
3. **Pluggable Upstream Adapters**: Speaks OpenAI-compatible streaming SSE (`/v1/chat/completions`) out of the box; easily extensible to Anthropic, Gemini, or custom gRPC endpoints.
4. **Hybrid Tool & HITL Engine**:
   - **Auto-Approve**: Runs tools server-side in a multi-turn autonomous loop until completion.
   - **Human-in-the-Loop (HITL)**: Pauses streams on sensitive actions, emits an `approval_required` event, and waits for operator sign-off.
   - **Client-Delegated**: Yields tool calls directly to the client when desired.
5. **Real-time SSE Agentic Protocol**: Granular streaming events for tokens, tool calls, tool execution, and lifecycle status.
6. **Built-in Production Guardrails & Memory Limits**:
   - **Context Budget Manager**: Heuristic token estimator + sliding-window compaction that preserves system prompts and recent turns, preventing 400 Context Length Exceeded errors on deep conversations.
   - **Tool Execution Guardrails**: Strict timeouts (`tokio::time::timeout`) and output character caps to prevent slow/hanging tools or massive JSON context bloat.
   - **Input & Turn Count Protection**: Rejects oversized payloads (>32KB) and prevents runaway episode spam (>500 turns/episode).
   - **Bounded RAM & LRU Eviction**: In-memory state store with capacity thresholds and TTL-based background eviction to prevent memory leaks in production.

---

## 🏛️ Architecture

```
                          ┌────────────────────────┐
                          │  Client Application    │
                          │ (Python / TS / cURL)   │
                          └───────────┬────────────┘
                                      │ REST + SSE Event Stream
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           episod Gateway (Rust)                             │
│                                                                             │
│  ┌───────────────────────┐             ┌─────────────────────────────────┐  │
│  │      Axum Engine      │             │         Episodic Engine         │  │
│  │   REST / SSE Routes   │◄───────────►│  - DAG of Turns (Nodes & Edges) │  │
│  │   Event Streamer      │             │  - Context Rehydration          │  │
│  └──────────┬────────────┘             └────────────────┬────────────────┘  │
│             │                                           │                   │
│             ▼                                           ▼                   │
│  ┌───────────────────────┐             ┌─────────────────────────────────┐  │
│  │    Affinity Router    │             │      Pluggable State Store      │  │
│  │  Consistent Hash Ring │             │  - In-Memory (DashMap)          │  │
│  │  (KV Cache Affinity)  │             │  - Redis / SQLite               │  │
│  └──────────┬────────────┘             └─────────────────────────────────┘  │
│             │                                                               │
│             ▼                                           ▲                   │
│  ┌───────────────────────┐                              │                   │
│  │   Upstream Adapters   │             ┌────────────────┴────────────────┐  │
│  │ - OpenAI-compatible   │             │    Hybrid Tool Orchestrator     │  │
│  │   (vLLM/SGLang/Ollama)│             │  - Built-in & MCP Tool Registry │  │
│  │ - (Anthropic/Gemini)  │             │  - Policy & HITL Approval Gate  │  │
│  └──────────┬────────────┘             └─────────────────────────────────┘  │
└─────────────┼───────────────────────────────────────────────────────────────┘
              │ Streaming Inference (HTTP/SSE)
              ▼
   ┌──────────────────────┐  ┌──────────────────────┐  ┌──────────────────────┐
   │ vLLM / SGLang Node 1 │  │ vLLM / SGLang Node 2 │  │ Ollama / OpenAI / etc│
   └──────────────────────┘  └──────────────────────┘  └──────────────────────┘
```

---

## 🚀 Quick Start

### 1. Instant Zero-Config Dev Mode (`episod dev`)

Auto-detects local Ollama (`http://localhost:11434`) or vLLM (`http://localhost:8000`), sets up embedded SQLite (`episod.db`), and starts the gateway with a live web inspector:

```bash
cargo build --release
./target/release/episod dev
```
```text
  ⚡ Episod Dev Gateway Started!
  -------------------------------------------------------------
  • Detected Upstream: Ollama on http://127.0.0.1:11434
  • Persistent Store:  SQLite (WAL mode) -> episod.db
  • Web Inspector:     http://localhost:8080/dashboard
  • OpenAI Responses:  http://localhost:8080/v1/responses
  • Chat Completions:  http://localhost:8080/v1/chat/completions
  -------------------------------------------------------------
```

### 2. Embedded Web Inspector (`/dashboard`)

Visit `http://localhost:8080/dashboard` in any browser to:
- 🌿 Explore the full DAG tree of conversation turns across branches.
- ⏱️ Inspect Time-To-First-Token (TTFT), execution latency, and token usages per turn.
- 🔀 Click *"Fork Branch"* on any past turn to explore alternate reasoning paths.
- 🛡️ One-click Human-in-the-Loop (HITL) tool execution approval drawer.

### 3. Terminal DAG Visualizer (`episod tree`)

View conversational turn trees directly in the terminal like `git log --graph`:

```bash
./target/release/episod tree ep_b7e914df088741348123abc456
```
```text
🌿 Episode: ep_b7e914df088741348123abc456 (Model: llama3.1:8b)
│  ⚙️ System Prompt: "You are an autonomous research assistant."
│
├── [1] Node ID: turn_aeec8e85a3b44e3e9c67b7ed79c17693
│   👤 User: "What is 7 * 8?"
│   🤖 Assistant: "7 multiplied by 8 is 56."
│   📊 Metrics: TTFT: 142ms | Total: 412ms | Tokens: 64 tok | Cache: 100%
│
└── [2] Node ID: turn_4f04ea90d5824572a6c60192a2476be9 🌿 [Active Leaf]
    ↳ Parent: turn_aeec8e85a3b44e3e9c67b7ed79c17693
    👤 User: "And add 4 to that."
    🤖 Assistant: "56 + 4 equals 60."
    📊 Metrics: TTFT: 108ms | Total: 280ms | Tokens: 88 tok | Cache: 100%
```

### 4. Create an Episode (Session via API)

```bash
curl -X POST http://localhost:8080/v1/episodes \
  -H "Content-Type: application/json" \
  -d '{
    "model": "meta-llama/Llama-3.1-8B-Instruct",
    "system_prompt": "You are an autonomous research assistant with access to tools."
  }'
```

Response:
```json
{
  "id": "ep_b7e914df088741348123abc456",
  "model": "meta-llama/Llama-3.1-8B-Instruct",
  "system_prompt": "You are an autonomous research assistant with access to tools.",
  "root_node_id": null,
  "active_leaf_id": null,
  "nodes": {},
  "pending_approvals": {},
  "created_at": 1756980000000,
  "updated_at": 1756980000000
}
```

### 3. Submit a Turn & Stream Events (SSE)

```bash
curl -N -X POST http://localhost:8080/v1/episodes/ep_b7e914df088741348123abc456/turns \
  -H "Content-Type: application/json" \
  -d '{
    "message": "What is 42 * 15?",
    "execute_tools": true
  }'
```

Stream Output:
```text
event: turn_started
data: {"event":"turn_started","data":{"episode_id":"ep_b7e...","turn_id":"turn_01...","parent_node_id":null}}

event: tool_call_delta
data: {"event":"tool_call_delta","data":{"arguments_delta":"{\"expression\":\"42 * 15\"}","index":0,"name":"calculator","tool_call_id":"call_123","turn_id":"turn_01..."}}

event: tool_execution_started
data: {"event":"tool_execution_started","data":{"tool_call_id":"call_123","tool_name":"calculator","turn_id":"turn_01..."}}

event: tool_execution_completed
data: {"event":"tool_execution_completed","data":{"is_error":false,"output":"630","tool_call_id":"call_123","tool_name":"calculator","turn_id":"turn_01..."}}

event: token_delta
data: {"event":"token_delta","data":{"delta":"The result of 42 * 15 is 630.","turn_id":"turn_01..."}}

event: turn_completed
data: {"event":"turn_completed","data":{"episode_id":"ep_b7e...","finish_reason":"stop","node_id":"turn_01...","turn_id":"turn_01..."}}
```

### 4. Human-In-The-Loop Approval (HITL)

When a model triggers a sensitive tool configured with `RequireApproval` policy (e.g. `system_command`):

1. The stream pauses and emits an `approval_required` event:
```text
event: approval_required
data: {"event":"approval_required","data":{"arguments":{"command":"rm -rf /tmp/data"},"episode_id":"ep_...","tool_call_id":"call_sys_1","tool_name":"system_command"}}
```

2. The human operator reviews and submits their approval:
```bash
curl -X POST http://localhost:8080/v1/episodes/ep_b7e.../approve \
  -H "Content-Type: application/json" \
  -d '{
    "tool_call_id": "call_sys_1",
    "approved": true
  }'
```

---

### 5. Branching & Conversation Forking

Fork a conversation from any historical turn node to explore counterfactual agent paths:

```bash
curl -X POST http://localhost:8080/v1/episodes/ep_b7e.../fork \
  -H "Content-Type: application/json" \
  -d '{
    "from_node_id": "turn_historical_01"
  }'
```

---

## 🐍 Python Client Example

```python
import httpx
import json

client = httpx.Client(base_url="http://localhost:8080")

# 1. Create episode
ep = client.post("/v1/episodes", json={
    "model": "meta-llama/Llama-3.1-70B-Instruct",
    "system_prompt": "You are an analytical assistant."
}).json()

episode_id = ep["id"]

# 2. Stream multi-turn response with automatic tool loops
with client.stream("POST", f"/v1/episodes/{episode_id}/turns", json={
    "message": "Calculate 1500 / 12 and explain the answer.",
    "execute_tools": True
}) as response:
    for line in response.iter_lines():
        if line.startswith("data: "):
            event = json.loads(line[6:])
            if event["event"] == "token_delta":
                print(event["data"]["delta"], end="", flush=True)
            elif event["event"] == "tool_execution_completed":
                print(f"\n[Tool {event['data']['tool_name']} executed: {event['data']['output']}]")
```

---

## 🔄 OpenAI API Drop-In Compatibility

Episod provides native drop-in endpoints for existing OpenAI client SDKs and tools:

### 1. OpenAI Responses API (`POST /v1/responses`)

Stateful conversations with automatic DAG rehydration using `previous_response_id`:

```bash
# First turn (Streaming SSE)
curl -N -X POST http://localhost:8080/v1/responses \
  -H "Content-Type: application/json" \
  -d '{
    "model": "meta-llama/Llama-3.1-8B-Instruct",
    "input": "Summarize the theory of general relativity in two sentences.",
    "stream": true
  }'

# Second turn chained with previous_response_id (Non-streaming)
curl -X POST http://localhost:8080/v1/responses \
  -H "Content-Type: application/json" \
  -d '{
    "model": "meta-llama/Llama-3.1-8B-Instruct",
    "input": "Now explain it like I am 5.",
    "previous_response_id": "resp_018f3a9e...",
    "stream": false
  }'
```

### 2. Standard Chat Completions (`POST /v1/chat/completions`)

Drop-in replacement for OpenAI SDKs (`openai.OpenAI(base_url="http://localhost:8080/v1")`):

```python
from openai import OpenAI

client = OpenAI(base_url="http://localhost:8080/v1", api_key="none")

response = client.chat.completions.create(
    model="meta-llama/Llama-3.1-8B-Instruct",
    messages=[{"role": "user", "content": "Hello!"}],
    extra_headers={"x-episod-session": "optional-session-id"} # Pin session & cache affinity
)
print(response.choices[0].message.content)
```

---

## 📊 Telemetry & KV-Cache Monitoring

Every turn completed event includes performance and token telemetry:

```json
{
  "event": "turn_completed",
  "data": {
    "node_id": "turn_389271a...",
    "timing": {
      "ttft_ms": 142,
      "total_ms": 684
    },
    "usage": {
      "prompt_tokens": 128,
      "completion_tokens": 64,
      "total_tokens": 192,
      "cached_tokens": 96,
      "cache_hit_rate": 0.75
    }
  }
}
```

---

## ⚙️ Configuration (`episod.toml`)

```toml
[server]
host = "0.0.0.0"
port = 8080

[[upstreams]]
id = "vllm-node-1"
url = "http://10.0.0.1:8000/v1"
weight = 1

[[upstreams]]
id = "vllm-node-2"
url = "http://10.0.0.2:8000/v1"
weight = 1

[storage]
storage_type = "memory"
```

---

## 🧪 Testing

Episod comes with a comprehensive suite of unit and end-to-end integration tests (DAG tree validation, router affinity failover, tool execution loops, and HITL streaming):

```bash
cargo test
```

---

## 📜 License

Licensed under the Apache License, Version 2.0.
