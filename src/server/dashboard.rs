use axum::response::Html;

pub async fn render_dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Episod — Gateway Inspector</title>
  <style>
    :root {
      --ds-background-100: #000000;
      --ds-background-200: #0a0a0a;
      --ds-card-bg: #121212;
      --ds-card-hover: #171717;
      --ds-recessed: #1a1a1a;
      
      --ds-border-subtle: rgba(255, 255, 255, 0.08);
      --ds-border-default: rgba(255, 255, 255, 0.14);
      --ds-border-active: rgba(255, 255, 255, 0.28);
      
      --ds-text-primary: #ededed;
      --ds-text-secondary: #a1a1a1;
      --ds-text-muted: #666666;
      
      --ds-accent-blue: #0072f5;
      --ds-focus-ring: 0 0 0 2px #000000, 0 0 0 4px #0072f5;
      
      --ds-status-green: #45a557;
      --ds-status-amber: #ff990a;
      --ds-status-red: #e5484d;
      --ds-status-blue: #0062d1;

      --font-sans: "Geist", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
      --font-mono: "Geist Mono", ui-monospace, SFMono-Regular, "Roboto Mono", Menlo, monospace;
    }

    * { box-sizing: border-box; margin: 0; padding: 0; -webkit-font-smoothing: antialiased; }
    
    body {
      background: var(--ds-background-100);
      color: var(--ds-text-primary);
      font-family: var(--font-sans);
      font-size: 13px;
      line-height: 1.5;
      height: 100vh;
      display: flex;
      flex-direction: column;
      overflow: hidden;
    }

    /* Top Navigation Bar */
    header {
      background: var(--ds-background-200);
      border-bottom: 1px solid var(--ds-border-subtle);
      height: 48px;
      padding: 0 20px;
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
      letter-spacing: -0.02em;
      color: var(--ds-text-primary);
      text-transform: uppercase;
    }

    .brand-subtitle {
      font-size: 11px;
      color: var(--ds-text-muted);
      letter-spacing: 0.04em;
      text-transform: uppercase;
      padding-left: 12px;
      border-left: 1px solid var(--ds-border-subtle);
    }

    .header-status {
      display: flex;
      align-items: center;
      gap: 8px;
      font-size: 12px;
      color: var(--ds-text-secondary);
    }

    .status-dot {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: var(--ds-status-green);
      display: inline-block;
    }
    .status-dot.amber { background: var(--ds-status-amber); }
    .status-dot.red { background: var(--ds-status-red); }

    /* Main Layout 3 Columns */
    .layout {
      display: flex;
      flex: 1;
      overflow: hidden;
    }

    /* Left Sidebar: Session Explorer */
    .sidebar {
      width: 300px;
      background: var(--ds-background-200);
      border-right: 1px solid var(--ds-border-subtle);
      display: flex;
      flex-direction: column;
      flex-shrink: 0;
    }

    .sidebar-header {
      padding: 12px 16px;
      border-bottom: 1px solid var(--ds-border-subtle);
      display: flex;
      justify-content: space-between;
      align-items: center;
    }

    .section-label {
      font-size: 11px;
      font-weight: 500;
      text-transform: uppercase;
      letter-spacing: 0.05em;
      color: var(--ds-text-muted);
    }

    .btn-ghost {
      background: transparent;
      border: 1px solid var(--ds-border-subtle);
      color: var(--ds-text-secondary);
      font-family: var(--font-sans);
      font-size: 11px;
      font-weight: 500;
      padding: 4px 10px;
      border-radius: 5px;
      cursor: pointer;
      transition: all 0.15s ease;
    }
    .btn-ghost:hover {
      background: var(--ds-card-hover);
      color: var(--ds-text-primary);
      border-color: var(--ds-border-default);
    }
    .btn-ghost:focus {
      outline: none;
      box-shadow: var(--ds-focus-ring);
    }

    .episode-list {
      flex: 1;
      overflow-y: auto;
      list-style: none;
    }

    .episode-item {
      padding: 14px 16px;
      border-bottom: 1px solid var(--ds-border-subtle);
      cursor: pointer;
      transition: background 0.12s ease;
    }
    .episode-item:hover {
      background: var(--ds-card-hover);
    }
    .episode-item.active {
      background: var(--ds-card-bg);
      border-left: 2px solid var(--ds-text-primary);
    }

    .episode-item-id {
      font-family: var(--font-mono);
      font-size: 12px;
      font-weight: 500;
      color: var(--ds-text-primary);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .episode-item-model {
      font-size: 11px;
      color: var(--ds-text-secondary);
      margin-top: 4px;
    }

    .episode-item-meta {
      font-size: 11px;
      color: var(--ds-text-muted);
      margin-top: 4px;
      display: flex;
      align-items: center;
      gap: 6px;
    }

    /* Center Content: DAG Conversation Canvas */
    .content-canvas {
      flex: 1;
      display: flex;
      flex-direction: column;
      background: var(--ds-background-100);
      overflow-y: auto;
      padding: 32px 40px;
    }

    .canvas-container {
      max-width: 820px;
      width: 100%;
      margin: 0 auto;
      display: flex;
      flex-direction: column;
      gap: 20px;
    }

    .canvas-header {
      padding-bottom: 16px;
      border-bottom: 1px solid var(--ds-border-subtle);
      display: flex;
      justify-content: space-between;
      align-items: flex-end;
    }

    .canvas-title {
      font-size: 18px;
      font-weight: 600;
      letter-spacing: -0.03em;
      color: var(--ds-text-primary);
    }

    .canvas-subtitle {
      font-family: var(--font-mono);
      font-size: 12px;
      color: var(--ds-text-muted);
      margin-top: 4px;
    }

    /* Turn Card */
    .turn-card {
      background: var(--ds-card-bg);
      border: 1px solid var(--ds-border-subtle);
      border-radius: 6px;
      padding: 20px;
      position: relative;
      transition: border-color 0.15s ease;
    }
    .turn-card:hover {
      border-color: var(--ds-border-default);
    }
    .turn-card.active-leaf {
      border-color: var(--ds-border-active);
    }

    .turn-topbar {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 12px;
    }

    .turn-node-id {
      font-family: var(--font-mono);
      font-size: 11px;
      color: var(--ds-text-muted);
      display: flex;
      align-items: center;
      gap: 8px;
    }

    .leaf-tag {
      font-size: 10px;
      text-transform: uppercase;
      letter-spacing: 0.06em;
      padding: 2px 6px;
      border-radius: 4px;
      border: 1px solid var(--ds-border-default);
      color: var(--ds-text-secondary);
    }

    .turn-role-tag {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      font-family: var(--font-mono);
      font-size: 10px;
      font-weight: 500;
      text-transform: uppercase;
      letter-spacing: 0.08em;
      padding: 3px 8px;
      border-radius: 4px;
      margin-bottom: 8px;
    }

    .turn-role-tag.system {
      color: var(--ds-text-secondary);
      background: rgba(255, 255, 255, 0.05);
      border: 1px solid var(--ds-border-subtle);
    }

    .turn-role-tag.user {
      color: #52aeff;
      background: rgba(0, 114, 245, 0.08);
      border: 1px solid rgba(0, 114, 245, 0.25);
    }

    .turn-role-tag.assistant {
      color: #6cda75;
      background: rgba(69, 165, 87, 0.08);
      border: 1px solid rgba(69, 165, 87, 0.25);
    }

    .role-dot {
      width: 6px;
      height: 6px;
      border-radius: 50%;
      display: inline-block;
    }
    .role-dot.user {
      background: #0072f5;
      box-shadow: 0 0 6px rgba(0, 114, 245, 0.6);
    }
    .role-dot.assistant {
      background: #45a557;
      box-shadow: 0 0 6px rgba(69, 165, 87, 0.6);
    }
    .role-dot.system {
      background: var(--ds-text-muted);
    }

    .turn-user-box {
      border-left: 2px solid rgba(0, 114, 245, 0.35);
      padding-left: 12px;
      margin-bottom: 14px;
    }

    .turn-assistant-box {
      border-left: 2px solid rgba(69, 165, 87, 0.35);
      padding-left: 12px;
      margin-top: 14px;
    }

    .turn-text-content {
      font-size: 13px;
      line-height: 1.6;
      color: var(--ds-text-primary);
      white-space: pre-wrap;
    }

    .tool-execution-block {
      background: var(--ds-recessed);
      border: 1px solid var(--ds-border-subtle);
      border-radius: 5px;
      padding: 12px 14px;
      margin: 12px 0;
      font-family: var(--font-mono);
      font-size: 11px;
    }

    .tool-header-row {
      display: flex;
      justify-content: space-between;
      color: var(--ds-text-secondary);
      margin-bottom: 6px;
    }

    .tool-args-pre {
      color: var(--ds-text-muted);
      white-space: pre-wrap;
      word-break: break-all;
      margin: 0;
      line-height: 1.4;
    }

    .turn-metrics-row {
      margin-top: 14px;
      padding-top: 10px;
      border-top: 1px solid var(--ds-border-subtle);
      display: flex;
      gap: 20px;
      font-size: 11px;
      color: var(--ds-text-muted);
    }

    .metric-item span {
      color: var(--ds-text-secondary);
      font-family: var(--font-mono);
    }

    /* Right Sidebar: Telemetry & Human Approvals */
    .inspector-pane {
      width: 320px;
      background: var(--ds-background-200);
      border-left: 1px solid var(--ds-border-subtle);
      padding: 20px;
      display: flex;
      flex-direction: column;
      gap: 24px;
      overflow-y: auto;
      flex-shrink: 0;
    }

    .panel-block {
      display: flex;
      flex-direction: column;
      gap: 10px;
    }

    .stat-card {
      background: var(--ds-card-bg);
      border: 1px solid var(--ds-border-subtle);
      border-radius: 6px;
      padding: 14px 16px;
    }

    .stat-card-label {
      font-size: 11px;
      color: var(--ds-text-muted);
      text-transform: uppercase;
      letter-spacing: 0.05em;
    }

    .stat-card-value {
      font-size: 24px;
      font-weight: 600;
      letter-spacing: -0.04em;
      color: var(--ds-text-primary);
      margin-top: 4px;
    }

    .hitl-alert-card {
      background: var(--ds-card-bg);
      border: 1px solid var(--ds-status-amber);
      border-radius: 6px;
      padding: 16px;
      display: flex;
      flex-direction: column;
      gap: 10px;
    }

    .hitl-alert-badge {
      display: flex;
      align-items: center;
      gap: 6px;
      font-size: 11px;
      font-weight: 500;
      text-transform: uppercase;
      letter-spacing: 0.06em;
      color: var(--ds-status-amber);
    }

    .hitl-call-name {
      font-family: var(--font-mono);
      font-size: 12px;
      font-weight: 500;
      color: var(--ds-text-primary);
    }

    .hitl-actions {
      display: flex;
      gap: 8px;
      margin-top: 4px;
    }

    .btn-solid-white {
      background: #ededed;
      color: #000000;
      border: none;
      font-family: var(--font-sans);
      font-size: 11px;
      font-weight: 500;
      padding: 6px 12px;
      border-radius: 5px;
      cursor: pointer;
      transition: opacity 0.15s ease;
    }
    .btn-solid-white:hover { opacity: 0.9; }

    .btn-danger {
      background: transparent;
      border: 1px solid var(--ds-status-red);
      color: var(--ds-status-red);
      font-family: var(--font-sans);
      font-size: 11px;
      font-weight: 500;
      padding: 6px 12px;
      border-radius: 5px;
      cursor: pointer;
      transition: all 0.15s ease;
    }
    .btn-danger:hover {
      background: rgba(229, 72, 77, 0.1);
    }

    .empty-state {
      text-align: center;
      padding: 40px 20px;
      color: var(--ds-text-muted);
      font-size: 12px;
    }
  </style>
</head>
<body>

  <!-- Top Navigation -->
  <header>
    <div class="brand-cluster">
      <div class="brand-title">EPISOD</div>
      <div class="brand-subtitle">Gateway Inspector</div>
    </div>
    <div class="header-status">
      <span class="status-dot" id="system-status-dot"></span>
      <span id="system-status-text">Operational</span>
    </div>
  </header>

  <!-- 3-Column Layout -->
  <div class="layout">
    
    <!-- Left Column: Sessions / Episodes List -->
    <aside class="sidebar">
      <div class="sidebar-header">
        <span class="section-label">Sessions</span>
        <button class="btn-ghost" onclick="fetchEpisodes()">Refresh</button>
      </div>
      <ul class="episode-list" id="episodes-container">
        <li class="empty-state">Loading sessions...</li>
      </ul>
    </aside>

    <!-- Center Column: DAG Canvas -->
    <main class="content-canvas">
      <div class="canvas-container">
        <div class="canvas-header">
          <div>
            <div class="canvas-title" id="canvas-session-title">Select a Session</div>
            <div class="canvas-subtitle" id="canvas-session-subtitle">Choose an episode from the left sidebar to view turn history.</div>
          </div>
          <div id="canvas-actions"></div>
        </div>

        <div id="tree-nodes-container">
          <div class="empty-state">No session selected.</div>
        </div>
      </div>
    </main>

    <!-- Right Column: Inspector / Telemetry / Approvals -->
    <aside class="inspector-pane">
      
      <!-- Telemetry Section -->
      <div class="panel-block">
        <div class="section-label">Session Telemetry</div>
        
        <div class="stat-card">
          <div class="stat-card-label">Total Turns</div>
          <div class="stat-card-value" id="stat-total-turns">-</div>
        </div>

        <div class="stat-card">
          <div class="stat-card-label">Model</div>
          <div style="font-family: var(--font-mono); font-size: 12px; color: var(--ds-text-primary); margin-top: 6px; word-break: break-all;" id="stat-model">-</div>
        </div>

        <div class="stat-card">
          <div class="stat-card-label">Active Leaf ID</div>
          <div style="font-family: var(--font-mono); font-size: 11px; color: var(--ds-text-muted); margin-top: 6px; word-break: break-all;" id="stat-leaf">-</div>
        </div>
      </div>

      <!-- Human-in-the-Loop Section -->
      <div class="panel-block">
        <div class="section-label">Human-in-the-Loop Gate</div>
        <div id="hitl-queue-container">
          <div class="empty-state" style="padding: 16px 0;">No approvals pending.</div>
        </div>
      </div>

    </aside>

  </div>

  <script>
    let activeEpisode = null;

    async function fetchEpisodes() {
      try {
        const res = await fetch('/v1/episodes');
        const episodes = await res.json();
        const container = document.getElementById('episodes-container');
        container.innerHTML = '';

        if (!episodes || episodes.length === 0) {
          container.innerHTML = '<li class="empty-state">No sessions found</li>';
          return;
        }

        episodes.forEach(ep => {
          const li = document.createElement('li');
          const isSelected = activeEpisode && activeEpisode.id === ep.id;
          li.className = 'episode-item' + (isSelected ? ' active' : '');
          const turnCount = Object.keys(ep.nodes || {}).length;

          li.innerHTML = `
            <div class="episode-item-id">${escapeHtml(ep.id)}</div>
            <div class="episode-item-model">${escapeHtml(ep.model)}</div>
            <div class="episode-item-meta">
              <span>${turnCount} turns</span>
              <span>&middot;</span>
              <span>${ep.active_leaf_id ? ep.active_leaf_id.slice(0, 10) + '...' : 'initial'}</span>
            </div>
          `;
          li.onclick = () => selectEpisode(ep);
          container.appendChild(li);
        });

        // Select first episode if none selected
        if (!activeEpisode && episodes.length > 0) {
          selectEpisode(episodes[0]);
        }
      } catch (err) {
        console.error('Failed to load episodes:', err);
      }
    }

    function selectEpisode(ep) {
      activeEpisode = ep;
      document.querySelectorAll('.episode-item').forEach(el => el.classList.remove('active'));
      
      document.getElementById('canvas-session-title').innerText = ep.id;
      document.getElementById('canvas-session-subtitle').innerText = 'Model: ' + ep.model;

      const turnCount = Object.keys(ep.nodes || {}).length;
      document.getElementById('stat-total-turns').innerText = turnCount;
      document.getElementById('stat-model').innerText = ep.model;
      document.getElementById('stat-leaf').innerText = ep.active_leaf_id || 'none';

      renderTree(ep);
      renderHITL(ep);
    }

    function renderTree(ep) {
      const container = document.getElementById('tree-nodes-container');
      container.innerHTML = '';

      if (ep.system_prompt) {
        const sysCard = document.createElement('div');
        sysCard.className = 'turn-card';
        sysCard.innerHTML = `
          <div class="turn-role-tag system"><span class="role-dot system"></span>SYSTEM INSTRUCTIONS</div>
          <div class="turn-text-content" style="color: var(--ds-text-secondary);">${escapeHtml(ep.system_prompt)}</div>
        `;
        container.appendChild(sysCard);
      }

      const turns = Object.values(ep.nodes || {}).sort((a, b) => a.created_at - b.created_at);

      if (turns.length === 0) {
        const empty = document.createElement('div');
        empty.className = 'empty-state';
        empty.innerText = 'No turns in this episode yet.';
        container.appendChild(empty);
        return;
      }

      turns.forEach((turn, idx) => {
        const card = document.createElement('div');
        const isLeaf = ep.active_leaf_id === turn.id;
        card.className = 'turn-card' + (isLeaf ? ' active-leaf' : '');

        let contentHtml = '';

        if (turn.user_message) {
          contentHtml += `
            <div class="turn-user-box">
              <div class="turn-role-tag user"><span class="role-dot user"></span>USER</div>
              <div class="turn-text-content">${escapeHtml(turn.user_message.content || '')}</div>
            </div>
          `;
        }

        if (turn.tool_calls && turn.tool_calls.length > 0) {
          turn.tool_calls.forEach(tc => {
            contentHtml += `
              <div class="tool-execution-block">
                <div class="tool-header-row">
                  <span>TOOL CALL: ${escapeHtml(tc.function.name)}</span>
                  <span>${escapeHtml(tc.id)}</span>
                </div>
                <pre class="tool-args-pre">${escapeHtml(tc.function.arguments)}</pre>
              </div>
            `;
          });
        }

        if (turn.tool_results && turn.tool_results.length > 0) {
          turn.tool_results.forEach(tr => {
            contentHtml += `
              <div class="tool-execution-block" style="border-color: rgba(69, 165, 87, 0.3);">
                <div class="tool-header-row">
                  <span style="color: var(--ds-status-green);">TOOL RESULT: ${escapeHtml(tr.tool_name)}</span>
                </div>
                <pre class="tool-args-pre">${escapeHtml(tr.output)}</pre>
              </div>
            `;
          });
        }

        if (turn.assistant_message) {
          contentHtml += `
            <div class="turn-assistant-box">
              <div class="turn-role-tag assistant"><span class="role-dot assistant"></span>ASSISTANT</div>
              <div class="turn-text-content">${escapeHtml(turn.assistant_message.content || '')}</div>
            </div>
          `;
        }

        const ttft = turn.timing && turn.timing.ttft_ms ? `${turn.timing.ttft_ms}ms` : 'n/a';
        const total = turn.timing ? `${turn.timing.total_ms}ms` : 'n/a';
        const tokens = turn.usage ? `${turn.usage.total_tokens}` : 'n/a';
        const hitRate = turn.usage && turn.usage.cache_hit_rate !== null && turn.usage.cache_hit_rate !== undefined 
          ? `${Math.round(turn.usage.cache_hit_rate * 100)}%` 
          : 'n/a';

        card.innerHTML = `
          <div class="turn-topbar">
            <div class="turn-node-id">
              <span>Node [${idx + 1}]</span>
              <span>&middot;</span>
              <span>${turn.id}</span>
              ${isLeaf ? '<span class="leaf-tag">Active Leaf</span>' : ''}
            </div>
            <button class="btn-ghost" onclick="forkTurn('${ep.id}', '${turn.id}')">Fork Branch</button>
          </div>
          ${contentHtml}
          <div class="turn-metrics-row">
            <div class="metric-item">TTFT: <span>${ttft}</span></div>
            <div class="metric-item">Duration: <span>${total}</span></div>
            <div class="metric-item">Tokens: <span>${tokens}</span></div>
            <div class="metric-item">Prefix Hit: <span>${hitRate}</span></div>
          </div>
        `;

        container.appendChild(card);
      });
    }

    function renderHITL(ep) {
      const container = document.getElementById('hitl-queue-container');
      container.innerHTML = '';

      const pending = Object.entries(ep.pending_approvals || {});
      if (pending.length === 0) {
        container.innerHTML = '<div class="empty-state" style="padding: 16px 0;">No approvals pending.</div>';
        return;
      }

      pending.forEach(([callId, item]) => {
        const card = document.createElement('div');
        card.className = 'hitl-alert-card';
        card.innerHTML = `
          <div class="hitl-alert-badge">
            <span class="status-dot amber"></span>
            <span>Approval Required</span>
          </div>
          <div class="hitl-call-name">${escapeHtml(item.tool_call.function.name)}</div>
          <pre class="tool-args-pre" style="max-height: 80px; overflow-y: auto; font-family: var(--font-mono); font-size: 11px;">${escapeHtml(item.tool_call.function.arguments)}</pre>
          <div class="hitl-actions">
            <button class="btn-solid-white" onclick="approveAction('${ep.id}', '${callId}', true)">Approve</button>
            <button class="btn-danger" onclick="approveAction('${ep.id}', '${callId}', false)">Reject</button>
          </div>
        `;
        container.appendChild(card);
      });
    }

    async function forkTurn(epId, nodeId) {
      if (!confirm(`Fork session from turn ${nodeId}?`)) return;
      await fetch(`/v1/episodes/${epId}/fork`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ from_node_id: nodeId })
      });
      fetchEpisodes();
    }

    async function approveAction(epId, callId, approved) {
      await fetch(`/v1/episodes/${epId}/approve`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ tool_call_id: callId, approved })
      });
      fetchEpisodes();
    }

    function escapeHtml(str) {
      return (str || '').replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
    }

    fetchEpisodes();
    setInterval(fetchEpisodes, 5000);
  </script>
</body>
</html>"#;
