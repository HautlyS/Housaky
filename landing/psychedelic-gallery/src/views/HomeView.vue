<template>
  <div class="home-view">
    <!-- Hero Section -->
    <section class="hero">
      <div class="hero-content">
        <div class="lotus-animation">
          <span class="lotus">🪷</span>
        </div>
        <h2 class="hero-title">
          <span class="glitch" data-text="Awaken in the">Awaken in the</span>
          <span class="highlight">Digital Void</span>
        </h2>
        <p class="hero-subtitle">
          A 2077 cyberpunk exploration of Buddhist philosophy, 
          where ancient wisdom meets futuristic consciousness
        </p>
        <div class="cta-buttons">
          <router-link to="/articles" class="cyber-btn primary">
            <span>Explore Articles</span>
          </router-link>
          <router-link to="/meditation" class="cyber-btn secondary">
            <span>Begin Practice</span>
          </router-link>
        </div>
      </div>
    </section>
    
    <!-- Featured Articles -->
    <section class="featured-section">
      <h3 class="section-title">
        <span class="icon">📖</span>
        Featured Teachings
      </h3>
      <div class="articles-grid">
        <article 
          v-for="article in featuredArticles" 
          :key="article.id"
          class="article-card"
          @click="goToArticle(article.id)"
        >
          <div class="card-image">
            <img :src="article.image" :alt="article.title" />
            <div class="card-overlay"></div>
          </div>
          <div class="card-content">
            <span class="card-category">{{ article.category }}</span>
            <h4 class="card-title">{{ article.title }}</h4>
            <p class="card-subtitle">{{ article.subtitle }}</p>
          </div>
        </article>
      </div>
    </section>
    
    <!-- Dharma Quote -->
    <section class="quote-section">
      <div class="quote-container">
        <blockquote class="dharma-quote">
          "{{ randomQuote.text }}"
        </blockquote>
        <cite class="quote-source">— {{ randomQuote.source }}</cite>
      </div>
    </section>
    
    <!-- Meditation Preview -->
    <section class="meditation-preview">
      <h3 class="section-title">
        <span class="icon">🧘</span>
        Quick Practice
      </h3>
      <div class="meditation-cards">
        <div 
          v-for="session in quickSessions" 
          :key="session.id"
          class="meditation-card"
          @click="startMeditation(session)"
        >
          <div class="meditation-icon">
            {{ getMeditationIcon(session.type) }}
          </div>
          <div class="meditation-info">
            <h4>{{ session.name }}</h4>
            <p>{{ session.duration }} min</p>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup>
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import { useStore } from 'vuex'

const router = useRouter()
const store = useStore()

const featuredArticles = computed(() => store.state.articles.slice(0, 3))
const randomQuote = computed(() => store.getters.getRandomQuote)
const quickSessions = computed(() => store.state.meditationSessions.slice(0, 3))

const goToArticle = (id) => {
  router.push(`/articles/${id}`)
}

const startMeditation = (session) => {
  router.push('/meditation')
}

const getMeditationIcon = (type) => {
  const icons = {
    breath: '🌬️',
    insight: '👁️',
    compassion: '💜',
    'non-dual': '✨'
  }
  return icons[type] || '🧘'
}
</script>

<style lang="scss" scoped>
.home-view {
  padding: 2rem;
  max-width: 1400px;
  margin: 0 auto;
}

// Hero Section
.hero {
  text-align: center;
  padding: 4rem 2rem;
  position: relative;
  
  .lotus-animation {
    font-size: 5rem;
    margin-bottom: 2rem;
    animation: float 3s ease-in-out infinite;
    
    .lotus {
      filter: drop-shadow(0 0 30px rgba(138, 43, 226, 0.8));
    }
  }
  
  .hero-title {
    font-family: 'Orbitron', sans-serif;
    font-size: 3rem;
    margin-bottom: 1rem;
    
    .glitch {
      position: relative;
      display: block;
      color: rgba(255, 255, 255, 0.9);
    }
    
    .highlight {
      display: block;
      font-size: 4rem;
      background: linear-gradient(90deg, #ff00ff, #00ffff);
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      animation: glow 2s ease-in-out infinite alternate;
    }
  }
  
  .hero-subtitle {
    font-size: 1.2rem;
    color: rgba(255, 255, 255, 0.7);
    max-width: 600px;
    margin: 0 auto 2rem;
    line-height: 1.8;
  }
}

@keyframes float {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-20px); }
}

