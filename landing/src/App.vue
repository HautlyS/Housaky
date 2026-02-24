<script setup lang="ts">
import { Zap, Terminal, Cpu, Shield, Cloud, Wrench, Github, Book, Download, ArrowRight } from 'lucide-vue-next'

const features = [
  {
    icon: Zap,
    title: 'Zero Overhead',
    description: 'Built in 100% Rust for maximum performance and minimal resource usage'
  },
  {
    icon: Terminal,
    title: 'CLI & TUI',
    description: 'Powerful command-line interface with interactive terminal UI'
  },
  {
    icon: Cpu,
    title: 'Hardware Control',
    description: 'USB, Serial, GPIO support Arduino, Raspberry for STM32, Pi'
  },
  {
    icon: Shield,
    title: 'Secure by Default',
    description: 'Encrypted secrets, webhook verification, sandboxed execution'
  },
  {
    icon: Cloud,
    title: 'Multi-Channel',
    description: 'Telegram, Discord, Email, WhatsApp integration ready'
  },
  {
    icon: Wrench,
    title: 'Extensible Skills',
    description: 'Plugin system for adding custom tools and capabilities'
  }
]

  const installCommands = [
  { os: 'macOS', command: 'brew install housaky' },
  { os: 'Linux', command: 'curl -fsSL https://get.housaky.dev | bash' },
  { os: 'Windows', command: 'winget install Housaky' },
]
</script>

