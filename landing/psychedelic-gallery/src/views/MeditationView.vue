<template>
  <div class="meditation-view">
    <header class="page-header">
      <h2 class="page-title">
        <span class="icon">🧘</span>
        Meditation Chamber
      </h2>
      <p class="page-subtitle">Rest in the natural state of mind</p>
    </header>
    
    <!-- Timer Section -->
    <section class="timer-section" v-if="!activeSession">
      <div class="session-selector">
        <h3>Choose Your Practice</h3>
        <div class="sessions-grid">
          <div 
            v-for="session in sessions" 
            :key="session.id"
            class="session-card"
            :class="{ selected: selectedSession?.id === session.id }"
            @click="selectedSession = session"
          >
            <div class="session-icon">{{ getIcon(session.type) }}</div>
            <h4>{{ session.name }}</h4>
            <p class="duration">{{ session.duration }} min</p>
            <p class="description">{{ session.description }}</p>
          </div>
        </div>
      </div>
      
      <button 
        v-if="selectedSession"
        class="start-btn"
        @click="startSession"
      >
        Begin Practice
      </button>
    </section>
    
    <!-- Active Session -->
    <section class="active-session" v-else>
      <div class="meditation-container">
        <div class="breath-circle" :class="{ breathing: isBreathing }">
          <span class="breath-text">{{ breathPhase }}</span>
        </div>
        
        <div class="timer-display">
          <span class="time">{{ formatTime(remainingTime) }}</span>
        </div>
        
        <h3 class="session-name">{{ activeSession.name }}</h3>
        <p class="session-instruction">{{ instruction }}</p>
        
        <div class="controls">
          <button v-if="!isPaused" @click="pauseSession" class="control-btn">Pause</button>
          <button v-else @click="resumeSession" class="control-btn">Resume</button>
          <button @click="endSession" class="control-btn end">End</button>
        </div>
      </div>
    </section>
    
    <!-- Completion Modal -->
    <div class="completion-modal" v-if="showCompletion">
      <div class="modal-content">
        <span class="completion-icon">🪷</span>
        <h3>Practice Complete</h3>
        <p>May all beings benefit from your practice.</p>
        <button @click="closeCompletion" class="close-btn">Close</button>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onUnmounted } from 'vue'
import { useStore } from 'vuex'

const store = useStore()

const sessions = computed(() => store.state.meditationSessions)
const selectedSession = ref(null)
const activeSession = ref(null)
const remainingTime = ref(0)
const isPaused = ref(false)
const isBreathing = ref(false)
const breathPhase = ref('')
const showCompletion = ref(false)
const instruction = ref('')

let timer = null
let breathTimer = null

const getIcon = (type) => {
  const icons = { breath: '🌬️', insight: '👁️', compassion: '💜', 'non-dual': '✨' }
  return icons[type] || '🧘'
}

const formatTime = (seconds) => {
  const mins = Math.floor(seconds / 60)
  const secs = seconds % 60
  return `${mins}:${secs.toString().padStart(2, '0')}`
}

const startSession = () => {
  if (!selectedSession.value) return
  
  activeSession.value = selectedSession.value
  remainingTime.value = selectedSession.value.duration * 60
  isPaused.value = false
  
  startBreathCycle()
  startTimer()
  setInstruction()
}

const startTimer = () => {
  timer = setInterval(() => {
    if (!isPaused.value) {
      remainingTime.value--
      if (remainingTime.value <= 0) {
        completeSession()
      }
    }
  }, 1000)
}

const startBreathCycle = () => {
  isBreathing.value = true
  runBreathCycle()
}

const runBreathCycle = () => {
  // In
  breathPhase.value = 'In'
  setTimeout(() => {
    if (!activeSession.value) return
    // Hold
    breathPhase.value = 'Hold'
    setTimeout(() => {
      if (!activeSession.value) return
      // Out
      breathPhase.value = 'Out'
      setTimeout(() => {
        if (!activeSession.value) return
        // Hold
        breathPhase.value = ''
        setTimeout(() => {
          if (activeSession.value && !isPaused.value) {
            runBreathCycle()
          }
        }, 2000)
      }, 4000)
    }, 4000)
  }, 4000)
}

const setInstruction = () => {
  const instructions = {
    breath: 'Focus on the sensation of breath at the nostrils.',
    insight: 'Observe thoughts and sensations without attachment.',
    compassion: 'Send love and kindness to all beings.',
    'non-dual': 'Rest in awareness itself, without object.'
  }
  instruction.value = instructions[activeSession.value?.type] || 'Rest in the present moment.'
}

const pauseSession = () => {
  isPaused.value = true
}

const resumeSession = () => {
  isPaused.value = false
  runBreathCycle()
}