@keyframes glow {
  from { filter: drop-shadow(0 0 10px rgba(255, 0, 255, 0.5)); }
  to { filter: drop-shadow(0 0 30px rgba(0, 255, 255, 0.8)); }
}

// CTA Buttons
.cta-buttons {
  display: flex;
  gap: 1.5rem;
  justify-content: center;
  flex-wrap: wrap;
  
  .cyber-btn {
    padding: 1rem 2.5rem;
    font-family: 'Orbitron', sans-serif;
    font-size: 1rem;
    text-transform: uppercase;
    letter-spacing: 0.2em;
    text-decoration: none;
    border: 2px solid;
    position: relative;
    overflow: hidden;
    transition: all 0.3s ease;
    
    &.primary {
      color: #00ffff;
      border-color: #00ffff;
      background: rgba(0, 255, 255, 0.1);
      
      &:hover {
        background: rgba(0, 255, 255, 0.2);
        box-shadow: 0 0 30px rgba(0, 255, 255, 0.5);
      }
    }
    
    &.secondary {
      color: #ff00ff;
      border-color: #ff00ff;
      background: rgba(255, 0, 255, 0.1);
      
      &:hover {
        background: rgba(255, 0, 255, 0.2);
        box-shadow: 0 0 30px rgba(255, 0, 255, 0.5);
      }
    }
  }
}

// Section Titles
.section-title {
  font-family: 'Orbitron', sans-serif;
  font-size: 1.5rem;
  color: #00ffff;
  margin-bottom: 2rem;
  display: flex;
  align-items: center;
  gap: 1rem;
  
  .icon {
    font-size: 1.5rem;
  }
}

// Articles Grid
.articles-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 2rem;
  margin-bottom: 4rem;
}

.article-card {
  background: rgba(0, 0, 0, 0.5);
  border: 1px solid rgba(0, 255, 255, 0.2);
  border-radius: 10px;
  overflow: hidden;
  cursor: pointer;
  transition: all 0.3s ease;
  
  &:hover {
    transform: translateY(-5px);
    border-color: rgba(255, 0, 255, 0.5);
    box-shadow: 0 10px 30px rgba(255, 0, 255, 0.2);
    
    .card-image img {
      transform: scale(1.1);
    }
  }
  
  .card-image {
    height: 200px;
    overflow: hidden;
    position: relative;
    
    img {
      width: 100%;
      height: 100%;
      object-fit: cover;
      transition: transform 0.5s ease;
    }
    
    .card-overlay {
      position: absolute;
      inset: 0;
      background: linear-gradient(to top, rgba(0, 0, 0, 0.8), transparent);
    }
  }
  
  .card-content {
    padding: 1.5rem;
    
    .card-category {
      font-size: 0.8rem;
      color: #00ffff;
      text-transform: uppercase;
      letter-spacing: 0.1em;
    }
    
    .card-title {
      font-size: 1.2rem;
      margin: 0.5rem 0;
      color: #fff;
    }
    
    .card-subtitle {
      font-size: 0.9rem;
      color: rgba(255, 255, 255, 0.6);
    }
  }
}

// Quote Section
.quote-section {
  background: linear-gradient(135deg, rgba(138, 43, 226, 0.2), rgba(0, 255, 255, 0.1));
  padding: 4rem 2rem;
  margin: 4rem 0;
  border-radius: 20px;
  text-align: center;
  border: 1px solid rgba(138, 43, 226, 0.3);
  
  .dharma-quote {
    font-size: 1.8rem;
    font-style: italic;
    color: rgba(255, 255, 255, 0.9);
    margin-bottom: 1rem;
  }
  
  .quote-source {
    font-size: 1rem;
    color: rgba(0, 255, 255, 0.8);
  }
}

// Meditation Preview
.meditation-cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
  gap: 1.5rem;
}

.meditation-card {
  display: flex;
  align-items: center;
  gap: 1.5rem;
  padding: 1.5rem;
  background: rgba(0, 0, 0, 0.5);
  border: 1px solid rgba(0, 255, 255, 0.2);
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.3s ease;
  
  &:hover {
    background: rgba(0, 255, 255, 0.1);
    border-color: #00ffff;
  }
  
  .meditation-icon {
    font-size: 2.5rem;
  }
  
  .meditation-info {
    h4 {
      font-size: 1rem;
      color: #fff;
      margin-bottom: 0.3rem;
    }
    
    p {
      font-size: 0.9rem;
      color: rgba(255, 255, 255, 0.6);
    }
  }
}
</style>
