# KV-Cache Affinity in Multi-Turn LLM Inference

An analytical and empirical analysis of why unpinned round-robin proxies destroy prefix cache reuse across multi-replica inference clusters, and how Episod solves it.

---

## 1. The Mathematical Foundation: Prefill vs Decode

In multi-turn agentic workflows (tool use, coding agents, sequential reasoning), prompts accumulate history with every turn:

$$\text{Prompt Length at Turn } t = L_{\text{system}} + \sum_{i=1}^{t-1} (L_{\text{user}, i} + L_{\text{assistant}, i})$$

Modern inference engines (vLLM, SGLang, Ollama) divide request processing into two distinct phases:

1. **Prefill Phase**: Computes Key-Value (KV) activations for all prompt tokens in parallel. Computational complexity scales quadratically with sequence length:
   $$\mathcal{O}(L^2 \cdot d_{\text{model}})$$
2. **Decode Phase**: Generates output tokens autoregressively one by one. Computational complexity scales linearly with sequence length:
   $$\mathcal{O}(L \cdot d_{\text{model}})$$

When an engine supports **Automatic Prefix Caching (APC)** or **RadixAttention**, identical prompt prefixes that already exist in GPU memory skip the prefill computation entirely, reducing TTFT (Time-To-First-Token) to essentially the first decode step.

---

## 2. The Unpinned Routing Failure Mode

When multiple GPU workers are placed behind standard load balancers (Nginx, AWS ALB, or basic LiteLLM proxies), requests are routed via round-robin or least-connections:

In a cluster of $K$ workers, the probability that turn $t$ hits the exact replica holding the warm KV cache from turn $t-1$ is:

$$P(\text{Cache Hit}_{\text{Round-Robin}}) = \frac{1}{K}$$

| Cluster Size ($K$) | Probability of Cache Hit | Probability of Cache Miss (Re-prefill) |
| :--- | :--- | :--- |
| **2 workers** | 50.0% | 50.0% |
| **4 workers** | 25.0% | **75.0%** |
| **8 workers** | 12.5% | **87.5%** |
| **16 workers** | 6.25% | **93.75%** |

In an 8-replica cluster, **almost 9 out of 10 requests hit a cold replica**, forcing the GPU to re-compute all previous tokens from scratch, wasting compute cycles and multiplying TTFT.

---

## 3. How Episod Solves It: Consistent Prefix Affinity

Episod replaces round-robin proxying with a **consistent hash ring** that routes requests using session keys (`x-episod-session`) and shared prompt prefix hashes:

$$P(\text{Cache Hit}_{\text{Episod}}) \approx 1.0 - \epsilon$$

Where $\epsilon$ is the node churn rate (near zero under stable operation).

### Simulated Multi-Turn Latency Comparison (4-Worker Cluster, 8k Context)

Modeled on published RadixAttention / vLLM prefix cache prefill latency curves:

| Metric | Unpinned Round-Robin ($K=4$) | Episod Affinity Ring | Delta |
| :--- | :--- | :--- | :--- |
| **Turn 1 TTFT (Cold)** | 820 ms | 820 ms | ~0% (Initial prefill) |
| **Turn 2 TTFT (Warm)** | 865 ms | 104 ms | **-88.0%** ⚡ |
| **Turn 3 TTFT (Warm)** | 910 ms | 98 ms | **-89.2%** ⚡ |
| **Turn 4 TTFT (Warm)** | 940 ms | 102 ms | **-89.1%** ⚡ |
| **Turn 5 TTFT (Deep)** | 980 ms | 108 ms | **-89.0%** ⚡ |
| **Mean Multi-Turn TTFT** | 903 ms | 246 ms | **-72.8%** 🚀 |
| **Prefix Cache Hit Rate** | 14.2% | 97.4% | **+83.2%** 🔥 |
| **Prefill Compute Used** | 100% (Baseline) | 24.1% | **-75.9% compute saved** 💰 |

---

## 4. Verifying On Your Own Setup

Run the built-in benchmark tool directly from the CLI:

```bash
# 1. Run the analytical 4-worker cluster simulation:
episod bench

# 2. Or benchmark live against your own running instance:
episod bench --target http://localhost:8080 -n 5 -s 4
```
