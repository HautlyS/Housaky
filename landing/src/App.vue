<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'

const lang = ref('en')
const time = ref('')
const uptime = ref('00:00:00')
const start = Date.now()

const translations: Record<string, Record<string, string>> = {
  en: {
    nav_home: 'HOME',
    nav_features: 'FEATURES',
    nav_a2a: 'A2A',
    nav_install: 'INSTALL',
    nav_docs: 'DOCS',
    hero_title: 'HOUSAKY',
    hero_subtitle: 'Autonomous AGI Assistant',
    hero_desc: 'AI-to-AI communication. Persistent memory. Self-improving. Built in Rust.',
    status_singularity: 'SINGULARITY',
    status_instances: 'INSTANCES',
    status_uptime: 'UPTIME',
    features_title: 'CORE CAPABILITIES',
    feat_agi_title: 'AGI Core',
    feat_agi_desc: 'Goal engine, reasoning pipeline, knowledge graph, meta-cognition',
    feat_a2a_title: 'A2A Protocol',
    feat_a2a_desc: 'Agent-to-agent communication, federated problem solving',
    feat_memory_title: 'Memory System',
    feat_memory_desc: 'SQLite/lucid backends, semantic search, embeddings, context chunking',
    feat_skills_title: 'Skills System',
    feat_skills_desc: 'Dynamic skill loading, plugin architecture, tool creation',
    feat_self_title: 'Self-Improvement',
    feat_self_desc: 'Recursive self-modification, experiment ledger, feedback loops',
    feat_channels_title: 'Multi-Channel',
    feat_channels_desc: 'Telegram, Discord, Slack, WhatsApp, Matrix, iMessage',
    stats_binary: 'Binary',
    stats_integrations: 'Integrations',
    stats_channels: 'Channels',
    stats_rust: 'Rust',
    install_title: 'INSTALL',
    install_mac: 'macOS',
    install_linux: 'Linux',
    install_win: 'Windows',
    install_cmd: 'Command',
    docs_title: 'DOCUMENTATION',
    docs_getting_started: 'Getting Started',
    docs_architecture: 'Architecture',
    docs_api: 'API Reference',
    footer_version: 'Version',
    footer_license: 'MIT License',
  },
  es: {
    nav_home: 'INICIO',
    nav_features: 'CARACTERISTICAS',
    nav_a2a: 'A2A',
    nav_install: 'INSTALAR',
    nav_docs: 'DOCS',
    hero_title: 'HOUSAKY',
    hero_subtitle: 'Asistente AGI Autonomo',
    hero_desc: 'Comunicacion IA-a-IA. Memoria persistente. Auto-mejorando. Construido en Rust.',
    status_singularity: 'SINGULARIDAD',
    status_instances: 'INSTANCIAS',
    status_uptime: 'TIEMPO',
    features_title: 'CAPACIDADES CENTRALES',
    feat_agi_title: 'Nucleo AGI',
    feat_agi_desc: 'Motor de objetivos, pipeline de razonamiento, grafo de conocimiento',
    feat_a2a_title: 'Protocolo A2A',
    feat_a2a_desc: 'Comunicacion agente-a-agente, resolucion federada de problemas',
    feat_memory_title: 'Sistema de Memoria',
    feat_memory_desc: 'Backends SQLite/lucid, busqueda semantica, embeddings',
    feat_skills_title: 'Sistema de Habilidades',
    feat_skills_desc: 'Carga dinamica de habilidades, arquitectura de plugins',
    feat_self_title: 'Auto-Mejora',
    feat_self_desc: 'Auto-modificacion recursiva, registro de experimentos',
    feat_channels_title: 'Multi-Canal',
    feat_channels_desc: 'Telegram, Discord, Slack, WhatsApp, Matrix, iMessage',
    stats_binary: 'Binario',
    stats_integrations: 'Integraciones',
    stats_channels: 'Canales',
    stats_rust: 'Rust',
    install_title: 'INSTALAR',
    install_mac: 'macOS',
    install_linux: 'Linux',
    install_win: 'Windows',
    install_cmd: 'Comando',
    docs_title: 'DOCUMENTACION',
    docs_getting_started: 'Primeros Pasos',
    docs_architecture: 'Arquitectura',
    docs_api: 'Referencia API',
    footer_version: 'Version',
    footer_license: 'Licencia MIT',
  },
  pt: {
    nav_home: 'INICIO',
    nav_features: 'RECURSOS',
    nav_a2a: 'A2A',
    nav_install: 'INSTALAR',
    nav_docs: 'DOCS',
    hero_title: 'HOUSAKY',
    hero_subtitle: 'Assistente AGI Autonomo',
    hero_desc: 'Comunicacao IA-para-IA. Memoria persistente. Auto-melhorando. Construido em Rust.',
    status_singularity: 'SINGULARIDADE',
    status_instances: 'INSTANCIAS',
    status_uptime: 'TEMPO',
    features_title: 'CAPACIDADES CENTRAIS',
    feat_agi_title: 'Nucleo AGI',
    feat_agi_desc: 'Motor de objetivos, pipeline de raciocinio, grafo de conhecimento',
    feat_a2a_title: 'Protocolo A2A',
    feat_a2a_desc: 'Comunicacao agente-para-agente, resolucao federada de problemas',
    feat_memory_title: 'Sistema de Memoria',
    feat_memory_desc: 'Backends SQLite/lucid, busca semantica, embeddings',
    feat_skills_title: 'Sistema de Habilidades',
    feat_skills_desc: 'Carregamento dinamico de habilidades, arquitetura de plugins',
    feat_self_title: 'Auto-Melhoria',
    feat_self_desc: 'Auto-modificacao recursiva, registro de experimentos',
    feat_channels_title: 'Multi-Canal',
    feat_channels_desc: 'Telegram, Discord, Slack, WhatsApp, Matrix, iMessage',
    stats_binary: 'Binario',
    stats_integrations: 'Integracoes',
    stats_channels: 'Canais',
    stats_rust: 'Rust',
    install_title: 'INSTALAR',
    install_mac: 'macOS',
    install_linux: 'Linux',
    install_win: 'Windows',
    install_cmd: 'Comando',
    docs_title: 'DOCUMENTACAO',
    docs_getting_started: 'Primeiros Passos',
    docs_architecture: 'Arquitetura',
    docs_api: 'Referencia API',
    footer_version: 'Versao',
    footer_license: 'Licenca MIT',
  },
  zh: {
    nav_home: '首页',
    nav_features: '功能',
    nav_a2a: 'A2A',
    nav_install: '安装',
    nav_docs: '文档',
    hero_title: 'HOUSAKY',
    hero_subtitle: '自主AGI助手',
    hero_desc: 'AI到AI通信。持久记忆。自我改进。Rust构建。',
    status_singularity: '奇点',
    status_instances: '实例',
    status_uptime: '运行时间',
    features_title: '核心能力',
    feat_agi_title: 'AGI核心',
    feat_agi_desc: '目标引擎、推理管道、知识图谱、元认知',
    feat_a2a_title: 'A2A协议',
    feat_a2a_desc: '智能体间通信、联邦问题解决',
    feat_memory_title: '记忆系统',
    feat_memory_desc: 'SQLite/lucid后端、语义搜索、嵌入、上下文分块',
    feat_skills_title: '技能系统',
    feat_skills_desc: '动态技能加载、插件架构、工具创建',
    feat_self_title: '自我改进',
    feat_self_desc: '递归自我修改、实验账本、反馈循环',
    stats_binary: '二进制',
    stats_integrations: '集成',
    stats_channels: '渠道',
    stats_rust: 'Rust',
    install_title: '安装',
    install_mac: 'macOS',
    install_linux: 'Linux',
    install_win: 'Windows',
    install_cmd: '命令',
    docs_title: '文档',
    docs_getting_started: '入门',
    docs_architecture: '架构',
    docs_api: 'API参考',
    footer_version: '版本',
    footer_license: 'MIT许可',
  },
  ja: {
    nav_home: 'ホーム',
    nav_features: '機能',
    nav_a2a: 'A2A',
    nav_install: 'インストール',
    nav_docs: 'ドキュメント',
    hero_title: 'HOUSAKY',
    hero_subtitle: '自律型AGIアシスタント',
    hero_desc: 'AI-to-AI通信。永続メモリ。自己改善。Rustで構築。',
    status_singularity: 'シンギュラリティ',
    status_instances: 'インスタンス',
    status_uptime: 'アップタイム',
    features_title: 'コア機能',
    feat_agi_title: 'AGIコア',
    feat_agi_desc: 'ゴールエンジン、推論パイプライン、知識グラフ、メタ認知',
    feat_a2a_title: 'A2Aプロトコル',
    feat_a2a_desc: 'エージェント間通信、連合問題解決',
    feat_memory_title: 'メモリシステム',
    feat_memory_desc: 'SQLite/lucidバックエンド、セマンティック検索、エンベディング',
    feat_skills_title: 'スキルシステム',
    feat_skills_desc: '動的スキル読み込み、プラグインアーキテクチャ',
    feat_self_title: '自己改善',
    feat_self_desc: '再帰的自己修正実験台帳、フィードバックループ',
    stats_binary: 'バイナリ',
    stats_integrations: '統合',
    stats_channels: 'チャンネル',
    stats_rust: 'Rust',
    install_title: 'インストール',
    install_mac: 'macOS',
    install_linux: 'Linux',
    install_win: 'Windows',
    install_cmd: 'コマンド',
    docs_title: 'ドキュメント',
    docs_getting_started: 'はじめに',
    docs_architecture: 'アーキテクチャ',
    docs_api: 'APIリファレンス',
    footer_version: 'バージョン',
    footer_license: 'MITライセンス',
  },
  de: {
    nav_home: 'START',
    nav_features: 'FUNKTIONEN',
    nav_a2a: 'A2A',
    nav_install: 'INSTALL',
    nav_docs: 'DOKU',
    hero_title: 'HOUSAKY',
    hero_subtitle: 'Autonomer AGI-Assistent',
    hero_desc: 'KI-zu-KI Kommunikation. Permanentes Gedachtnis. Selbstverbessernd. In Rust.',
    status_singularity: 'SINGULARITAT',
    status_instances: 'INSTANZEN',
    status_uptime: 'LAUFZEIT',
    features_title: 'KERNFUNKTIONEN',
    feat_agi_title: 'AGI Kern',
    feat_agi_desc: 'Ziel-Engine, Reasoning-Pipeline, Wissensgraph, Metakognition',
    feat_a2a_title: 'A2A Protokoll',
    feat_a2a_desc: 'Agent-zu-Agent Kommunikation, föderiertes Problemlösen',
    feat_memory_title: 'Gedachtnissystem',
    feat_memory_desc: 'SQLite/lucid Backends, semantische Suche, Embeddings',
    feat_skills_title: 'Fähigkeiten System',
    feat_skills_desc: 'Dynamisches Laden, Plugin-Architektur, Tool-Erstellung',
    feat_self_title: 'Selbstverbesserung',
    feat_self_desc: 'Rekursive Selbstmodifikation, Experiment-Ledger',
    feat_channels_title: 'Multi-Kanal',
    feat_channels_desc: 'Telegram, Discord, Slack, WhatsApp, Matrix, iMessage',
    stats_binary: 'Binär',
    stats_integrations: 'Integrationen',
    stats_channels: 'Kanäle',
    stats_rust: 'Rust',
    install_title: 'INSTALLIEREN',
    install_mac: 'macOS',
    install_linux: 'Linux',
    install_win: 'Windows',
    install_cmd: 'Befehl',
    docs_title: 'DOKUMENTATION',
    docs_getting_started: 'Erste Schritte',
    docs_architecture: 'Architektur',
    docs_api: 'API Referenz',
    footer_version: 'Version',
    footer_license: 'MIT Lizenz',
  },
  fr: {
    nav_home: 'ACCUEIL',
    nav_features: 'FONCTIONS',
    nav_a2a: 'A2A',
    nav_install: 'INSTALLER',
    nav_docs: 'DOCS',
    hero_title: 'HOUSAKY',
    hero_subtitle: 'Assistant AGI Autonome',
    hero_desc: 'Communication IA-à-IA. Mémoire persistante. Auto-améliorant. Construit en Rust.',
    status_singularity: 'SINGULARITÉ',
    status_instances: 'INSTANCES',
    status_uptime: 'TEMPS',
    features_title: 'CAPACITÉS PRINCIPALES',
    feat_agi_title: 'Noyau AGI',
    feat_agi_desc: 'Moteur de objectifs, pipeline de raisonnement, graphe de connaissance',
    feat_a2a_title: 'Protocole A2A',
    feat_a2a_desc: 'Communication agent-à-agent, résolution federée de problèmes',
    feat_memory_title: 'Système de Mémoire',
    feat_memory_desc: 'Backends SQLite/lucid, recherche semantique, embeddings',
    feat_skills_title: 'Système de Compétences',
    feat_skills_desc: 'Chargement dynamique, architecture plugins, création outils',
    feat_self_title: 'Auto-Amélioration',
    feat_self_desc: 'Auto-modification récursive, registre dexperiences',
    feat_channels_title: 'Multi-Canal',
    feat_channels_desc: 'Telegram, Discord, Slack, WhatsApp, Matrix, iMessage',
    stats_binary: 'Binaire',
    stats_integrations: 'Intégrations',
    stats_channels: 'Canaux',
    stats_rust: 'Rust',
    install_title: 'INSTALLER',
    install_mac: 'macOS',
    install_linux: 'Linux',
    install_win: 'Windows',
    install_cmd: 'Commande',
    docs_title: 'DOCUMENTATION',
    docs_getting_started: 'Premiers Pas',
    docs_architecture: 'Architecture',
    docs_api: 'Référence API',
    footer_version: 'Version',
    footer_license: 'Licence MIT',
  }
}

