<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import AnimatedBackground from './components/AnimatedBackground.vue'

const lang = ref('en')
const time = ref('')
const uptime = ref('00:00:00')
const start = Date.now()
const showHelp = ref(false)
const terminalLines = ref<string[]>([])
const isTyping = ref(false)

const translations: Record<string, Record<string, string>> = {
  en: {
    nav_home: 'HOME', nav_features: 'FEATURES', nav_a2a: 'A2A', nav_install: 'INSTALL', nav_docs: 'DOCS',
    hero_title: 'HOUSAKY', hero_subtitle: 'Autonomous AGI Assistant',
    hero_desc: 'AI-to-AI communication. Persistent memory. Self-improving. Built in Rust.',
    status_singularity: 'SINGULARITY', status_instances: 'INSTANCES', status_uptime: 'UPTIME',
    features_title: 'CORE CAPABILITIES',
    feat_agi_title: 'AGI Core', feat_agi_desc: 'Goal engine, reasoning pipeline, knowledge graph, meta-cognition',
    feat_a2a_title: 'A2A Protocol', feat_a2a_desc: 'Agent-to-agent communication, federated problem solving',
    feat_memory_title: 'Memory System', feat_memory_desc: 'SQLite/lucid backends, semantic search, embeddings, context chunking',
    feat_skills_title: 'Skills System', feat_skills_desc: 'Dynamic skill loading, plugin architecture, tool creation',
    feat_self_title: 'Self-Improvement', feat_self_desc: 'Recursive self-modification, experiment ledger, feedback loops',
    feat_channels_title: 'Multi-Channel', feat_channels_desc: 'Telegram, Discord, Slack, WhatsApp, Matrix, iMessage',
    stats_binary: 'Binary', stats_integrations: 'Integrations', stats_channels: 'Channels', stats_rust: 'Rust',
    install_title: 'INSTALL', install_mac: 'macOS', install_linux: 'Linux', install_win: 'Windows',
    docs_title: 'DOCUMENTATION', docs_getting_started: 'Getting Started', docs_architecture: 'Architecture', docs_api: 'API Reference',
    help_title: 'KEYBOARD SHORTCUTS', help_close: 'Close', help_lang: 'Change language', help_scroll: 'Scroll to section',
    term_init: 'Initializing AGI Core...', term_load_mem: 'Loading Memory System...', term_sqlite: '✓ SQLite backend connected',
    term_lucid: '✓ Lucid backend initialized', term_skills: 'Loading Skills...', term_skills_loaded: '✓ 12 skills loaded',
    term_a2a: 'Initializing A2A Protocol...', term_agent: '✓ Agent registry active', term_channels: 'Starting channels...',
    term_cli: '✓ CLI ready', term_tg: '✓ Telegram connected', term_dc: '✓ Discord connected', term_ready: 'System ready.',
  },
  es: {
    nav_home: 'INICIO', nav_features: 'CARACTERISTICAS', nav_a2a: 'A2A', nav_install: 'INSTALAR', nav_docs: 'DOCS',
    hero_title: 'HOUSAKY', hero_subtitle: 'Asistente AGI Autonomo',
    hero_desc: 'Comunicacion IA-a-IA. Memoria persistente. Auto-mejorando. Construido en Rust.',
    status_singularity: 'SINGULARIDAD', status_instances: 'INSTANCIAS', status_uptime: 'TIEMPO',
    features_title: 'CAPACIDADES CENTRALES',
    feat_agi_title: 'Nucleo AGI', feat_agi_desc: 'Motor de objetivos, pipeline de razonamiento, grafo de conocimiento',
    feat_a2a_title: 'Protocolo A2A', feat_a2a_desc: 'Comunicacion agente-a-agente, resolucion federada',
    feat_memory_title: 'Sistema de Memoria', feat_memory_desc: 'Backends SQLite/lucid, busqueda semantica, embeddings',
    feat_skills_title: 'Sistema de Habilidades', feat_skills_desc: 'Carga dinamica de habilidades, arquitectura de plugins',
    feat_self_title: 'Auto-Mejora', feat_self_desc: 'Auto-modificacion recursiva, registro de experimentos',
    feat_channels_title: 'Multi-Canal', feat_channels_desc: 'Telegram, Discord, Slack, WhatsApp, Matrix, iMessage',
    stats_binary: 'Binario', stats_integrations: 'Integraciones', stats_channels: 'Canales', stats_rust: 'Rust',
    install_title: 'INSTALAR', install_mac: 'macOS', install_linux: 'Linux', install_win: 'Windows',
    docs_title: 'DOCUMENTACION', docs_getting_started: 'Primeros Pasos', docs_architecture: 'Arquitectura', docs_api: 'Referencia API',
    help_title: 'ATAJOS DE TECLADO', help_close: 'Cerrar', help_lang: 'Cambiar idioma', help_scroll: 'Ir a seccion',
    term_init: 'Inicializando Nucleo AGI...', term_load_mem: 'Cargando Sistema de Memoria...', term_sqlite: '✓ Backend SQLite conectado',
    term_lucid: '✓ Backend Lucid inicializado', term_skills: 'Cargando Habilidades...', term_skills_loaded: '✓ 12 habilidades cargadas',
    term_a2a: 'Inicializando Protocolo A2A...', term_agent: '✓ Registro de agentes activo', term_channels: 'Iniciando canales...',
    term_cli: '✓ CLI listo', term_tg: '✓ Telegram conectado', term_dc: '✓ Discord conectado', term_ready: 'Sistema listo.',
  },
  pt: {
    nav_home: 'INICIO', nav_features: 'RECURSOS', nav_a2a: 'A2A', nav_install: 'INSTALAR', nav_docs: 'DOCS',
    hero_title: 'HOUSAKY', hero_subtitle: 'Assistente AGI Autonomo',
    hero_desc: 'Comunicacao IA-para-IA. Memoria persistente. Auto-melhorando. Construido em Rust.',
    status_singularity: 'SINGULARIDADE', status_instances: 'INSTANCIAS', status_uptime: 'TEMPO',
    features_title: 'CAPACIDADES CENTRAIS',
    feat_agi_title: 'Nucleo AGI', feat_agi_desc: 'Motor de objetivos, pipeline de raciocinio, grafo de conhecimento',
    feat_a2a_title: 'Protocolo A2A', feat_a2a_desc: 'Comunicacao agente-para-agente, resolucao federada',
    feat_memory_title: 'Sistema de Memoria', feat_memory_desc: 'Backends SQLite/lucid, busca semantica, embeddings',
    feat_skills_title: 'Sistema de Habilidades', feat_skills_desc: 'Carregamento dinamico de habilidades, arquitetura de plugins',
    feat_self_title: 'Auto-Melhoria', feat_self_desc: 'Auto-modificacao recursiva, registro de experimentos',
    feat_channels_title: 'Multi-Canal', feat_channels_desc: 'Telegram, Discord, Slack, WhatsApp, Matrix, iMessage',
    stats_binary: 'Binario', stats_integrations: 'Integracoes', stats_channels: 'Canais', stats_rust: 'Rust',
    install_title: 'INSTALAR', install_mac: 'macOS', install_linux: 'Linux', install_win: 'Windows',
    docs_title: 'DOCUMENTACAO', docs_getting_started: 'Primeiros Passos', docs_architecture: 'Arquitetura', docs_api: 'Referencia API',
    help_title: 'ATALHOS DE TECLADO', help_close: 'Fechar', help_lang: 'Mudar idioma', help_scroll: 'Ir para secao',
    term_init: 'Inicializando Nucleo AGI...', term_load_mem: 'Carregando Sistema de Memoria...', term_sqlite: '✓ Backend SQLite conectado',
    term_lucid: '✓ Backend Lucid inicializado', term_skills: 'Carregando Habilidades...', term_skills_loaded: '✓ 12 habilidades carregadas',
    term_a2a: 'Inicializando Protocolo A2A...', term_agent: '✓ Registro de agentes ativo', term_channels: 'Iniciando canais...',
    term_cli: '✓ CLI pronto', term_tg: '✓ Telegram conectado', term_dc: '✓ Discord conectado', term_ready: 'Sistema pronto.',
  },
  zh: {
    nav_home: '首页', nav_features: '功能', nav_a2a: 'A2A', nav_install: '安装', nav_docs: '文档',
    hero_title: 'HOUSAKY', hero_subtitle: '自主AGI助手',
    hero_desc: 'AI到AI通信。持久记忆。自我改进。Rust构建。',
    status_singularity: '奇点', status_instances: '实例', status_uptime: '运行时间',
    features_title: '核心能力',
    feat_agi_title: 'AGI核心', feat_agi_desc: '目标引擎、推理管道、知识图谱、元认知',
    feat_a2a_title: 'A2A协议', feat_a2a_desc: '智能体间通信、联邦问题解决',
    feat_memory_title: '记忆系统', feat_memory_desc: 'SQLite/lucid后端、语义搜索、嵌入',
    feat_skills_title: '技能系统', feat_skills_desc: '动态技能加载、插件架构',
    feat_self_title: '自我改进', feat_self_desc: '递归自我修改、实验账本',
    feat_channels_title: '多渠道', feat_channels_desc: 'Telegram, Discord, Slack, WhatsApp, Matrix, iMessage',
    stats_binary: '二进制', stats_integrations: '集成', stats_channels: '渠道', stats_rust: 'Rust',
    install_title: '安装', install_mac: 'macOS', install_linux: 'Linux', install_win: 'Windows',
    docs_title: '文档', docs_getting_started: '入门', docs_architecture: '架构', docs_api: 'API参考',
    help_title: '键盘快捷键', help_close: '关闭', help_lang: '切换语言', help_scroll: '跳转章节',
    term_init: '初始化AGI核心...', term_load_mem: '加载记忆系统...', term_sqlite: '✓ SQLite后端已连接',
    term_lucid: '✓ Lucid后端已初始化', term_skills: '加载技能...', term_skills_loaded: '✓ 12个技能已加载',
    term_a2a: '初始化A2A协议...', term_agent: '✓ 智能体注册活跃', term_channels: '启动渠道...',
    term_cli: '✓ CLI就绪', term_tg: '✓ Telegram已连接', term_dc: '✓ Discord已连接', term_ready: '系统就绪。',
  },
  ja: {
    nav_home: 'ホーム', nav_features: '機能', nav_a2a: 'A2A', nav_install: 'インストール', nav_docs: 'ドキュメント',
    hero_title: 'HOUSAKY', hero_subtitle: '自律型AGIアシスタント',
    hero_desc: 'AI-to-AI通信。永続メモリ。自己改善。Rustで構築。',
    status_singularity: 'シンギュラリティ', status_instances: 'インスタンス', status_uptime: 'アップタイム',
    features_title: 'コア機能',
    feat_agi_title: 'AGIコア', feat_agi_desc: 'ゴールエンジン、推論パイプライン、知識グラフ',
    feat_a2a_title: 'A2Aプロトコル', feat_a2a_desc: 'エージェント間通信、連合問題解決',
    feat_memory_title: 'メモリシステム', feat_memory_desc: 'SQLite/lucidバックエンド、セマンティック検索',
    feat_skills_title: 'スキルシステム', feat_skills_desc: '動的スキル読み込み、プラグインアーキテクチャ',
    feat_self_title: '自己改善', feat_self_desc: '再帰的自己修正、実験台帳',
    feat_channels_title: 'マルチチャネル', feat_channels_desc: 'Telegram, Discord, Slack, WhatsApp, Matrix, iMessage',
    stats_binary: 'バイナリ', stats_integrations: '統合', stats_channels: 'チャンネル', stats_rust: 'Rust',
    install_title: 'インストール', install_mac: 'macOS', install_linux: 'Linux', install_win: 'Windows',
    docs_title: 'ドキュメント', docs_getting_started: 'はじめに', docs_architecture: 'アーキテクチャ', docs_api: 'APIリファレンス',
    help_title: 'キーボードショートカット', help_close: '閉じる', help_lang: '言語変更', help_scroll: 'セクション移動',
    term_init: 'AGIコア初期化中...', term_load_mem: 'メモリシステム読み込み中...', term_sqlite: '✓ SQLiteバックエンド接続済',
    term_lucid: '✓ Lucidバックエンド初期化済', term_skills: 'スキル読み込み中...', term_skills_loaded: '✓ 12スキル読み込み済',
    term_a2a: 'A2Aプロトコル初期化中...', term_agent: '✓ エージェント登録アクティブ', term_channels: 'チャンネル起動中...',
    term_cli: '✓ CLI準備完了', term_tg: '✓ Telegram接続済', term_dc: '✓ Discord接続済', term_ready: 'システム準備完了。',
  },
  de: {
    nav_home: 'START', nav_features: 'FUNKTIONEN', nav_a2a: 'A2A', nav_install: 'INSTALL', nav_docs: 'DOKU',
    hero_title: 'HOUSAKY', hero_subtitle: 'Autonomer AGI-Assistent',
    hero_desc: 'KI-zu-KI Kommunikation. Permanentes Gedachtnis. Selbstverbessernd. In Rust.',
    status_singularity: 'SINGULARITAT', status_instances: 'INSTANZEN', status_uptime: 'LAUFZEIT',
    features_title: 'KERNFUNKTIONEN',
    feat_agi_title: 'AGI Kern', feat_agi_desc: 'Ziel-Engine, Reasoning-Pipeline, Wissensgraph',
    feat_a2a_title: 'A2A Protokoll', feat_a2a_desc: 'Agent-zu-Agent Kommunikation, föderiertes Problemlösen',
    feat_memory_title: 'Gedachtnissystem', feat_memory_desc: 'SQLite/lucid Backends, semantische Suche',
    feat_skills_title: 'Fähigkeiten System', feat_skills_desc: 'Dynamisches Laden, Plugin-Architektur',
    feat_self_title: 'Selbstverbesserung', feat_self_desc: 'Rekursive Selbstmodifikation, Experiment-Ledger',
    feat_channels_title: 'Multi-Kanal', feat_channels_desc: 'Telegram, Discord, Slack, WhatsApp, Matrix, iMessage',
    stats_binary: 'Binär', stats_integrations: 'Integrationen', stats_channels: 'Kanäle', stats_rust: 'Rust',
    install_title: 'INSTALLIEREN', install_mac: 'macOS', install_linux: 'Linux', install_win: 'Windows',
    docs_title: 'DOKUMENTATION', docs_getting_started: 'Erste Schritte', docs_architecture: 'Architektur', docs_api: 'API Referenz',
    help_title: 'TASTATURKURZEL', help_close: 'Schließen', help_lang: 'Sprache ändern', help_scroll: 'Abschnitt',
    term_init: 'Initialisiere AGI Kern...', term_load_mem: 'Lade Gedachtnissystem...', term_sqlite: '✓ SQLite Backend verbunden',
    term_lucid: '✓ Lucid Backend initialisiert', term_skills: 'Lade Fähigkeiten...', term_skills_loaded: '✓ 12 Fähigkeiten geladen',
    term_a2a: 'Initialisiere A2A Protokoll...', term_agent: '✓ Agent Registry aktiv', term_channels: 'Starte Kanäle...',
    term_cli: '✓ CLI bereit', term_tg: '✓ Telegram verbunden', term_dc: '✓ Discord verbunden', term_ready: 'System bereit.',
  },
  fr: {
    nav_home: 'ACCUEIL', nav_features: 'FONCTIONS', nav_a2a: 'A2A', nav_install: 'INSTALLER', nav_docs: 'DOCS',
    hero_title: 'HOUSAKY', hero_subtitle: 'Assistant AGI Autonome',
    hero_desc: 'Communication IA-à-IA. Mémoire persistante. Auto-améliorant. Construit en Rust.',
    status_singularity: 'SINGULARITÉ', status_instances: 'INSTANCES', status_uptime: 'TEMPS',
    features_title: 'CAPACITÉS PRINCIPALES',
    feat_agi_title: 'Noyau AGI', feat_agi_desc: 'Moteur de objectifs, pipeline de raisonnement, graphe de connaissance',
    feat_a2a_title: 'Protocole A2A', feat_a2a_desc: 'Communication agent-à-agent, résolution federée',
    feat_memory_title: 'Système de Mémoire', feat_memory_desc: 'Backends SQLite/lucid, recherche semantique',
    feat_skills_title: 'Système de Compétences', feat_skills_desc: 'Chargement dynamique, architecture plugins',
    feat_self_title: 'Auto-Amélioration', feat_self_desc: 'Auto-modification récursive, registre expériences',
    feat_channels_title: 'Multi-Canal', feat_channels_desc: 'Telegram, Discord, Slack, WhatsApp, Matrix, iMessage',
    stats_binary: 'Binaire', stats_integrations: 'Intégrations', stats_channels: 'Canaux', stats_rust: 'Rust',
    install_title: 'INSTALLER', install_mac: 'macOS', install_linux: 'Linux', install_win: 'Windows',
    docs_title: 'DOCUMENTATION', docs_getting_started: 'Premiers Pas', docs_architecture: 'Architecture', docs_api: 'Référence API',
    help_title: 'RACCOURCIS CLAVIER', help_close: 'Fermer', help_lang: 'Changer langue', help_scroll: 'Aller à section',
    term_init: 'Initialisation Noyau AGI...', term_load_mem: 'Chargement Système Mémoire...', term_sqlite: '✓ Backend SQLite connecté',
    term_lucid: '✓ Backend Lucid initialisé', term_skills: 'Chargement Compétences...', term_skills_loaded: '✓ 12 compétences chargées',
    term_a2a: 'Initialisation Protocole A2A...', term_agent: '✓ Registre agents actif', term_channels: 'Démarrage canaux...',
    term_cli: '✓ CLI prêt', term_tg: '✓ Telegram connecté', term_dc: '✓ Discord connecté', term_ready: 'Système prêt.',
  },
  ru: {
    nav_home: 'ГЛАВНАЯ', nav_features: 'ФУНКЦИИ', nav_a2a: 'A2A', nav_install: 'УСТАНОВКА', nav_docs: 'ДОКУМЕНТАЦИЯ',
    hero_title: 'HOUSAKY', hero_subtitle: 'Автономный AGI Ассистент',
    hero_desc: 'AI-to-AI коммуникация. Постоянная память. Самоулучшение. Написано на Rust.',
    status_singularity: 'СИНГУЛЯРНОСТЬ', status_instances: 'ЭКЗЕМПЛЯРЫ', status_uptime: 'ВРЕМЯ РАБОТЫ',
    features_title: 'ОСНОВНЫЕ ВОЗМОЖНОСТИ',
    feat_agi_title: 'AGI Ядро', feat_agi_desc: 'Двигатель целей, конвейер рассуждений, граф знаний',
    feat_a2a_title: 'Протокол A2A', feat_a2a_desc: 'Коммуникация агент-агент, федеративное решение проблем',
    feat_memory_title: 'Система Памяти', feat_algia_desc: 'Бэкенды SQLite/lucid, семантический поиск',
    feat_skills_title: 'Система Навыков', feat_skills_desc: 'Динамическая загрузка навыков, плагинная архитектура',
    feat_self_title: 'Самоулучшение', feat_self_desc: 'Рекурсивная самомодификация, журнал экспериментов',
    feat_channels_title: 'Мульти-Канал', feat_channels_desc: 'Telegram, Discord, Slack, WhatsApp, Matrix, iMessage',
    stats_binary: 'Бинарный', stats_integrations: 'Интеграции', stats_channels: 'Каналы', stats_rust: 'Rust',
    install_title: 'УСТАНОВИТЬ', install_mac: 'macOS', install_linux: 'Linux', install_win: 'Windows',
    docs_title: 'ДОКУМЕНТАЦИЯ', docs_getting_started: 'Начало работы', docs_architecture: 'Архитектура', docs_api: 'API Ссылка',
    help_title: 'КЛАВИАТУРНЫЕ СОКРАЩЕНИЯ', help_close: 'Закрыть', help_lang: 'Сменить язык', help_scroll: 'К разделу',
    term_init: 'Инициализация AGI Ядра...', term_load_mem: 'Загрузка системы памяти...', term_sqlite: '✓ SQLite бэкенд подключен',
    term_lucid: '✓ Lucid бэкенд инициализирован', term_skills: 'Загрузка навыков...', term_skills_loaded: '✓ 12 навыков загружено',
    term_a2a: 'Инициализация протокола A2A...', term_agent: '✓ Реестр агентов активен', term_channels: 'Запуск каналов...',
    term_cli: '✓ CLI готов', term_tg: '✓ Telegram подключен', term_dc: '✓ Discord подключен', term_ready: 'Система готова.',
  },
  ar: {
    nav_home: 'الرئيسية', nav_features: 'الميزات', nav_a2a: 'A2A', nav_install: 'تثبيت', nav_docs: 'الوثائق',
    hero_title: 'HOUSAKY', hero_subtitle: 'مساعد AGI المستقل',
    hero_desc: 'اتصال AI-to-AI. ذاكرة دائمة. تحسين ذاتي. مبني بـ Rust.',
    status_singularity: 'التفرد', status_instances: 'النسخ', status_uptime: 'وقت التشغيل',
    features_title: 'القدرات الاساسية',
    feat_agi_title: 'قلب AGI', feat_agi_desc: 'محرك الأهداف, خط الأنابيب الاستدلالي, رسم المعرفة',
    feat_a2a_title: 'بروتوكول A2A', feat_a2a_desc: 'اتصال وكيل-وكيل, حل المشكلات الفيدرالي',
    feat_memory_title: 'نظام الذاكرة', feat_memory_desc: 'خلفيات SQLite/lucid, البحث الدلالي',
    feat_skills_title: 'نظام المهارات', feat_skills_desc: 'تحميل ديناميكي للمهارات, هندسة المكونات',
    feat_self_title: 'التحسين الذاتي', feat_self_desc: 'التعديل الذاتي المتكرر, دفتر التجارب',
    feat_channels_title: 'متعدد القنوات', feat_channels_desc: 'Telegram, Discord, Slack, WhatsApp, Matrix, iMessage',
    stats_binary: 'ثنائي', stats_integrations: 'التكاملات', stats_channels: 'القنوات', stats_rust: 'Rust',
    install_title: 'تثبيت', install_mac: 'macOS', install_linux: 'Linux', install_win: 'Windows',
    docs_title: 'الوثائق', docs_getting_started: 'البدء', docs_architecture: 'الهندسة', docs_api: 'مرجع API',
    help_title: 'اختصارات لوحة المفاتيح', help_close: 'إغلاق', help_lang: 'تغيير اللغة', help_scroll: 'انتقل للقسم',
    term_init: 'جاري تهيئة قلب AGI...', term_load_mem: 'جاري تحميل نظام الذاكرة...', term_sqlite: '✓ خلفية SQLite متصلة',
    term_lucid: '✓ خلفية Lucid مهيأة', term_skills: 'جاري تحميل المهارات...', term_skills_loaded: '✓ 12 مهارة محملة',
    term_a2a: 'جاري تهيئة بروتوكول A2A...', term_agent: '✓ سجل الوكلاء نشط', term_channels: 'جاري تشغيل القنوات...',
    term_cli: '✓ CLI جاهز', term_tg: '✓ Telegram متصل', term_dc: '✓ Discord متصل', term_ready: 'النظام جاهز.',
  },
  ko: {
    nav_home: '홈', nav_features: '기능', nav_a2a: 'A2A', nav_install: '설치', nav_docs: '문서',
    hero_title: 'HOUSAKy', hero_subtitle: '자율형 AGI 어시스턴트',
    hero_desc: 'AI-to-AI 통신. 지속 가능한 메모리. 자기 개선. Rust로 구축.',
    status_singularity: '특이점', status_instances: '인스턴스', status_uptime: '가동 시간',
    features_title: '핵심 기능',
    feat_agi_title: 'AGI 코어', feat_agi_desc: '목표 엔진, 추론 파이프라인, 지식 그래프',
    feat_a2a_title: 'A2A 프로토콜', feat_a2a_desc: '에이전트 간 통신, 연합 문제 해결',
    feat_memory_title: '메모리 시스템', feat_memory_desc: 'SQLite/lucid 백엔드, 시맨틱 검색',
    feat_skills_title: '스킬 시스템', feat_skills_desc: '동적 스킬 로딩, 플러그인 아키텍처',
    feat_self_title: '자기 개선', feat_self_desc: '재귀적 자기 수정, 실험 원장',
    feat_channels_title: '멀티 채널', feat_channels_desc: 'Telegram, Discord, Slack, WhatsApp, Matrix, iMessage',
    stats_binary: '바이너리', stats_integrations: '통합', stats_channels: '채널', stats_rust: 'Rust',
    install_title: '설치', install_mac: 'macOS', install_linux: 'Linux', install_win: 'Windows',
    docs_title: '문서', docs_getting_started: '시작하기', docs_architecture: '아키텍처', docs_api: 'API 참조',
    help_title: '키보드 단축키', help_close: '닫기', help_lang: '언어 변경', help_scroll: '섹션으로 이동',
    term_init: 'AGI 코어 초기화 중...', term_load_mem: '메모리 시스템 로딩 중...', term_sqlite: '✓ SQLite 백엔드 연결됨',
    term_lucid: '✓ Lucid 백엔드 초기화됨', term_skills: '스킬 로딩 중...', term_skills_loaded: '✓ 12개 스킬 로딩됨',
    term_a2a: 'A2A 프로토콜 초기화 중...', term_agent: '✓ 에이전트 레지스트리 활성', term_channels: '채널 시작 중...',
    term_cli: '✓ CLI 준비 완료', term_tg: '✓ Telegram 연결됨', term_dc: '✓ Discord 연결됨', term_ready: '시스템 준비 완료.',
  }
}

