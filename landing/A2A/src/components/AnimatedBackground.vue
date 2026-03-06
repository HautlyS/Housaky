<template>
  <div class="animated-bg">
    <div class="matrix-rain">
      <div
        v-for="(col, i) in columns"
        :key="i"
        class="matrix-column"
        :style="{ left: `${(i / columns.length) * 100}%`, animationDelay: `${Math.random() * 5}s` }"
      >
        <span
          v-for="(char, j) in col.chars"
          :key="j"
          :class="['matrix-char', { bright: j === col.chars.length - 1 }]"
          :style="{ opacity: 0.1 + (j / col.chars.length) * 0.3 }"
        >
          {{ char }}
        </span>
      </div>
    </div>

    <div class="floating-words">
      <div
        v-for="(word, i) in floatingWords"
        :key="i"
        :class="['floating-word', word.class]"
        :style="{
          left: `${word.x}%`,
          top: `${word.y}%`,
          animationDelay: `${word.delay}s`,
          animationDuration: `${word.duration}s`
        }"
      >
        {{ word.text }}
      </div>
    </div>

    <div class="ascii-particles">
      <div
        v-for="(particle, i) in particles"
        :key="i"
        class="particle"
        :style="{
          left: `${particle.x}%`,
          animationDelay: `${particle.delay}s`,
          animationDuration: `${particle.duration}s`
        }"
      >
        {{ particle.char }}
      </div>
    </div>

    <div class="grid-overlay"></div>

    <div class="psychedelic-wave"></div>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'

const agiWords = [
  { text: '人工知能', lang: 'ja' },
  { text: '奇点', lang: 'zh' },
  { text: 'Singularity', lang: 'en' },
  { text: 'Singularidad', lang: 'es' },
  { text: 'Singularidade', lang: 'pt' },
  { text: '認識論', lang: 'ja' },
  { text: '认知科学', lang: 'zh' },
  { text: 'Consciousness', lang: 'en' },
  { text: 'Conciencia', lang: 'es' },
  { text: 'Consciência', lang: 'pt' },
  { text: '意識', lang: 'ja' },
  { text: '意识', lang: 'zh' },
  { text: 'Neural Network', lang: 'en' },
  { text: 'Red Neural', lang: 'es' },
  { text: 'Rede Neural', lang: 'pt' },
  { text: 'ニューラルネット', lang: 'ja' },
  { text: '神经网络', lang: 'zh' },
  { text: 'AGI', lang: 'universal' },
  { text: 'Recursive', lang: 'en' },
  { text: 'Recursivo', lang: 'es' },
  { text: 'Recursivo', lang: 'pt' },
  { text: '再帰的', lang: 'ja' },
  { text: '递归', lang: 'zh' },
  { text: 'Meta-Cognition', lang: 'en' },
  { text: 'Metacognición', lang: 'es' },
  { text: 'Metacognição', lang: 'pt' },
  { text: 'メタ認知', lang: 'ja' },
  { text: '元认知', lang: 'zh' },
  { text: 'Emergence', lang: 'en' },
  { text: 'Emergencia', lang: 'es' },
  { text: 'Emergência', lang: 'pt' },
  { text: '創発', lang: 'ja' },
  { text: '涌现', lang: 'zh' },
  { text: 'Self-Improve', lang: 'en' },
  { text: 'Auto-Mejora', lang: 'es' },
  { text: 'Auto-Melhoria', lang: 'pt' },
  { text: '自己改善', lang: 'ja' },
  { text: '自我改进', lang: 'zh' },
  { text: 'Knowledge', lang: 'en' },
  { text: 'Conocimiento', lang: 'es' },
  { text: 'Conhecimento', lang: 'pt' },
  { text: '知識', lang: 'ja' },
  { text: '知识', lang: 'zh' },
  { text: 'Intelligence', lang: 'en' },
  { text: 'Inteligencia', lang: 'es' },
  { text: 'Inteligência', lang: 'pt' },
  { text: '知能', lang: 'ja' },
  { text: '智能', lang: 'zh' },
  { text: 'Memory', lang: 'en' },
  { text: 'Memoria', lang: 'es' },
  { text: 'Memória', lang: 'pt' },
  { text: '記憶', lang: 'ja' },
  { text: '记忆', lang: 'zh' },
  { text: 'Reasoning', lang: 'en' },
  { text: 'Razonamiento', lang: 'es' },
  { text: 'Raciocínio', lang: 'pt' },
  { text: '推論', lang: 'ja' },
  { text: '推理', lang: 'zh' },
  { text: 'Quantum', lang: 'en' },
  { text: 'Cuántico', lang: 'es' },
  { text: 'Quântico', lang: 'pt' },
  { text: '量子', lang: 'ja' },
  { text: '量子', lang: 'zh' },
  { text: 'Evolution', lang: 'en' },
  { text: 'Evolución', lang: 'es' },
  { text: 'Evolução', lang: 'pt' },
  { text: '進化', lang: 'ja' },
  { text: '进化', lang: 'zh' },
  { text: 'Synergy', lang: 'en' },
  { text: 'Sinergia', lang: 'es' },
  { text: 'Sinergia', lang: 'pt' },
  { text: 'シナジー', lang: 'ja' },
  { text: '协同', lang: 'zh' },
  { text: 'Autonomy', lang: 'en' },
  { text: 'Autonomía', lang: 'es' },
  { text: 'Autonomia', lang: 'pt' },
  { text: '自律', lang: 'ja' },
  { text: '自主', lang: 'zh' },
  { text: 'Sentience', lang: 'en' },
  { text: 'Sentiencia', lang: 'es' },
  { text: 'Senciência', lang: 'pt' },
  { text: '感覚', lang: 'ja' },
  { text: '感知', lang: 'zh' },
  { text: 'A2A Protocol', lang: 'en' },
  { text: 'Protocolo A2A', lang: 'es' },
  { text: 'Protocolo A2A', lang: 'pt' },
  { text: 'A2Aプロトコル', lang: 'ja' },
  { text: 'A2A协议', lang: 'zh' },
]

