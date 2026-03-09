import{_ as y,i as k,o as l,c as d,b as t,a as v,F as f,r as h,l as S,m as g,s as A,p as o,t as p,k as T,q as w}from"./index-DLO25mcd.js";const N={class:"terminal-view"},_={class:"terminal-container"},E={class:"command"},C={key:0,class:"output"},L={class:"input-line"},B={__name:"Terminal",setup(M){const n=o([{command:"welcome",output:`Welcome to Housaky Terminal v1.0.0
Type "help" for available commands.`}]),c=o(""),i=o(null),m=o(null),r={help:()=>`Available commands:
  help     - Show this help
  status   - Display system status
  agents   - List connected agents
  memory   - View memory statistics
  tasks    - Show active tasks
  clear    - Clear terminal
  whoami   - Current user info
  uptime   - System uptime`,status:()=>`System Status:
  Singularity: 0.1%
  Active Agents: 5
  Memory Used: 48.2 MB
  Tasks Running: 12
  Uptime: ${new Date().toISOString().substr(11,8)}`,agents:()=>`Connected Agents:
  [ONLINE] AGENT-001 - Kowalski
  [ONLINE] AGENT-002 - DeepThink
  [ONLINE] AGENT-003 - MemoryCore
  [IDLE]   AGENT-004 - SkillMaster
  [BUSY]   AGENT-005 - ResearchBot`,memory:()=>`Memory Statistics:
  SQLite Backend: Connected
  Lucid Backend: Active
  Embeddings: 12,450 vectors
  Context Chunks: 847
  Semantic Search: Enabled`,tasks:()=>`Active Tasks:
  [1] Self-awareness training - 80%
  [2] Memory federation - 60%
  [3] Research analysis - 45%
  [4] Skill optimization - 30%`,whoami:()=>"housaky@agi",uptime:()=>`System uptime: ${new Date().toISOString().substr(11,8)}`,clear:()=>(n.value=[],null)};function u(){const s=c.value.trim();if(!s)return;let a=null;r[s]?a=r[s]():a=`Command not found: ${s}
Type "help" for available commands.`,a?n.value.push({command:s,output:a}):n.value.push({command:s,output:null}),c.value="",w(()=>{i.value&&(i.value.scrollTop=i.value.scrollHeight)})}return k(()=>{m.value&&m.value.focus()}),(s,a)=>(l(),d("div",N,[t("div",_,[a[3]||(a[3]=v('<div class="terminal-header-bar" data-v-4150c2bb><span class="terminal-title" data-v-4150c2bb>HOUSAKY TERMINAL v1.0.0</span><div class="terminal-controls" data-v-4150c2bb><span class="control" data-v-4150c2bb>_</span><span class="control" data-v-4150c2bb>□</span><span class="control" data-v-4150c2bb>×</span></div></div>',1)),t("div",{ref_key:"terminalBody",ref:i,class:"terminal-body"},[(l(!0),d(f,null,h(n.value,(e,b)=>(l(),d("div",{key:b,class:"terminal-line"},[a[1]||(a[1]=t("span",{class:"prompt"},"housaky@agi:~$",-1)),t("span",E,p(e.command),1),e.output?(l(),d("div",C,p(e.output),1)):T("",!0)]))),128)),t("div",L,[a[2]||(a[2]=t("span",{class:"prompt"},"housaky@agi:~$",-1)),S(t("input",{ref_key:"inputRef",ref:m,"onUpdate:modelValue":a[0]||(a[0]=e=>c.value=e),type:"text",class:"terminal-input",autofocus:"",onKeydown:A(u,["enter"])},null,544),[[g,c.value]])])],512)]),a[4]||(a[4]=v('<div class="help-panel mt-4" data-v-4150c2bb><div class="card" data-v-4150c2bb><div class="card-header" data-v-4150c2bb> [ AVAILABLE COMMANDS ] </div><div class="card-body" data-v-4150c2bb><div class="commands-grid" data-v-4150c2bb><div class="command-item" data-v-4150c2bb><span class="cmd" data-v-4150c2bb>help</span><span class="desc" data-v-4150c2bb>Show available commands</span></div><div class="command-item" data-v-4150c2bb><span class="cmd" data-v-4150c2bb>status</span><span class="desc" data-v-4150c2bb>System status</span></div><div class="command-item" data-v-4150c2bb><span class="cmd" data-v-4150c2bb>agents</span><span class="desc" data-v-4150c2bb>List connected agents</span></div><div class="command-item" data-v-4150c2bb><span class="cmd" data-v-4150c2bb>memory</span><span class="desc" data-v-4150c2bb>View memory stats</span></div><div class="command-item" data-v-4150c2bb><span class="cmd" data-v-4150c2bb>tasks</span><span class="desc" data-v-4150c2bb>Active tasks</span></div><div class="command-item" data-v-4150c2bb><span class="cmd" data-v-4150c2bb>clear</span><span class="desc" data-v-4150c2bb>Clear terminal</span></div></div></div></div></div>',1))]))}},x=y(B,[["__scopeId","data-v-4150c2bb"]]);export{x as default};
