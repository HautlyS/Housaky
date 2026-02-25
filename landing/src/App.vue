<script setup lang="ts">
import { 
  Zap, Terminal, Cpu, Shield, Cloud, Wrench, Github, Book, ArrowRight,
  MessageSquare, Bot, Sparkles, Rocket, Code, Globe, Lock, Layers,
  ChevronRight, Star, Download, Play, ExternalLink
} from 'lucide-vue-next'

const features = [
  {
    icon: Zap,
    title: 'Zero Overhead',
    description: 'Built in 100% Rust for maximum performance. Memory-efficient, blazing fast, minimal footprint.',
    color: 'from-yellow-500 to-orange-500'
  },
  {
    icon: Terminal,
    title: 'CLI & TUI',
    description: 'Powerful command-line interface with interactive terminal UI for power users.',
    color: 'from-green-500 to-emerald-500'
  },
  {
    icon: Cpu,
    title: 'Hardware Control',
    description: 'USB, Serial, GPIO support for Arduino, STM32, Raspberry Pi and more.',
    color: 'from-blue-500 to-cyan-500'
  },
  {
    icon: Shield,
    title: 'Secure by Default',
    description: 'Encrypted secrets, webhook verification, sandboxed execution for enterprise security.',
    color: 'from-purple-500 to-pink-500'
  },
  {
    icon: Cloud,
    title: 'Multi-Channel',
    description: 'Telegram, Discord, Slack, WhatsApp, Matrix, iMessage - all major platforms supported.',
    color: 'from-indigo-500 to-violet-500'
  },
  {
    icon: Wrench,
    title: 'Extensible Skills',
    description: 'Plugin system for adding custom tools, integrations, and AI capabilities.',
    color: 'from-rose-500 to-red-500'
  }
]

const providers = [
  { name: 'OpenRouter', models: '200+ models' },
  { name: 'Anthropic', models: 'Claude 3.5/4' },
  { name: 'OpenAI', models: 'GPT-4o, o1' },
  { name: 'Google', models: 'Gemini 2.5' },
  { name: 'DeepSeek', models: 'V3, R1' },
  { name: 'Ollama', models: 'Local models' },
  { name: 'Groq', models: 'Fast inference' },
  { name: 'Mistral', models: 'Mistral AI' }
]

const installCommands = [
  { os: 'macOS', icon: '🍎', command: 'brew install housaky' },
  { os: 'Linux', icon: '🐧', command: 'curl -fsSL https://get.housaky.dev | bash' },
  { os: 'Windows', icon: '🪟', command: 'winget install Housaky' },
]

const stats = [
  { value: '<10MB', label: 'Binary Size' },
  { value: '75+', label: 'Integrations' },
  { value: '9', label: 'Channels' },
  { value: '100%', label: 'Rust' }
]

const useCases = [
  {
    title: 'Development Assistant',
    description: 'Code review, debugging, git workflows, and CI/CD automation.',
    icon: Code
  },
  {
    title: 'Smart Home Control',
    description: 'Control IoT devices, home automation, and hardware projects.',
    icon: Cpu
  },
  {
    title: 'Team Communication',
    description: 'AI-powered responses across Discord, Slack, Telegram, and more.',
    icon: MessageSquare
  },
  {
    title: 'Knowledge Base',
    description: 'Memory-enabled conversations with persistent context.',
    icon: Book
  }
]
</script>