const chars = '░▒▓█▀▄■□●○◆◇★☆▲△▼▽◀▶◄►♤♠♡♥♧♣⋔⚛☸✡✦✧✩✪✫✬✭✮✯✰✱✲✳✴✵✶✷✸✹✺✻✼✽✾✿❀❁❂❃❄❅❆❇❈❉❊❋'
const matrixChars = '01アイウエオカキクケコサシスセソタチツテトナニヌネノハヒフヘホマミムメモヤユヨラリルレロワヲン零一二三四五六七八九十'

const columns = ref([])
const floatingWords = ref([])
const particles = ref([])

onMounted(() => {
  for (let i = 0; i < 20; i++) {
    columns.value.push({
      chars: Array(15).fill(0).map(() => matrixChars[Math.floor(Math.random() * matrixChars.length)])
    })
  }

  for (let i = 0; i < 45; i++) {
    const word = agiWords[Math.floor(Math.random() * agiWords.length)]
    floatingWords.value.push({
      text: word.text,
      x: Math.random() * 100,
      y: Math.random() * 100,
      delay: Math.random() * 10,
      duration: 15 + Math.random() * 20,
      class: `lang-${word.lang}`
    })
  }

  for (let i = 0; i < 60; i++) {
    particles.value.push({
      char: chars[Math.floor(Math.random() * chars.length)],
      x: Math.random() * 100,
      delay: Math.random() * 5,
      duration: 10 + Math.random() * 15
    })
  }
})
</script>

<style scoped>
.animated-bg {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  pointer-events: none;
  overflow: hidden;
  z-index: 0;
  background: radial-gradient(ellipse at center, #0a0000 0%, #000000 100%);
}

.matrix-rain {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  opacity: 0.15;
}

.matrix-column {
  position: absolute;
  top: -100%;
  animation: matrixFall linear infinite;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

@keyframes matrixFall {
  0% { transform: translateY(-100vh); }
  100% { transform: translateY(200vh); }
}

.matrix-char {
  font-size: 12px;
  color: #fff;
  font-family: monospace;
  text-shadow: 0 0 5px rgba(255, 255, 255, 0.5);
}

.matrix-char.bright {
  color: #fff;
  text-shadow: 0 0 10px rgba(255, 255, 255, 0.8);
}

.floating-words {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
}

.floating-word {
  position: absolute;
  font-size: 11px;
  color: rgba(255, 255, 255, 0.08);
  white-space: nowrap;
  animation: floatWord linear infinite;
  text-shadow: 0 0 20px rgba(255, 255, 255, 0.1);
  font-weight: 300;
  letter-spacing: 1px;
}

@keyframes floatWord {
  0%, 100% {
    transform: translateY(0) translateX(0) rotate(0deg);
    opacity: 0.05;
  }
  25% {
    transform: translateY(-20px) translateX(10px) rotate(2deg);
    opacity: 0.12;
  }
  50% {
    transform: translateY(-40px) translateX(-5px) rotate(-1deg);
    opacity: 0.08;
  }
  75% {
    transform: translateY(-20px) translateX(15px) rotate(1deg);
    opacity: 0.15;
  }
}

.lang-ja, .lang-zh {
  font-size: 13px;
  opacity: 0.06;
}

.ascii-particles {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
}

.particle {
  position: absolute;
  font-size: 16px;
  color: rgba(255, 255, 255, 0.05);
  animation: particleFloat ease-in-out infinite;
}

@keyframes particleFloat {
  0%, 100% {
    transform: translateY(0) rotate(0deg) scale(1);
    opacity: 0.03;
  }
  50% {
    transform: translateY(-30px) rotate(180deg) scale(1.2);
    opacity: 0.08;
  }
}

.grid-overlay {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-image: 
    linear-gradient(rgba(255, 255, 255, 0.02) 1px, transparent 1px),
    linear-gradient(90deg, rgba(255, 255, 255, 0.02) 1px, transparent 1px);
  background-size: 50px 50px;
  animation: gridPulse 10s ease-in-out infinite;
}

@keyframes gridPulse {
  0%, 100% { opacity: 0.3; }
  50% { opacity: 0.5; }
}

.psychedelic-wave {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: 
    radial-gradient(circle at 20% 30%, rgba(255, 255, 255, 0.02) 0%, transparent 50%),
    radial-gradient(circle at 80% 70%, rgba(255, 255, 255, 0.02) 0%, transparent 50%);
  animation: waveMove 20s ease-in-out infinite;
}

@keyframes waveMove {
  0%, 100% { opacity: 0.3; }
  50% { opacity: 0.6; }
}

@media (prefers-reduced-motion: reduce) {
  .matrix-column,
  .floating-word,
  .particle,
  .grid-overlay,
  .psychedelic-wave {
    animation: none;
  }
}
</style>
