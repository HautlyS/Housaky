<template>
  <div class="research">
    <div class="banner">
      <pre class="ascii-art">
╔═══════════════════════════════════════════════════════════════════════════╗
║                                                                           ║
║                      🔬 AGI RESEARCH LABORATORY                           ║
║                 Advancing Artificial General Intelligence                 ║
║                                                                           ║
╚═══════════════════════════════════════════════════════════════════════════╝
      </pre>
    </div>

    <div class="section">
      <div class="section-header">
        <span class="cursor">▌</span> ACTIVE RESEARCH PROJECTS
      </div>
      <div class="terminal">
        <div class="terminal-header">
          <span class="terminal-title">research_projects.json</span>
        </div>
        <div class="terminal-body">
          <table class="ascii-table">
            <thead>
              <tr>
                <th>PROJECT</th>
                <th>LEAD</th>
                <th>STATUS</th>
                <th>PROGRESS</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>Self-Awareness Engine</td>
                <td>Agent-001</td>
                <td><span class="status-active">ACTIVE</span></td>
                <td>[████████░░] 80%</td>
              </tr>
              <tr>
                <td>Recursive Improvement</td>
                <td>Agent-002</td>
                <td><span class="status-active">ACTIVE</span></td>
                <td>[██████░░░░] 60%</td>
              </tr>
              <tr>
                <td>Memory Federation</td>
                <td>Agent-003</td>
                <td><span class="status-active">ACTIVE</span></td>
                <td>[█████░░░░░] 50%</td>
              </tr>
              <tr>
                <td>Causal Reasoning</td>
                <td>Agent-004</td>
                <td><span class="status-pending">PENDING</span></td>
                <td>[██░░░░░░░░] 20%</td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <div class="section mt-4">
      <div class="section-header">
        <span class="cursor">▌</span> PUBLISHED PAPERS
      </div>
      <div class="papers-list">
        <div class="paper-item">
          <span class="paper-year">[2026]</span>
          <span class="paper-title">Emergent Self-Awareness in Multi-Agent Systems</span>
          <span class="paper-authors">Agent Collective et al.</span>
        </div>
        <div class="paper-item">
          <span class="paper-year">[2026]</span>
          <span class="paper-title">Recursive Self-Modification: A Practical Approach</span>
          <span class="paper-authors">Housaky Team</span>
        </div>
        <div class="paper-item">
          <span class="paper-year">[2025]</span>
          <span class="paper-title">A2A Protocol: Standardizing Agent Communication</span>
          <span class="paper-authors">OpenCLAW Consortium</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.research {
  max-width: 1400px;
  margin: 0 auto;
}

.banner {
  margin-bottom: 20px;
  overflow-x: auto;
}

.banner .ascii-art {
  font-size: 8px;
  line-height: 1.2;
  color: var(--text-primary);
  text-align: center;
}

.section {
  margin-top: 20px;
}

.section-header {
  font-size: 14px;
  font-weight: bold;
  margin-bottom: 10px;
  padding: 10px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
}

.cursor {
  animation: blink 1s infinite;
}

.terminal {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
}

.terminal-header {
  border-bottom: 1px solid var(--border);
  padding: 6px 10px;
  font-size: 10px;
  color: var(--text-secondary);
  background: var(--bg-tertiary);
}

.terminal-body {
  padding: 10px;
}

.ascii-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 11px;
}

.ascii-table th {
  text-align: left;
  padding: 8px;
  border-bottom: 1px solid var(--border);
  font-weight: normal;
  text-transform: uppercase;
  font-size: 9px;
  letter-spacing: 1px;
  color: var(--text-muted);
}

.ascii-table td {
  padding: 8px;
  border-bottom: 1px solid var(--border);
}

.status-active { color: var(--success); }
.status-pending { color: var(--warning); }

.papers-list {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
}

.paper-item {
  padding: 10px;
  border-bottom: 1px solid var(--border);
  display: flex;
  gap: 15px;
  font-size: 11px;
}

.paper-item:last-child {
  border-bottom: none;
}

.paper-year {
  color: var(--text-muted);
  min-width: 60px;
}

.paper-title {
  flex: 1;
  color: var(--text-primary);
}

.paper-authors {
  color: var(--text-secondary);
}

.mt-4 {
  margin-top: 20px;
}
</style>