<template>
  <div class="min-h-screen">
    <!-- Header -->
    <header class="border-b bg-white/80 backdrop-blur-sm sticky top-0 z-50">
      <div class="max-w-6xl mx-auto px-4 py-4 flex items-center justify-between">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-blue-700 flex items-center justify-center">
            <span class="text-white font-bold text-xl">H</span>
          </div>
          <span class="font-bold text-xl">Housaky</span>
        </div>
        <nav class="hidden md:flex items-center gap-6">
          <a href="#features" class="text-sm font-medium hover:text-blue-600 transition-colors">Features</a>
          <a href="#install" class="text-sm font-medium hover:text-blue-600 transition-colors">Install</a>
          <a href="#docs" class="text-sm font-medium hover:text-blue-600 transition-colors">Docs</a>
          <a href="https://github.com/HautlyS/Housaky" target="_blank" class="text-sm font-medium hover:text-blue-600 transition-colors flex items-center gap-1">
            <Github class="w-4 h-4" />
            GitHub
          </a>
        </nav>
      </div>
    </header>

    <!-- Hero -->
    <section class="py-20 px-4">
      <div class="max-w-4xl mx-auto text-center">
        <div class="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-blue-100 text-blue-700 text-sm font-medium mb-6">
          <Zap class="w-4 h-4" />
          100% Rust
        </div>
        <h1 class="text-5xl md:text-6xl font-bold mb-6 tracking-tight">
          Zero overhead.<br/>
          <span class="text-blue-600">Zero compromise.</span>
        </h1>
        <p class="text-xl text-gray-600 mb-8 max-w-2xl mx-auto">
          The fastest, smallest AI assistant. Built for speed, security, and extensibility.
          Hardware-ready, multi-channel, and enterprise-grade.
        </p>
        <div class="flex flex-col sm:flex-row items-center justify-center gap-4">
          <a href="#install" class="inline-flex items-center gap-2 px-6 py-3 bg-blue-700 text-white rounded-lg font-semibold hover:bg-blue-800 transition-colors">
            Get Started
            <ArrowRight class="w-4 h-4" />
          </a>
          <a href="https://github.com/HautlyS/Housaky" target="_blank" class="inline-flex items-center gap-2 px-6 py-3 border border-gray-300 rounded-lg font-semibold hover:bg-gray-50 transition-colors">
            <Github class="w-4 h-4" />
            View on GitHub
          </a>
        </div>
      </div>
    </section>

    <!-- Features -->
    <section id="features" class="py-20 px-4 bg-gray-50">
      <div class="max-w-6xl mx-auto">
        <h2 class="text-3xl font-bold text-center mb-4">Everything you need</h2>
        <p class="text-gray-600 text-center mb-12 max-w-2xl mx-auto">
          A complete AI assistant platform with hardware control, security, and extensibility built-in.
        </p>
        <div class="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
          <div v-for="feature in features" :key="feature.title" class="bg-white p-6 rounded-xl border shadow-sm hover:shadow-md transition-shadow">
            <div class="w-12 h-12 rounded-lg bg-blue-100 flex items-center justify-center mb-4">
              <component :is="feature.icon" class="w-6 h-6 text-blue-700" />
            </div>
            <h3 class="text-lg font-semibold mb-2">{{ feature.title }}</h3>
            <p class="text-gray-600 text-sm">{{ feature.description }}</p>
          </div>
        </div>
      </div>
    </section>

    <!-- Install -->
    <section id="install" class="py-20 px-4">
      <div class="max-w-4xl mx-auto">
        <h2 class="text-3xl font-bold text-center mb-4">Install Housaky</h2>
        <p class="text-gray-600 text-center mb-12">Choose your platform and get started in seconds.</p>
        <div class="grid md:grid-cols-3 gap-6">
          <div v-for="item in installCommands" :key="item.os" class="bg-gray-900 rounded-xl p-6">
            <div class="text-sm font-medium text-gray-400 mb-2">{{ item.os }}</div>
            <code class="text-green-400 font-mono text-sm break-all">{{ item.command }}</code>
          </div>
        </div>
        <p class="text-center mt-8 text-gray-600">
          Or <a href="https://github.com/HautlyS/Housaky/releases" target="_blank" class="text-blue-600 hover:underline">download binaries</a> directly from releases.
        </p>
      </div>
    </section>

    <!-- Dashboard -->
    <section class="py-20 px-4 bg-gray-900 text-white">
      <div class="max-w-6xl mx-auto">
        <div class="grid lg:grid-cols-2 gap-12 items-center">
          <div>
            <h2 class="text-3xl font-bold mb-4">Beautiful Dashboard</h2>
            <p class="text-gray-400 mb-6">
              Configure, monitor, and control Housaky from a beautiful cross-platform desktop application.
              Real-time metrics, skill management, and hardware monitoring.
            </p>
            <ul class="space-y-3 mb-8">
              <li class="flex items-center gap-2">
                <div class="w-2 h-2 rounded-full bg-green-500"></div>
                <span class="text-gray-300">Cross-platform (Windows, macOS, Linux)</span>
              </li>
              <li class="flex items-center gap-2">
                <div class="w-2 h-2 rounded-full bg-green-500"></div>
                <span class="text-gray-300">Hardware monitoring</span>
              </li>
              <li class="flex items-center gap-2">
                <div class="w-2 h-2 rounded-full bg-green-500"></div>
                <span class="text-gray-300">Channel management</span>
              </li>
              <li class="flex items-center gap-2">
                <div class="w-2 h-2 rounded-full bg-green-500"></div>
                <span class="text-gray-300">Skills & integrations</span>
              </li>
            </ul>
          </div>
          <div class="bg-gray-800 rounded-xl p-4 border border-gray-700">
            <div class="bg-gray-900 rounded-lg p-6">
              <div class="flex items-center gap-2 mb-4">
                <div class="w-3 h-3 rounded-full bg-red-500"></div>
                <div class="w-3 h-3 rounded-full bg-yellow-500"></div>
                <div class="w-3 h-3 rounded-full bg-green-500"></div>
              </div>
              <div class="space-y-3">
                <div class="h-4 bg-gray-700 rounded w-3/4"></div>
                <div class="h-4 bg-gray-700 rounded w-1/2"></div>
                <div class="h-4 bg-gray-700 rounded w-5/6"></div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- Docs -->
    <section id="docs" class="py-20 px-4">
      <div class="max-w-4xl mx-auto text-center">
        <h2 class="text-3xl font-bold mb-4">Documentation</h2>
        <p class="text-gray-600 mb-8">Everything you need to know about Housaky.</p>
        <div class="grid md:grid-cols-2 gap-4">
          <a href="https://github.com/HautlyS/Housaky#readme" target="_blank" class="flex items-center gap-3 p-4 border rounded-lg hover:border-blue-500 hover:bg-blue-50 transition-colors">
            <Book class="w-6 h-6 text-blue-600" />
            <div class="text-left">
              <div class="font-semibold">Getting Started</div>
              <div class="text-sm text-gray-600">Quick start guide</div>
            </div>
          </a>
          <a href="https://github.com/HautlyS/Housaky/tree/main/docs" target="_blank" class="flex items-center gap-3 p-4 border rounded-lg hover:border-blue-500 hover:bg-blue-50 transition-colors">
            <Book class="w-6 h-6 text-blue-600" />
            <div class="text-left">
              <div class="font-semibold">Architecture</div>
              <div class="text-sm text-gray-600">Deep dive docs</div>
            </div>
          </a>
        </div>
      </div>
    </section>

    <!-- Footer -->
    <footer class="border-t py-8 px-4">
      <div class="max-w-6xl mx-auto flex flex-col md:flex-row items-center justify-between gap-4">
        <div class="flex items-center gap-2">
          <div class="w-8 h-8 rounded-lg bg-blue-700 flex items-center justify-center">
            <span class="text-white font-bold">H</span>
          </div>
          <span class="font-semibold">Housaky</span>
        </div>
        <p class="text-sm text-gray-600">100% Rust. Open Source. MIT License.</p>
        <div class="flex items-center gap-4">
          <a href="https://github.com/HautlyS/Housaky" target="_blank" class="text-gray-600 hover:text-gray-900">
            <Github class="w-5 h-5" />
          </a>
        </div>
      </div>
    </footer>
  </div>
</template>
