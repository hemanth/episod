# episod

The stateful agentic gateway for LLM inference.

```bash
cargo install episod
```

## Quick start

```bash
episod dev
```

Auto-detects local LLMs (Ollama, vLLM), initializes SQLite, and serves the gateway at `http://localhost:8080`.

```python
import httpx, json

client = httpx.Client(base_url="http://localhost:8080")
ep = client.post("/v1/episodes", json={"model": "llama3.1:8b"}).json()

with client.stream("POST", f"/v1/episodes/{ep['id']}/turns", json={
    "message": "Calculate 42 * 15 and explain the answer.",
    "execute_tools": True
}) as res:
    for line in res.iter_lines():
        if line.startswith("data: "):
            event = json.loads(line[6:])
            if event.get("event") == "token_delta":
                print(event["data"]["delta"], end="", flush=True)
```

Creates an episode and streams token deltas while running tool execution loops server-side.

## Conversational DAG

```bash
# Branch from an earlier turn without mutating history
curl -X POST http://localhost:8080/v1/episodes/:id/fork \
  -H "Content-Type: application/json" \
  -d '{"from_node_id": "turn_01"}'
```

Every conversation turn is an immutable node in a directed acyclic graph. Replay, fork, and explore counterfactual reasoning trees.

## Tree view

```bash
episod tree ep_b7e914df088741348123abc456
```

Renders conversational turn trees in the terminal with token counts, cache hit rates, and TTFT latency metrics.

## KV-cache affinity

```python
from openai import OpenAI

client = OpenAI(base_url="http://localhost:8080/v1", api_key="none")
response = client.chat.completions.create(
    model="llama3.1:8b",
    messages=[{"role": "user", "content": "Hello"}],
    extra_headers={"x-episod-session": "agent-42"}
)
```

Consistent hash ring routes turns with the same session key to the same GPU worker to maximize prefix cache hits.

## Tool execution & HITL

```bash
# Approve a pending human-in-the-loop action
curl -X POST http://localhost:8080/v1/episodes/:id/approve \
  -H "Content-Type: application/json" \
  -d '{"tool_call_id": "call_123", "approved": true}'
```

Sensitive tools pause the SSE stream with an `approval_required` event and resume upon operator sign-off.

## OpenAI Responses API

```bash
# Stateful multi-turn via previous_response_id
curl -X POST http://localhost:8080/v1/responses \
  -H "Content-Type: application/json" \
  -d '{
    "model": "llama3.1:8b",
    "input": "Continue from earlier",
    "previous_response_id": "resp_01",
    "stream": false
  }'
```

Drop-in compatibility with OpenAI's stateful Responses API with zero prompt re-transmission over the wire.

## Web inspector

Available at `http://localhost:8080/dashboard` for interactive turn tree traversal, fork triggering, and live HITL approvals.

## Configuration

```toml
# episod.toml
[server]
host = "0.0.0.0"
port = 8080

[[upstreams]]
id = "vllm-1"
url = "http://10.0.0.1:8000/v1"
weight = 1

[storage]
storage_type = "sqlite"
sqlite_path = "episod.db"
```

## CLI

- `episod dev [--port 8080] [--db episod.db]` - start zero-config development server
- `episod start [-c episod.toml]` - run production gateway from config
- `episod tree <id>` - print DAG tree for an episode

## License

Apache-2.0
