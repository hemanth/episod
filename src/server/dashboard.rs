use axum::response::Html;

pub async fn render_dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>⚡ Episod Gateway Inspector</title>
  <style>
    :root {
      --bg: #0d1117;
      --card-bg: #161b22;
      --border: #30363d;
      --text: #c9d1d9;
      --text-muted: #8b949e;
      --accent: #58a6ff;
      --accent-green: #3fb950;
      --accent-orange: #d29922;
      --accent-red: #f85149;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, monospace; }
    body { background: var(--bg); color: var(--text); height: 100vh; display: flex; flex-direction: column; overflow: hidden; }
    
    header { background: var(--card-bg); border-bottom: 1px solid var(--border); padding: 12px 24px; display: flex; align-items: center; justify-content: space-between; }
    .brand { font-size: 1.1rem; font-weight: 700; color: #fff; display: flex; align-items: center; gap: 8px; }
    .badge { background: rgba(63, 185, 80, 0.15); color: var(--accent-green); border: 1px solid var(--accent-green); padding: 2px 8px; border-radius: 12px; font-size: 0.75rem; }
    
    .layout { display: flex; flex: 1; overflow: hidden; }
    
    /* Left Sidebar: Episodes */
    .sidebar { width: 320px; background: var(--card-bg); border-right: 1px solid var(--border); display: flex; flex-direction: column; }
    .sidebar-header { padding: 12px; border-bottom: 1px solid var(--border); display: flex; justify-content: space-between; align-items: center; }
    .episode-list { flex: 1; overflow-y: auto; list-style: none; }
    .episode-item { padding: 12px; border-bottom: 1px solid var(--border); cursor: pointer; transition: background 0.15s; }
    .episode-item:hover, .episode-item.active { background: #21262d; border-left: 3px solid var(--accent); }
    .episode-item .id { font-size: 0.8rem; font-weight: 600; color: #fff; word-break: break-all; }
    .episode-item .model { font-size: 0.75rem; color: var(--accent); margin-top: 4px; }
    .episode-item .turns { font-size: 0.7rem; color: var(--text-muted); margin-top: 2px; }

    /* Center: DAG Tree */
    .content { flex: 1; display: flex; flex-direction: column; background: var(--bg); overflow-y: auto; padding: 24px; }
    .node-tree { display: flex; flex-direction: column; gap: 16px; max-width: 800px; margin: 0 auto; width: 100%; }
    .turn-card { background: var(--card-bg); border: 1px solid var(--border); border-radius: 8px; padding: 16px; position: relative; }
    .turn-card.active-leaf { border-color: var(--accent-green); box-shadow: 0 0 12px rgba(63, 185, 80, 0.1); }
    .turn-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; font-size: 0.8rem; color: var(--text-muted); }
    .turn-body { font-size: 0.9rem; line-height: 1.5; color: #e6edf3; white-space: pre-wrap; }
    .role-badge { padding: 2px 6px; border-radius: 4px; font-weight: 600; font-size: 0.7rem; }
    .role-user { background: #1f6feb; color: #fff; }
    .role-assistant { background: #238636; color: #fff; }
    .metrics { margin-top: 8px; display: flex; gap: 12px; font-size: 0.75rem; color: var(--text-muted); border-top: 1px solid var(--border); padding-top: 8px; }

    /* Right Sidebar: HITL Approvals & Telemetry */
    .telemetry { width: 340px; background: var(--card-bg); border-left: 1px solid var(--border); padding: 16px; display: flex; flex-direction: column; gap: 16px; overflow-y: auto; }
    .panel-title { font-size: 0.85rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.5px; color: var(--text-muted); margin-bottom: 8px; }
    .stat-card { background: #0d1117; border: 1px solid var(--border); border-radius: 6px; padding: 12px; }
    .stat-label { font-size: 0.75rem; color: var(--text-muted); }
    .stat-val { font-size: 1.25rem; font-weight: 700; color: #fff; margin-top: 4px; }
    
    .hitl-item { background: rgba(210, 153, 34, 0.1); border: 1px solid var(--accent-orange); border-radius: 6px; padding: 12px; margin-top: 8px; }
    .btn { cursor: pointer; padding: 6px 12px; border-radius: 6px; border: none; font-size: 0.75rem; font-weight: 600; transition: opacity 0.2s; }
    .btn:hover { opacity: 0.8; }
    .btn-approve { background: var(--accent-green); color: #fff; margin-right: 8px; }
    .btn-reject { background: var(--accent-red); color: #fff; }
    .btn-fork { background: #21262d; border: 1px solid var(--border); color: var(--text); }
  </style>
</head>
<body>
  <header>
    <div class="brand">
      <span>⚡ Episod Gateway Inspector</span>
      <span class="badge" id="backend-status">Active</span>
    </div>
    <div style="font-size: 0.8rem; color: var(--text-muted);">
      Prefix-Affinity & DAG Memory Layer
    </div>
  </header>

  <div class="layout">
    <!-- Episodes List -->
    <div class="sidebar">
      <div class="sidebar-header">
        <span style="font-weight: 600; font-size: 0.85rem;">Episodes</span>
        <button class="btn btn-fork" onclick="refreshEpisodes()">🔄 Refresh</button>
      </div>
      <ul class="episode-list" id="episodes-list">
        <li style="padding: 16px; color: var(--text-muted); text-align: center;">Loading episodes...</li>
      </ul>
    </div>

    <!-- Active Conversation Tree -->
    <div class="content">
      <div id="tree-container" class="node-tree">
        <div style="text-align: center; color: var(--text-muted); margin-top: 40px;">
          Select an episode from the left sidebar to view its DAG of turns.
        </div>
      </div>
    </div>

    <!-- Telemetry & HITL -->
    <div class="telemetry">
      <div class="panel-title">Session Telemetry</div>
      <div class="stat-card">
        <div class="stat-label">Active Episode</div>
        <div class="stat-val" id="telemetry-ep-id" style="font-size: 0.85rem; word-break: break-all;">-</div>
      </div>
      <div class="stat-card">
        <div class="stat-label">Model</div>
        <div class="stat-val" id="telemetry-model" style="font-size: 0.9rem;">-</div>
      </div>
      <div class="stat-card">
        <div class="stat-label">Total DAG Turns</div>
        <div class="stat-val" id="telemetry-turns">0</div>
      </div>

      <div class="panel-title" style="margin-top: 12px;">HITL Approval Gate</div>
      <div id="hitl-container">
        <div style="font-size: 0.75rem; color: var(--text-muted);">No pending human approvals.</div>
      </div>
    </div>
  </div>

  <script>
    let currentEpisode = null;

    async function refreshEpisodes() {
      try {
        const res = await fetch('/v1/episodes');
        const episodes = await res.json();
        const listEl = document.getElementById('episodes-list');
        listEl.innerHTML = '';
        
        if (episodes.length === 0) {
          listEl.innerHTML = '<li style="padding: 16px; color: var(--text-muted); text-align: center;">No episodes yet</li>';
          return;
        }

        episodes.forEach((ep, idx) => {
          const li = document.createElement('li');
          li.className = 'episode-item' + (currentEpisode && currentEpisode.id === ep.id ? ' active' : '');
          const turnCount = Object.keys(ep.nodes || {}).length;
          li.innerHTML = `
            <div class="id">${ep.id}</div>
            <div class="model">🤖 ${ep.model}</div>
            <div class="turns">${turnCount} turns • Active Leaf: ${(ep.active_leaf_id || 'none').slice(0, 10)}...</div>
          `;
          li.onclick = () => selectEpisode(ep);
          listEl.appendChild(li);
        });

        if (!currentEpisode && episodes.length > 0) {
          selectEpisode(episodes[0]);
        }
      } catch (err) {
        console.error('Failed to fetch episodes:', err);
      }
    }

    async function selectEpisode(ep) {
      currentEpisode = ep;
      document.querySelectorAll('.episode-item').forEach(el => el.classList.remove('active'));
      
      document.getElementById('telemetry-ep-id').innerText = ep.id;
      document.getElementById('telemetry-model').innerText = ep.model;
      const turnCount = Object.keys(ep.nodes || {}).length;
      document.getElementById('telemetry-turns').innerText = turnCount;

      renderTree(ep);
      renderHITL(ep);
    }

    function renderTree(ep) {
      const container = document.getElementById('tree-container');
      container.innerHTML = '';

      if (ep.system_prompt) {
        const sysCard = document.createElement('div');
        sysCard.className = 'turn-card';
        sysCard.innerHTML = `
          <div class="turn-header">
            <span class="role-badge" style="background: #6e7681; color: #fff;">SYSTEM PROMPT</span>
          </div>
          <div class="turn-body">${escapeHtml(ep.system_prompt)}</div>
        `;
        container.appendChild(sysCard);
      }

      // Linearize nodes by chronological order
      const nodes = Object.values(ep.nodes || {}).sort((a, b) => a.created_at - b.created_at);

      nodes.forEach(node => {
        const card = document.createElement('div');
        const isLeaf = ep.active_leaf_id === node.id;
        card.className = 'turn-card' + (isLeaf ? ' active-leaf' : '');

        let contentHtml = '';
        if (node.user_message) {
          contentHtml += `
            <div style="margin-bottom: 8px;">
              <span class="role-badge role-user">USER</span>
              <div class="turn-body" style="margin-top: 4px;">${escapeHtml(node.user_message.content || '')}</div>
            </div>
          `;
        }

        if (node.tool_calls && node.tool_calls.length > 0) {
          node.tool_calls.forEach(tc => {
            contentHtml += `
              <div style="background: #0d1117; border: 1px solid var(--border); border-radius: 6px; padding: 8px; margin: 8px 0; font-size: 0.8rem;">
                <span style="color: var(--accent-orange); font-weight: 600;">🛠️ Tool Call:</span> <code>${tc.function.name}</code>
                <pre style="color: var(--text-muted); font-size: 0.75rem; margin-top: 4px;">${escapeHtml(tc.function.arguments)}</pre>
              </div>
            `;
          });
        }

        if (node.assistant_message) {
          contentHtml += `
            <div style="margin-top: 8px;">
              <span class="role-badge role-assistant">ASSISTANT</span>
              <div class="turn-body" style="margin-top: 4px;">${escapeHtml(node.assistant_message.content || '')}</div>
            </div>
          `;
        }

        const ttft = node.timing ? `${node.timing.ttft_ms}ms` : 'n/a';
        const totalMs = node.timing ? `${node.timing.total_ms}ms` : 'n/a';
        const tokens = node.token_usage ? `${node.token_usage.total_tokens} tokens` : 'n/a';

        card.innerHTML = `
          <div class="turn-header">
            <span>Node: <code>${node.id}</code> ${isLeaf ? '🌿 (Active Leaf)' : ''}</span>
            <button class="btn btn-fork" onclick="forkFromNode('${ep.id}', '${node.id}')">Fork Branch</button>
          </div>
          ${contentHtml}
          <div class="metrics">
            <span>TTFT: <strong>${ttft}</strong></span>
            <span>Total: <strong>${totalMs}</strong></span>
            <span>Usage: <strong>${tokens}</strong></span>
          </div>
        `;
        container.appendChild(card);
      });
    }

    function renderHITL(ep) {
      const container = document.getElementById('hitl-container');
      container.innerHTML = '';

      const pending = Object.entries(ep.pending_approvals || {});
      if (pending.length === 0) {
        container.innerHTML = '<div style="font-size: 0.75rem; color: var(--text-muted);">No pending human approvals.</div>';
        return;
      }

      pending.forEach(([callId, item]) => {
        const div = document.createElement('div');
        div.className = 'hitl-item';
        div.innerHTML = `
          <div style="font-weight: 600; font-size: 0.8rem; color: var(--accent-orange);">
            ⚠️ Approval Required: <code>${item.tool_call.function.name}</code>
          </div>
          <pre style="font-size: 0.7rem; color: var(--text-muted); margin: 6px 0; max-height: 80px; overflow: auto;">${escapeHtml(item.tool_call.function.arguments)}</pre>
          <div style="margin-top: 8px;">
            <button class="btn btn-approve" onclick="approveTool('${ep.id}', '${callId}', true)">Approve</button>
            <button class="btn btn-reject" onclick="approveTool('${ep.id}', '${callId}', false)">Reject</button>
          </div>
        `;
        container.appendChild(div);
      });
    }

    async function forkFromNode(epId, nodeId) {
      if (!confirm(`Fork conversation from turn ${nodeId}?`)) return;
      await fetch(`/v1/episodes/${epId}/fork`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ from_node_id: nodeId })
      });
      refreshEpisodes();
    }

    async function approveTool(epId, callId, approved) {
      await fetch(`/v1/episodes/${epId}/approve`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ tool_call_id: callId, approved: approved })
      });
      refreshEpisodes();
    }

    function escapeHtml(str) {
      return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
    }

    refreshEpisodes();
    setInterval(refreshEpisodes, 5000);
  </script>
</body>
</html>"#;