const endSession = () => {
  clearInterval(timer)
  activeSession.value = null
  isBreathing.value = false
  selectedSession.value = null
}

const completeSession = () => {
  clearInterval(timer)
  activeSession.value = null
  isBreathing.value = false
  showCompletion.value = true
}

const closeCompletion = () => {
  showCompletion.value = false
  selectedSession.value = null
}

onUnmounted(() => {
  clearInterval(timer)
})
</script>

<style lang="scss" scoped>
.meditation-view {
  padding: 2rem;
  max-width: 1000px;
  margin: 0 auto;
  min-height: 80vh;
}

.page-header {
  text-align: center;
  margin-bottom: 3rem;
  
  .page-title {
    font-family: 'Orbitron', sans-serif;
    font-size: 2.5rem;
    color: #00ffff;
  }
}

.sessions-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 1.5rem;
  margin-bottom: 2rem;
}

.session-card {
  background: rgba(0, 0, 0, 0.5);
  border: 1px solid rgba(0, 255, 255, 0.2);
  border-radius: 15px;
  padding: 1.5rem;
  text-align: center;
  cursor: pointer;
  transition: all 0.3s ease;
  
  &:hover, &.selected {
    border-color: #ff00ff;
    background: rgba(255, 0, 255, 0.1);
    transform: scale(1.02);
  }
  
  .session-icon {
    font-size: 2.5rem;
    margin-bottom: 0.5rem;
  }
  
  h4 {
    color: #fff;
    margin-bottom: 0.3rem;
  }
  
  .duration {
    color: #00ffff;
    font-size: 0.9rem;
  }
  
  .description {
    color: rgba(255, 255, 255, 0.5);
    font-size: 0.8rem;
    margin-top: 0.5rem;
  }
}

.start-btn {
  display: block;
  margin: 0 auto;
  padding: 1rem 3rem;
  font-family: 'Orbitron', sans-serif;
  font-size: 1.2rem;
  background: linear-gradient(90deg, #ff00ff, #00ffff);
  border: none;
  border-radius: 30px;
  color: #000;
  cursor: pointer;
  transition: all 0.3s ease;
  
  &:hover {
    transform: scale(1.05);
    box-shadow: 0 0 30px rgba(255, 0, 255, 0.5);
  }
}

.active-session {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 60vh;
}

.meditation-container {
  text-align: center;
}

.breath-circle {
  width: 200px;
  height: 200px;
  border-radius: 50%;
  border: 3px solid rgba(0, 255, 255, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  margin: 0 auto 2rem;
  transition: all 4s ease;
  
  &.breathing {
    animation: pulse 12s infinite ease-in-out;
  }
  
  .breath-text {
    font-size: 1.5rem;
    color: #00ffff;
    font-family: 'Orbitron', sans-serif;
  }
}

@keyframes pulse {
  0%, 100% { transform: scale(1); border-color: rgba(0, 255, 255, 0.5); }
  33% { transform: scale(1.3); border-color: rgba(255, 0, 255, 0.8); }
  66% { transform: scale(1); border-color: rgba(138, 43, 226, 0.5); }
}

.timer-display {
  margin-bottom: 2rem;
  
  .time {
    font-family: 'Orbitron', sans-serif;
    font-size: 4rem;
    color: #fff;
  }
}

.session-name {
  color: #ff00ff;
  margin-bottom: 0.5rem;
}

.session-instruction {
  color: rgba(255, 255, 255, 0.7);
  max-width: 400px;
  margin: 0 auto 2rem;
}

.controls {
  display: flex;
  gap: 1rem;
  justify-content: center;
  
  .control-btn {
    padding: 0.8rem 2rem;
    background: transparent;
    border: 1px solid #00ffff;
    color: #00ffff;
    border-radius: 20px;
    cursor: pointer;
    transition: all 0.3s ease;
    
    &:hover {
      background: rgba(0, 255, 255, 0.1);
    }
    
    &.end {
      border-color: #ff00ff;
      color: #ff00ff;
    }
  }
}

.completion-modal {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.9);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
  
  .modal-content {
    text-align: center;
    padding: 3rem;
    
    .completion-icon {
      font-size: 4rem;
      display: block;
      margin-bottom: 1rem;
    }
    
    h3 {
      font-size: 2rem;
      color: #00ffff;
      margin-bottom: 1rem;
    }
    
    p {
      color: rgba(255, 255, 255, 0.7);
      margin-bottom: 2rem;
    }
    
    .close-btn {
      padding: 0.8rem 2rem;
      background: linear-gradient(90deg, #ff00ff, #00ffff);
      border: none;
      border-radius: 20px;
      color: #000;
      cursor: pointer;
    }
  }
}
</style>
