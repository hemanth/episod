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

In a $K$-worker cluster, round-robin routing hits the warm cache with probability $P = 1/K$ (75% miss rate on 4 workers). Episod's hash ring pins turns to guarantee $P \approx 100\%$:

| Metric | Round-Robin ($K=4$) | Episod Ring | Delta |
|:---|:---|:---|:---|
| Turn 1 TTFT (Cold) | 820 ms | 820 ms | ~0% |
| Turn 2 TTFT (Warm) | 865 ms | 104 ms | **-88.0%** |
| Turn 5 TTFT (Deep) | 980 ms | 108 ms | **-89.0%** |
| Prefix Cache Hit % | 14.2% | 97.4% | **+83.2%** |
| GPU Prefill Compute | 100% | 24.1% | **-75.9%** |

Full empirical methodology and mathematical proofs in [BENCHMARKS.md](BENCHMARKS.md).

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
storage_type = "sqlite"
sqlite_path = "episod.db"
```

## CLI

- `episod dev [--port 8080]` - zero-config local development server
- `episod bench [-n 5] [-s 4]` - benchmark KV-cache latency savings
- `episod tree <id>` - print conversational DAG tree in terminal
- `episod start [-c config.toml]` - run production gateway

## License

Apache-2.0
