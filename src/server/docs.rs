use axum::response::Html;

pub async fn render_docs() -> Html<&'static str> {
    Html(DOCS_HTML)
}

const DOCS_HTML: &str = r###"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Episod — API Reference</title>
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
      --ds-text-secondary: #a1a1a1;
      --ds-text-muted: #666666;
      
      --ds-accent-blue: #0072f5;
      --ds-status-green: #45a557;
      --ds-status-amber: #ff990a;
      --ds-status-red: #e5484d;

      --font-sans: "Geist", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      --font-mono: "Geist Mono", ui-monospace, SFMono-Regular, "Roboto Mono", Menlo, monospace;
    }

    * { box-sizing: border-box; margin: 0; padding: 0; -webkit-font-smoothing: antialiased; }

    body {
      background: var(--ds-background-100);
      color: var(--ds-text-primary);
      font-family: var(--font-sans);
      font-size: 13px;
      line-height: 1.6;
      display: flex;
      flex-direction: column;
      height: 100vh;
      overflow: hidden;
    }

    header {
      background: var(--ds-background-200);
      border-bottom: 1px solid var(--ds-border-subtle);
      height: 52px;
      padding: 0 24px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      flex-shrink: 0;
    }

    .brand-cluster {
      display: flex;
      align-items: center;
      gap: 12px;
    }

    .brand-title {
      font-size: 13px;
      font-weight: 600;
      color: var(--ds-text-primary);
      text-transform: uppercase;
      text-decoration: none;
      letter-spacing: -0.02em;
    }

    .brand-subtitle {
      font-size: 11px;
      color: var(--ds-text-muted);
      letter-spacing: 0.04em;
      text-transform: uppercase;
      padding-left: 12px;
      border-left: 1px solid var(--ds-border-subtle);
    }

    .header-links {
      display: flex;
      align-items: center;
      gap: 16px;
    }

    .btn-ghost {
      background: transparent;
      border: 1px solid var(--ds-border-subtle);
      color: var(--ds-text-secondary);
      text-decoration: none;
      font-size: 12px;
      padding: 5px 12px;
      border-radius: 5px;
      transition: all 0.15s ease;
    }
    .btn-ghost:hover {
      background: var(--ds-card-hover);
      color: var(--ds-text-primary);
      border-color: var(--ds-border-default);
    }

    .btn-solid {
      background: #ededed;
      color: #000;
      text-decoration: none;
      font-size: 12px;
      font-weight: 500;
      padding: 5px 12px;
      border-radius: 5px;
      transition: opacity 0.15s ease;
    }
    .btn-solid:hover { opacity: 0.9; }

    /* Layout */
    .docs-layout {
      display: flex;
      flex: 1;
      overflow: hidden;
    }

    /* Sidebar Navigation */
    .docs-nav {
      width: 260px;
      background: var(--ds-background-200);
      border-right: 1px solid var(--ds-border-subtle);
      padding: 24px 16px;
      overflow-y: auto;
      flex-shrink: 0;
    }

    .nav-section-title {
      font-size: 11px;
      font-weight: 500;
      text-transform: uppercase;
      letter-spacing: 0.06em;
      color: var(--ds-text-muted);
      margin: 16px 0 8px 8px;
    }
    .nav-section-title:first-child { margin-top: 0; }

    .nav-link-item {
      display: block;
      color: var(--ds-text-secondary);
      text-decoration: none;
      font-size: 12px;
      padding: 6px 10px;
      border-radius: 5px;
      transition: all 0.12s ease;
      font-family: var(--font-mono);
    }
    .nav-link-item:hover {
      background: var(--ds-card-hover);
      color: var(--ds-text-primary);
    }

    /* Main Docs Content */
    .docs-content {
      flex: 1;
      overflow-y: auto;
      padding: 40px 48px;
      background: var(--ds-background-100);
    }

    .docs-container {
      max-width: 880px;
      margin: 0 auto;
      display: flex;
      flex-direction: column;
      gap: 48px;
    }

    .docs-hero {
      border-bottom: 1px solid var(--ds-border-subtle);
      padding-bottom: 24px;
    }

    .docs-title {
      font-size: 32px;
      font-weight: 600;
      letter-spacing: -1.28px;
      color: var(--ds-text-primary);
      margin-bottom: 8px;
    }

    .docs-desc {
      font-size: 14px;
      color: var(--ds-text-secondary);
      line-height: 1.6;
    }

    /* Endpoint Card */
    .endpoint-card {
      background: var(--ds-card-bg);
      border: 1px solid var(--ds-border-subtle);
      border-radius: 8px;
      overflow: hidden;
      margin-top: 16px;
    }

    .endpoint-header {
      padding: 14px 20px;
      background: var(--ds-background-200);
      border-bottom: 1px solid var(--ds-border-subtle);
      display: flex;
      align-items: center;
      gap: 12px;
    }

    .method-badge {
      font-family: var(--font-mono);
      font-size: 11px;
      font-weight: 600;
      padding: 3px 8px;
      border-radius: 4px;
      text-transform: uppercase;
      letter-spacing: 0.04em;
    }
    .method-post {
      color: #52aeff;
      background: rgba(0, 114, 245, 0.1);
      border: 1px solid rgba(0, 114, 245, 0.25);
    }
    .method-get {
      color: #6cda75;
      background: rgba(69, 165, 87, 0.1);
      border: 1px solid rgba(69, 165, 87, 0.25);
    }

    .endpoint-path {
      font-family: var(--font-mono);
      font-size: 13px;
      font-weight: 500;
      color: var(--ds-text-primary);
    }

    .endpoint-body {
      padding: 20px;
      display: flex;
      flex-direction: column;
      gap: 16px;
    }

    .endpoint-desc {
      font-size: 13px;
      color: var(--ds-text-secondary);
    }

    /* Param Table */
    .table-container {
      border: 1px solid var(--ds-border-subtle);
      border-radius: 6px;
      overflow: hidden;
    }

    table {
      width: 100%;
      border-collapse: collapse;
      text-align: left;
      font-size: 12px;
    }

    th {
      background: var(--ds-background-200);
      padding: 8px 14px;
      color: var(--ds-text-muted);
      font-weight: 500;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      border-bottom: 1px solid var(--ds-border-subtle);
    }

    td {
      padding: 10px 14px;
      border-bottom: 1px solid var(--ds-border-subtle);
      color: var(--ds-text-secondary);
    }
    tr:last-child td { border-bottom: none; }

    .type-badge {
      font-family: var(--font-mono);
      font-size: 11px;
      color: #bf89ec;
    }

    .required-badge {
      font-family: var(--font-mono);
      font-size: 10px;
      color: var(--ds-status-amber);
    }

    /* Code Snippet */
    .code-box {
      background: var(--ds-recessed);
      border: 1px solid var(--ds-border-subtle);
      border-radius: 6px;
      padding: 14px 16px;
      font-family: var(--font-mono);
      font-size: 11px;
      color: #d1d5db;
      overflow-x: auto;
      line-height: 1.5;
    }
  </style>
