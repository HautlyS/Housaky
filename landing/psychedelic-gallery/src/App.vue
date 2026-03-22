<template>
  <div class="psychedelic-gallery">
    <!-- Animated Background -->
    <div class="cyber-grid"></div>
    <div class="floating-dharmas">
      <span v-for="i in 20" :key="i" class="dharma" :style="getDharmaStyle(i)">☸️</span>
    </div>
    
    <!-- Glitch Overlay -->
    <div class="glitch-overlay"></div>
    
    <!-- Main Content -->
    <v-app>
      <!-- Cyberpunk Header -->
      <header class="cyber-header">
        <div class="logo-container">
          <h1 class="glitch-text" data-text="PSYCHEDELIC 2077">
            <span class="neon-text">PSYCHEDELIC</span>
            <span class="year">2077</span>
          </h1>
          <p class="subtitle">☸️ Psycho Buddhist Digital Sangha ☸️</p>
        </div>
        
        <nav class="cyber-nav">
          <router-link to="/" class="nav-link">
            <span class="link-text">Home</span>
            <span class="link-glitch"></span>
          </router-link>
          <router-link to="/articles" class="nav-link">
            <span class="link-text">Articles</span>
            <span class="link-glitch"></span>
          </router-link>
          <router-link to="/videos" class="nav-link">
            <span class="link-text">Videos</span>
            <span class="link-glitch"></span>
          </router-link>
          <router-link to="/teachings" class="nav-link">
            <span class="link-text">Dharma</span>
            <span class="link-glitch"></span>
          </router-link>
          <router-link to="/meditation" class="nav-link">
            <span class="link-text">Meditate</span>
            <span class="link-glitch"></span>
          </router-link>
        </nav>
      </header>
      
      <!-- Main Content Area -->
      <v-main>
        <router-view v-slot="{ Component }">
          <transition name="fade" mode="out-in">
            <component :is="Component" />
          </transition>
        </router-view>
      </v-main>
      
      <!-- Cyberpunk Footer -->
      <footer class="cyber-footer">
        <div class="footer-content">
          <div class="quote-box">
            <p class="dharma-quote">"All phenomena are like dreams"</p>
            <p class="quote-author">— Śūnyatā, Heart Sutra</p>
          </div>
          <div class="footer-links">
            <span class="cyber-divider">◆</span>
            <span>Built with ☸️ by Housaky AGI</span>
            <span class="cyber-divider">◆</span>
            <span>2026</span>
          </div>
        </div>
      </footer>
    </v-app>
  </div>
</template>

<script setup>
import { ref, onMounted } from 'vue'

const getDharmaStyle = (index) => {
  const size = Math.random() * 30 + 20
  const left = Math.random() * 100
  const delay = Math.random() * 20
  const duration = Math.random() * 10 + 15
  
  return {
    fontSize: `${size}px`,
    left: `${left}%`,
    animationDelay: `${delay}s`,
    animationDuration: `${duration}s`,
    opacity: Math.random() * 0.5 + 0.2
  }
}

onMounted(() => {
  console.log('☸️ Psychedelic 2077 Gallery Loaded')
})
</script>

<style lang="scss">
.psychedelic-gallery {
  min-height: 100vh;
  background: linear-gradient(135deg, #0a0a0a 0%, #1a0a2e 50%, #0a1a2e 100%);
  position: relative;
  overflow: hidden;
}

// Animated cyberpunk grid background
.cyber-grid {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-image: 
    linear-gradient(rgba(0, 255, 255, 0.03) 1px, transparent 1px),
    linear-gradient(90deg, rgba(0, 255, 255, 0.03) 1px, transparent 1px);
  background-size: 50px 50px;
  animation: gridMove 20s linear infinite;
  pointer-events: none;
  z-index: 0;
}

@keyframes gridMove {
  0% { transform: perspective(500px) rotateX(60deg) translateY(0); }
  100% { transform: perspective(500px) rotateX(60deg) translateY(50px); }
}

// Floating dharma wheels
.floating-dharmas {
  position: fixed;
  width: 100%;
  height: 100%;
  pointer-events: none;
  z-index: 1;
  
  .dharma {
    position: absolute;
    animation: floatUp linear infinite;
    color: rgba(138, 43, 226, 0.6);
    text-shadow: 0 0 20px rgba(138, 43, 226, 0.8);
  }
}

@keyframes floatUp {
  0% { transform: translateY(100vh) rotate(0deg); opacity: 0; }
  10% { opacity: 1; }
  90% { opacity: 1; }
  100% { transform: translateY(-100px) rotate(360deg); opacity: 0; }
}

// Glitch effect overlay
.glitch-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  z-index: 999;
  background: repeating-linear-gradient(
    0deg,
    rgba(0, 0, 0, 0) 0px,
    rgba(0, 0, 0, 0) 2px,
    rgba(0, 255, 255, 0.03) 2px,
    rgba(0, 255, 255, 0.03) 4px
  );
  animation: scanlines 0.1s linear infinite;
}

