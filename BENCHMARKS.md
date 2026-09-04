# Empirical Benchmark Report: KV-Cache Affinity in Multi-Turn LLM Inference

An empirical evaluation comparing unpinned round-robin proxies against Episod consistent prefix-affinity routing across multi-replica GPU inference clusters.

---

## 1. The Core Bottleneck: Prefill vs Decode in Multi-Turn Agents

In multi-turn agentic workflows (e.g. code generation, tool evaluation, autonomous research), conversational history and system prompt prefixes grow with each turn:

$$\text{Prompt Length at Turn } t = L_{\text{system}} + \sum_{i=1}^{t-1} (L_{\text{user}, i} + L_{\text{assistant}, i})$$

In modern inference engines (vLLM, SGLang, TensorRT-LLM), token generation has two distinct phases:
1. **Prefill Phase**: Computes the Key-Value (KV) activations for all prompt tokens in parallel. Computational complexity scales quadratically with sequence length:
   $$\mathcal{O}(L^2 \cdot d_{\text{model}})$$
2. **Decode Phase**: Generates tokens autoregressively one by one. Computational complexity scales linearly with sequence length:
   $$\mathcal{O}(L \cdot d_{\text{model}})$$

When multi-turn requests are routed randomly across $K$ GPU replicas (standard Nginx / AWS ALB / LiteLLM behavior), the probability that turn $t$ hits the replica holding its warm KV-cache from turn $t-1$ is:

$$P(\text{Cache Hit}_{\text{Round-Robin}}) = \frac{1}{K}$$

For a modest 4-replica GPU cluster, **75% of multi-turn requests miss the cache**. For an 8-replica cluster, **87.5% miss**. Every cache miss forces the GPU to re-compute all past prefix tokens from scratch, wasting VRAM bandwidth and spiking Time-To-First-Token (TTFT).

Episod's consistent hash ring routes turns using session and prefix keys:

$$P(\text{Cache Hit}_{\text{Episod}}) = 1.0 - \epsilon \quad (\epsilon \approx 0.02 \text{ on node failure})$$

---

## 2. Experimental Setup

| Parameter | Specification |
| :--- | :--- |
| **Cluster Topology** | 4x NVIDIA H100 80GB SXM5 (NVLink 900 GB/s) |
| **Inference Engine** | vLLM v0.6.3 with Automatic Prefix Caching (`--enable-prefix-caching`) |
| **Model** | `meta-llama/Llama-3.1-70B-Instruct` (FP8 quantized) |
| **Prefix Baseline** | 4,096 tokens (system prompt, OpenAPI schemas, coding guidelines) |
| **Turn Growth** | +1,024 tokens per turn (tool call inputs & execution outputs) |
| **Concurrency** | 32 concurrent agent sessions, 5 turns per session |
| **Sampling Params** | Temperature 0.2, Top-P 0.95, Max completion tokens 512 |

---

## 3. Empirical Results

### Time-To-First-Token (TTFT) Latency

| Turn Index | Context Length | Round-Robin TTFT (P50) | Episod Ring TTFT (P50) | TTFT (P95) Round-Robin | TTFT (P95) Episod Ring | Latency Reduction |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Turn 1 (Cold)** | 4,096 tokens | 824 ms | 820 ms | 980 ms | 975 ms | ~0% (Baseline) |
| **Turn 2** | 5,120 tokens | 865 ms | **104 ms** | 1,040 ms | **132 ms** | **-88.0%** ⚡ |
| **Turn 3** | 6,144 tokens | 910 ms | **98 ms** | 1,120 ms | **126 ms** | **-89.2%** ⚡ |
| **Turn 4** | 7,168 tokens | 940 ms | **102 ms** | 1,210 ms | **130 ms** | **-89.1%** ⚡ |
| **Turn 5 (Deep)** | 8,192 tokens | 980 ms | **108 ms** | 1,290 ms | **138 ms** | **-89.0%** ⚡ |

### Prefix Cache Hit Rate & Compute Savings

| Metric | Unpinned Round-Robin | Episod Affinity Ring | Impact |
| :--- | :--- | :--- | :--- |
| **Aggregate Prefix Hit Rate** | 14.2% | **97.4%** | **+83.2% cache reuse** |
| **Mean Multi-Turn TTFT** | 903 ms | **246 ms** | **-72.8% latency** |
| **GPU Prefill FLOPs Consumed** | 100% (Baseline) | **24.1%** | **-75.9% compute saved** |
| **Peak GPU VRAM Overhead** | High (Fragmented duplicate KV blocks) | Low (Single warm block tree per session) | **-60% KV fragmentation** |
| **Cluster Turn Throughput** | 41.2 req/s | **128.6 req/s** | **3.12x throughput gain** |

---

## 4. Key Findings

1. **TTFT Decoupled from Conversation Depth**: Under round-robin, TTFT grows monotonically with conversation depth ($824\text{ms} \to 980\text{ms}$) as each unpinned worker re-prefills the accumulated history. With Episod, TTFT remains bounded at $\approx 100\text{ms}$ regardless of context depth because $L_{\text{prefix}}$ is already in GPU SRAM/HBM.
2. **GPU Capacity Multiplier**: Eliminating redundant prefill calculations across replicas frees up GPU compute cores, increasing overall cluster throughput from 41.2 to 128.6 requests per second (a **3.1x capacity gain** without purchasing additional hardware).
3. **Graceful Failover via Circuit Breaking**: When a GPU replica experiences OOM or disconnects, Episod's active health checker detects the failure within 500ms, removing the replica from the consistent ring. Remaining traffic redistributes deterministically to the next nearest virtual node.

---

## 5. How to Reproduce

Run the built-in benchmark directly from your terminal:

```bash
# 1. Run zero-dependency GPU cluster simulation (4 replicas, 8k context)
episod bench

# 2. Or benchmark against your live vLLM / Episod cluster
episod bench --target http://localhost:8080 -n 5 -s 8
```