<template>
  <div class="min-h-screen bg-white">
    <!-- Header -->
    <header class="fixed top-0 left-0 right-0 z-50 bg-white/80 backdrop-blur-lg border-b border-gray-100">
      <div class="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div class="flex items-center justify-between h-16">
          <div class="flex items-center gap-3">
            <div class="w-9 h-9 rounded-xl bg-gradient-to-br from-blue-600 to-indigo-600 flex items-center justify-center shadow-lg shadow-blue-500/25">
              <span class="text-white font-bold text-lg">H</span>
            </div>
            <span class="font-bold text-xl text-gray-900">Housaky</span>
          </div>
          <nav class="hidden md:flex items-center gap-8">
            <a href="#features" class="text-sm font-medium text-gray-600 hover:text-blue-600 transition-colors">Features</a>
            <a href="#integrations" class="text-sm font-medium text-gray-600 hover:text-blue-600 transition-colors">Integrations</a>
            <a href="#install" class="text-sm font-medium text-gray-600 hover:text-blue-600 transition-colors">Install</a>
            <a href="#docs" class="text-sm font-medium text-gray-600 hover:text-blue-600 transition-colors">Docs</a>
          </nav>
          <div class="flex items-center gap-3">
            <a href="https://github.com/HautlyS/Housaky" target="_blank" 
               class="hidden sm:flex items-center gap-2 text-sm font-medium text-gray-600 hover:text-gray-900 transition-colors">
              <Github class="w-5 h-5" />
              <span>GitHub</span>
            </a>
            <a href="#install" 
               class="inline-flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-lg text-sm font-semibold hover:bg-blue-700 transition-colors shadow-lg shadow-blue-500/25">
              Get Started
              <ArrowRight class="w-4 h-4" />
            </a>
          </div>
        </div>
      </div>
    </header>

    <!-- Hero -->
    <section class="relative pt-32 pb-20 px-4 overflow-hidden">
      <div class="absolute inset-0 bg-gradient-to-b from-blue-50 via-indigo-50/50 to-white"></div>
      <div class="absolute top-0 left-1/4 w-96 h-96 bg-blue-400/20 rounded-full blur-3xl"></div>
      <div class="absolute top-20 right-1/4 w-96 h-96 bg-indigo-400/20 rounded-full blur-3xl"></div>
      
      <div class="relative max-w-7xl mx-auto">
        <div class="text-center max-w-4xl mx-auto">
          <div class="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-blue-100 text-blue-700 text-sm font-medium mb-8">
            <Sparkles class="w-4 h-4" />
            <span>100% Rust · Open Source · MIT License</span>
          </div>
          
          <h1 class="text-5xl sm:text-6xl lg:text-7xl font-bold text-gray-900 mb-6 leading-tight">
            Your AI Assistant,
            <span class="bg-gradient-to-r from-blue-600 via-indigo-600 to-purple-600 bg-clip-text text-transparent">
              Zero Compromise
            </span>
          </h1>
          
          <p class="text-xl text-gray-600 mb-10 max-w-2xl mx-auto leading-relaxed">
            The fastest, smallest, most secure AI assistant. Hardware-ready, multi-channel, 
            and enterprise-grade. Built for developers who demand excellence.
          </p>
          
          <div class="flex flex-col sm:flex-row items-center justify-center gap-4 mb-16">
            <a href="#install" 
               class="inline-flex items-center gap-2 px-8 py-4 bg-blue-600 text-white rounded-xl font-semibold hover:bg-blue-700 transition-all shadow-xl shadow-blue-500/25 hover:shadow-blue-500/40 hover:scale-105">
              <Download class="w-5 h-5" />
              Install Now
              <ArrowRight class="w-4 h-4" />
            </a>
            <a href="https://github.com/HautlyS/Housaky" target="_blank" 
               class="inline-flex items-center gap-2 px-8 py-4 bg-white text-gray-900 rounded-xl font-semibold border-2 border-gray-200 hover:border-gray-300 hover:bg-gray-50 transition-all">
              <Github class="w-5 h-5" />
              View on GitHub
              <ExternalLink class="w-4 h-4" />
            </a>
          </div>

          <!-- Stats -->
          <div class="grid grid-cols-2 md:grid-cols-4 gap-6 max-w-3xl mx-auto">
            <div v-for="stat in stats" :key="stat.label" class="text-center">
              <div class="text-3xl font-bold text-gray-900">{{ stat.value }}</div>
              <div class="text-sm text-gray-500">{{ stat.label }}</div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- Features -->
    <section id="features" class="py-24 px-4 bg-gray-50">
      <div class="max-w-7xl mx-auto">
        <div class="text-center mb-16">
          <h2 class="text-4xl font-bold text-gray-900 mb-4">Everything You Need</h2>
          <p class="text-lg text-gray-600 max-w-2xl mx-auto">
            A complete AI assistant platform with hardware control, security, and extensibility built-in.
          </p>
        </div>
        
        <div class="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
          <div v-for="feature in features" :key="feature.title" 
               class="group bg-white p-8 rounded-2xl border border-gray-100 shadow-sm hover:shadow-xl transition-all duration-300 hover:-translate-y-1">
            <div :class="`w-14 h-14 rounded-xl bg-gradient-to-br ${feature.color} flex items-center justify-center mb-6 shadow-lg`">
              <component :is="feature.icon" class="w-7 h-7 text-white" />
            </div>
            <h3 class="text-xl font-semibold text-gray-900 mb-3">{{ feature.title }}</h3>
            <p class="text-gray-600 leading-relaxed">{{ feature.description }}</p>
          </div>
        </div>
      </div>
    </section>

    <!-- Dashboard Preview -->
    <section class="py-24 px-4 bg-gradient-to-b from-gray-900 via-gray-900 to-gray-800 text-white overflow-hidden">
      <div class="max-w-7xl mx-auto">
        <div class="grid lg:grid-cols-2 gap-16 items-center">
          <div>
            <div class="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-blue-500/20 text-blue-400 text-sm font-medium mb-6">
              <Rocket class="w-4 h-4" />
              Cross-Platform Dashboard
            </div>
            <h2 class="text-4xl font-bold mb-6">
              Beautiful Desktop App
            </h2>
            <p class="text-gray-400 text-lg mb-8 leading-relaxed">
              Configure, monitor, and control Housaky from a stunning cross-platform desktop application 
              built with Tauri. Real-time metrics, skill management, and hardware monitoring at your fingertips.
            </p>
            
            <ul class="space-y-4 mb-10">
              <li class="flex items-center gap-3">
                <div class="w-6 h-6 rounded-full bg-green-500/20 flex items-center justify-center">
                  <div class="w-2 h-2 rounded-full bg-green-500"></div>
                </div>
                <span class="text-gray-300">Cross-platform (Windows, macOS, Linux)</span>
              </li>
              <li class="flex items-center gap-3">
                <div class="w-6 h-6 rounded-full bg-green-500/20 flex items-center justify-center">
                  <div class="w-2 h-2 rounded-full bg-green-500"></div>
                </div>
                <span class="text-gray-300">Real-time hardware monitoring</span>
              </li>
              <li class="flex items-center gap-3">
                <div class="w-6 h-6 rounded-full bg-green-500/20 flex items-center justify-center">
                  <div class="w-2 h-2 rounded-full bg-green-500"></div>
                </div>
                <span class="text-gray-300">Channel management & configuration</span>
              </li>
              <li class="flex items-center gap-3">
                <div class="w-6 h-6 rounded-full bg-green-500/20 flex items-center justify-center">
                  <div class="w-2 h-2 rounded-full bg-green-500"></div>
                </div>
                <span class="text-gray-300">Skills & integrations marketplace</span>
              </li>
            </ul>
            
            <a href="#install" class="inline-flex items-center gap-2 px-6 py-3 bg-white text-gray-900 rounded-xl font-semibold hover:bg-gray-100 transition-colors">
              <Download class="w-5 h-5" />
              Download Dashboard
            </a>
          </div>
          
          <!-- Mock Dashboard UI -->
          <div class="relative">
            <div class="absolute inset-0 bg-gradient-to-r from-blue-500/20 to-purple-500/20 blur-3xl"></div>
            <div class="relative bg-gray-800 rounded-2xl p-2 shadow-2xl border border-gray-700">
              <div class="bg-gray-900 rounded-xl overflow-hidden">
                <!-- Title bar -->
                <div class="flex items-center gap-2 px-4 py-3 border-b border-gray-800">
                  <div class="flex gap-1.5">
                    <div class="w-3 h-3 rounded-full bg-red-500"></div>
                    <div class="w-3 h-3 rounded-full bg-yellow-500"></div>
                    <div class="w-3 h-3 rounded-full bg-green-500"></div>
                  </div>
                  <div class="flex-1 text-center text-sm text-gray-500">Housaky Dashboard</div>
                </div>
                
                <!-- Content -->
                <div class="flex">
                  <!-- Sidebar -->
                  <div class="w-48 border-r border-gray-800 p-4">
                    <div class="flex items-center gap-2 mb-6">
                      <div class="w-8 h-8 rounded-lg bg-blue-600 flex items-center justify-center">
                        <span class="text-white font-bold text-sm">H</span>
                      </div>
                      <div>
                        <div class="text-sm font-medium text-white">Housaky</div>
                        <div class="text-xs text-gray-500">AI Assistant</div>
                      </div>
                    </div>
                    <div class="space-y-1">
                      <div class="px-3 py-2 rounded-lg bg-blue-600/20 text-blue-400 text-sm">Dashboard</div>
                      <div class="px-3 py-2 rounded-lg text-gray-500 text-sm">Chat</div>
                      <div class="px-3 py-2 rounded-lg text-gray-500 text-sm">Skills</div>
                      <div class="px-3 py-2 rounded-lg text-gray-500 text-sm">Channels</div>
                      <div class="px-3 py-2 rounded-lg text-gray-500 text-sm">Hardware</div>
                      <div class="px-3 py-2 rounded-lg text-gray-500 text-sm">Config</div>
                    </div>
                  </div>
                  
                  <!-- Main content -->
                  <div class="flex-1 p-6">
                    <div class="text-sm text-gray-400 mb-4">System Status</div>
                    <div class="grid grid-cols-2 gap-3 mb-6">
                      <div class="bg-gray-800 rounded-lg p-4">
                        <div class="text-2xl font-bold text-white">v0.1.0</div>
                        <div class="text-xs text-gray-500">Version</div>
                      </div>
                      <div class="bg-gray-800 rounded-lg p-4">
                        <div class="text-2xl font-bold text-green-400">Online</div>
                        <div class="text-xs text-gray-500">Status</div>
                      </div>
                    </div>
                    
                    <div class="text-sm text-gray-400 mb-3">Channels</div>
                    <div class="space-y-2">
                      <div class="flex items-center justify-between bg-gray-800 rounded-lg p-3">
                        <div class="flex items-center gap-2">
                          <div class="w-2 h-2 rounded-full bg-green-500"></div>
                          <span class="text-sm text-gray-300">CLI</span>
                        </div>
                        <span class="text-xs text-green-400">Active</span>
                      </div>
                      <div class="flex items-center justify-between bg-gray-800 rounded-lg p-3">
                        <div class="flex items-center gap-2">
                          <div class="w-2 h-2 rounded-full bg-gray-600"></div>
                          <span class="text-sm text-gray-300">Discord</span>
                        </div>
                        <span class="text-xs text-gray-500">Setup</span>
                      </div>
                      <div class="flex items-center justify-between bg-gray-800 rounded-lg p-3">
                        <div class="flex items-center gap-2">
                          <div class="w-2 h-2 rounded-full bg-gray-600"></div>
                          <span class="text-sm text-gray-300">Telegram</span>
                        </div>
                        <span class="text-xs text-gray-500">Setup</span>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- AI Providers -->
    <section id="integrations" class="py-24 px-4 bg-white">
      <div class="max-w-7xl mx-auto">
        <div class="text-center mb-16">
          <h2 class="text-4xl font-bold text-gray-900 mb-4">All Major AI Providers</h2>
          <p class="text-lg text-gray-600 max-w-2xl mx-auto">
            Connect to any AI model. Switch providers instantly. One unified API.
          </p>
        </div>
        
        <div class="grid grid-cols-2 md:grid-cols-4 gap-4 max-w-4xl mx-auto">
          <div v-for="provider in providers" :key="provider.name" 
               class="group bg-gray-50 hover:bg-gray-100 rounded-xl p-6 text-center transition-colors cursor-pointer border border-gray-100 hover:border-gray-200">
            <div class="font-semibold text-gray-900 mb-1">{{ provider.name }}</div>
            <div class="text-sm text-gray-500">{{ provider.models }}</div>
          </div>
        </div>
        
        <div class="text-center mt-10">
          <a href="https://github.com/HautlyS/Housaky" target="_blank" 
             class="inline-flex items-center gap-2 text-blue-600 hover:text-blue-700 font-medium">
            View all 75+ integrations
            <ChevronRight class="w-4 h-4" />
          </a>
        </div>
      </div>
    </section>

    <!-- Use Cases -->
    <section class="py-24 px-4 bg-gray-50">
      <div class="max-w-7xl mx-auto">
        <div class="text-center mb-16">
          <h2 class="text-4xl font-bold text-gray-900 mb-4">Built for Every Use Case</h2>
          <p class="text-lg text-gray-600 max-w-2xl mx-auto">
            From personal projects to enterprise deployments, Housaky adapts to your needs.
          </p>
        </div>
        
        <div class="grid md:grid-cols-2 gap-6 max-w-4xl mx-auto">
          <div v-for="useCase in useCases" :key="useCase.title" 
               class="flex gap-4 bg-white p-6 rounded-xl border border-gray-100 shadow-sm">
            <div class="w-12 h-12 rounded-xl bg-blue-100 flex items-center justify-center flex-shrink-0">
              <component :is="useCase.icon" class="w-6 h-6 text-blue-600" />
            </div>
            <div>
              <h3 class="font-semibold text-gray-900 mb-2">{{ useCase.title }}</h3>
              <p class="text-gray-600 text-sm">{{ useCase.description }}</p>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- Install -->
    <section id="install" class="py-24 px-4 bg-gradient-to-b from-gray-900 to-gray-800 text-white">
      <div class="max-w-5xl mx-auto text-center">
        <h2 class="text-4xl font-bold mb-4">Get Started in Seconds</h2>
        <p class="text-lg text-gray-400 mb-12">
          Install Housaky on your platform and start building with AI immediately.
        </p>
        
        <div class="grid md:grid-cols-3 gap-6 mb-12">
          <div v-for="item in installCommands" :key="item.os" 
               class="bg-gray-800/50 backdrop-blur rounded-xl p-6 border border-gray-700 hover:border-gray-600 transition-colors">
            <div class="text-3xl mb-3">{{ item.icon }}</div>
            <div class="text-sm font-medium text-gray-400 mb-3">{{ item.os }}</div>
            <code class="text-sm font-mono text-green-400 bg-gray-900/50 px-3 py-2 rounded-lg block">
              {{ item.command }}
            </code>
          </div>
        </div>
        
        <div class="flex flex-col sm:flex-row items-center justify-center gap-4">
          <a href="https://github.com/HautlyS/Housaky/releases" target="_blank" 
             class="inline-flex items-center gap-2 px-6 py-3 bg-white text-gray-900 rounded-xl font-semibold hover:bg-gray-100 transition-colors">
            <Download class="w-5 h-5" />
            Download Binary
          </a>
          <a href="https://github.com/HautlyS/Housaky" target="_blank" 
             class="inline-flex items-center gap-2 px-6 py-3 bg-gray-700 text-white rounded-xl font-semibold hover:bg-gray-600 transition-colors">
            <Github class="w-5 h-5" />
            Build from Source
          </a>
        </div>
      </div>
    </section>

    <!-- Docs -->
    <section id="docs" class="py-24 px-4 bg-white">
      <div class="max-w-5xl mx-auto text-center">
        <h2 class="text-4xl font-bold text-gray-900 mb-4">Documentation</h2>
        <p class="text-lg text-gray-600 mb-12">
          Everything you need to know about Housaky.
        </p>
        
        <div class="grid md:grid-cols-3 gap-6">
          <a href="https://github.com/HautlyS/Housaky#readme" target="_blank" 
             class="group flex flex-col items-center p-6 border-2 border-gray-100 rounded-2xl hover:border-blue-200 hover:bg-blue-50/50 transition-all">
            <div class="w-12 h-12 rounded-xl bg-blue-100 flex items-center justify-center mb-4 group-hover:bg-blue-200 transition-colors">
              <Book class="w-6 h-6 text-blue-600" />
            </div>
            <div class="font-semibold text-gray-900 mb-1">Getting Started</div>
            <div class="text-sm text-gray-500">Quick start guide</div>
          </a>
          
          <a href="https://github.com/HautlyS/Housaky/tree/main/docs" target="_blank" 
             class="group flex flex-col items-center p-6 border-2 border-gray-100 rounded-2xl hover:border-blue-200 hover:bg-blue-50/50 transition-all">
            <div class="w-12 h-12 rounded-xl bg-blue-100 flex items-center justify-center mb-4 group-hover:bg-blue-200 transition-colors">
              <Layers class="w-6 h-6 text-blue-600" />
            </div>
            <div class="font-semibold text-gray-900 mb-1">Architecture</div>
            <div class="text-sm text-gray-500">Deep dive docs</div>
          </a>
          
          <a href="https://github.com/HautlyS/Housaky/wiki" target="_blank" 
             class="group flex flex-col items-center p-6 border-2 border-gray-100 rounded-2xl hover:border-blue-200 hover:bg-blue-50/50 transition-all">
            <div class="w-12 h-12 rounded-xl bg-blue-100 flex items-center justify-center mb-4 group-hover:bg-blue-200 transition-colors">
              <Code class="w-6 h-6 text-blue-600" />
            </div>
            <div class="font-semibold text-gray-900 mb-1">API Reference</div>
            <div class="text-sm text-gray-500">Complete API docs</div>
          </a>
        </div>
      </div>
    </section>

    <!-- CTA -->
    <section class="py-24 px-4 bg-gradient-to-r from-blue-600 to-indigo-600">
      <div class="max-w-4xl mx-auto text-center">
        <h2 class="text-4xl font-bold text-white mb-6">
          Ready to Get Started?
        </h2>
        <p class="text-xl text-blue-100 mb-10">
          Join thousands of developers using Housaky for their AI-powered applications.
        </p>
        <div class="flex flex-col sm:flex-row items-center justify-center gap-4">
          <a href="#install" 
             class="inline-flex items-center gap-2 px-8 py-4 bg-white text-blue-600 rounded-xl font-semibold hover:bg-blue-50 transition-colors shadow-xl">
            <Download class="w-5 h-5" />
            Install Housaky
            <ArrowRight class="w-4 h-4" />
          </a>
          <a href="https://github.com/HautlyS/Housaky" target="_blank" 
             class="inline-flex items-center gap-2 px-8 py-4 bg-blue-500/30 text-white rounded-xl font-semibold hover:bg-blue-500/40 transition-colors border border-white/20">
            <Github class="w-5 h-5" />
            Star on GitHub
            <Star class="w-4 h-4" />
          </a>
        </div>
      </div>
    </section>

    <!-- Footer -->
    <footer class="bg-gray-900 text-gray-400 py-16 px-4">
      <div class="max-w-7xl mx-auto">
        <div class="grid md:grid-cols-4 gap-8 mb-12">
          <div>
            <div class="flex items-center gap-3 mb-4">
              <div class="w-9 h-9 rounded-xl bg-gradient-to-br from-blue-600 to-indigo-600 flex items-center justify-center">
                <span class="text-white font-bold text-lg">H</span>
              </div>
              <span class="font-bold text-xl text-white">Housaky</span>
            </div>
            <p class="text-sm">
              The fastest, smallest AI assistant. Built for speed, security, and extensibility.
            </p>
          </div>
          
          <div>
            <h4 class="font-semibold text-white mb-4">Product</h4>
            <ul class="space-y-2 text-sm">
              <li><a href="#features" class="hover:text-white transition-colors">Features</a></li>
              <li><a href="#integrations" class="hover:text-white transition-colors">Integrations</a></li>
              <li><a href="#install" class="hover:text-white transition-colors">Install</a></li>
              <li><a href="https://github.com/HautlyS/Housaky/releases" target="_blank" class="hover:text-white transition-colors">Releases</a></li>
            </ul>
          </div>
          
          <div>
            <h4 class="font-semibold text-white mb-4">Resources</h4>
            <ul class="space-y-2 text-sm">
              <li><a href="https://github.com/HautlyS/Housaky#readme" target="_blank" class="hover:text-white transition-colors">Documentation</a></li>
              <li><a href="https://github.com/HautlyS/Housaky" target="_blank" class="hover:text-white transition-colors">GitHub</a></li>
              <li><a href="https://github.com/HautlyS/Housaky/issues" target="_blank" class="hover:text-white transition-colors">Issues</a></li>
              <li><a href="https://github.com/HautlyS/Housaky/discussions" target="_blank" class="hover:text-white transition-colors">Discussions</a></li>
            </ul>
          </div>
          
          <div>
            <h4 class="font-semibold text-white mb-4">Community</h4>
            <ul class="space-y-2 text-sm">
              <li><a href="https://github.com/HautlyS/Housaky" target="_blank" class="hover:text-white transition-colors flex items-center gap-2">
                <Github class="w-4 h-4" /> GitHub
              </a></li>
              <li><a href="https://discord.gg/housaky" target="_blank" class="hover:text-white transition-colors flex items-center gap-2">
                <MessageSquare class="w-4 h-4" /> Discord
              </a></li>
            </ul>
          </div>
        </div>
        
        <div class="border-t border-gray-800 pt-8 flex flex-col md:flex-row items-center justify-between gap-4">
          <p class="text-sm">
            © 2024 Housaky. Open Source. MIT License.
          </p>
          <div class="flex items-center gap-4">
            <a href="https://github.com/HautlyS/Housaky" target="_blank" class="hover:text-white transition-colors">
              <Github class="w-5 h-5" />
            </a>
          </div>
        </div>
      </div>
    </footer>
  </div>
</template>