</head>
<body>

  <header>
    <div class="brand-cluster">
      <a href="/" class="brand-title">EPISOD</a>
      <div class="brand-subtitle">API Reference</div>
    </div>
    <div class="header-links">
      <a href="/" class="btn-ghost">Landing Page</a>
      <a href="/dashboard" class="btn-solid">Open Dashboard</a>
    </div>
  </header>

  <div class="docs-layout">
    
    <!-- Sidebar Navigation -->
    <nav class="docs-nav">
      <div class="nav-section-title">Overview</div>
      <a href="#overview" class="nav-link-item">Introduction</a>
      <a href="#affinity" class="nav-link-item">Prefix Affinity</a>

      <div class="nav-section-title">Episodic Engine</div>
      <a href="#create-episode" class="nav-link-item">POST /v1/episodes</a>
      <a href="#list-episodes" class="nav-link-item">GET /v1/episodes</a>
      <a href="#get-episode" class="nav-link-item">GET /v1/episodes/{id}</a>
      <a href="#submit-turn" class="nav-link-item">POST /v1/episodes/{id}/turns</a>
      <a href="#fork-episode" class="nav-link-item">POST /v1/episodes/{id}/fork</a>
      <a href="#approve-tool" class="nav-link-item">POST /v1/episodes/{id}/approve</a>

      <div class="nav-section-title">OpenAI Drop-In</div>
      <a href="#responses-api" class="nav-link-item">POST /v1/responses</a>
      <a href="#chat-completions" class="nav-link-item">POST /v1/chat/completions</a>

      <div class="nav-section-title">System</div>
      <a href="#health" class="nav-link-item">GET /health</a>
    </nav>

    <!-- Main Content -->
    <main class="docs-content">
      <div class="docs-container">

        <!-- Overview -->
        <section id="overview" class="docs-hero">
          <h1 class="docs-title">API Reference</h1>
          <p class="docs-desc">
            Episod provides a stateful agentic layer and consistent hash routing ring in front of any generic LLM inference backend (vLLM, SGLang, Ollama, LiteLLM). All endpoints support sub-millisecond execution and persistent SQLite storage.
          </p>
        </section>

        <!-- POST /v1/episodes -->
        <section id="create-episode">
          <h2>Create Episode</h2>
          <div class="endpoint-card">
            <div class="endpoint-header">
              <span class="method-badge method-post">POST</span>
              <span class="endpoint-path">/v1/episodes</span>
            </div>
            <div class="endpoint-body">
              <p class="endpoint-desc">Initializes a new episodic session with a model identifier, optional system instructions, and arbitrary metadata.</p>
              
              <div class="table-container">
                <table>
                  <thead>
                    <tr><th>Parameter</th><th>Type</th><th>Required</th><th>Description</th></tr>
                  </thead>
                  <tbody>
                    <tr><td><code>model</code></td><td><span class="type-badge">string</span></td><td><span class="required-badge">required</span></td><td>Upstream model identifier (e.g. <code>meta-llama/Llama-3.1-8B-Instruct</code>).</td></tr>
                    <tr><td><code>system_prompt</code></td><td><span class="type-badge">string</span></td><td>optional</td><td>System instruction prepended to conversation history.</td></tr>
                    <tr><td><code>metadata</code></td><td><span class="type-badge">object</span></td><td>optional</td><td>Arbitrary key-value metadata tags for tenant or environment tracking.</td></tr>
                  </tbody>
                </table>
              </div>

              <div class="code-box">curl -X POST http://localhost:8080/v1/episodes \
  -H "Content-Type: application/json" \
  -d '{
    "model": "meta-llama/Llama-3.1-8B-Instruct",
    "system_prompt": "You are an analytical research assistant."
  }'</div>
            </div>
          </div>
        </section>

        <!-- POST /v1/episodes/{id}/turns -->
        <section id="submit-turn">
          <h2>Submit Turn & Stream Events</h2>
          <div class="endpoint-card">
            <div class="endpoint-header">
              <span class="method-badge method-post">POST</span>
              <span class="endpoint-path">/v1/episodes/{id}/turns</span>
            </div>
            <div class="endpoint-body">
              <p class="endpoint-desc">Submits a user prompt to an existing episode and streams back real-time Server-Sent Events (SSE). Rehydrates past conversational history from the DAG active leaf and manages tool loops automatically.</p>

              <div class="table-container">
                <table>
                  <thead>
                    <tr><th>Parameter</th><th>Type</th><th>Required</th><th>Description</th></tr>
                  </thead>
                  <tbody>
                    <tr><td><code>message</code></td><td><span class="type-badge">string</span></td><td>optional</td><td>User prompt text to append to the conversation.</td></tr>
                    <tr><td><code>execute_tools</code></td><td><span class="type-badge">boolean</span></td><td>optional</td><td>Whether Episod should execute tool calls autonomously (default: true).</td></tr>
                  </tbody>
                </table>
              </div>

              <div class="code-box">curl -N -X POST http://localhost:8080/v1/episodes/ep_39a82b/turns \
  -H "Content-Type: application/json" \
  -d '{
    "message": "Calculate 1500 / 12 and explain the answer.",
    "execute_tools": true
  }'</div>
            </div>
          </div>
        </section>

        <!-- POST /v1/episodes/{id}/fork -->
        <section id="fork-episode">
          <h2>Fork Branch</h2>
          <div class="endpoint-card">
            <div class="endpoint-header">
              <span class="method-badge method-post">POST</span>
              <span class="endpoint-path">/v1/episodes/{id}/fork</span>
            </div>
            <div class="endpoint-body">
              <p class="endpoint-desc">Branches an episode from any historical turn node. The active leaf pointer is reset to the target node, allowing non-destructive counter-factual reasoning without mutating prior branches.</p>

              <div class="table-container">
                <table>
                  <thead>
                    <tr><th>Parameter</th><th>Type</th><th>Required</th><th>Description</th></tr>
                  </thead>
                  <tbody>
                    <tr><td><code>from_node_id</code></td><td><span class="type-badge">string</span></td><td><span class="required-badge">required</span></td><td>The turn node ID to fork from (e.g. <code>turn_389271a</code>).</td></tr>
                  </tbody>
                </table>
              </div>

              <div class="code-box">curl -X POST http://localhost:8080/v1/episodes/ep_39a82b/fork \
  -H "Content-Type: application/json" \
  -d '{
    "from_node_id": "turn_389271a"
  }'</div>
            </div>
          </div>
        </section>

        <!-- POST /v1/episodes/{id}/approve -->
        <section id="approve-tool">
          <h2>Human-in-the-Loop Approval</h2>
          <div class="endpoint-card">
            <div class="endpoint-header">
              <span class="method-badge method-post">POST</span>
              <span class="endpoint-path">/v1/episodes/{id}/approve</span>
            </div>
            <div class="endpoint-body">
              <p class="endpoint-desc">Resolves a pending tool execution gate requiring human operator sign-off. Resumes the agent loop with either execution or custom rejection feedback.</p>

              <div class="table-container">
                <table>
                  <thead>
                    <tr><th>Parameter</th><th>Type</th><th>Required</th><th>Description</th></tr>
                  </thead>
                  <tbody>
                    <tr><td><code>tool_call_id</code></td><td><span class="type-badge">string</span></td><td><span class="required-badge">required</span></td><td>ID of the pending tool call.</td></tr>
                    <tr><td><code>approved</code></td><td><span class="type-badge">boolean</span></td><td><span class="required-badge">required</span></td><td>True to execute, false to reject.</td></tr>
                    <tr><td><code>feedback</code></td><td><span class="type-badge">string</span></td><td>optional</td><td>Custom rejection message returned to the model.</td></tr>
                  </tbody>
                </table>
              </div>

              <div class="code-box">curl -X POST http://localhost:8080/v1/episodes/ep_39a82b/approve \
  -H "Content-Type: application/json" \
  -d '{
    "tool_call_id": "call_delete_cluster",
    "approved": true
  }'</div>
            </div>
          </div>
        </section>

        <!-- POST /v1/responses -->
        <section id="responses-api">
          <h2>OpenAI Responses API</h2>
          <div class="endpoint-card">
            <div class="endpoint-header">
              <span class="method-badge method-post">POST</span>
              <span class="endpoint-path">/v1/responses</span>
            </div>
            <div class="endpoint-body">
              <p class="endpoint-desc">Drop-in compatibility with the OpenAI Responses API specification. Supports multi-turn chaining via <code>previous_response_id</code> to rehydrate state automatically without client-side message replay.</p>

              <div class="code-box">curl -N -X POST http://localhost:8080/v1/responses \
  -H "Content-Type: application/json" \
  -d '{
    "model": "meta-llama/Llama-3.1-8B-Instruct",
    "input": "Summarize general relativity.",
    "stream": true
  }'</div>
            </div>
          </div>
        </section>

        <!-- POST /v1/chat/completions -->
        <section id="chat-completions">
          <h2>OpenAI Chat Completions</h2>
          <div class="endpoint-card">
            <div class="endpoint-header">
              <span class="method-badge method-post">POST</span>
              <span class="endpoint-path">/v1/chat/completions</span>
            </div>
            <div class="endpoint-body">
              <p class="endpoint-desc">Drop-in replacement for OpenAI client libraries. Pass the <code>x-episod-session</code> header to bind requests to a specific session on the consistent hash ring for 100% KV-cache prefix affinity.</p>

              <div class="code-box">curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "x-episod-session: my_user_session_42" \
  -d '{
    "model": "meta-llama/Llama-3.1-8B-Instruct",
    "messages": [
      {"role": "user", "content": "Hello world!"}
    ]
  }'</div>
            </div>
          </div>
        </section>

        <!-- GET /health -->
        <section id="health">
          <h2>System Health Check</h2>
          <div class="endpoint-card">
            <div class="endpoint-header">
              <span class="method-badge method-get">GET</span>
              <span class="endpoint-path">/health</span>
            </div>
            <div class="endpoint-body">
              <p class="endpoint-desc">Returns service status for load balancers and Kubernetes readiness/liveness probes.</p>
              <div class="code-box">curl http://localhost:8080/health
# {"service":"episod","status":"ok"}</div>
            </div>
          </div>
        </section>

      </div>
    </main>

  </div>

</body>
</html>
"###;
