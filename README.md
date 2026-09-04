# episod

The KV-cache affinity gateway for LLM inference.

```bash
curl -fsSL https://raw.githubusercontent.com/hemanth/episod/master/install.sh | sh
# or: cargo install episod
```

## Quick start

```bash
episod dev
```

Auto-detects local LLMs (Ollama, vLLM), initializes SQLite, and serves the gateway at `http://localhost:8080`.

```diff
  from openai import OpenAI

- client = OpenAI()
+ client = OpenAI(base_url="http://localhost:8080/v1", api_key="none")

  response = client.chat.completions.create(
      model="llama3.1:8b",
      messages=[{"role": "user", "content": "Analyze system telemetry."}],
+     extra_headers={"x-episod-session": "agent-session-42"},
  )
```

Point `base_url` to episod and tag requests with `x-episod-session` to pin KV-cache and turn history across GPU replicas.

## Benchmark

```bash
episod bench
```

```text
  Turn        Observed TTFT      Total Latency   Status
  ────────────────────────────────────────────────────────────
  Turn [1]           120 ms             180 ms   Cold Prefill (Miss)
  Turn [2]            14 ms              24 ms   Warm Pinned (100% Hit)
  Turn [3]            11 ms              20 ms   Warm Pinned (100% Hit)
```

In a $K$-worker cluster, round-robin routing hits the warm cache with probability $P = 1/K$ (75% miss rate on 4 workers). Consistent hashing pins sessions to guarantee $P \approx 100\%$. Analysis in [BENCHMARKS.md](BENCHMARKS.md).

## Tool execution

```bash
# Gateway executes tools or yields them directly to the client:
curl -X POST http://localhost:8080/v1/episodes/:id/turns \
  -H "Content-Type: application/json" \
  -d '{"message": "Run query", "execute_tools": false}'
```

Set `execute_tools: false` for client-delegated execution in your own services, or `true` for server-side autonomous loops with Human-in-the-Loop approval gates.

## Conversational DAG

```bash
# Fork an earlier turn without mutating history
curl -X POST http://localhost:8080/v1/episodes/:id/fork \
  -H "Content-Type: application/json" \
  -d '{"from_node_id": "turn_01"}'
```

Turns form an immutable directed acyclic graph in SQLite WAL. Replay, fork, and branch counterfactual reasoning trees.

## OpenAI Responses API

```bash
curl -X POST http://localhost:8080/v1/responses \
  -H "Content-Type: application/json" \
  -d '{"model": "llama3.1:8b", "input": "Next turn", "previous_response_id": "resp_01"}'
```

Stateful conversations via `previous_response_id` with zero prompt re-transmission over the wire.

## Tree view

```bash
episod tree ep_b7e914df088741348123abc456
```

Renders conversational turn trees in the terminal with TTFT, tokens, and cache hit metrics.

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
# Options: "sqlite" (default) | "redis" | "http" (webhook) | "memory"
storage_type = "sqlite"
database_path = "episod.db"

# Or connect to Redis for multi-pod Kubernetes clusters:
# storage_type = "redis"
# redis_url = "redis://127.0.0.1:6379"

# Or bridge to your existing Postgres / Supabase stack via HTTP webhook:
# storage_type = "http"
# endpoint_url = "https://api.internal/episodes"
```

Supports SQLite WAL (local dev), Redis (multi-pod clusters), and HTTP webhooks (to bridge existing Postgres, DynamoDB, or Supabase backends with zero Rust code).

## CLI

- `episod dev [--port 8080]` - zero-config local development server
- `episod bench [-n 5] [-s 4]` - benchmark KV-cache latency savings
- `episod tree <id>` - print conversational DAG tree in terminal
- `episod start [-c config.toml]` - run production gateway

## License

Apache-2.0