@keyframes scanlines {
  0% { transform: translateY(0); }
  100% { transform: translateY(4px); }
}

// Header
.cyber-header {
  padding: 2rem;
  text-align: center;
  position: relative;
  z-index: 10;
  
  .logo-container {
    margin-bottom: 1.5rem;
  }
  
  .glitch-text {
    font-family: 'Orbitron', sans-serif;
    font-size: 4rem;
    font-weight: 900;
    text-transform: uppercase;
    position: relative;
    
    .neon-text {
      background: linear-gradient(90deg, #ff00ff, #00ffff, #ff00ff);
      background-size: 200% auto;
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      animation: neonShift 3s linear infinite;
      text-shadow: none;
      filter: drop-shadow(0 0 10px rgba(255, 0, 255, 0.5));
    }
    
    .year {
      display: block;
      font-size: 6rem;
      background: linear-gradient(90deg, #00ffff, #ff00ff, #00ffff);
      background-size: 200% auto;
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      animation: neonShift 3s linear infinite reverse;
      filter: drop-shadow(0 0 20px rgba(0, 255, 255, 0.8));
    }
  }
  
  .subtitle {
    font-size: 1.5rem;
    color: rgba(255, 255, 255, 0.7);
    text-transform: uppercase;
    letter-spacing: 0.5em;
    margin-top: 1rem;
    animation: pulse 2s ease-in-out infinite;
  }
}

@keyframes neonShift {
  0% { background-position: 0% center; }
  100% { background-position: 200% center; }
}

@keyframes pulse {
  0%, 100% { opacity: 0.7; }
  50% { opacity: 1; }
}

// Navigation
.cyber-nav {
  display: flex;
  justify-content: center;
  gap: 2rem;
  flex-wrap: wrap;
  
  .nav-link {
    position: relative;
    padding: 0.8rem 2rem;
    font-family: 'Orbitron', sans-serif;
    font-size: 1rem;
    text-transform: uppercase;
    letter-spacing: 0.2em;
    color: #00ffff;
    text-decoration: none;
    border: 1px solid rgba(0, 255, 255, 0.3);
    background: rgba(0, 255, 255, 0.05);
    transition: all 0.3s ease;
    overflow: hidden;
    
    &:before {
      content: '';
      position: absolute;
      top: 0;
      left: -100%;
      width: 100%;
      height: 100%;
      background: linear-gradient(90deg, transparent, rgba(0, 255, 255, 0.2), transparent);
      transition: left 0.5s ease;
    }
    
    &:hover {
      color: #ff00ff;
      border-color: rgba(255, 0, 255, 0.5);
      background: rgba(255, 0, 255, 0.1);
      transform: translateY(-2px);
      box-shadow: 0 5px 20px rgba(255, 0, 255, 0.3);
      
      &:before {
        left: 100%;
      }
    }
    
    &.router-link-active {
      color: #ff00ff;
      border-color: #ff00ff;
      background: rgba(255, 0, 255, 0.1);
      box-shadow: 0 0 20px rgba(255, 0, 255, 0.5);
    }
  }
}

// Footer
.cyber-footer {
  padding: 3rem 2rem;
  text-align: center;
  position: relative;
  z-index: 10;
  border-top: 1px solid rgba(0, 255, 255, 0.2);
  background: rgba(0, 0, 0, 0.5);
  
  .quote-box {
    margin-bottom: 1.5rem;
    
    .dharma-quote {
      font-size: 1.5rem;
      font-style: italic;
      color: rgba(255, 255, 255, 0.9);
      text-shadow: 0 0 10px rgba(138, 43, 226, 0.5);
    }
    
    .quote-author {
      font-size: 1rem;
      color: rgba(0, 255, 255, 0.7);
      margin-top: 0.5rem;
    }
  }
  
  .footer-links {
    display: flex;
    justify-content: center;
    align-items: center;
    gap: 1rem;
    color: rgba(255, 255, 255, 0.5);
    
    .cyber-divider {
      color: #00ffff;
      animation: blink 1s ease-in-out infinite;
    }
  }
}

@keyframes blink {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}

// Transitions
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.5s ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