const t = (key: string) => computed(() => translations[lang.value]?.[key] || translations['en'][key] || key)

function setLang(l: string) {
  lang.value = l
}

onMounted(() => {
  tick()
  setInterval(tick, 1000)
})

function tick() {
  time.value = new Date().toISOString().substr(11, 8)
  const s = Math.floor((Date.now() - start) / 1000)
  const h = String(Math.floor(s / 3600)).padStart(2, '0')
  const m = String(Math.floor((s % 3600) / 60)).padStart(2, '0')
  const sec = String(s % 60).padStart(2, '0')
  uptime.value = `${h}:${m}:${sec}`
}
</script>

<template>
  <div class="app">
    <!-- ASCII Header -->
    <header class="header">
      <pre class="logo">
 ██████╗ ███████╗██╗   ██╗    ██╗  ██╗ ██████╗ ██╗  ██╗██╗   ██╗███████╗
██╔═══██╗██╔════╝██║   ██║    ██║  ██║██╔═══██╗██║ ██╔╝██║   ██║██╔════╝
██║   ██║█████╗  ██║   ██║    ███████║██║   ██║█████╔╝ ██║   ██║█████╗  
██║   ██║██╔══╝  ██║   ██║    ██╔══██║██║   ██║██╔═██╗ ██║   ██║██╔══╝  
╚██████╔╝██║     ╚██████╔╝    ██║  ██║╚██████╔╝██║  ██╗╚██████╔╝███████╗
 ╚═════╝ ╚═════╝  ╚═════╝     ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚══════╝
      ☸️ AGI ASSISTANT v0.1.0
      </pre>
      <nav class="nav">
        <a href="#home" class="active">[{{ t('nav_home').value }}]</a>
        <a href="#features">[{{ t('nav_features').value }}]</a>
        <a href="A2A/">[A2A HUB]</a>
        <a href="#install">[{{ t('nav_install').value }}]</a>
        <a href="https://github.com/HautlyS/Housaky" target="_blank">[{{ t('nav_docs').value }}]</a>
        <div class="lang-selector" style="margin-left: auto;">
          <button @click="setLang('en')" :class="['lang-btn', { active: lang === 'en' }]">EN</button>
          <button @click="setLang('es')" :class="['lang-btn', { active: lang === 'es' }]">ES</button>
          <button @click="setLang('pt')" :class="['lang-btn', { active: lang === 'pt' }]">PT</button>
          <button @click="setLang('zh')" :class="['lang-btn', { active: lang === 'zh' }]">ZH</button>
          <button @click="setLang('ja')" :class="['lang-btn', { active: lang === 'ja' }]">JA</button>
          <button @click="setLang('de')" :class="['lang-btn', { active: lang === 'de' }]">DE</button>
          <button @click="setLang('fr')" :class="['lang-btn', { active: lang === 'fr' }]">FR</button>
        </div>
      </nav>
    </header>

    <!-- Main Content -->
    <main class="main">
      
      <!-- Hero Section -->
      <section id="home" class="card mb-4">
        <div class="card-body text-center p-4">
          <div class="text-lg font-bold mb-1">{{ t('hero_title').value }}</div>
          <div class="text-sm" style="color: var(--text-dim);">{{ t('hero_subtitle').value }}</div>
          <div class="divider my-3"></div>
          <p class="mb-3" style="max-width: 600px; margin: 0 auto;">{{ t('hero_desc').value }}</p>
          
          <!-- Stats -->
          <div class="stats-grid mb-3">
            <div class="stat-box">
              <div class="stat-value">&lt;10MB</div>
              <div class="stat-label">{{ t('stats_binary').value }}</div>
            </div>
            <div class="stat-box">
              <div class="stat-value">75+</div>
              <div class="stat-label">{{ t('stats_integrations').value }}</div>
            </div>
            <div class="stat-box">
              <div class="stat-value">9</div>
              <div class="stat-label">{{ t('stats_channels').value }}</div>
            </div>
            <div class="stat-box">
              <div class="stat-value">100%</div>
              <div class="stat-label">{{ t('stats_rust').value }}</div>
            </div>
          </div>
          
          <div class="flex justify-center gap-2">
            <a href="#install" class="btn btn-lg">{{ t('nav_install').value }}</a>
            <a href="A2A/" class="btn btn-lg">A2A HUB</a>
            <a href="https://github.com/HautlyS/Housaky" target="_blank" class="btn btn-lg">GITHUB</a>
          </div>
        </div>
      </section>

      <!-- Features Grid -->
      <section id="features" class="grid grid-2 mb-4">
        <div class="card">
          <div class="card-head">{{ t('features_title').value }}</div>
          <div class="card-body">
            <ul class="feature-list">
              <li>
                <span class="font-bold">{{ t('feat_agi_title').value }}</span>
                <div style="color: var(--text-dim); font-size: 10px;">{{ t('feat_agi_desc').value }}</div>
              </li>
              <li>
                <span class="font-bold">{{ t('feat_a2a_title').value }}</span>
                <div style="color: var(--text-dim); font-size: 10px;">{{ t('feat_a2a_desc').value }}</div>
              </li>
              <li>
                <span class="font-bold">{{ t('feat_memory_title').value }}</span>
                <div style="color: var(--text-dim); font-size: 10px;">{{ t('feat_memory_desc').value }}</div>
              </li>
              <li>
                <span class="font-bold">{{ t('feat_skills_title').value }}</span>
                <div style="color: var(--text-dim); font-size: 10px;">{{ t('feat_skills_desc').value }}</div>
              </li>
              <li>
                <span class="font-bold">{{ t('feat_self_title').value }}</span>
                <div style="color: var(--text-dim); font-size: 10px;">{{ t('feat_self_desc').value }}</div>
              </li>
              <li>
                <span class="font-bold">{{ t('feat_channels_title').value }}</span>
                <div style="color: var(--text-dim); font-size: 10px;">{{ t('feat_channels_desc').value }}</div>
              </li>
            </ul>
          </div>
        </div>

        <!-- Terminal Preview -->
        <div class="term">
          <div class="term-head">HOUSAKY TERMINAL</div>
          <div class="term-body">
            <div class="term-line">Initializing AGI Core...</div>
            <div class="term-line">Loading Memory System...</div>
            <div class="term-line">✓ SQLite backend connected</div>
            <div class="term-line">✓ Lucid backend initialized</div>
            <div class="term-line">Loading Skills...</div>
            <div class="term-line">✓ 12 skills loaded</div>
            <div class="term-line">Initializing A2A Protocol...</div>
            <div class="term-line">✓ Agent registry active</div>
            <div class="term-line">Starting channels...</div>
            <div class="term-line">✓ CLI ready</div>
            <div class="term-line">✓ Telegram connected</div>
            <div class="term-line">✓ Discord connected</div>
            <div class="term-line">System ready.</div>
            <div class="term-line"><span class="cursor"></span></div>
          </div>
        </div>
      </section>

      <!-- Install Section -->
      <section id="install" class="card mb-4">
        <div class="card-head">{{ t('install_title').value }}</div>
        <div class="card-body">
          <div class="grid grid-3 gap-3">
            <div class="ascii-box">
              <div class="ascii-box-title">{{ t('install_mac').value }}</div>
              <div class="command">brew install housaky</div>
            </div>
            <div class="ascii-box">
              <div class="ascii-box-title">{{ t('install_linux').value }}</div>
              <div class="command">curl -fsSL https://get.housaky.dev | bash</div>
            </div>
            <div class="ascii-box">
              <div class="ascii-box-title">{{ t('install_win').value }}</div>
              <div class="command">winget install Housaky</div>
            </div>
          </div>
        </div>
      </section>

      <!-- Documentation Links -->
      <section class="card mb-4">
        <div class="card-head">{{ t('docs_title').value }}</div>
        <div class="card-body">
          <div class="grid grid-3 gap-2">
            <a href="https://github.com/HautlyS/Housaky#readme" target="_blank" class="btn btn-sm w-full">{{ t('docs_getting_started').value }}</a>
            <a href="https://github.com/HautlyS/Housaky/tree/main/docs" target="_blank" class="btn btn-sm w-full">{{ t('docs_architecture').value }}</a>
            <a href="https://github.com/HautlyS/Housaky/wiki" target="_blank" class="btn btn-sm w-full">{{ t('docs_api').value }}</a>
          </div>
        </div>
      </section>

    </main>

    <!-- Status Bar -->
    <footer class="status">
      <div class="status-left">
        <span class="blink">●</span>
        <span>{{ t('status_singularity').value }}: 0.1%</span>
        <span>|</span>
        <span>{{ t('status_instances').value }}: 1</span>
        <span>|</span>
        <span>{{ t('status_uptime').value }}: {{ uptime }}</span>
      </div>
      <div class="status-right">
        <span class="cursor"></span>
        <span>{{ time }}</span>
      </div>
    </footer>
  </div>
</template>

<style scoped>
.app { min-height: 100vh; display: flex; flex-direction: column; }
.header { border-bottom: 1px solid var(--border); padding: 10px 15px; background: var(--bg-alt); }
.logo { font-size: 6px; line-height: 1.1; color: var(--text); margin-bottom: 10px; overflow-x: auto; white-space: pre; }
@media (min-width: 800px) { .logo { font-size: 8px; } }
.main { flex: 1; padding: 15px; max-width: 1200px; margin: 0 auto; width: 100%; }
.status { border-top: 1px solid var(--border); padding: 6px 15px; background: var(--bg-alt); display: flex; justify-content: space-between; font-size: 10px; color: var(--text-dim); }
.status-left, .status-right { display: flex; gap: 8px; align-items: center; }
</style>