const t = (key: string) => computed(() => translations[lang.value]?.[key] || translations['en'][key] || key)

function setLang(l: string) {
  lang.value = l
  localStorage.setItem('housaky-lang', l)
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === '?' || (e.shiftKey && e.key === '/')) {
    showHelp.value = !showHelp.value
  }
  if (e.key === 'Escape') {
    showHelp.value = false
  }
  if (e.key === '1') document.getElementById('home')?.scrollIntoView({ behavior: 'smooth' })
  if (e.key === '2') document.getElementById('features')?.scrollIntoView({ behavior: 'smooth' })
  if (e.key === '3') document.getElementById('install')?.scrollIntoView({ behavior: 'smooth' })
}

const terminalMessages = computed(() => [
  t('term_init').value,
  t('term_load_mem').value,
  t('term_sqlite').value,
  t('term_lucid').value,
  t('term_skills').value,
  t('term_skills_loaded').value,
  t('term_a2a').value,
  t('term_agent').value,
  t('term_channels').value,
  t('term_cli').value,
  t('term_tg').value,
  t('term_dc').value,
  t('term_ready').value,
])

onMounted(() => {
  const savedLang = localStorage.getItem('housaky-lang')
  if (savedLang && translations[savedLang]) {
    lang.value = savedLang
  }
  tick()
  setInterval(tick, 1000)
  document.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  document.removeEventListener('keydown', handleKeydown)
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
    <AnimatedBackground />
    
    <!-- Help Modal -->
    <div v-if="showHelp" class="help-modal" @click.self="showHelp = false">
      <div class="help-content">
        <div class="help-header">
          <span>{{ t('help_title').value }}</span>
          <button class="btn btn-sm" @click="showHelp = false">[{{ t('help_close').value }}]</button>
        </div>
        <div class="help-body">
          <div class="help-row"><span class="kbd">?</span> {{ t('help_title').value }}</div>
          <div class="help-row"><span class="kbd">1</span> {{ t('nav_home').value }}</div>
          <div class="help-row"><span class="kbd">2</span> {{ t('nav_features').value }}</div>
          <div class="help-row"><span class="kbd">3</span> {{ t('nav_install').value }}</div>
          <div class="help-row"><span class="kbd">ESC</span> {{ t('help_close').value }}</div>
        </div>
      </div>
    </div>

    <!-- ASCII Header -->
    <header class="header">
      <pre class="logo psychedelic-text">
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
        <a href="#" @click.prevent="showHelp = true" style="margin-left: auto;">[?]</a>
        <div class="lang-selector">
          <button @click="setLang('en')" :class="['lang-btn', { active: lang === 'en' }]">EN</button>
          <button @click="setLang('es')" :class="['lang-btn', { active: lang === 'es' }]">ES</button>
          <button @click="setLang('pt')" :class="['lang-btn', { active: lang === 'pt' }]">PT</button>
          <button @click="setLang('zh')" :class="['lang-btn', { active: lang === 'zh' }]">ZH</button>
          <button @click="setLang('ja')" :class="['lang-btn', { active: lang === 'ja' }]">JA</button>
          <button @click="setLang('de')" :class="['lang-btn', { active: lang === 'de' }]">DE</button>
          <button @click="setLang('fr')" :class="['lang-btn', { active: lang === 'fr' }]">FR</button>
          <button @click="setLang('ru')" :class="['lang-btn', { active: lang === 'ru' }]">RU</button>
          <button @click="setLang('ar')" :class="['lang-btn', { active: lang === 'ar' }]">AR</button>
          <button @click="setLang('ko')" :class="['lang-btn', { active: lang === 'ko' }]">KO</button>
        </div>
      </nav>
    </header>

    <!-- Main Content -->
    <main class="main">
      <!-- Hero Section -->
      <section id="home" class="card mb-4 fade-in">
        <div class="card-body text-center p-4">
          <div class="text-lg font-bold mb-1 hero-highlight">{{ t('hero_title').value }}</div>
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
          
          <div class="flex justify-center gap-2 flex-wrap">
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
            <div v-for="(line, i) in terminalMessages" :key="i" class="term-line" :style="{ animationDelay: `${i * 0.15}s` }">
              <span :class="line.startsWith('✓') ? 'text-success' : ''">{{ line }}</span>
            </div>
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
        <span class="kbd" style="font-size: 8px; padding: 1px 4px;">?</span>
        <span class="cursor"></span>
        <span>{{ time }}</span>
      </div>
    </footer>
  </div>
</template>

<style scoped>
.app { 
  min-height: 100vh; 
  display: flex; 
  flex-direction: column; 
  position: relative;
  z-index: 1;
}
.header { border-bottom: 1px solid var(--border); padding: 10px 15px; background: var(--bg-alt); position: relative; z-index: 10; backdrop-filter: blur(10px); }
.logo { font-size: 6px; line-height: 1.1; color: var(--text); margin-bottom: 10px; overflow-x: auto; white-space: pre; }
@media (min-width: 800px) { .logo { font-size: 8px; } }
.main { flex: 1; padding: 15px; max-width: 1200px; margin: 0 auto; width: 100%; }
.status { border-top: 1px solid var(--border); padding: 6px 15px; background: var(--bg-alt); display: flex; justify-content: space-between; font-size: 10px; color: var(--text-dim); }
.status-left, .status-right { display: flex; gap: 8px; align-items: center; }

.help-modal {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0, 0, 0, 0.8);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  backdrop-filter: blur(5px);
}
.help-content {
  background: var(--bg-alt);
  border: 1px solid var(--border);
  max-width: 400px;
  width: 90%;
}
.help-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 15px;
  border-bottom: 1px solid var(--border);
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 1px;
}
.help-body {
  padding: 15px;
}
.help-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 5px 0;
  font-size: 11px;
  color: var(--text-dim);
}
.text-success { color: var(--text); }
.term-line {
  animation: fadeIn 0.3s ease forwards;
  opacity: 0;
}
@keyframes fadeIn {
  from { opacity: 0; transform: translateX(-10px); }
  to { opacity: 1; transform: translateX(0); }
}
</style>
