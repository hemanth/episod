use axum::response::Html;

pub async fn render_landing_page() -> Html<&'static str> {
    Html(LANDING_PAGE_HTML)
}

const LANDING_PAGE_HTML: &str = r###"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Episod — Stateful Memory & Affinity Gateway for LLMs</title>
  <style>
    :root {
      --ds-background-100: #000000;
      --ds-background-200: #0a0a0a;
      --ds-card-bg: #111111;
      --ds-card-hover: #161616;
      --ds-recessed: #181818;
      
      --ds-border-subtle: rgba(255, 255, 255, 0.08);
      --ds-border-default: rgba(255, 255, 255, 0.14);
      --ds-border-active: rgba(255, 255, 255, 0.28);
      
      --ds-text-primary: #ededed;
      --ds-text-secondary: #888888;
      --ds-text-muted: #555555;
      
      --ds-accent-blue: #0072f5;
      --ds-focus-ring: 0 0 0 2px #000000, 0 0 0 4px #0072f5;
      
      --ds-status-green: #45a557;

      --font-sans: "Geist", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      --font-mono: "Geist Mono", ui-monospace, SFMono-Regular, "Roboto Mono", Menlo, monospace;
    }

    * { box-sizing: border-box; margin: 0; padding: 0; -webkit-font-smoothing: antialiased; }

    body {
      background: var(--ds-background-100);
      color: var(--ds-text-primary);
      font-family: var(--font-sans);
      font-size: 14px;
      line-height: 1.5;
      overflow-x: hidden;
    }

    /* Navigation */
    nav {
      position: sticky;
      top: 0;
      z-index: 100;
      background: rgba(0, 0, 0, 0.8);
      backdrop-filter: blur(12px);
      border-bottom: 1px solid var(--ds-border-subtle);
      height: 60px;
      padding: 0 32px;
      display: flex;
      align-items: center;
      justify-content: space-between;
    }

    .nav-brand {
      display: flex;
      align-items: center;
      gap: 12px;
    }

    .brand-logo {
      font-size: 14px;
      font-weight: 600;
      letter-spacing: -0.03em;
      color: var(--ds-text-primary);
      text-transform: uppercase;
    }

    .nav-tag {
      font-size: 11px;
      font-family: var(--font-mono);
      color: var(--ds-text-muted);
      border: 1px solid var(--ds-border-subtle);
      padding: 2px 8px;
      border-radius: 4px;
    }

    .nav-links {
      display: flex;
      align-items: center;
      gap: 20px;
    }

    .nav-link {
      color: var(--ds-text-secondary);
      text-decoration: none;
      font-size: 13px;
      transition: color 0.15s ease;
    }
    .nav-link:hover { color: var(--ds-text-primary); }

    /* Buttons */
    .btn-solid {
      background: #ededed;
      color: #000000;
      text-decoration: none;
      font-size: 13px;
      font-weight: 500;
      padding: 8px 16px;
      border-radius: 6px;
      display: inline-flex;
      align-items: center;
      gap: 6px;
      transition: opacity 0.15s ease;
    }
    .btn-solid:hover { opacity: 0.9; }

    .btn-ghost {
      background: transparent;
      border: 1px solid var(--ds-border-subtle);
      color: var(--ds-text-primary);
      text-decoration: none;
      font-size: 13px;
      font-weight: 500;
      padding: 8px 16px;
      border-radius: 6px;
      display: inline-flex;
      align-items: center;
      gap: 6px;
      transition: all 0.15s ease;
    }
    .btn-ghost:hover {
      background: var(--ds-card-hover);
      border-color: var(--ds-border-default);
    }

    /* Hero Section */
    .hero {
      max-width: 960px;
      margin: 0 auto;
      padding: 96px 24px 64px 24px;
      text-align: center;
    }

    .hero-badge {
      display: inline-flex;
      align-items: center;
      gap: 8px;
      border: 1px solid var(--ds-border-subtle);
      background: var(--ds-background-200);
      padding: 4px 12px;
      border-radius: 20px;
      font-size: 12px;
      color: var(--ds-text-secondary);
      margin-bottom: 24px;
    }

    .status-dot {
      width: 6px;
      height: 6px;
      border-radius: 50%;
      background: var(--ds-status-green);
    }

    .hero-title {
      font-size: 52px;
      font-weight: 600;
      line-height: 1.05;
      letter-spacing: -2.28px;
      color: var(--ds-text-primary);
      margin-bottom: 20px;
    }

    .hero-desc {
      font-size: 18px;
      font-weight: 400;
      line-height: 1.5;
      color: var(--ds-text-secondary);
      max-width: 680px;
      margin: 0 auto 32px auto;
    }

    .hero-cta-group {
      display: flex;
      justify-content: center;
      gap: 12px;
      margin-bottom: 56px;
    }

    /* Terminal / Code Window */
    .code-window {
      max-width: 780px;
      margin: 0 auto;
      background: var(--ds-card-bg);
      border: 1px solid var(--ds-border-subtle);
      border-radius: 8px;
      overflow: hidden;
      text-align: left;
    }

    .code-topbar {
      padding: 10px 16px;
      background: var(--ds-background-200);
      border-bottom: 1px solid var(--ds-border-subtle);
      display: flex;
      justify-content: space-between;
      align-items: center;
    }

    .code-tabs {
      display: flex;
      gap: 16px;
      font-size: 12px;
    }

    .code-tab {
      color: var(--ds-text-muted);
      cursor: pointer;
      padding-bottom: 2px;
    }
    .code-tab.active {
      color: var(--ds-text-primary);
      border-bottom: 1px solid var(--ds-text-primary);
    }

    .code-body {
      padding: 20px 24px;
      font-family: var(--font-mono);
      font-size: 12px;
      line-height: 1.6;
      color: #d1d5db;
      overflow-x: auto;
      white-space: pre;
    }

    .hl-cmd { color: #52aeff; }
    .hl-kw { color: #6cda75; }
    .hl-str { color: #ff990a; }
    .hl-comment { color: var(--ds-text-muted); }

    /* Bento Grid */
    .bento-section {
      max-width: 960px;
      margin: 0 auto;
      padding: 64px 24px;
    }

    .section-title {
      font-size: 28px;
      font-weight: 600;
      letter-spacing: -1.28px;
      margin-bottom: 8px;
    }

    .section-subtitle {
      font-size: 15px;
      color: var(--ds-text-secondary);
      margin-bottom: 40px;
    }

    .bento-grid {
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: 16px;
    }

    @media (max-width: 768px) {
      .bento-grid { grid-template-columns: 1fr; }
      .hero-title { font-size: 36px; letter-spacing: -1.28px; }
    }

    .bento-card {
      background: var(--ds-card-bg);
      border: 1px solid var(--ds-border-subtle);
      border-radius: 8px;
      padding: 24px;
      transition: all 0.15s ease;
      display: flex;
      flex-direction: column;
      justify-content: space-between;
    }
    .bento-card:hover {
      background: var(--ds-card-hover);
      border-color: var(--ds-border-default);
    }

    .bento-card-title {
      font-size: 15px;
      font-weight: 500;
      color: var(--ds-text-primary);
      margin-bottom: 8px;
    }

    .bento-card-desc {
      font-size: 13px;
      color: var(--ds-text-secondary);
      line-height: 1.5;
    }

    .bento-tag {
      font-family: var(--font-mono);
      font-size: 11px;
      color: var(--ds-text-muted);
      margin-top: 20px;
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }

    /* Footer */
    footer {
      border-top: 1px solid var(--ds-border-subtle);
      padding: 32px 24px;
      text-align: center;
      color: var(--ds-text-muted);
      font-size: 12px;
      background: var(--ds-background-200);
      margin-top: 64px;
    }
  </style>
</head>
<body>

  <!-- Navigation -->
  <nav>
    <div class="nav-brand">
      <div class="brand-logo">EPISOD</div>
      <div class="nav-tag">v0.1.0</div>
    </div>
    <div class="nav-links">
      <a href="#features" class="nav-link">Architecture</a>
      <a href="/docs" class="nav-link">API Reference</a>
      <a href="/dashboard" class="btn-solid">Open Dashboard</a>
    </div>
  </nav>

  <!-- Hero -->
  <section class="hero">
    <div class="hero-badge">
      <span class="status-dot"></span>
      <span>The stateful agentic gateway for LLM inference</span>
    </div>

    <h1 class="hero-title">The Stateful Agentic Gateway for LLM Inference</h1>
    <p class="hero-desc">
      Empower stateless LLM engines like vLLM, SGLang, and Ollama with conversational DAGs, prefix-cache affinity routing, and hybrid Human-in-the-Loop tool execution.
    </p>

    <div class="hero-cta-group">
      <a href="/dashboard" class="btn-solid">Explore Live Dashboard &rarr;</a>
      <a href="/docs" class="btn-ghost">View API Reference &rarr;</a>
    </div>

    <!-- Code Block -->
    <div class="code-window">
      <div class="code-topbar">
        <div class="code-tabs">
          <div class="code-tab active" onclick="switchCode('sdk')">OpenAI Drop-In</div>
          <div class="code-tab" onclick="switchCode('responses')">Responses API</div>
          <div class="code-tab" onclick="switchCode('cli')">Dev CLI</div>
        </div>
        <div style="font-family: var(--font-mono); font-size: 11px; color: var(--ds-text-muted);">http://localhost:8080</div>
      </div>
      <div class="code-body" id="code-content">
<span class="hl-comment"># Drop-in compatibility with standard OpenAI clients:</span>
<span class="hl-kw">from</span> openai <span class="hl-kw">import</span> OpenAI

client = OpenAI(base_url=<span class="hl-str">"http://localhost:8080/v1"</span>, api_key=<span class="hl-str">"none"</span>)

response = client.chat.completions.create(
    model=<span class="hl-str">"meta-llama/Llama-3.1-8B-Instruct"</span>,
    messages=[{<span class="hl-str">"role"</span>: <span class="hl-str">"user"</span>, <span class="hl-str">"content"</span>: <span class="hl-str">"Explain quantum computing."</span>}],
    extra_headers={<span class="hl-str">"x-episod-session"</span>: <span class="hl-str">"session_user_01"</span>} <span class="hl-comment"># 100% prefix cache affinity</span>
)
print(response.choices[0].message.content)
      </div>
    </div>
  </section>

  <!-- Bento Features Grid -->
  <section class="bento-section" id="features">
    <div class="section-title">Engineered for Agent Systems</div>
    <div class="section-subtitle">A high-performance state layer that sits between your application and inference replicas.</div>

    <div class="bento-grid">
      
      <div class="bento-card">
        <div>
          <div class="bento-card-title">Conversational Turn DAG</div>
          <div class="bento-card-desc">Conversations form a non-destructive Directed Acyclic Graph. Branch, rollback, and rehydrate state at any historical turn without prompt mutation.</div>
        </div>
        <div class="bento-tag">Graph Memory</div>
      </div>

      <div class="bento-card">
        <div>
          <div class="bento-card-title">Prefix-Affinity Routing</div>
          <div class="bento-card-desc">A 64-bit consistent hash ring pins multi-turn sessions to identical GPU workers, maximizing KV-cache reuse and eliminating redundant token prefill.</div>
        </div>
        <div class="bento-tag">Consistent Hash Ring</div>
      </div>

      <div class="bento-card">
        <div>
          <div class="bento-card-title">Human-in-the-Loop Gate</div>
          <div class="bento-card-desc">Autonomous multi-turn tool loops with safety gates. Stream execution halts on sensitive actions until an operator clicks approve.</div>
        </div>
        <div class="bento-tag">Tool Orchestrator</div>
      </div>

      <div class="bento-card">
        <div>
          <div class="bento-card-title">Embedded SQLite (WAL Mode)</div>
          <div class="bento-card-desc">Survives server restarts with memory-mapped I/O and Write-Ahead Logging. Inspect sessions directly via standard SQL tooling.</div>
        </div>
        <div class="bento-tag">Storage Subsystem</div>
      </div>

      <div class="bento-card">
        <div>
          <div class="bento-card-title">Production Guardrails</div>
          <div class="bento-card-desc">Sliding-window context budget compaction, strict tool execution timeouts, output character truncation, and stream cancellation on client disconnect.</div>
        </div>
        <div class="bento-tag">Resilience & Safety</div>
      </div>

      <div class="bento-card">
        <div>
          <div class="bento-card-title">OpenAI Drop-In Parity</div>
          <div class="bento-card-desc">Native support for both POST /v1/responses with previous_response_id chaining and standard POST /v1/chat/completions.</div>
        </div>
        <div class="bento-tag">Protocol Parity</div>
      </div>

    </div>
  </section>

  <!-- Footer -->
  <footer>
    <div>EPISOD &mdash; Stateful Agentic Gateway &bull; Apache 2.0 License</div>
  </footer>

  <script>
    const snippets = {
      sdk: `<span class="hl-comment"># Drop-in compatibility with standard OpenAI clients:</span>
<span class="hl-kw">from</span> openai <span class="hl-kw">import</span> OpenAI

client = OpenAI(base_url=<span class="hl-str">"http://localhost:8080/v1"</span>, api_key=<span class="hl-str">"none"</span>)

response = client.chat.completions.create(
    model=<span class="hl-str">"meta-llama/Llama-3.1-8B-Instruct"</span>,
    messages=[{<span class="hl-str">"role"</span>: <span class="hl-str">"user"</span>, <span class="hl-str">"content"</span>: <span class="hl-str">"Explain quantum computing."</span>}],
    extra_headers={<span class="hl-str">"x-episod-session"</span>: <span class="hl-str">"session_user_01"</span>} <span class="hl-comment"># 100% prefix cache affinity</span>
)
print(response.choices[0].message.content)`,

      responses: `<span class="hl-comment"># State rehydration using previous_response_id:</span>
curl -N -X POST http://localhost:8080/v1/responses \\
  -H <span class="hl-str">"Content-Type: application/json"</span> \\
  -d '{
    <span class="hl-str">"model"</span>: <span class="hl-str">"meta-llama/Llama-3.1-8B-Instruct"</span>,
    <span class="hl-str">"input"</span>: <span class="hl-str">"Continue from where we left off."</span>,
    <span class="hl-str">"previous_response_id"</span>: <span class="hl-str">"resp_018f3a9e4b"</span>,
    <span class="hl-str">"stream"</span>: <span class="hl-kw">true</span>
  }'`,

      cli: `<span class="hl-comment"># Zero-config local dev mode (auto-detects Ollama / vLLM):</span>
<span class="hl-cmd">$</span> episod dev

<span class="hl-comment"># Visualize DAG turn history directly in terminal:</span>
<span class="hl-cmd">$</span> episod tree ep_b7e914df088741348123abc456`
    };

    function switchCode(tab) {
      document.querySelectorAll('.code-tab').forEach(el => el.classList.remove('active'));
      event.target.classList.add('active');
      document.getElementById('code-content').innerHTML = snippets[tab];
    }
  </script>
</body>
</html>"###;
